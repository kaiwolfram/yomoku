#!/usr/bin/env python3
"""Measure the real Full adapter in a standalone Android native process.

Requires the pinned NDK, installed Rust target, prepared Full data and one ADB
device. Uses locked/offline Cargo, never installs or changes the app. Temporarily
disables radios, restores their prior settings, and removes its own device files.
Results exclude APK extraction, JVM/Compose overhead and app lifecycle behavior.
Mapped runs require the backing file to remain immutable until every probe exits;
the driver owns its unique directory and never rewrites the published dictionary.
"""

import argparse
from datetime import datetime, timezone
import hashlib
import os
from pathlib import Path
import shlex
import subprocess
import sys
import time
import uuid


REPO = Path(__file__).resolve().parent.parent
CHECKSUM = "7c797531dea76f06b629fad39cb3a5bdb3a9e54c67ef3d9a869e61418426a7d1"
NDK_VERSION = "30.0.16248370"
TARGETS = {"x86_64": "x86_64-linux-android", "arm64-v8a": "aarch64-linux-android"}


def run(command, **kwargs):
    result = subprocess.run(command, text=True, **kwargs)
    if result.returncode and result.stderr:
        print(result.stderr, file=sys.stderr)
    result.check_returncode()
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serial", default=os.environ.get("ANDROID_SERIAL"))
    parser.add_argument("--profile", choices=["debug", "release"], default="release")
    parser.add_argument(
        "--storage", choices=["owned", "mapped", "both"], default="owned",
        help="both alternates fresh owned/mapped processes for each run",
    )
    parser.add_argument(
        "--runs", type=int, default=3, help="fresh processes; OS caches stay warm"
    )
    args = parser.parse_args()
    if args.runs < 1:
        parser.error("--runs must be positive")
    sdk = Path(os.environ.get("ANDROID_HOME", Path.home() / "Android/Sdk"))
    adb = str(sdk / "platform-tools/adb")
    if not args.serial:
        devices = run([adb, "devices"], capture_output=True).stdout.splitlines()[1:]
        serials = [line.split()[0] for line in devices if line.endswith("\tdevice")]
        if len(serials) != 1:
            parser.error("select one connected device with --serial")
        args.serial = serials[0]

    def device(*command):
        # ADB shell joins its arguments; quote each explicitly before sending.
        return run(
            [adb, "-s", args.serial, "shell", shlex.join(command)], capture_output=True
        ).stdout.strip()

    def await_radios(expected):
        # Android applies svc changes asynchronously. Check actual settings with
        # a bounded wait before analysis and after restoring the original values.
        deadline = time.monotonic() + 10
        while True:
            actual = {key: device("settings", "get", "global", key) for key in expected}
            if actual == expected:
                return actual
            if time.monotonic() >= deadline:
                raise RuntimeError(f"radio settings did not settle: {actual}; expected={expected}")
            time.sleep(0.2)

    abi = device("getprop", "ro.product.cpu.abi")
    if abi not in TARGETS:
        parser.error(f"unsupported ABI: {abi}")
    target = TARGETS[abi]
    dictionary = REPO / ".local/sudachi/full-20261015/system_full.dic"
    with dictionary.open("rb") as stream:
        if hashlib.file_digest(stream, "sha256").hexdigest() != CHECKSUM:
            raise ValueError("prepared Full dictionary checksum mismatch")
    run([sys.executable, str(REPO / "tools/verify-sudachi-source.py")], cwd=REPO)
    ndk = sdk / "ndk" / NDK_VERSION
    if f"Pkg.Revision = {NDK_VERSION}" not in (ndk / "source.properties").read_text():
        raise ValueError("wrong NDK revision")
    toolchain = ndk / "toolchains/llvm/prebuilt/linux-x86_64/bin"
    env = os.environ.copy()
    compiler = str(toolchain / f"{target}29-clang")
    env[f"CARGO_TARGET_{target.upper().replace('-', '_')}_LINKER"] = compiler
    # Examples also build the storage test dependency's bundled SQLite.
    env[f"CC_{target.replace('-', '_')}"] = compiler
    env[f"AR_{target.replace('-', '_')}"] = str(toolchain / "llvm-ar")
    command = [
        "cargo", "build", "-p", "yomoku-analysis", "--example", "android_probe",
        "--target", target, "--locked", "--offline",
    ]
    if args.profile == "release":
        command.append("--release")
    run(command, cwd=REPO, env=env)
    binary = REPO / "target" / target / args.profile / "examples/android_probe"
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    reports = REPO / ".local/test-reports/sudachi-android"
    reports.mkdir(parents=True, exist_ok=True)
    report = reports / f"{timestamp}-{abi}-{args.profile}-{args.storage}.txt"
    print(f"Report: {report}", flush=True)
    remote = f"/data/local/tmp/yomoku-sudachi-{uuid.uuid4().hex}"
    original = {
        key: device("settings", "get", "global", key)
        for key in ["airplane_mode_on", "wifi_on", "mobile_data"]
    }
    if any(value not in ["0", "1"] for value in original.values()):
        raise ValueError(f"cannot restore unsupported radio settings: {original}")
    try:
        device("mkdir", remote)
        device("chmod", "700", remote)
        run([adb, "-s", args.serial, "push", str(binary), f"{remote}/probe"])
        run([adb, "-s", args.serial, "push", str(dictionary), f"{remote}/system_full.dic"])
        device("chmod", "700", f"{remote}/probe")
        device("chmod", "400", f"{remote}/system_full.dic")
        device("cmd", "connectivity", "airplane-mode", "enable")
        device("svc", "wifi", "disable")
        device("svc", "data", "disable")
        offline = await_radios({"airplane_mode_on": "1", "wifi_on": "0", "mobile_data": "0"})
        with report.open("x") as output:
            output.write(f"UTC={timestamp}\nserial={args.serial}\nprofile={args.profile}\n")
            output.write(f"storage={args.storage}\n")
            output.write(f"offline={offline}\noriginal_radios={original}\n")
            output.write(
                "Standalone native process; no APK extraction/JVM/UI overhead.\n"
                "Fresh processes; filesystem caches NOT flushed.\n"
            )
            for prop in [
                "ro.build.fingerprint", "ro.product.model", "ro.product.cpu.abi",
                "ro.build.version.sdk", "ro.kernel.qemu",
            ]:
                output.write(f"{prop}={device('getprop', prop)}\n")
            output.write(device("uname", "-a") + "\n")
            output.write(device("cat", "/proc/meminfo") + "\n")
            output.flush()
            storage_modes = ["owned", "mapped"] if args.storage == "both" else [args.storage]
            processes = [
                (index, storage)
                for index in range(args.runs)
                for storage in storage_modes
            ]
            for index, storage in processes:
                print(
                    f"Measuring run {index + 1}/{args.runs} ({args.profile}, {storage})",
                    flush=True,
                )
                result = subprocess.run(
                    [adb, "-s", args.serial, "shell", shlex.join([
                        f"{remote}/probe", f"{remote}/system_full.dic", storage,
                    ])],
                    capture_output=True,
                    text=True,
                )
                output.write(f"\nrun={index + 1} storage={storage}\n{result.stdout}{result.stderr}")
                output.flush()
                result.check_returncode()
                if (
                    "validation=PASS" not in result.stdout
                    or f"storage={storage}" not in result.stdout
                ):
                    raise ValueError("probe did not complete validation")
    finally:
        # Attempt every restoration even if another one fails.
        failures = []
        for command in [
            (
                "cmd", "connectivity", "airplane-mode",
                "enable" if original["airplane_mode_on"] == "1" else "disable",
            ),
            ("svc", "wifi", "enable" if original["wifi_on"] == "1" else "disable"),
            ("svc", "data", "enable" if original["mobile_data"] == "1" else "disable"),
            ("rm", "-rf", remote),
        ]:
            try:
                device(*command)
            except subprocess.CalledProcessError as error:
                failures.append(str(error))
        if failures:
            raise RuntimeError(
                f"device restoration failed: {failures}; original={original}"
            )
        await_radios(original)
    print(f"PASS: measurements saved; radio settings restored. Report: {report}")


if __name__ == "__main__":
    main()
