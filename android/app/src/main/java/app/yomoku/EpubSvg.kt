package app.yomoku

import android.graphics.Bitmap
import android.graphics.Canvas
import android.util.Xml
import androidx.compose.ui.geometry.Size
import androidx.core.graphics.createBitmap
import com.caverock.androidsvg.SVG
import kotlin.math.ceil
import org.xmlpull.v1.XmlPullParser

internal const val EPUB_SVG_BYTES = 2 * 1024 * 1024
private const val SVG_NODES = 4096
private const val SVG_DEPTH = 64
private const val SVG_EXPANSIONS = 32768
// AndroidSVG also accepts an IRI missing its closing parenthesis.
private val svgUrls = Regex("url\\s*\\(([^)]*)(?:\\)|$)", RegexOption.IGNORE_CASE)

/** Preflight before AndroidSVG builds a tree or follows recursive local references. */
private fun boundedSvg(bytes: ByteArray): Boolean {
    data class Node(
        val edges: MutableList<Int> = mutableListOf(),
        val references: MutableList<String> = mutableListOf(),
    )
    val nodes = mutableListOf<Node>()
    val parents = ArrayDeque<Int>()
    val ids = mutableMapOf<String, Int>()
    val styles = StringBuilder()
    var styleDepth = 0
    val parser = Xml.newPullParser()
    parser.setFeature(XmlPullParser.FEATURE_PROCESS_NAMESPACES, true)
    parser.setFeature(XmlPullParser.FEATURE_PROCESS_DOCDECL, false)
    bytes.inputStream().use { input ->
        parser.setInput(input, null)
        var token = parser.eventType
        while (token != XmlPullParser.END_DOCUMENT) {
            when (token) {
                XmlPullParser.DOCDECL,
                XmlPullParser.PROCESSING_INSTRUCTION -> return false
                XmlPullParser.START_TAG -> {
                    if (styleDepth > 0) return false
                    if (nodes.size >= SVG_NODES || parents.size >= SVG_DEPTH) return false
                    if (nodes.isEmpty() && parser.name != "svg") return false
                    if (parser.namespace !in listOf("", "http://www.w3.org/2000/svg")) return false
                    // AndroidSVG decodes data-URI rasters without bounds. Compound images need
                    // a separate bounded resource pipeline; never silently render them blank.
                    // Patterns/masks/markers can recurse via inherited paint outside this graph.
                    if (
                        parser.name in
                            setOf(
                                "image",
                                "script",
                                "foreignObject",
                                "filter",
                                "pattern",
                                "mask",
                                "marker",
                                "set",
                            ) || parser.name.startsWith("animate")
                    )
                        return false
                    val index = nodes.size
                    val node = Node()
                    nodes.add(node)
                    parents.lastOrNull()?.let { nodes[it].edges.add(index) }
                    parents.addLast(index)
                    if (parser.name == "style") styleDepth = parents.size
                    for (attribute in 0 until parser.attributeCount) {
                        val name = parser.getAttributeName(attribute)
                        val value = parser.getAttributeValue(attribute).trim()
                        if (value.contains('\\')) return false // No escaped CSS references.
                        if (name == "id" && ids.put(value, index) != null) return false
                        if (name == "href" && parser.name != "a") {
                            if (!value.startsWith('#') || value.length < 2) return false
                            node.references.add(value.substring(1))
                        }
                        for (url in svgUrls.findAll(value)) {
                            val target = url.groupValues[1].trim().trim('\'', '"')
                            if (!target.startsWith('#') || target.length < 2) return false
                            node.references.add(target.substring(1))
                        }
                    }
                }
                XmlPullParser.END_TAG -> {
                    if (styleDepth == parents.size) styleDepth = 0
                    parents.removeLast()
                }
                XmlPullParser.TEXT,
                XmlPullParser.CDSECT,
                XmlPullParser.ENTITY_REF -> {
                    if (styleDepth > 0) styles.append(parser.text ?: return false)
                }
            }
            token = parser.nextToken()
        }
    }
    // Color/class CSS is safe. Resource-valued CSS must be inline so its owning node is
    // known to the reference graph. No imported stylesheets or escaped resource names.
    if (styles.contains('\\') || styles.contains('@') || svgUrls.containsMatchIn(styles))
        return false
    if (nodes.isEmpty()) return false
    for (node in nodes) for (reference in node.references) {
        node.edges.add(ids[reference] ?: return false)
    }
    val visiting = BooleanArray(nodes.size)
    val costs = IntArray(nodes.size)
    val depths = IntArray(nodes.size)
    fun visit(index: Int, depth: Int): Boolean {
        if (depth > SVG_DEPTH || visiting[index]) return false
        if (costs[index] != 0) return depth + depths[index] - 1 <= SVG_DEPTH
        visiting[index] = true
        var cost = 1
        var height = 1
        for (edge in nodes[index].edges) {
            if (!visit(edge, depth + 1)) return false
            cost += costs[edge]
            height = maxOf(height, depths[edge] + 1)
            if (cost > SVG_EXPANSIONS) return false
        }
        visiting[index] = false
        costs[index] = cost
        depths[index] = height
        return true
    }
    return visit(0, 1)
}

/** Called only by the serial, off-main image decoder. The output shares its page pixel budget. */
internal fun decodeEpubSvg(bytes: ByteArray, pixelBudget: Int): DecodedEpubImage? {
    if (bytes.isEmpty() || bytes.size > EPUB_SVG_BYTES || pixelBudget < 1) return null
    var bitmap: Bitmap? = null
    return try {
        if (!boundedSvg(bytes)) return null
        SVG.setInternalEntitiesEnabled(false)
        SVG.deregisterExternalFileResolver()
        val svg = bytes.inputStream().use { SVG.getFromInputStream(it) }
        val ratio = svg.documentAspectRatio
        if (!ratio.isFinite() || ratio <= 0) return null
        val viewBox = svg.documentViewBox
        if (
            viewBox != null &&
                (!viewBox.left.isFinite() ||
                    !viewBox.top.isFinite() ||
                    !viewBox.width().isFinite() ||
                    !viewBox.height().isFinite() ||
                    viewBox.width() <= 0 ||
                    viewBox.height() <= 0)
        )
            return null
        var width = svg.documentWidth
        var height = svg.documentHeight
        if (width <= 0 || height <= 0) {
            width = viewBox?.width() ?: return null
            height = viewBox.height()
        }
        if (
            !width.isFinite() ||
                !height.isFinite() ||
                width <= 0 ||
                height <= 0 ||
                width > 32768 ||
                height > 32768 ||
                width.toDouble() * height > 100_000_000
        )
            return null
        val sourceWidth = ceil(width).toInt()
        val sourceHeight = ceil(height).toInt()
        var sample = 1
        fun sampled(edge: Int) = (edge + sample - 1) / sample
        while (
            sampled(sourceWidth) > EPUB_IMAGE_EDGE ||
                sampled(sourceHeight) > EPUB_IMAGE_EDGE ||
                sampled(sourceWidth).toLong() * sampled(sourceHeight) > pixelBudget
        ) sample *= 2
        bitmap =
            createBitmap(
                sampled(sourceWidth),
                sampled(sourceHeight),
                Bitmap.Config.ARGB_8888,
            )
        svg.documentWidth = width
        svg.documentHeight = height
        val canvas = Canvas(bitmap)
        canvas.scale(bitmap.width / width, bitmap.height / height)
        svg.renderToCanvas(canvas)
        DecodedEpubImage(bitmap, sourceWidth, sourceHeight, Size(width, height))
    } catch (_: Exception) {
        bitmap?.recycle()
        null
    } catch (_: StackOverflowError) {
        bitmap?.recycle()
        null
    } catch (_: OutOfMemoryError) {
        bitmap?.recycle()
        null
    }
}
