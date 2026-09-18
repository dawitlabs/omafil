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

## GTK4 toolkit spike

`spike/gtk4` is a minimal GTK4 file list in Rust — `DirectoryList` feeding a
`ListView` through a `SignalListItemFactory`, the idiomatic path a real port
would take. Same window size and release profile as omafil. It has no
operations, search, thumbnails, tabs, theming or D-Bus service, so read it as a
**floor**, not a like-for-like comparison.

All three launched interleaved in one session, 3 repetitions, medians:

| Files | GTK4 spike | omafil (Tauri) | Nautilus |
| --- | --- | --- | --- |
| 1,000 | **346 ms · 32 MB** | 1045 ms · 233 MB | 986 ms · 97 MB |
| 10,000 | **355 ms · 39 MB** | 1143 ms · 232 MB | 1055 ms · 102 MB |
| 50,000 | **358 ms · 71 MB** | 1103 ms · 233 MB | 842 ms · 96 MB |

The spike starts about 2.8x faster than Nautilus and 3x faster than omafil, on
a third of Nautilus's memory and a seventh of omafil's.

**Absolute figures drift between sessions.** Nautilus measured 560 ms in the
run recorded above and 986 ms here under a busier machine. Only comparisons
within a single interleaved run are meaningful; do not compare a number in this
section against one in the section above.

### Shape

- **Startup is flat for the spike and for omafil**, at every folder size. Both
  map a window before enumerating.
- **The spike's memory scales** with the directory — 32 to 71 MB across
  1,000 to 50,000 files — because `DirectoryList` materialises a `FileInfo` per
  entry. Omafil's paged listing stays flat at 233 MB. The architectures cross
  over somewhere far beyond 50,000 files.
- **Omafil's 233 MB is WebKit**, not the application: it does not move between
  1,000 and 50,000 files.

### Reading

A GTK4 port clears Nautilus on both metrics with room to spare. The gap the
port would actually recover is the WebKit floor; the features the spike lacks
would cost tens of megabytes in GTK, not hundreds. A realistic ported omafil
landing near 400-500 ms and 100-150 MB would still beat Nautilus.

This does not by itself justify the port: it costs the 5,332-line Svelte UI and
the design velocity that produced it. It establishes only that the ceiling is
real and measured rather than estimated.

## GTK4 vertical slice

`spike/gtk4` now compiles omafil's own `listing.rs`, `paths.rs` and `omarchy.rs`
from `src-tauri/src` directly — via `#[path]`, not copied — behind a GTK4
`ColumnView` with name, size and modified columns, Omarchy's `colors.toml`
applied as GTK CSS, and ColumnView's built-in keyboard navigation. It calls
`read_directory_listing_revealing`, the same entry point the Tauri command uses.

Still missing: operations, search, thumbnails, tabs, split panes, the inspector,
drives, context menus, drag and drop, rename, undo and the D-Bus service.

| Files | GTK4 slice | omafil (Tauri) | Nautilus |
| --- | --- | --- | --- |
| 1,000 | **553 ms · 37 MB** | 1022 ms · 233 MB | 1035 ms · 96 MB |
| 10,000 | **453 ms · 37 MB** | 1120 ms · 233 MB | 844 ms · 102 MB |
| 50,000 | **521 ms · 37 MB** | 1112 ms · 233 MB | 1002 ms · 92 MB |

### Reading

**The real backend costs about 150 ms over the bare spike** (350 ms to ~500 ms)
and almost nothing in memory. Theme parsing and the first listing are the
difference.

**Memory is flat at 37 MB across every directory size**, where the bare spike
scaled 32 to 71 MB. The difference is omafil's paged listing: `MAX_PAGE_SIZE`
entries regardless of how large the folder is. The architecture that could not
pay off under WebKit's floor pays off immediately without it.

**Against Nautilus: roughly 2x faster on about 40% of the memory**, with the
real listing code and live theming already in place.

The features still missing are GTK widgets over Rust that mostly already exists
and is already counted here. They will add tens of megabytes, not hundreds.
Nothing in these numbers suggests a ported omafil would fail to beat Nautilus.

### Change required in the main crate

`DirectoryEntry` and `DirectoryListing` fields became `pub(crate)` so a second
binary in the workspace can read them. No external API changed and the library
suite still passes; the simpler `read_directory_listing` and
`read_directory_page` wrappers remain `#[cfg(test)]`.
