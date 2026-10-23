#!/usr/bin/env python3
"""Verify the dependency-only Sudachi snapshot against pinned upstream hashes."""

import hashlib
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    resources = Path(__file__).resolve().parent.parent / "third_party/sudachi"
    expected = {}
    for line in (resources / "UPSTREAM_SHA256SUMS").read_text().splitlines():
        digest, name = line.split("  ", 1)
        expected[name] = digest

    # Reverse the complete, reviewable patch in an isolated copy. This checks
    # both the runtime cfg change and dependency-only manifest adjustments.
    with tempfile.TemporaryDirectory(prefix="yomoku-sudachi-source-") as scratch:
        source = Path(scratch) / "engine"
        shutil.copytree(resources / "engine", source)
        subprocess.run(
            [
                "git", "apply", "--reverse", "--whitespace=error",
                str(resources / "patches/android-loader.patch"),
            ],
            cwd=source,
            check=True,
        )
        actual = {str(p.relative_to(source)) for p in source.rglob("*") if p.is_file()}
        if actual != expected.keys():
            raise SystemExit(
                f"Unexpected source file set: missing={expected.keys() - actual}, "
                f"extra={actual - expected.keys()}"
            )
        for name, digest in expected.items():
            if hashlib.sha256((source / name).read_bytes()).hexdigest() != digest:
                raise SystemExit(f"Upstream source checksum mismatch: {name}")
    print(f"Verified {len(expected)} pinned Sudachi files and Android patch.")


if __name__ == "__main__":
    main()
