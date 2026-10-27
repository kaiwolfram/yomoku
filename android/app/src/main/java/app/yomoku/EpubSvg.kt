package app.yomoku

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.util.Base64
import android.util.Xml
import androidx.compose.ui.geometry.Size
import androidx.core.graphics.createBitmap
import com.caverock.androidsvg.SVG
import com.caverock.androidsvg.SVGExternalFileResolver
import java.io.StringWriter
import kotlin.math.ceil
import org.xmlpull.v1.XmlPullParser

internal const val EPUB_SVG_BYTES = 2 * 1024 * 1024
private const val SVG_NODES = 4096
private const val SVG_DEPTH = 64
private const val SVG_EXPANSIONS = 32768
private const val SVG_RASTERS = 64
private const val XLINK_NAMESPACE = "http://www.w3.org/1999/xlink"
private val svgDecodeLock = Any()
private val imageViewportAttributes =
    setOf("x", "y", "width", "height", "preserveAspectRatio", "overflow", "href")
private val svgRasterData = Regex("data:image/(png|jpeg);base64,([A-Za-z0-9+/=\\s]+)")

private data class PreparedSvg(val xml: String, val rasters: List<SvgRasterSource>)

// AndroidSVG also accepts an IRI missing its closing parenthesis.
private val svgUrls = Regex("url\\s*\\(([^)]*)(?:\\)|$)", RegexOption.IGNORE_CASE)

/** Preflight before AndroidSVG builds a tree or follows recursive local references. */
private fun prepareSvg(bytes: ByteArray): PreparedSvg? {
    data class Node(
        val edges: MutableList<Int> = mutableListOf(),
        val references: MutableList<String> = mutableListOf(),
        val weight: Int = 1,
    )
    val nodes = mutableListOf<Node>()
    val parents = ArrayDeque<Int>()
    val ids = mutableMapOf<String, Int>()
    val styles = StringBuilder()
    var styleDepth = 0
    var imageDepth = 0
    val rasters = linkedMapOf<String, Int>()
    val sources = mutableListOf<SvgRasterSource>()
    val output = StringWriter()
    val serializer = Xml.newSerializer().apply { setOutput(output) }
    val parser = Xml.newPullParser()
    parser.setFeature(XmlPullParser.FEATURE_PROCESS_NAMESPACES, true)
    parser.setFeature(XmlPullParser.FEATURE_PROCESS_DOCDECL, false)
    bytes.inputStream().use { input ->
        parser.setInput(input, null)
        var token = parser.eventType
        while (token != XmlPullParser.END_DOCUMENT) {
            when (token) {
                XmlPullParser.DOCDECL,
                XmlPullParser.PROCESSING_INSTRUCTION -> return null
                XmlPullParser.START_TAG -> {
                    if (styleDepth > 0 || imageDepth > 0) return null
                    if (nodes.size >= SVG_NODES || parents.size >= SVG_DEPTH) return null
                    if (nodes.isEmpty() && parser.name != "svg") return null
                    if (parser.namespace !in listOf("", "http://www.w3.org/2000/svg")) return null
                    // Patterns/masks/markers can recurse via inherited paint outside this graph.
                    if (
                        parser.name in
                            setOf(
                                "script",
                                "foreignObject",
                                "filter",
                                "pattern",
                                "mask",
                                "marker",
                                "set",
                            ) || parser.name.startsWith("animate")
                    )
                        return null
                    for (namespace in
                        parser.getNamespaceCount(parser.depth - 1) until
                            parser.getNamespaceCount(parser.depth)) {
                        serializer.setPrefix(
                            parser.getNamespacePrefix(namespace) ?: "",
                            parser.getNamespaceUri(namespace),
                        )
                    }
                    val image = parser.name == "image"
                    if (image) imageDepth = parser.depth
                    if (image) {
                        val names =
                            (0 until parser.attributeCount).map { parser.getAttributeName(it) }
                        if ("width" !in names || "height" !in names) return null
                        val hrefs =
                            (0 until parser.attributeCount).filter {
                                parser.getAttributeName(it) == "href"
                            }
                        if (hrefs.size != 1) return null
                        val attribute = hrefs.single()
                        if (parser.getAttributeNamespace(attribute) !in listOf("", XLINK_NAMESPACE))
                            return null
                        val uri = parser.getAttributeValue(attribute).trim()
                        val key =
                            rasters.getOrPut(uri) {
                                if (sources.size >= SVG_RASTERS) return null
                                sources.add(SvgRasterSource.read(uri) ?: return null)
                                sources.lastIndex
                            }
                        // An SVG viewport preserves the ORIGINAL raster ratio even when its
                        // bounded bitmap rounds to e.g. 1x1. Upstream uses bitmap dimensions.
                        serializer.startTag(parser.namespace, "g")
                        for (attr in 0 until parser.attributeCount) {
                            if (parser.getAttributeName(attr) !in imageViewportAttributes) {
                                serializer.attribute(
                                    parser.getAttributeNamespace(attr),
                                    parser.getAttributeName(attr),
                                    parser.getAttributeValue(attr),
                                )
                            }
                        }
                        serializer.startTag(parser.namespace, "svg")
                        for (attr in 0 until parser.attributeCount) {
                            val name = parser.getAttributeName(attr)
                            if (name in imageViewportAttributes && name != "href") {
                                serializer.attribute("", name, parser.getAttributeValue(attr))
                            }
                        }
                        val source = sources[key]
                        serializer.attribute("", "viewBox", "0 0 ${source.width} ${source.height}")
                        serializer.startTag(parser.namespace, "image")
                        serializer.attribute("", "href", "yomoku-svg-image:$key")
                        serializer.attribute("", "width", source.width.toString())
                        serializer.attribute("", "height", source.height.toString())
                        serializer.attribute("", "preserveAspectRatio", "none")
                    } else serializer.startTag(parser.namespace, parser.name)
                    val index = nodes.size
                    val node = Node(weight = if (image) 3 else 1)
                    nodes.add(node)
                    parents.lastOrNull()?.let { nodes[it].edges.add(index) }
                    parents.addLast(index)
                    if (parser.name == "style") styleDepth = parents.size
                    for (attribute in 0 until parser.attributeCount) {
                        val name = parser.getAttributeName(attribute)
                        val original = parser.getAttributeValue(attribute)
                        val value = original.trim()
                        val namespace = parser.getAttributeNamespace(attribute)
                        if (name == "href" && namespace !in listOf("", XLINK_NAMESPACE)) return null
                        if (image && name == "href") continue
                        if (!image) serializer.attribute(namespace, name, original)
                        if (value.contains('\\')) return null // No escaped CSS references.
                        if (name == "id" && ids.put(value, index) != null) return null
                        if (name == "href" && parser.name != "a") {
                            if (!value.startsWith('#') || value.length < 2) return null
                            node.references.add(value.substring(1))
                        }
                        for (url in svgUrls.findAll(value)) {
                            val target = url.groupValues[1].trim().trim('\'', '"')
                            if (!target.startsWith('#') || target.length < 2) return null
                            node.references.add(target.substring(1))
                        }
                    }
                }
                XmlPullParser.END_TAG -> {
                    if (styleDepth == parents.size) styleDepth = 0
                    if (imageDepth == parser.depth) imageDepth = 0
                    parents.removeLast()
                    serializer.endTag(parser.namespace, parser.name)
                    if (parser.name == "image") {
                        serializer.endTag(parser.namespace, "svg")
                        serializer.endTag(parser.namespace, "g")
                    }
                }
                XmlPullParser.TEXT,
                XmlPullParser.CDSECT,
                XmlPullParser.ENTITY_REF -> {
                    val value = parser.text ?: return null
                    if (styleDepth > 0) styles.append(value)
                    serializer.text(value)
                }
            }
            token = parser.nextToken()
        }
    }
    // Color/class CSS is safe. Resource-valued CSS must be inline so its owning node is
    // known to the reference graph. No imported stylesheets or escaped resource names.
    if (styles.contains('\\') || styles.contains('@') || svgUrls.containsMatchIn(styles))
        return null
    // Rewriting image viewports changes selector ancestry/type matching. Keep stylesheet-
    // styled compound SVGs on fallback until selectors can be applied before rewriting.
    if (sources.isNotEmpty() && styles.isNotBlank()) return null
    if (nodes.isEmpty()) return null
    for (node in nodes) for (reference in node.references) {
        node.edges.add(ids[reference] ?: return null)
    }
    val visiting = BooleanArray(nodes.size)
    val costs = IntArray(nodes.size)
    val depths = IntArray(nodes.size)
    fun visit(index: Int, depth: Int): Boolean {
        if (depth > SVG_DEPTH || visiting[index]) return false
        if (costs[index] != 0) return depth + depths[index] - 1 <= SVG_DEPTH
        visiting[index] = true
        var cost = nodes[index].weight
        var height = nodes[index].weight
        for (edge in nodes[index].edges) {
            if (!visit(edge, depth + 1)) return false
            cost += costs[edge]
            height = maxOf(height, depths[edge] + 1)
            if (cost > SVG_EXPANSIONS) return false
        }
        if (depth + height - 1 > SVG_DEPTH) return false
        visiting[index] = false
        costs[index] = cost
        depths[index] = height
        return true
    }
    if (!visit(0, 1)) return null
    serializer.flush()
    return PreparedSvg(output.toString(), sources)
}

/** Called only by the serial, off-main image decoder. The output shares its page pixel budget. */
internal fun decodeEpubSvg(bytes: ByteArray, pixelBudget: Int): DecodedEpubImage? =
    synchronized(svgDecodeLock) { decodeEpubSvgLocked(bytes, pixelBudget) }

private fun decodeEpubSvgLocked(bytes: ByteArray, pixelBudget: Int): DecodedEpubImage? {
    if (bytes.isEmpty() || bytes.size > EPUB_SVG_BYTES || pixelBudget < 1) return null
    var bitmap: Bitmap? = null
    var rasters: SvgRasterResources? = null
    return try {
        val prepared = prepareSvg(bytes) ?: return null
        SVG.setInternalEntitiesEnabled(false)
        SVG.deregisterExternalFileResolver()
        val svg = SVG.getFromString(prepared.xml)
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
        rasters = SvgRasterResources.decode(prepared.rasters, pixelBudget) ?: return null
        SVG.registerExternalFileResolver(rasters)
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
    } finally {
        SVG.deregisterExternalFileResolver()
        rasters?.close()
    }
}

/** Bounds-only inspection is safe before graph validation; it never allocates source pixels. */
internal data class SvgRasterSource(
    val bytes: ByteArray,
    val media: String,
    val width: Int,
    val height: Int,
) {
    companion object {
        fun read(uri: String): SvgRasterSource? {
            if (uri.length > EPUB_SVG_BYTES) return null
            val match = svgRasterData.matchEntire(uri) ?: return null
            val bytes =
                try {
                    Base64.decode(match.groupValues[2], Base64.DEFAULT)
                } catch (_: IllegalArgumentException) {
                    return null
                }
            val media = "image/${match.groupValues[1]}"
            val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
            BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
            val width = bounds.outWidth
            val height = bounds.outHeight
            if (
                bounds.outMimeType != media ||
                    width !in 1..32768 ||
                    height !in 1..32768 ||
                    width.toLong() * height > 100_000_000
            )
                return null
            return SvgRasterSource(bytes, media, width, height)
        }
    }
}

/** Private keys only. Temporary pixels total <= the output budget, in addition to output pixels. */
internal class SvgRasterResources private constructor(internal val images: List<Bitmap>) :
    SVGExternalFileResolver(), AutoCloseable {
    override fun resolveImage(filename: String): Bitmap? {
        val index = filename.removePrefix("yomoku-svg-image:").toIntOrNull() ?: return null
        return images.getOrNull(index).takeIf { filename == "yomoku-svg-image:$index" }
    }

    override fun isFormatSupported(mimeType: String) = mimeType in listOf("image/png", "image/jpeg")

    override fun close() = images.forEach { it.recycle() }

    companion object {
        fun decode(sources: List<SvgRasterSource>, pixelBudget: Int): SvgRasterResources? {
            if (sources.size > SVG_RASTERS || sources.size > pixelBudget) return null
            val images = mutableListOf<Bitmap>()
            var success = false
            try {
                val share = pixelBudget / maxOf(1, sources.size)
                for (source in sources) {
                    val image = decodeEpubImage(source.bytes, share, source.media) ?: return null
                    images.add(image.bitmap)
                }
                success = true
                return SvgRasterResources(images)
            } finally {
                if (!success) images.forEach { it.recycle() }
            }
        }
    }
}
