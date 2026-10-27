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
private const val SVG_IMAGE_RESOURCES = 64
private const val SVG_IMAGE_DEPTH = 8
private const val XLINK_NAMESPACE = "http://www.w3.org/1999/xlink"
private val svgDecodeLock = Any()
private val imageViewportAttributes =
    setOf("x", "y", "width", "height", "preserveAspectRatio", "overflow", "href")
private val svgRasterData = Regex("data:image/(png|jpeg);base64,([A-Za-z0-9+/=\\s]+)")

internal sealed class SvgImageSource(val size: Size) {
    class Vector(val svg: SVG, size: Size) : SvgImageSource(size)
}

internal data class PreparedSvgImages(
    val root: SvgImageSource.Vector,
    val sources: List<SvgImageSource>,
)

// One compilation owns the entire nested-image graph. Children precede parents, so each
// unique URI renders once with already-decoded dependencies and one shared pixel budget.
private class SvgPreparation {
    var bytes = 0
    var nodes = 0
    var expansions = 0
    var resources = 0
    val keys = mutableMapOf<String, Int>()
    val sources = mutableListOf<SvgImageSource>()

    fun source(uri: String, depth: Int): Int? {
        keys[uri]?.let {
            return it
        }
        if (++resources > SVG_IMAGE_RESOURCES || depth >= SVG_IMAGE_DEPTH) return null
        val source =
            if (uri.startsWith("data:image/svg+xml;base64,")) {
                if (uri.length > EPUB_SVG_BYTES) return null
                val encoded = uri.substringAfter(',')
                if (!encoded.matches(Regex("[A-Za-z0-9+/=\\s]+"))) return null
                val decoded = Base64.decode(encoded, Base64.DEFAULT)
                prepareSvg(decoded, this, depth + 1) ?: return null
            } else SvgRasterSource.read(uri) ?: return null
        sources.add(source)
        return sources.lastIndex.also { keys[uri] = it }
    }
}

/** Prepares all document-scoped graphs before allocating any source pixels. */
internal fun prepareSvgImages(bytes: ByteArray): PreparedSvgImages? {
    val preparation = SvgPreparation()
    val root = prepareSvg(bytes, preparation, 0) ?: return null
    return PreparedSvgImages(root, preparation.sources)
}

// AndroidSVG also accepts an IRI missing its closing parenthesis.
private val svgUrls = Regex("url\\s*\\(([^)]*)(?:\\)|$)", RegexOption.IGNORE_CASE)

/** Preflight before AndroidSVG builds a tree or follows recursive local references. */
private fun prepareSvg(
    bytes: ByteArray,
    preparation: SvgPreparation,
    imageDepthLevel: Int,
): SvgImageSource.Vector? {
    preparation.bytes += bytes.size
    if (bytes.isEmpty() || preparation.bytes > EPUB_SVG_BYTES) return null
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
    var hasImages = false
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
                    if (parents.isEmpty() && (nodes.isNotEmpty() || parser.name != "svg"))
                        return null
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
                    preparation.nodes += if (image) 3 else 1
                    if (preparation.nodes > SVG_NODES) return null
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
                        hasImages = true
                        val key = preparation.source(uri, imageDepthLevel) ?: return null
                        // An SVG viewport preserves the ORIGINAL source ratio even when its
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
                        val source = preparation.sources[key].size
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
    if (hasImages && styles.isNotBlank()) return null
    if (nodes.isEmpty() || parents.isNotEmpty()) return null
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
    preparation.expansions += costs[0]
    if (preparation.expansions > SVG_EXPANSIONS) return null
    serializer.flush()
    val svg = SVG.getFromString(output.toString())
    return SvgImageSource.Vector(svg, svgSourceSize(svg) ?: return null)
}

/** Called only by the serial, off-main image decoder. The output shares its page pixel budget. */
internal fun decodeEpubSvg(bytes: ByteArray, pixelBudget: Int): DecodedEpubImage? =
    synchronized(svgDecodeLock) { decodeEpubSvgLocked(bytes, pixelBudget) }

private fun decodeEpubSvgLocked(bytes: ByteArray, pixelBudget: Int): DecodedEpubImage? {
    if (bytes.isEmpty() || bytes.size > EPUB_SVG_BYTES || pixelBudget < 1) return null
    var resources: SvgImageResources? = null
    return try {
        SVG.setInternalEntitiesEnabled(false)
        SVG.deregisterExternalFileResolver()
        val prepared = prepareSvgImages(bytes) ?: return null
        resources = SvgImageResources.decode(prepared.sources, pixelBudget) ?: return null
        SVG.registerExternalFileResolver(resources)
        renderSvg(prepared.root, pixelBudget)
    } catch (_: Exception) {
        null
    } catch (_: StackOverflowError) {
        null
    } catch (_: OutOfMemoryError) {
        null
    } finally {
        SVG.deregisterExternalFileResolver()
        resources?.close()
    }
}

private fun svgSourceSize(svg: SVG): Size? {
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
    return Size(width, height)
}

private fun renderSvg(source: SvgImageSource.Vector, pixelBudget: Int): DecodedEpubImage {
    val svg = source.svg
    val width = source.size.width
    val height = source.size.height
    val sourceWidth = ceil(width).toInt()
    val sourceHeight = ceil(height).toInt()
    var sample = 1
    fun sampled(edge: Int) = (edge + sample - 1) / sample
    while (
        sampled(sourceWidth) > EPUB_IMAGE_EDGE ||
            sampled(sourceHeight) > EPUB_IMAGE_EDGE ||
            sampled(sourceWidth).toLong() * sampled(sourceHeight) > pixelBudget
    ) sample *= 2
    val bitmap = createBitmap(sampled(sourceWidth), sampled(sourceHeight), Bitmap.Config.ARGB_8888)
    try {
        svg.documentWidth = width
        svg.documentHeight = height
        val canvas = Canvas(bitmap)
        canvas.scale(bitmap.width / width, bitmap.height / height)
        svg.renderToCanvas(canvas)
        return DecodedEpubImage(bitmap, sourceWidth, sourceHeight, source.size)
    } catch (error: Throwable) {
        bitmap.recycle()
        throw error
    }
}

/** Bounds-only inspection is safe before graph validation; it never allocates source pixels. */
internal data class SvgRasterSource(
    val bytes: ByteArray,
    val media: String,
    val width: Int,
    val height: Int,
) : SvgImageSource(Size(width.toFloat(), height.toFloat())) {
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
internal class SvgImageResources private constructor(internal val images: List<Bitmap>) :
    SVGExternalFileResolver(), AutoCloseable {
    override fun resolveImage(filename: String): Bitmap? {
        val index = filename.removePrefix("yomoku-svg-image:").toIntOrNull() ?: return null
        return images.getOrNull(index).takeIf { filename == "yomoku-svg-image:$index" }
    }

    override fun isFormatSupported(mimeType: String) =
        mimeType in listOf("image/png", "image/jpeg", "image/svg+xml")

    override fun close() = images.forEach { it.recycle() }

    companion object {
        fun decode(sources: List<SvgImageSource>, pixelBudget: Int): SvgImageResources? =
            synchronized(svgDecodeLock) { decodeLocked(sources, pixelBudget) }

        private fun decodeLocked(
            sources: List<SvgImageSource>,
            pixelBudget: Int,
        ): SvgImageResources? {
            if (pixelBudget < 1 || sources.size > SVG_IMAGE_RESOURCES || sources.size > pixelBudget)
                return null
            val images = mutableListOf<Bitmap>()
            var success = false
            try {
                SVG.registerExternalFileResolver(SvgImageResources(images))
                val share = pixelBudget / maxOf(1, sources.size)
                for (source in sources) {
                    val image =
                        when (source) {
                            is SvgRasterSource ->
                                decodeEpubImage(source.bytes, share, source.media) ?: return null
                            is SvgImageSource.Vector -> renderSvg(source, share)
                        }
                    images.add(image.bitmap)
                }
                success = true
                return SvgImageResources(images)
            } finally {
                SVG.deregisterExternalFileResolver()
                if (!success) images.forEach { it.recycle() }
            }
        }
    }
}
