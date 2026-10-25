#!/usr/bin/env python3
"""Offline dictionary compiler checks; synthetic fixtures require no downloads.

--full PATH also verifies a prepared pinned full-data index read-only.
"""

import copy
from contextlib import closing
import gzip
import importlib.util
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest


SPEC = importlib.util.spec_from_file_location(
    "prepare_dictionaries", Path(__file__).with_name("prepare-dictionaries.py")
)
compiler = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(compiler)
FULL = None
if "--full" in sys.argv:
    argument = sys.argv.index("--full")
    FULL = Path(sys.argv[argument + 1])
    del sys.argv[argument:argument + 2]

# Invented lexical data for testing structure; not extracts of either dictionary.
WORD = """<?xml version="1.0"?><!DOCTYPE JMdict [
<!ENTITY n "noun (common) (futsuumeishi)"><!ENTITY v1 "Ichidan verb">
]><!-- JMdict created: 2026-10-25 --><JMdict>
<entry><ent_seq>1</ent_seq>
<k_ele><keb>試験語</keb><ke_inf>test spelling</ke_inf><ke_pri>spec1</ke_pri></k_ele>
<k_ele><keb>試験語𠮷</keb></k_ele>
<r_ele><reb>しけんご</reb><re_restr>試験語</re_restr><re_inf>test reading</re_inf><re_pri>spec1</re_pri></r_ele>
<r_ele><reb>テスト</reb><re_nokanji/></r_ele>
<sense><pos>&n;</pos><gloss>test meaning</gloss><gloss g_type="lit">literal &amp; test</gloss></sense>
<sense><stagk>試験語𠮷</stagk><stagr>テスト</stagr><field>testing</field><misc>rare</misc>
<s_inf>explanation</s_inf><dial>test dialect</dial><xref>別語</xref><ant>反語</ant>
<lsource xml:lang="fre" ls_type="part" ls_wasei="y"/>
<gloss>second sense</gloss><gloss xml:lang="ger">not English</gloss></sense>
<sense><pos>&v1;</pos><gloss xml:lang="fre">omitted</gloss></sense>
<sense><gloss g_gend="neut">third English sense</gloss></sense></entry>
<entry><ent_seq>2</ent_seq><r_ele><reb>かな</reb></r_ele><sense><pos>&n;</pos><gloss>kana only</gloss></sense></entry>
<entry><ent_seq>3</ent_seq><r_ele><reb>しけんご</reb></r_ele><sense><gloss>homophone</gloss></sense></entry>
<entry><ent_seq>4</ent_seq><r_ele><reb>か\u3099</reb></r_ele><sense><gloss>decomposed</gloss></sense></entry>
</JMdict>"""
NAME = """<?xml version="1.0"?><!DOCTYPE JMnedict [<!ENTITY surname "family or surname">]>
<!-- JMnedict created: 2026-10-25 --><JMnedict><entry><ent_seq>1</ent_seq>
<k_ele><keb>試験語</keb></k_ele><r_ele><reb>しけんご</reb></r_ele>
<trans><name_type>&surname;</name_type><trans_det>Test Name</trans_det><trans_det>Other Name</trans_det>
<trans_det xml:lang="rus">omitted</trans_det></trans>
<trans><name_type>place name</name_type><xref>別名</xref><trans_det>Test Place</trans_det></trans>
</entry></JMnedict>"""


class CompilerTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.sources = self.root / "sources"
        self.sources.mkdir()
        self.output = self.root / "published"
        self.lock = json.loads(compiler.LOCK_PATH.read_text())
        self.write("JMdict", WORD, 4)
        self.write("JMnedict", NAME, 1)

    def write(self, source, xml, entries):
        spec = self.lock["sources"][source]
        path = self.sources / spec["filename"]
        path.write_bytes(gzip.compress(xml.encode(), mtime=0))
        spec["sha256"] = compiler.checksum(path)
        spec["entries"] = entries

    def build(self):
        return compiler.prepare(self.sources, self.output, self.lock)

    def connect(self):
        db = sqlite3.connect((self.output / "dictionary.sqlite3").as_uri() + "?mode=ro", uri=True)
        self.addCleanup(db.close)
        return db

    def test_all_senses_metadata_restrictions_and_inheritance(self):
        self.build()
        word, name = compiler.lookup(self.connect(), "試験語")
        self.assertEqual((word["source"], name["source"]), ("JMdict", "JMnedict"))
        self.assertEqual(word["kanji"][0]["ke_inf"], ["test spelling"])
        self.assertEqual(word["readings"][0]["re_restr"], ["試験語"])
        self.assertTrue(word["readings"][1]["no_kanji"])
        senses = word["senses"]
        self.assertEqual(len(senses), 3)
        self.assertEqual([len(s["glosses"]) for s in senses], [2, 1, 1])
        self.assertEqual(senses[0]["glosses"][1]["text"], "literal & test")
        self.assertEqual(senses[0]["glosses"][1]["g_type"], "lit")
        self.assertEqual(senses[1]["pos"], senses[0]["pos"])
        self.assertEqual(senses[2]["pos"], ["Ichidan verb"])
        self.assertEqual(senses[2]["glosses"][0]["g_gend"], "neut")
        self.assertEqual(senses[1]["stagk"], ["試験語𠮷"])
        self.assertEqual(senses[1]["stagr"], ["テスト"])
        for tag in ("field", "misc", "s_inf", "dial", "xref", "ant"):
            self.assertIn(tag, senses[1])
        self.assertEqual(
            senses[1]["loan_sources"],
            [{"text": "", "lang": "fre", "type": "part", "wasei": True}],
        )
        self.assertEqual(len(name["names"]), 2)
        self.assertEqual(name["names"][0]["name_type"], ["family or surname"])
        self.assertEqual(len(name["names"][0]["translations"]), 2)

    def test_exact_unicode_kana_all_homophones_and_separate_sources(self):
        self.build()
        db = self.connect()
        self.assertEqual(len(compiler.lookup(db, "しけんご")), 3)
        self.assertEqual(len(compiler.lookup(db, "かな")), 1)
        self.assertEqual(len(compiler.lookup(db, "試験語𠮷")), 1)
        self.assertEqual(len(compiler.lookup(db, "か\u3099")), 1)
        for absent in ("が", "試験", "unknown", "試験語\x00", "' OR 1=1 --"):
            self.assertEqual(compiler.lookup(db, absent), [])

    def test_reproducible_complete_bundle_and_immutable_reuse(self):
        self.build()
        before = compiler.file_hashes(self.output)
        self.build()
        other = self.root / "second"
        compiler.prepare(self.sources, other, self.lock)
        self.assertEqual(compiler.file_hashes(other), before)
        self.assertEqual(compiler.file_hashes(self.output), before)
        build = json.loads((self.output / "build.json").read_text())
        for name, digest in build["files"].items():
            self.assertEqual(compiler.checksum(self.output / name), digest)
        self.assertIn("notices/ATTRIBUTION.txt", before)
        self.assertIn("upstream/JMdict_e.gz", before)
        self.assertEqual((self.output / "dictionary.sqlite3").stat().st_mode & 0o222, 0)

    def test_read_only_indexed_lookup_and_schema_identity(self):
        self.build()
        before = compiler.file_hashes(self.output)
        db = self.connect()
        self.assertEqual(db.execute("PRAGMA application_id").fetchone()[0], compiler.APPLICATION_ID)
        self.assertEqual(db.execute("PRAGMA user_version").fetchone()[0], 1)
        plan = db.execute("EXPLAIN QUERY PLAN " + compiler.LOOKUP_SQL, ("試験語",)).fetchall()
        self.assertTrue(any("SEARCH f USING PRIMARY KEY (surface=?)" in p[3] for p in plan), plan)
        with self.assertRaises(sqlite3.OperationalError):
            db.execute("DELETE FROM entries")
        compiler.lookup(db, "試験語")
        self.assertEqual(compiler.file_hashes(self.output), before)

    def test_corrupt_input_preserves_published_output(self):
        self.build()
        before = compiler.file_hashes(self.output)
        (self.sources / "JMdict_e.gz").write_bytes(b"corrupt")
        with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
            self.build()
        self.assertEqual(compiler.file_hashes(self.output), before)
        self.assertEqual(sorted(p.name for p in self.root.iterdir()), ["published", "sources"])

    def test_changed_valid_input_cannot_overwrite_published_output(self):
        self.build()
        before = compiler.file_hashes(self.output)
        self.write("JMdict", WORD.replace("test meaning", "changed meaning"), 4)
        with self.assertRaisesRegex(ValueError, "Published output differs"):
            self.build()
        self.assertEqual(compiler.file_hashes(self.output), before)

    def test_wrong_date_count_duplicate_and_malformed_xml_publish_nothing(self):
        cases = [(WORD.replace("2026-10-25", "2026-10-24"), 4, ValueError),
                 (WORD, 5, ValueError),
                 (WORD.replace("<ent_seq>2", "<ent_seq>1"), 4, sqlite3.IntegrityError),
                 (WORD[:-9], 4, compiler.ET.ParseError),
                 (WORD.replace("</entry>", "<future>data</future></entry>", 1), 4, ValueError),
                 (WORD.replace("<re_restr>試験語</re_restr>", "<re_restr>不存在</re_restr>"), 4, ValueError),
                 (WORD.replace("<reb>かな</reb>", ""), 4, ValueError)]
        for xml, count, error in cases:
            with self.subTest(error=error, xml=xml[-100:]):
                self.write("JMdict", xml, count)
                with self.assertRaises(error):
                    self.build()
                self.assertFalse(self.output.exists())
                self.assertEqual([p.name for p in self.root.iterdir()], ["sources"])

    def test_notice_corruption_fails_before_publication(self):
        broken = copy.deepcopy(self.lock)
        broken["notices"]["CC-BY-SA-4.0.txt"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
            compiler.prepare(self.sources, self.output, broken)
        self.assertFalse(self.output.exists())


@unittest.skipUnless(FULL, "Pass --full PATH for pinned real-data verification")
class FullDataTests(unittest.TestCase):
    def test_manifest_counts_and_real_lookups_without_writes(self):
        before = compiler.file_hashes(FULL)
        manifest = json.loads((FULL / "build.json").read_text())
        for name, digest in manifest["files"].items():
            self.assertEqual(compiler.checksum(FULL / name), digest)
        lock = json.loads(compiler.LOCK_PATH.read_text())
        uri = (FULL.resolve() / "dictionary.sqlite3").as_uri() + "?mode=ro"
        with closing(sqlite3.connect(uri, uri=True)) as db:
            stored = db.execute("SELECT value FROM metadata WHERE key='sources'").fetchone()[0]
            self.assertEqual(json.loads(stored), lock)
            counts = dict(db.execute("SELECT source, count(*) FROM entries GROUP BY source"))
            self.assertEqual(counts, {s: v["entries"] for s, v in lock["sources"].items()})
            eat = [v for v in compiler.lookup(db, "食べる") if v["source"] == "JMdict"]
            self.assertTrue(any(
                "to eat" == g["text"] for e in eat for s in e["senses"] for g in s["glosses"]
            ))
            self.assertTrue(any("Ichidan" in p for e in eat for s in e["senses"] for p in s["pos"]))
            self.assertTrue(compiler.lookup(db, "たべる"))
            tokyo = compiler.lookup(db, "東京")
            self.assertEqual({e["source"] for e in tokyo}, {"JMdict", "JMnedict"})
            self.assertTrue(any(
                s.get("name_type") for e in tokyo if e["source"] == "JMnedict" for s in e["names"]
            ))
            kana = [e for e in compiler.lookup(db, "ありがとう") if e["source"] == "JMdict"]
            self.assertTrue(kana)
            self.assertEqual(compiler.lookup(db, "存在しない試験語𠮷abc"), [])
        self.assertEqual(compiler.file_hashes(FULL), before)


if __name__ == "__main__":
    unittest.main()
