#!/usr/bin/env python3
"""Measure real emulator reclaim with Full mapped, then reuse the same analyzer.

Requires an installed current debug APK. Builds only the small NDK shell helper.
Allocations are bounded, stop once >=64 MiB of dictionary residency is reclaimed,
and have a device-side watchdog. Never drops caches, sends synthetic trim events,
changes device settings or writes published dictionary bytes.
"""

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import selectors
import shlex
import subprocess
import time
import uuid


REPO = Path(__file__).resolve().parent.parent
PACKAGE = "app.yomoku"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serial", required=True)
    parser.add_argument("--max-mib", type=int, default=1536)
    args = parser.parse_args()
    if not 32 <= args.max_mib <= 2048 or args.max_mib % 32:
        parser.error("--max-mib must be a multiple of 32 in 32..2048")
    sdk = Path(os.environ.get("ANDROID_HOME", Path.home() / "Android/Sdk"))
    adb = [str(sdk / "platform-tools/adb"), "-s", args.serial]

    def device(*command, check=True):
        result = subprocess.run(
            [*adb, "shell", shlex.join(command)], capture_output=True,
            text=True, timeout=30,
        )
        if check and result.returncode:
            raise RuntimeError(
                f"ADB failed ({result.returncode}): {shlex.join(command)}\n"
                f"{result.stdout[-1000:]}{result.stderr[-1000:]}"
            )
        return result

    if device("getprop", "ro.kernel.qemu").stdout.strip() != "1":
        parser.error("This experiment is restricted to emulators")
    if device("run-as", PACKAGE, "pwd", check=False).returncode:
        parser.error("Install the current debug APK first")
    total_kib = int(re.search(r"MemTotal:\s+(\d+)", device("cat", "/proc/meminfo").stdout)[1])
    if args.max_mib * 1024 > total_kib * 0.7:
        parser.error("Allocation bound exceeds 70% of emulator RAM")
    abi = device("getprop", "ro.product.cpu.abi").stdout.strip()
    target = {"x86_64": "x86_64-linux-android", "arm64-v8a": "aarch64-linux-android"}[abi]
    ndk = sdk / "ndk/30.0.16248370"
    if "Pkg.Revision = 30.0.16248370" not in (ndk / "source.properties").read_text():
        raise RuntimeError("Install pinned NDK 30.0.16248370")
    binary = REPO / f".local/memory-pressure/{abi}/memory-pressure"
    binary.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run([
        str(ndk / f"toolchains/llvm/prebuilt/linux-x86_64/bin/{target}29-clang"),
        "-std=c11", "-D_GNU_SOURCE", "-O2", "-Wall", "-Wextra", "-Werror",
        str(REPO / "tools/android-memory-pressure.c"), "-o", str(binary),
    ], check=True)

    def app_processes():
        return [line.split()[0] for line in
                device("ps", "-A", "-o", "PID,NAME").stdout.splitlines()[1:]
                if line.split()[-1].split(":")[0] == PACKAGE]

    def stop():
        device("am", "force-stop", PACKAGE)
        deadline = time.monotonic() + 15
        while app_processes():
            if time.monotonic() > deadline:
                raise RuntimeError("App survived force-stop; refusing dictionary cleanup")
            time.sleep(0.2)

    token = uuid.uuid4().hex
    directory = f"no_backup/process-probe-{token}"
    remote = f"/data/local/tmp/yomoku-pressure-{token}"
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    report_path = REPO / f".local/test-reports/sudachi-memory-pressure/{stamp}.json"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report = {"serial": args.serial, "abi": abi, "max_mib": args.max_mib,
              "run": token, "app_directory": directory, "helper_path": remote,
              "fingerprint": device("getprop", "ro.build.fingerprint").stdout.strip(),
              "method": "bounded anonymous allocation; no cache drop/trim/madvise",
              "samples": []}
    print(f"Report: {report_path}", flush=True)

    def save():
        report_path.write_text(json.dumps(report, indent=2) + "\n")

    def read_report(name):
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            result = device("run-as", PACKAGE, "cat", f"{directory}/{name}", check=False)
            if not result.returncode:
                value = json.loads(result.stdout)
                if value["status"] != "PASS":
                    raise RuntimeError(f"App validation failed: {value}")
                return value
            time.sleep(0.25)
        raise TimeoutError(f"No completed app report: {name}")

    def snapshot(pid, mib):
        if str(pid) not in app_processes():
            raise RuntimeError("App died under pressure; same-analyzer reuse is unverified")
        smaps = device("run-as", PACKAGE, "cat", f"/proc/{pid}/smaps").stdout
        rss, size, selected = 0, 0, False
        for line in smaps.splitlines():
            if re.match(r"^[0-9a-f]+-[0-9a-f]+ ", line):
                selected = f"/process-probe-{token}/sudachi/full-" in line
            elif selected and line.startswith("Rss:"):
                rss += int(line.split()[1])
            elif selected and line.startswith("Size:"):
                size += int(line.split()[1])
        if size < 330_506_339 // 1024:
            raise RuntimeError("Full dictionary mapping vanished")
        vmstat = device("cat", "/proc/vmstat").stdout
        counters = {key: int(value) for key, value in (line.split() for line in vmstat.splitlines())
                    if key.startswith(("pgscan", "pgsteal", "pswp", "oom_kill"))}
        psi = device("cat", "/proc/pressure/memory", check=False)
        return {"mib": mib, "dictionary_rss_kib": rss, "dictionary_size_kib": size,
                "vmstat": counters, "meminfo": device("cat", "/proc/meminfo").stdout,
                "psi": {"available": psi.returncode == 0, "output": psi.stdout + psi.stderr},
                "app_status": device("run-as", PACKAGE, "cat", f"/proc/{pid}/status").stdout}

    pressure = None
    selector = selectors.DefaultSelector()
    try:
        stop()
        subprocess.run([*adb, "push", str(binary), remote], check=True, capture_output=True)
        device("chmod", "700", remote)
        device("am", "start", "-W", "-n", f"{PACKAGE}/.MainActivity")
        device("am", "start", "-W", "-n", f"{PACKAGE}/.SudachiRecoveryProbeActivity",
               "--es", "run", token, "--es", "phase", "pressure")
        before = report["before"] = read_report("pressure.json")
        baseline = snapshot(before["pid"], 0)
        report["samples"].append(baseline)
        save()
        pressure = subprocess.Popen(
            [*adb, "shell", shlex.join([remote, str(args.max_mib)])],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        )
        selector.register(pressure.stdout, selectors.EVENT_READ)

        def allocation_reply():
            if not selector.select(timeout=30):
                raise TimeoutError("Pressure helper stalled; stopping allocation")
            line = pressure.stdout.readline()
            if not line:
                raise RuntimeError("Pressure helper exited/killed; see retained samples")
            return json.loads(line)

        helper = allocation_reply()
        report["helper_pid"] = helper["pid"]
        reclaimed = False
        for mib in range(32, args.max_mib + 1, 32):
            pressure.stdin.write("add\n")
            pressure.stdin.flush()
            reply = allocation_reply()
            if reply != {"pid": helper["pid"], "mib": mib}:
                raise RuntimeError(f"Unexpected allocation reply: {reply}")
            if mib % 128 and mib != args.max_mib:
                continue
            sample = snapshot(before["pid"], mib)
            report["samples"].append(sample)
            save()
            reduction = baseline["dictionary_rss_kib"] - sample["dictionary_rss_kib"]
            steals = sample["vmstat"]["pgsteal_file"] - baseline["vmstat"]["pgsteal_file"]
            print(f"{mib} MiB allocated: dictionary {sample['dictionary_rss_kib']} KiB; "
                  f"reclaimed {reduction} KiB", flush=True)
            if reduction >= 64 * 1024 and steals > 0:
                reclaimed = True
                break
        pressure.stdin.write("q\n")
        pressure.stdin.flush()
        pressure.communicate(timeout=15)
        if pressure.returncode != 0:
            raise RuntimeError(f"Pressure helper exit: {pressure.returncode}")
        report["released"] = snapshot(before["pid"], 0)
        device("run-as", PACKAGE, "touch", f"{directory}/revalidate")
        resumed = report["resumed"] = read_report("resumed.json")
        if any(before[key] != resumed[key] for key in ["pid", "generation"]):
            raise RuntimeError("Validation did not reuse the original analyzer/worker")
        report["after_validation"] = snapshot(before["pid"], 0)
        if not reclaimed:
            raise RuntimeError("Bound reached without >=64 MiB measured reclaim; gate stays open")
        report["status"] = "PASS"
    except Exception as error:
        report["status"] = "FAIL"
        report["error"] = str(error)
        raise
    finally:
        # EOF releases memory on failures too. Device alarm is the final fallback.
        if pressure is not None:
            if pressure.stdin and not pressure.stdin.closed:
                try:
                    pressure.stdin.close()
                except BrokenPipeError:
                    pass  # Helper may already have exited or been killed.
            try:
                pressure.wait(timeout=15)
            except subprocess.TimeoutExpired:
                if "helper_pid" in report:
                    device("kill", "-TERM", str(report["helper_pid"]))
                else:
                    pressure.terminate()  # Device-side alarm bounds an unresponsive startup.
                pressure.wait(timeout=15)
        selector.close()
        try:
            stop()
            device("run-as", PACKAGE, "rm", "-rf", directory)
            device("rm", "-f", remote)
            device("am", "start", "-W", "-n", f"{PACKAGE}/.MainActivity")
            report["cleanup"] = "PASS"
        except Exception as error:
            report["cleanup"] = str(error)
            report["status"] = "FAIL"
            raise
        finally:
            save()
    print(f"PASS: real reclaim and same-analyzer validation. Report: {report_path}")


if __name__ == "__main__":
    main()
