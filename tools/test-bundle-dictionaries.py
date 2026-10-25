#!/usr/bin/env python3
"""Data-free verification of immutable Android dictionary bundling."""

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


SPEC = importlib.util.spec_from_file_location(
    "bundle_dictionaries", Path(__file__).with_name("bundle-dictionaries.py")
)
bundler = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(bundler)


class BundleTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.source = self.root / "prepared"
        self.source.mkdir()
        self.output = self.root / "assets/dictionaries"
        for name in ("dictionary.sqlite3", "notices/legal.txt", "upstream/source.gz"):
            target = self.source / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(name.encode())
        self.build = {"schema_version": 1, "files": bundler.hashes(self.source),
                      "builder_sha256": bundler.checksum(bundler.BUILDER)}
        self.pin = {"index_size": (self.source / "dictionary.sqlite3").stat().st_size,
                    "index_sha256": self.build["files"]["dictionary.sqlite3"]}
        self.write_build()

    def write_build(self):
        (self.source / "build.json").write_text(json.dumps(self.build))
        self.pin["build_sha256"] = bundler.checksum(self.source / "build.json")

    def bundle(self):
        bundler.bundle(self.source, self.output, self.pin)

    def test_complete_reproducible_bundle_and_immutable_reuse(self):
        self.bundle()
        before = bundler.hashes(self.output)
        self.bundle()
        self.assertEqual(before, bundler.hashes(self.output))
        for name, expected in self.build["files"].items():
            packaged = bundler.asset_name(name)
            self.assertEqual(before[packaged], expected)
            self.assertEqual((self.output / packaged).stat().st_mode & 0o222, 0)
        self.assertEqual(before["prepare-dictionaries.py"], self.build["builder_sha256"])
        self.assertEqual(json.loads((self.output / "android.lock.json").read_text()), self.pin)

    def test_corrupt_index_notice_source_or_manifest_preserves_published_output(self):
        self.bundle()
        before = bundler.hashes(self.output)
        for name in (*self.build["files"], "build.json"):
            with self.subTest(name=name):
                path = self.source / name
                original = path.read_bytes()
                path.write_bytes(bytes(len(original)))
                with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
                    self.bundle()
                self.assertEqual(before, bundler.hashes(self.output))
                self.assertEqual([p.name for p in self.output.parent.iterdir()], ["dictionaries"])
                path.write_bytes(original)

    def test_changed_valid_bundle_cannot_replace_published_output(self):
        self.bundle()
        before = bundler.hashes(self.output)
        self.pin["index_size"] += 1
        with self.assertRaisesRegex(ValueError, "size"):
            self.bundle()
        self.pin["index_size"] -= 1
        self.build["files"]["notices/legal.txt"] = "0" * 64
        self.write_build()
        with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
            self.bundle()
        self.assertEqual(before, bundler.hashes(self.output))
        self.build["files"]["notices/legal.txt"] = bundler.checksum(self.source / "notices/legal.txt")
        self.write_build()
        # A reviewed manifest change is still a different published bundle.
        self.build["tool_version"] = "changed"
        self.write_build()
        with self.assertRaisesRegex(ValueError, "Published Android assets differ"):
            self.bundle()
        self.assertEqual(before, bundler.hashes(self.output))

    def test_unsupported_schema_changed_compiler_and_unsafe_paths_publish_nothing(self):
        for field, value in (("schema_version", 2), ("builder_sha256", "0" * 64)):
            original = self.build[field]
            self.build[field] = value
            self.write_build()
            with self.assertRaises(ValueError):
                self.bundle()
            self.assertFalse(self.output.exists())
            self.build[field] = original
        for name in ("../escape", "/escape", "notices/../escape"):
            self.build["files"][name] = "0" * 64
            self.write_build()
            with self.assertRaisesRegex(ValueError, "Invalid bundle path"):
                self.bundle()
            self.assertFalse(self.output.exists())
            del self.build["files"][name]


if __name__ == "__main__":
    unittest.main()
