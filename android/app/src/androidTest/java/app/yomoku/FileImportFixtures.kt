package app.yomoku

import android.net.Uri
import android.util.Base64
import java.io.ByteArrayOutputStream
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

internal object FileImportFixtures {
    fun uri(name: String, bytes: ByteArray, gate: String? = null): Uri =
        Uri.Builder()
            .scheme("content")
            .authority("app.yomoku.test.txt-fixtures")
            .appendPath(name)
            .appendQueryParameter("bytes", Base64.encodeToString(bytes, Base64.NO_WRAP))
            .apply { if (gate != null) appendQueryParameter("gate", gate) }
            .build()

    fun epub(
        title: String,
        firstBody: String = "<h1>猫</h1><p>猫と犬。</p>",
        images: Map<String, Pair<String, ByteArray>> = emptyMap(),
        secondBody: String = "<p>鳥と魚。</p>",
    ): ByteArray {
        val output = ByteArrayOutputStream()
        ZipOutputStream(output).use { zip ->
            fun entry(path: String, text: String) {
                zip.putNextEntry(ZipEntry(path))
                zip.write(text.toByteArray())
                zip.closeEntry()
            }
            entry("mimetype", "application/epub+zip")
            entry(
                "META-INF/container.xml",
                """<container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0">
                <rootfiles><rootfile full-path="book.opf" media-type="application/oebps-package+xml"/></rootfiles>
                </container>""",
            )
            entry(
                "book.opf",
                """<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id">
                <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
                <dc:identifier id="id">$title</dc:identifier><dc:title>$title</dc:title><dc:language>ja</dc:language>
                </metadata><manifest>
                <item id="one" href="one.xhtml" media-type="application/xhtml+xml"/>
                <item id="two" href="two.xhtml" media-type="application/xhtml+xml"/>
                ${images.entries.mapIndexed { index, (path, image) -> "<item id='image$index' href='$path' media-type='${image.first}'/>" }.joinToString("")}
                </manifest><spine><itemref idref="one"/><itemref idref="two"/></spine></package>""",
            )
            entry(
                "one.xhtml",
                """<html xmlns="http://www.w3.org/1999/xhtml"><body>$firstBody</body></html>""",
            )
            entry(
                "two.xhtml",
                """<html xmlns="http://www.w3.org/1999/xhtml"><body>$secondBody</body></html>""",
            )
            for ((path, image) in images) {
                zip.putNextEntry(ZipEntry(path))
                zip.write(image.second)
                zip.closeEntry()
            }
        }
        return output.toByteArray()
    }
}
