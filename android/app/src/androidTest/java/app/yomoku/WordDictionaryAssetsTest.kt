package app.yomoku

import android.content.Context
import android.content.ContextWrapper
import android.database.sqlite.SQLiteDatabase
import android.system.Os
import android.util.Log
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.File
import java.security.MessageDigest
import java.util.UUID
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class WordDictionaryAssetsTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test
    fun installedIndexSourcesCompilerAndCompleteNoticesMatchPinnedManifest() {
        val assets = compose.activity.assets
        fun json(name: String) =
            JSONObject(assets.open("dictionaries/$name").bufferedReader().use { it.readText() })
        fun checksum(name: String): String {
            val digest = MessageDigest.getInstance("SHA-256")
            assets.open("dictionaries/$name").use { input ->
                val buffer = ByteArray(64 * 1024)
                while (true) {
                    val count = input.read(buffer)
                    if (count == -1) break
                    digest.update(buffer, 0, count)
                }
            }
            return digest.digest().joinToString("") { "%02x".format(it) }
        }
        val pin = json("android.lock.json")
        assertEquals(WORD_DICTIONARY_SIZE, pin.getLong("index_size"))
        assertEquals(WORD_DICTIONARY_SHA256, pin.getString("index_sha256"))
        assertEquals(pin.getString("build_sha256"), checksum("build.json"))
        val build = json("build.json")
        assertEquals(build.getString("builder_sha256"), checksum("prepare-dictionaries.py"))
        val files = build.getJSONObject("files")
        for (name in files.keys()) {
            val packaged = if (name.endsWith(".gz")) "$name.bin" else name
            assertEquals(name, files.getString(name), checksum(packaged))
        }
        assertEquals(WORD_DICTIONARY_SHA256, files.getString("dictionary.sqlite3"))
        for (name in wordDictionaryNoticeNames) assertTrue(files.has("notices/$name"))
    }

    @Test
    fun privateIndexIsLazyVerifiedReadOnlyQueryableAndReusesItsInode() {
        val context = compose.activity.applicationContext
        val directory = File(context.noBackupFilesDir, "word-index-test-${UUID.randomUUID()}")
        val privateContext =
            object : ContextWrapper(context) {
                override fun getApplicationContext(): Context = this

                override fun getNoBackupFilesDir(): File = directory
            }
        try {
            val store = wordDictionaryFiles(privateContext)
            assertFalse(directory.exists())
            compose.runOnUiThread {
                assertThrows(IllegalStateException::class.java) { store.verifiedPath() }
            }
            assertFalse(directory.exists())
            val start = android.os.SystemClock.elapsedRealtime()
            val path = store.verifiedPath()
            val extracted = android.os.SystemClock.elapsedRealtime()
            assertEquals(WORD_DICTIONARY_SIZE, path.length())
            assertEquals(0b100000000, Os.stat(path.path).st_mode and 0b111111111)
            val inode = Os.stat(path.path).st_ino
            SQLiteDatabase.openDatabase(path.path, null, SQLiteDatabase.OPEN_READONLY).use { db ->
                assertTrue(db.isReadOnly)
                db.rawQuery("PRAGMA application_id", null).use {
                    assertTrue(it.moveToFirst())
                    assertEquals(0x594f4449, it.getInt(0))
                }
                db.rawQuery("PRAGMA user_version", null).use {
                    assertTrue(it.moveToFirst())
                    assertEquals(1, it.getInt(0))
                }
                db.rawQuery("SELECT DISTINCT source FROM forms WHERE surface = ?", arrayOf("東京"))
                    .use {
                        val sources = mutableSetOf<String>()
                        while (it.moveToNext()) sources += it.getString(0)
                        assertEquals(setOf("JMdict", "JMnedict"), sources)
                    }
            }
            val reopened = wordDictionaryFiles(privateContext).verifiedPath()
            assertEquals(path, reopened)
            assertEquals(inode, Os.stat(reopened.path).st_ino)
            assertFalse(path.parentFile!!.listFiles()!!.any { it.name.endsWith(".partial") })
            assertFalse(File(path.path + "-journal").exists())
            assertFalse(File(path.path + "-wal").exists())
            Log.i(
                "WordDictionaryExtraction",
                "extract_ms=${extracted - start} query_reopen_ms=${android.os.SystemClock.elapsedRealtime() - extracted}",
            )
        } finally {
            assertTrue(!directory.exists() || directory.deleteRecursively())
        }
    }

    @Test
    fun settingsShowsAllOfflineCreditsAcrossRecreation() {
        val expected =
            loadOfflineNotices(
                compose.activity.assets,
                "dictionaries/notices",
                wordDictionaryNoticeNames,
            )
        assertTrue(expected.contains("Attribution-ShareAlike 4.0"))
        assertTrue(expected.contains("JMdict"))
        assertTrue(expected.contains("JMnedict"))
        compose.onNodeWithText("Settings").performClick()
        compose.onNodeWithText("Word and name dictionary credits").performScrollTo().performClick()
        compose.waitUntil(10_000) {
            compose.onAllNodes(hasText(expected)).fetchSemanticsNodes().size == 1
        }
        compose.onNodeWithTag("word_dictionary_notices_text").assertTextEquals(expected)
        compose.activityRule.scenario.recreate()
        compose.waitUntil(10_000) {
            compose.onAllNodes(hasText(expected)).fetchSemanticsNodes().size == 1
        }
        compose.onNodeWithTag("word_dictionary_notices_text").assertTextEquals(expected)
        compose.onNodeWithText("Close").performClick()
        compose.onNodeWithTag("screen_title").assertTextEquals("Settings")
    }
}
