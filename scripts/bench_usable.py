#!/usr/bin/env python3
"""Time from launch until the file list is actually on screen.

`bench.py` measures when a window appears, which is not the same thing: a
window can map empty and fill later. This captures the window with grim and
reports the first moment the contents have both appeared and stopped changing.

The accessibility tree would be a cleaner signal, but GTK creates accessible
objects lazily and never exposes the rows without a screen reader attached.

Must run under /usr/bin/python3 — grim's output is decoded with the system
Pillow.

usage: /usr/bin/python3 scripts/bench_usable.py <app> <folder> [reps]
"""
import io
import json
import os
import statistics
import subprocess
import sys
import time

from PIL import Image, ImageChops, ImageStat

TIMEOUT_SECS = 20
POLL_SECS = 0.05
THUMB = (160, 120)
# A blank window is nearly uniform; rows of text are not. Below this the frame
# is still an empty shell.
CONTENT_STDDEV = 6.0
# Two consecutive frames this close count as settled.
SETTLED_DIFF = 1.0


def signature():
    found = os.environ.get("HYPRLAND_INSTANCE_SIGNATURE")
    if found:
        return found
    runtime = os.environ.get("XDG_RUNTIME_DIR", "")
    instances = sorted(os.listdir(os.path.join(runtime, "hypr"))) if runtime else []
    if not instances:
        sys.exit("Hyprland is not running.")
    return instances[-1]


def geometry_for(pid, sig):
    env = {**os.environ, "HYPRLAND_INSTANCE_SIGNATURE": sig}
    try:
        raw = subprocess.run(["hyprctl", "-j", "clients"], capture_output=True, text=True, env=env, timeout=5).stdout
        for client in json.loads(raw):
            if client.get("pid") == pid:
                x, y = client["at"]
                width, height = client["size"]
                return f"{x},{y} {width}x{height}"
    except Exception:
        return None
    return None


def frame(region):
    shot = subprocess.run(["grim", "-l", "1", "-g", region, "-"], capture_output=True, timeout=5)
    if shot.returncode != 0 or not shot.stdout:
        return None
    image = Image.open(io.BytesIO(shot.stdout)).convert("L").resize(THUMB)
    return image


def settle(binary):
    while subprocess.run(["pgrep", "-x", binary], capture_output=True).returncode == 0:
        subprocess.run(["pkill", "-x", binary], capture_output=True)
        time.sleep(0.3)
    time.sleep(0.4)


def measure(command, folder, sig):
    binary = command.rsplit("/", 1)[-1]
    settle(binary)

    started = time.perf_counter()
    child = subprocess.Popen([command, folder], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    region, previous, readable = None, None, None

    while time.perf_counter() - started < TIMEOUT_SECS:
        if region is None:
            region = geometry_for(child.pid, sig)
            if region is None:
                time.sleep(POLL_SECS)
                continue
        current = frame(region)
        if current is not None:
            has_content = ImageStat.Stat(current).stddev[0] > CONTENT_STDDEV
            if has_content and previous is not None:
                drift = ImageStat.Stat(ImageChops.difference(current, previous)).mean[0]
                if drift < SETTLED_DIFF:
                    readable = time.perf_counter() - started
                    break
            previous = current
        time.sleep(POLL_SECS)

    child.terminate()
    try:
        child.wait(timeout=10)
    except subprocess.TimeoutExpired:
        child.kill()
    settle(binary)
    return readable


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    command, folder = sys.argv[1], sys.argv[2]
    reps = int(sys.argv[3]) if len(sys.argv) > 3 else 3

    sig = signature()
    times = []
    for rep in range(reps):
        elapsed = measure(command, folder, sig)
        if elapsed is None:
            print(f"  run {rep + 1}: never settled within {TIMEOUT_SECS}s")
            continue
        times.append(elapsed)
        print(f"  run {rep + 1}: {elapsed * 1000:.0f} ms")
    if times:
        print(f"  median {statistics.median(times) * 1000:.0f} ms   best {min(times) * 1000:.0f} ms")


if __name__ == "__main__":
    main()
