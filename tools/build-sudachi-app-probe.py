#!/usr/bin/env python3
"""Build the measurement-only JNI library for debug APKs, never release APKs."""

import fcntl
import os
from pathlib import Path
import shutil
import subprocess

REPO = Path(__file__).resolve().parent.parent
lock_path = REPO / "target/generated/native-generation.lock"
lock_path.parent.mkdir(parents=True, exist_ok=True)
lock = lock_path.open("w")
fcntl.flock(lock, fcntl.LOCK_EX)
sdk = Path(os.environ.get("ANDROID_HOME", Path.home() / "Android/Sdk"))
ndk = sdk / "ndk/30.0.16248370"
if "Pkg.Revision = 30.0.16248370" not in (ndk / "source.properties").read_text():
    raise ValueError("Install pinned NDK 30.0.16248370")
subprocess.run(["python3", "tools/verify-sudachi-source.py"], cwd=REPO, check=True)
toolchain = ndk / "toolchains/llvm/prebuilt/linux-x86_64/bin"
for abi, target in [("arm64-v8a", "aarch64-linux-android"), ("x86_64", "x86_64-linux-android")]:
    env = os.environ.copy()
    compiler = str(toolchain / f"{target}29-clang")
    env[f"CARGO_TARGET_{target.upper().replace('-', '_')}_LINKER"] = compiler
    env[f"CC_{target.replace('-', '_')}"] = compiler
    env[f"AR_{target.replace('-', '_')}"] = str(toolchain / "llvm-ar")
    # Release Rust avoids conflating debug hashing/tokenization with app costs.
    subprocess.run([
        "cargo", "build", "-p", "yomoku-analysis", "--example", "android_app_probe",
        "--target", target, "--release", "--locked", "--offline",
    ], cwd=REPO, env=env, check=True)
    output = REPO / "target/generated/sudachi-probe/jniLibs" / abi
    output.mkdir(parents=True, exist_ok=True)
    shutil.copy2(REPO / "target" / target / "release/examples/libandroid_app_probe.so", output)
