# Startup and memory against Nautilus

Measured 2026-09-18 with `scripts/bench.py`, 3 repetitions per cell, median
reported. Omafil built from `f40adf8` in release mode; GNOME Nautilus 50.2.2.
Both launched with a folder argument on an otherwise idle Hyprland 0.56.2
session.

## What the numbers are

- **ms** — from `exec` to the compositor reporting `openwindow`.
- **MB** — proportional set size (PSS) of the whole process tree, 1.5 s after
  the window maps. PSS rather than RSS because a Tauri app runs several WebKit
  processes that share mappings; summing RSS counts those pages once per
  process and roughly triples the figure.

| Files | App | Startup (median) | Best | Memory (PSS) |
| --- | --- | --- | --- | --- |
| 1,000 | omafil | 828 ms | 826 ms | 233 MB |
| 1,000 | nautilus | 590 ms | 569 ms | 99 MB |
| 10,000 | omafil | 823 ms | 806 ms | 233 MB |
| 10,000 | nautilus | 557 ms | 546 ms | 112 MB |
| 50,000 | omafil | 833 ms | 821 ms | 233 MB |
| 50,000 | nautilus | 560 ms | 554 ms | 119 MB |

## Reading

**Nautilus wins both, today.** It reaches a window about 1.5x sooner and holds
roughly half the memory.

**Omafil's cost is fixed, not scaling.** Startup (828/823/833 ms) and memory
(233 MB at every size) do not move between 1,000 and 50,000 files: this is
WebKit and runtime overhead, not directory handling. The paged listing is doing
its job.

**Nautilus's memory scales with the directory** — 99 MB to 119 MB across the
same range — so the gap narrows as folders grow, though it does not close
anywhere near 50,000 files.

**No claim of being lighter or faster than Nautilus is currently supportable.**
Omafil's argument is fewer installed packages, live Omarchy theming, split panes
and Vim keys. It is not resource use.

## What this does not measure

- **Time to a usable file list.** Only window-mapped is measured. Neither app is
  introspected for render completion, and the app that maps first is not
  necessarily the app that is usable first. This is the number that matters most
  and it is still missing.
- **Cold start.** Dropping the page cache needs root, so every figure here is a
  warm start.
- **Thumbnail cost.** The fixtures hold empty text files. The thumbnail path is
  never entered; measuring it needs a media fixture.
- **Search latency**, and any directory beyond 50,000 entries.

## Reproducing

```sh
scripts/bench.py --reps 3 --sizes 1000,10000,50000 --apps ./path/to/omafil,nautilus
```

Fixtures are built under `~/.cache/omafil-bench` (on disk deliberately: `/tmp`
is tmpfs here and these file counts would be charged to RAM). The harness runs
on its own Hyprland workspace and returns to the previous one, and kills a
leftover instance between runs — both apps forward a second launch to a running
instance, which would otherwise make every repeat measure forwarding.
