#!/usr/bin/env python3
"""SIGKILL an actual background TXT batch; verify committed files and no resume.

Install current debug and androidTest APKs first. Uses an isolated database, the
test APK's gated URI provider and a debug-only DUMP-protected Activity. Tests both
notification permission states, restoring the original permission afterward.
Stops app.yomoku but never clears its data or deletes its normal Library.
This is abrupt process death, not a low-memory-killer or storage-cache experiment.
"""

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import time
import uuid


REPO = Path(__file__).resolve().parent.parent
PACKAGE = "app.yomoku"
PERMISSION = "android.permission.POST_NOTIFICATIONS"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serial", required=True)
    args = parser.parse_args()
    sdk = Path(os.environ.get("ANDROID_HOME", Path.home() / "Android/Sdk"))
    adb = [str(sdk / "platform-tools/adb"), "-s", args.serial]

    def device(*command, check=True):
        result = subprocess.run(
            [*adb, "shell", shlex.join(command)],
            capture_output=True, text=True, timeout=30,
        )
        if check and result.returncode:
            raise RuntimeError(f"{shlex.join(command)}: {result.stdout}{result.stderr}")
        return result

    if int(device("getprop", "ro.build.version.sdk").stdout) < 33:
        parser.error("This permission/death acceptance runner requires API 33+")
    if device("run-as", PACKAGE, "pwd", check=False).returncode:
        parser.error("Install the current debug APK first")
    if not device("pm", "path", f"{PACKAGE}.test").stdout.strip():
        parser.error("Install the current androidTest APK with its gated provider first")
    permissions = device("dumpsys", "package", PACKAGE).stdout
    match = re.search(r"android\.permission\.POST_NOTIFICATIONS: granted=(true|false)", permissions)
    if match is None:
        parser.error("Installed debug APK does not declare POST_NOTIFICATIONS")
    originally_granted = match.group(1) == "true"

    def processes():
        lines = device("ps", "-A", "-o", "PID,NAME").stdout.splitlines()[1:]
        return [line.split()[0] for line in lines if line.split()[-1] == PACKAGE]

    def stop():
        device("am", "force-stop", PACKAGE)
        if processes():
            raise RuntimeError("App survived force-stop")

    def read_report(path):
        deadline = time.monotonic() + 100
        while time.monotonic() < deadline:
            result = device("run-as", PACKAGE, "cat", path, check=False)
            if result.returncode == 0:
                report = json.loads(result.stdout)
                if report["status"] != "PASS":
                    raise RuntimeError(f"Probe failed: {report}")
                return report
            time.sleep(0.25)
        raise TimeoutError(f"Missing report: {path}")

    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    report_path = REPO / f".local/test-reports/txt-process-death/{timestamp}.json"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report = {
        "serial": args.serial,
        "method": "HOME then run-as SIGKILL; no onDestroy, automatic resume or data clear",
        "fingerprint": device("getprop", "ro.build.fingerprint").stdout.strip(),
        "abi": device("getprop", "ro.product.cpu.abi").stdout.strip(),
        "runs": [],
    }
    print(f"Report: {report_path}", flush=True)
    try:
        for granted in [True, False]:
            stop()
            device("pm", "grant" if granted else "revoke", PACKAGE, PERMISSION)
            token = uuid.uuid4().hex
            directory = f"no_backup/txt-recovery-{token}"
            pair = {"run": token, "notifications_granted": granted}
            report["runs"].append(pair)
            for phase in ["before", "after"]:
                launch = device(
                    "am", "start", "-W", "-n", f"{PACKAGE}/.TxtImportRecoveryProbeActivity",
                    "--es", "run", token, "--es", "phase", phase,
                )
                if "Error" in launch.stdout + launch.stderr:
                    raise RuntimeError(f"Probe launch failed: {launch.stdout}{launch.stderr}")
                measured = read_report(f"{directory}/{phase}.json")
                pair[phase] = measured
                report_path.write_text(json.dumps(report, indent=2) + "\n")
                if measured["notifications_enabled"] != granted:
                    raise RuntimeError("Notification/channel permission differs from requested mode")
                if phase == "before":
                    if not (measured["active"] and measured["foreground_service"]
                            and measured["processed"] == 1 and measured["total"] == 3
                            and measured["documents"] == 1):
                        raise RuntimeError(f"Expected one committed + one blocked + one queued: {measured}")
                    device("input", "keyevent", "KEYCODE_HOME")
                    pid = str(measured["pid"])
                    if pid not in processes():
                        raise RuntimeError("Import process exited before SIGKILL")
                    device("run-as", PACKAGE, "kill", "-9", pid)
                    time.sleep(2)
                    if processes():
                        raise RuntimeError("Nonsticky import service restarted the app after SIGKILL")
                    if device("run-as", PACKAGE, "test", "-e",
                              f"{directory}/before-closed.json", check=False).returncode == 0:
                        raise RuntimeError("Import closed gracefully before death")
                elif (measured["active"] or measured["foreground_service"]
                      or measured["processed"] != 0 or measured["total"] != 0
                      or measured["documents"] != 1):
                    raise RuntimeError(f"Batch resumed or committed file lost: {measured}")
            if pair["before"]["pid"] == pair["after"]["pid"]:
                raise RuntimeError("Probe reused the original process")
            pair["status"] = "PASS"
            print(f"PASS: abrupt background death/no resume; notifications={granted}", flush=True)
        report["status"] = "PASS"
    except Exception as error:
        report["status"] = "FAIL"
        report["error"] = str(error)
        raise
    finally:
        stop()
        device("pm", "grant" if originally_granted else "revoke", PACKAGE, PERMISSION)
        report_path.write_text(json.dumps(report, indent=2) + "\n")
    device("am", "start", "-W", "-n", f"{PACKAGE}/.MainActivity")
    print(f"PASS: committed TXT retained, unfinished batch absent. Report: {report_path}")


if __name__ == "__main__":
    main()
