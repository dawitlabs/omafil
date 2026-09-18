#!/usr/bin/env python3
"""Startup and memory comparison between omafil and another file manager.

Measures launch -> window mapped (via the Hyprland event socket) and the
resident memory of the whole process tree once the window settles. Tauri and
GTK both spawn helper processes, so only the tree total is a fair comparison.

It does NOT measure time to a painted file list: nothing here introspects
either app's rendering. Nor does it measure thumbnail cost, because the
fixtures hold no media. Both need their own harness.

usage: scripts/bench.py [--reps N] [--sizes 1000,10000] [--apps omafil,nautilus]
"""
import argparse
import json
import os
import shutil
import socket
import statistics
import subprocess
import sys
import time
from pathlib import Path

FIXTURES = Path.home() / ".cache/omafil-bench"
SETTLE_SECS = 1.5
WINDOW_TIMEOUT_SECS = 20
# Fixtures live on disk: /tmp is tmpfs here, and these file counts would be
# charged to RAM.
MIN_FREE_BYTES = 2 * 1024**3


def hypr_socket():
    signature = os.environ.get("HYPRLAND_INSTANCE_SIGNATURE")
    runtime = os.environ.get("XDG_RUNTIME_DIR", "")
    if not signature:
        instances = sorted(Path(runtime, "hypr").glob("*_*")) if runtime else []
        if not instances:
            sys.exit("Hyprland is not running; this harness reads its event socket.")
        signature = instances[-1].name
    return Path(runtime, "hypr", signature, ".socket2.sock"), signature


def hyprctl(signature, *args):
    env = {**os.environ, "HYPRLAND_INSTANCE_SIGNATURE": signature}
    done = subprocess.run(["hyprctl", *args], capture_output=True, text=True, env=env, timeout=10)
    return done.stdout.strip()


def build_fixture(count):
    folder = FIXTURES / f"files-{count}"
    marker = folder / ".complete"
    if marker.exists():
        return folder
    shutil.rmtree(folder, ignore_errors=True)
    folder.mkdir(parents=True)
    for index in range(count):
        (folder / f"item-{index:06d}.txt").touch()
    marker.touch()
    return folder


def tree_pss_kb(root_pid):
    """Sum PSS across the process and its descendants.

    Proportional set size, not RSS: a Tauri app runs several WebKit processes
    that share their mappings, and summing RSS would count those shared pages
    once per process and make a multi-process app look far heavier than it is.
    """
    pids, total = [root_pid], 0
    seen = set()
    while pids:
        pid = pids.pop()
        if pid in seen:
            continue
        seen.add(pid)
        try:
            rollup = Path(f"/proc/{pid}/smaps_rollup").read_text()
            children = Path(f"/proc/{pid}/task/{pid}/children").read_text().split()
        except OSError:
            continue
        for line in rollup.splitlines():
            if line.startswith("Pss:"):
                total += int(line.split()[1])
        pids.extend(int(child) for child in children)
    return total


def measure(command, folder, signature):
    """Launch, wait for the first window to map, then read the tree's memory."""
    events = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    events.connect(str(hypr_socket()[0]))
    events.settimeout(WINDOW_TIMEOUT_SECS)

    started = time.perf_counter()
    child = subprocess.Popen([*command, str(folder)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    mapped, window_class, buffer = None, "", b""
    try:
        while mapped is None:
            chunk = events.recv(4096)
            if not chunk:
                break
            buffer += chunk
            for line in buffer.split(b"\n")[:-1]:
                if line.startswith(b"openwindow>>"):
                    mapped = time.perf_counter()
                    window_class = line.decode("utf-8", "replace").split(",")[2]
                    break
            buffer = buffer.split(b"\n")[-1]
    except socket.timeout:
        pass
    finally:
        events.close()

    rss = 0
    if mapped is not None:
        time.sleep(SETTLE_SECS)
        rss = tree_pss_kb(child.pid)

    child.terminate()
    try:
        child.wait(timeout=10)
    except subprocess.TimeoutExpired:
        child.kill()
    return (None if mapped is None else mapped - started), rss, window_class


def wait_until_gone(app, timeout=15):
    """Both apps forward a second launch to an already-running instance, so a
    leftover process would make every repeat measure forwarding, not startup."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        if subprocess.run(["pgrep", "-x", app], capture_output=True).returncode != 0:
            time.sleep(0.4)
            return True
        subprocess.run(["pkill", "-x", app], capture_output=True)
        time.sleep(0.3)
    return False


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--reps", type=int, default=5)
    parser.add_argument("--sizes", default="1000,10000,50000")
    parser.add_argument("--apps", default="omafil,nautilus")
    options = parser.parse_args()

    _, signature = hypr_socket()
    sizes = [int(value) for value in options.sizes.split(",")]
    apps = [name.strip() for name in options.apps.split(",")]

    for app in apps:
        if shutil.which(app) is None:
            sys.exit(f"{app} is not installed.")

    FIXTURES.mkdir(parents=True, exist_ok=True)
    if shutil.disk_usage(FIXTURES).free < MIN_FREE_BYTES:
        sys.exit("Less than 2 GB free; refusing to create fixtures.")

    folders = {}
    for count in sizes:
        print(f"fixture {count} files...", flush=True)
        folders[count] = build_fixture(count)

    home = json.loads(hyprctl(signature, "-j", "activeworkspace")).get("name", "1")
    hyprctl(signature, "dispatch", "workspace", "name:omafil-bench")
    print(f"\nrunning on workspace omafil-bench; returning to {home} when done\n", flush=True)

    rows = []
    try:
        for count in sizes:
            for app in apps:
                launches, memory, seen = [], [], ""
                for rep in range(options.reps):
                    if not wait_until_gone(app):
                        print(f"  {app}: a previous instance will not exit; skipping", flush=True)
                        break
                    elapsed, rss, window_class = measure([app], folders[count], signature)
                    seen = window_class or seen
                    if elapsed is None:
                        print(f"  {app} {count}: no window within {WINDOW_TIMEOUT_SECS}s", flush=True)
                        continue
                    launches.append(elapsed)
                    memory.append(rss)
                    print(f"  {app} {count} run {rep + 1}: {elapsed * 1000:.0f} ms, {rss / 1024:.0f} MB pss", flush=True)
                if launches:
                    rows.append((count, app, seen, statistics.median(launches) * 1000,
                                 min(launches) * 1000, statistics.median(memory) / 1024))
    finally:
        hyprctl(signature, "dispatch", "workspace", f"name:{home}" if not home.isdigit() else home)

    print(f"\n{'files':>7}  {'app':<10} {'class':<22} {'median ms':>10} {'best ms':>9} {'median MB':>10}")
    for count, app, window_class, median_ms, best_ms, median_mb in rows:
        print(f"{count:>7}  {app:<10} {window_class:<22} {median_ms:>10.0f} {best_ms:>9.0f} {median_mb:>10.0f}")
    print("\nFirst run of each pair is cold-ish only in the page-cache sense; dropping\n"
          "caches needs root, so treat these as warm-start numbers.")


if __name__ == "__main__":
    main()
