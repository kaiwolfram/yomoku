#!/usr/bin/env python3
"""Prepare one exact Full dictionary, with its complete redistribution notices.

No build downloads data. --archive supports fully offline preparation.
--android-assets only bundles already prepared, verified data for Gradle.
Writes only ignored local/generated paths; checksum failures preserve usable data.
"""

import argparse
import hashlib
from pathlib import Path
import shutil
import tempfile
import urllib.request
import zipfile


REPO = Path(__file__).resolve().parent.parent
DATA = REPO / ".local/sudachi"
URL = (
    "https://github.com/WorksApplications/SudachiDict/releases/download/"
    "v20261015/sudachi-dictionary-20261015-full.zip"
)
ARCHIVE_SHA256 = "b38318ca26a444cd713a5a377a353308ef38c733c64cc0be905122527516c793"
FILES = {
    "system_full.dic": "7c797531dea76f06b629fad39cb3a5bdb3a9e54c67ef3d9a869e61418426a7d1",
    "LEGAL": "725a8776b38e058b185e905594bc9a2437dbf3787df022fffeefedb9a84e4665",
    "LICENSE-2.0.txt": "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30",
}
ANDROID_ASSETS = REPO / "target/generated/sudachi-assets"


def verify(path, expected):
    with path.open("rb") as stream:
        actual = hashlib.file_digest(stream, "sha256").hexdigest()
    if actual != expected:
        raise ValueError(f"SHA-256 mismatch: {path.name}")


def bundle_android():
    source = DATA / "full-20261015"
    if not source.is_dir():
        raise ValueError(
            "Prepare pinned Full data first: python3 tools/prepare-sudachi.py "
            "[--archive /path/to/pinned.zip]. Android builds never download it."
        )
    # Verify the entire input set before touching previous build outputs.
    for name, checksum in FILES.items():
        verify(source / name, checksum)
    ANDROID_ASSETS.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="bundle-", dir=ANDROID_ASSETS.parent) as temporary:
        staged = Path(temporary)
        for name, checksum in FILES.items():
            shutil.copyfile(source / name, staged / name)
            verify(staged / name, checksum)
        destination = ANDROID_ASSETS / "sudachi"
        destination.mkdir(exist_ok=True)
        for name in FILES:
            (staged / name).replace(destination / name)
    print(f"Bundled verified Full 20261015 + complete notices: {ANDROID_ASSETS}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--archive", type=Path, help="use an existing ZIP; no network")
    mode.add_argument(
        "--android-assets", action="store_true", help="bundle prepared data; no network"
    )
    args = parser.parse_args()
    if args.android_assets:
        bundle_android()
        return
    DATA.mkdir(parents=True, exist_ok=True)
    destination = DATA / "full-20261015"
    with tempfile.TemporaryDirectory(prefix="prepare-", dir=DATA) as temporary:
        temporary = Path(temporary)
        archive = args.archive
        if archive is None:
            archive = temporary / "full.zip"
            print(f"Downloading {URL}", flush=True)
            with urllib.request.urlopen(URL, timeout=60) as response:
                with archive.open("wb") as stream:
                    shutil.copyfileobj(response, stream)
        verify(archive, ARCHIVE_SHA256)
        prepared = temporary / "prepared"
        prepared.mkdir()
        with zipfile.ZipFile(archive) as bundle:
            for name, checksum in FILES.items():
                target = prepared / name
                # Read only named members; never extract arbitrary archive paths.
                with bundle.open(f"sudachi-dictionary-20261015/{name}") as source:
                    with target.open("wb") as output:
                        shutil.copyfileobj(source, output)
                verify(target, checksum)
        if destination.exists():
            # Never replace existing resources (possibly in use by an analyzer).
            for name, checksum in FILES.items():
                verify(destination / name, checksum)
        else:
            prepared.rename(destination)
    print(f"Verified Full 20261015 + notices: {destination}")


if __name__ == "__main__":
    main()
