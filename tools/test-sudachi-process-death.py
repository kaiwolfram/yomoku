#!/usr/bin/env python3
"""Kill an installed debug app with Full mapped, then validate cache recovery.

Build/install the current debug APK first. No build or installation is implicit.
This force-stops app.yomoku, preserving its data, and removes only the unique
probe directory after confirming every app process has exited. It does not
simulate memory pressure, flush storage caches, or change radios/viewport.
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


REPO = Path(__file__).resolve().parent.parent
PACKAGE = "app.yomoku"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serial", required=True)
    parser.add_argument("--runs", type=int, default=2)
    args = parser.parse_args()
    if args.runs < 1:
        parser.error("--runs must be positive")
    sdk = Path(os.environ.get("ANDROID_HOME", Path.home() / "Android/Sdk"))
    adb = [str(sdk / "platform-tools/adb"), "-s", args.serial]

    def device(*command, check=True):
        result = subprocess.run(
            [*adb, "shell", shlex.join(command)], capture_output=True,
            text=True, timeout=30,
        )
        if check and result.returncode:
            raise RuntimeError(
                f"ADB command failed: {shlex.join(command)}\n{result.stdout}{result.stderr}"
            )
        return result

    if device("run-as", PACKAGE, "pwd", check=False).returncode:
        parser.error(
            "Install the current debug APK first; connected instrumentation may uninstall it"
        )

    def app_processes():
        listing = device("ps", "-A", "-o", "PID,NAME").stdout.splitlines()[1:]
        return [line for line in listing if line.split()[-1].split(":")[0] == PACKAGE]

    def stop():
        device("am", "force-stop", PACKAGE)
        deadline = time.monotonic() + 15
        while app_processes():
            if time.monotonic() > deadline:
                raise RuntimeError("App processes survived force-stop; refusing file cleanup")
            time.sleep(0.2)

    def read_report(path):
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            result = device("run-as", PACKAGE, "cat", path, check=False)
            if result.returncode == 0:
                return json.loads(result.stdout)
            time.sleep(0.25)
        raise TimeoutError(f"No completed probe report: {path}")

    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    report_path = REPO / f".local/test-reports/sudachi-process-death/{timestamp}.json"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report = {
        "serial": args.serial, "method": "am force-stop; no pressure or cache flush",
        "fingerprint": device("getprop", "ro.build.fingerprint").stdout.strip(),
        "abi": device("getprop", "ro.product.cpu.abi").stdout.strip(), "runs": [],
    }
    print(f"Report: {report_path}", flush=True)
    try:
        for index in range(args.runs):
            token = uuid.uuid4().hex
            directory = f"no_backup/process-probe-{token}"
            pair = {"run": token}
            report["runs"].append(pair)
            stop()
            try:
                for phase in ["before", "after"]:
                    # Keep the real Compose/CoreSession Activity in the same process.
                    device("am", "start", "-W", "-n", f"{PACKAGE}/.MainActivity")
                    launch = device(
                        "am", "start", "-W", "-n",
                        f"{PACKAGE}/.SudachiRecoveryProbeActivity",
                        "--es", "run", token, "--es", "phase", phase,
                    )
                    if "Error" in launch.stdout or "Error" in launch.stderr:
                        raise RuntimeError(f"Probe launch failed: {launch.stdout}{launch.stderr}")
                    measured = read_report(f"{directory}/{phase}.json")
                    pair[phase] = measured
                    report_path.write_text(json.dumps(report, indent=2) + "\n")
                    if measured["status"] != "PASS" or not measured["mapped"]:
                        raise RuntimeError(f"Probe validation failed: {measured}")
                    if str(measured["pid"]) not in [
                        line.split()[0] for line in app_processes()
                    ]:
                        raise RuntimeError("Measured app process is no longer alive")
                    stop()  # Actual death while the validated analyzer is still mapped.
                    if device(
                        "run-as", PACKAGE, "test", "-e",
                        f"{directory}/{phase}-closed.json", check=False,
                    ).returncode == 0:
                        raise RuntimeError("Probe closed gracefully before process death")
                before, after = pair["before"], pair["after"]
                if before["pid"] == after["pid"]:
                    raise RuntimeError("Recovery reused the original process")
                for field in ["inode", "mtime", "size", "mode"]:
                    if before[field] != after[field]:
                        raise RuntimeError(f"Recovery replaced cached dictionary: {field}")
                if before["size"] != 330_506_339 or before["mode"] & 0o222:
                    raise RuntimeError("Published dictionary size/permissions changed")
                pair["status"] = "PASS"
                print(f"PASS: death/recovery pair {index + 1}/{args.runs}", flush=True)
            finally:
                stop()  # No removal while any analyzer could still map these inodes.
                device("run-as", PACKAGE, "rm", "-rf", directory)
        report["status"] = "PASS"
    except Exception as error:
        report["status"] = "FAIL"
        report["error"] = str(error)
        raise
    finally:
        report_path.write_text(json.dumps(report, indent=2) + "\n")
    # Leave the ordinary app available; the debug probe is never a startup caller.
    device("am", "start", "-W", "-n", f"{PACKAGE}/.MainActivity")
    print(f"PASS: process death and verified recovery. Report: {report_path}")


if __name__ == "__main__":
    main()
