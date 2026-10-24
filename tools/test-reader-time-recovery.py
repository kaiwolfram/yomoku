#!/usr/bin/env python3
"""Verify real Android reader-time close/reopen and emulator reboot recovery.

Install both debug APKs first. This force-stops app.yomoku and reboots only the
selected emulator, preserves ordinary app data, and removes its unique test
directory after the app has stopped. Reports stay under .local/test-reports.
"""

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shlex
import subprocess
import time
import uuid


PACKAGE = "app.yomoku"
REPO = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serial", required=True)
    args = parser.parse_args()
    sdk = Path(os.environ.get("ANDROID_HOME", Path.home() / "Android/Sdk"))
    adb = [str(sdk / "platform-tools/adb"), "-s", args.serial]

    def command(*parts, timeout=45, check=True):
        return subprocess.run(
            [*adb, *parts], capture_output=True, text=True,
            timeout=timeout, check=check,
        )

    def shell(*parts, **kwargs):
        return command("shell", shlex.join(parts), **kwargs)

    if shell("getprop", "ro.kernel.qemu").stdout.strip() != "1":
        parser.error("This reboot test requires an emulator; physical devices are excluded")
    if shell("run-as", PACKAGE, "pwd", check=False).returncode:
        parser.error("Install app-debug.apk and app-debug-androidTest.apk first")

    def stop():
        shell("am", "force-stop", PACKAGE)
        deadline = time.monotonic() + 15
        while True:
            listing = shell("ps", "-A", "-o", "PID,NAME").stdout.splitlines()[1:]
            if not any(line.split()[-1].split(":")[0] == PACKAGE for line in listing):
                return
            if time.monotonic() > deadline:
                raise TimeoutError("App did not exit; refusing test directory cleanup")
            time.sleep(0.2)

    token = uuid.uuid4().hex
    directory = f"no_backup/reader-recovery-{token}"
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    report_path = REPO / f".local/test-reports/reader-time-recovery/{timestamp}.json"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report = {
        "serial": args.serial,
        "fingerprint": shell("getprop", "ro.build.fingerprint").stdout.strip(),
        "abi": shell("getprop", "ro.product.cpu.abi").stdout.strip(),
        "run": token, "phases": [],
    }
    print(f"Report: {report_path}", flush=True)
    try:
        for phase in ["before", "afterKill", "afterReboot"]:
            stop()
            if phase == "afterReboot":
                command("reboot")
                # Observe disconnect before waiting for the new Android boot.
                deadline = time.monotonic() + 180
                old_boot = report["boot_id"]
                while time.monotonic() < deadline:
                    boot = shell("cat", "/proc/sys/kernel/random/boot_id", check=False).stdout.strip()
                    ready = shell("getprop", "sys.boot_completed", check=False).stdout.strip()
                    if boot and boot != old_boot and ready == "1":
                        report["reboot_id"] = boot
                        break
                    time.sleep(1)
                else:
                    raise TimeoutError("Emulator did not complete a new boot")
            elif phase == "before":
                report["boot_id"] = shell("cat", "/proc/sys/kernel/random/boot_id").stdout.strip()
            result = shell(
                "am", "instrument", "-w", "-r",
                "-e", "class", "app.yomoku.ReaderClockRecoveryTest",
                "-e", "readerClockRun", token,
                "-e", "readerClockPhase", phase,
                "app.yomoku.test/androidx.test.runner.AndroidJUnitRunner",
                timeout=120,
            )
            if "OK (1 test)" not in result.stdout or "FAILURES" in result.stdout:
                raise RuntimeError(f"Recovery instrumentation failed:\n{result.stdout}{result.stderr}")
            measured = json.loads(shell("run-as", PACKAGE, "cat", f"{directory}/{phase}.json").stdout)
            report["phases"].append(measured)
            if measured["status"] != "PASS":
                raise RuntimeError(f"Recovery validation failed: {measured}")
            if len(report["phases"]) > 1:
                previous = report["phases"][-2]
                if measured["restored_ms"] != previous["saved_ms"]:
                    raise RuntimeError("Recovered duration differs from committed final sample")
                if phase == "afterKill" and measured["pid"] == previous["pid"]:
                    raise RuntimeError("Recovery reused the original process")
            report_path.write_text(json.dumps(report, indent=2) + "\n")
            print(f"PASS: {phase}: {measured}", flush=True)
        report["status"] = "PASS"
    except Exception as error:
        report["status"] = "FAIL"
        report["error"] = str(error)
        raise
    finally:
        report_path.write_text(json.dumps(report, indent=2) + "\n")
        stop()
        shell("run-as", PACKAGE, "rm", "-rf", directory)
    print(f"PASS: close/reopen/process/reboot recovery. Report: {report_path}")


if __name__ == "__main__":
    main()
