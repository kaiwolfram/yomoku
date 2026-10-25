#!/usr/bin/env python3
"""Bundle an explicitly prepared, pinned dictionary index; never download data.

Stage and hash copies before immutable directory publication. Preserve an existing
output on failure or changed input. Use a fresh output for reviewed data updates.
"""

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import shutil
import tempfile


REPO = Path(__file__).resolve().parent.parent
PIN = REPO / "third_party/dictionaries/android.lock.json"
PREPARED = REPO / ".local/dictionaries/index-v1-20261025"
OUTPUT = REPO / "target/generated/dictionary-assets/dictionaries"
BUILDER = REPO / "tools/prepare-dictionaries.py"


def checksum(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def verify(path, expected):
    if checksum(path) != expected:
        raise ValueError(f"SHA-256 mismatch: {path.name}")


def hashes(directory):
    return {p.relative_to(directory).as_posix(): checksum(p)
            for p in sorted(directory.rglob("*")) if p.is_file()}


def asset_name(name):
    # Android's asset merger transparently expands .gz files and strips the suffix.
    # Preserve the exact compressed upstream bytes under a neutral extension.
    return name + ".bin" if name.endswith(".gz") else name


def bundle(source, destination, pin, builder=BUILDER):
    verify(source / "build.json", pin["build_sha256"])
    build = json.loads((source / "build.json").read_text(encoding="utf-8"))
    if build["schema_version"] != 1:
        raise ValueError("Unsupported dictionary index format")
    verify(builder, build["builder_sha256"])
    files = build["files"]
    if files["dictionary.sqlite3"] != pin["index_sha256"]:
        raise ValueError("Index checksum differs from Android pin")
    if (source / "dictionary.sqlite3").stat().st_size != pin["index_size"]:
        raise ValueError("Incorrect dictionary index size")
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="bundle-", dir=destination.parent) as temporary:
        staged = Path(temporary) / "dictionaries"
        staged.mkdir()
        for name, expected in {**files, "build.json": pin["build_sha256"]}.items():
            path = PurePosixPath(name)
            if path.is_absolute() or ".." in path.parts or str(path) != name:
                raise ValueError("Invalid bundle path")
            target = staged / asset_name(name)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source / name, target)
            verify(target, expected)
        # Retain the exact compiler and original compressed inputs for reproduction.
        shutil.copyfile(builder, staged / "prepare-dictionaries.py")
        verify(staged / "prepare-dictionaries.py", build["builder_sha256"])
        (staged / "android.lock.json").write_text(
            json.dumps(pin, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8"
        )
        if destination.exists():
            if hashes(destination) != hashes(staged):
                raise ValueError("Published Android assets differ; use a fresh --output")
        else:
            for path in staged.rglob("*"):
                if path.is_file():
                    path.chmod(0o444)
            staged.rename(destination)
    return destination


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prepared", type=Path, default=PREPARED)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    if not args.prepared.is_dir():
        parser.error("Prepare pinned data first: python3 tools/prepare-dictionaries.py")
    pin = json.loads(PIN.read_text(encoding="utf-8"))
    print(f"Bundled verified offline dictionaries: {bundle(args.prepared, args.output, pin)}")


if __name__ == "__main__":
    main()
