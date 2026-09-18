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

## GTK4 slice two: navigation, selection, context menu

Adds breadcrumbs from `listing::path_crumbs`, Back/Up with history, folder
activation on Enter and double click, multi-selection with a live count, a
right-click menu, and a rename dialog backed by `operations::rename_entry`.
That pulls omafil's whole I/O chain into the binary — `operation_io`,
`recycle`, and the `rustix`, `tempfile` and `trash` crates. Destructive actions
are deliberately not wired.

| Files | GTK4 slice 2 | omafil (Tauri) | Nautilus |
| --- | --- | --- | --- |
| 1,000 | **686 ms · 38 MB** | 1127 ms · 233 MB | 1001 ms · 89 MB |
| 10,000 | **596 ms · 38 MB** | 1030 ms · 234 MB | 921 ms · 105 MB |
| 50,000 | **695 ms · 38 MB** | 1081 ms · 233 MB | 835 ms · 97 MB |

### Cost of each slice

| | Startup | Memory | Binary |
| --- | --- | --- | --- |
| Bare list | ~350 ms | 32-71 MB | 328 KB |
| + real listing and theme | ~500 ms | 37 MB | 444 KB |
| + navigation, selection, menus, I/O chain | ~660 ms | 38 MB | 528 KB |

**Features cost startup, not memory.** The second slice added navigation,
selection, a context menu, a dialog and four crates for **one megabyte**, and
memory stayed flat at 38 MB across every directory size.

### The risk this exposes

Startup has gone 350 to 500 to 660 ms over three slices, roughly 150 ms each,
while Nautilus sits at 835-1000 ms in the same run. Search, thumbnails, tabs,
split panes, the inspector, drives, drag and drop, undo and the D-Bus service
are all still missing. At 150 ms per subsystem the startup lead would be gone
before the port is feature-complete.

**The memory conclusion is robust; the startup conclusion is not yet.** 38 MB
against Nautilus's ~97 MB has enormous headroom. A 200 ms lead does not. The
next slice should be whichever subsystem is most likely to be expensive at
startup — the D-Bus service and drive enumeration are the candidates — to learn
whether the curve flattens or keeps climbing.

## GTK4 slice three: D-Bus service and drives

Adds the two subsystems most likely to be expensive at launch:
`file_manager_service::serve` on a background thread exactly as the Tauri build
starts it, `drives::read_drives` populating a sidebar — which shells out to
`lsblk` and stats every mount through `sysinfo` — plus the theme watcher and the
`/dev/disk/by-path` drive watcher the real app parks in application state. That
pulls in `zbus`, `sysinfo` and `serde_json`.

| Files | GTK4 slice 3 | omafil (Tauri) | Nautilus |
| --- | --- | --- | --- |
| 1,000 | **523 ms · 40 MB** | 1181 ms · 233 MB | 961 ms · 97 MB |
| 10,000 | **604 ms · 40 MB** | 1130 ms · 233 MB | 864 ms · 100 MB |
| 50,000 | **725 ms · 39 MB** | 1077 ms · 233 MB | 895 ms · 96 MB |

### The curve flattened

| | Startup | Memory | Binary |
| --- | --- | --- | --- |
| Bare list | ~350 ms | 32-71 MB | 328 KB |
| + real listing and theme | ~500 ms | 37 MB | 444 KB |
| + navigation, selection, menus, I/O chain | ~660 ms | 38 MB | 528 KB |
| + D-Bus service, drives, watchers | **~620 ms** | 40 MB | 2.1 MB |

Slice three is **indistinguishable from slice two** despite adding two
heavyweight subsystems and three crates. Run-to-run noise on this machine is
roughly +/-150 ms, and the two slices overlap well within it. Binary size grew
four-fold; startup did not move.

The 350 to 500 to 660 ms climb was therefore **front-loaded on the first real
work** — parsing the theme and reading the first directory — not a per-feature
tax. Starting the bus service on a thread keeps it off the launch path
entirely, and `lsblk` is cheap.

### Where this leaves the port

Both conclusions now hold:

- **Memory:** ~40 MB flat at every directory size against Nautilus's ~97 MB.
- **Startup:** ~620 ms against Nautilus's ~900 ms, and no longer trending toward
  it as subsystems land.

Still missing: search, thumbnails, tabs, split panes, the inspector, drag and
drop and undo. Search and thumbnails are lazy by nature and should not touch
launch. Tabs and split panes are widget construction, which slice two showed is
close to free.

Nothing measured so far suggests a ported omafil would fail to beat Nautilus on
both axes.

## Time to a readable list

`scripts/bench_usable.py` captures the window with `grim` and reports the first
moment the contents have both appeared and stopped changing. Two repetitions
per cell, medians.

The accessibility tree would have been a cleaner signal, but GTK creates
accessible objects lazily and exposes no rows without a screen reader attached;
forcing `GTK_A11Y=atspi` did not change that.

| Files | GTK4 slice | omafil (Tauri) | Nautilus |
| --- | --- | --- | --- |
| 1,000 | **1061 ms** | 1379 ms | 1155 ms |
| 10,000 | **1049 ms** | 1338 ms | 1096 ms |
| 50,000 | **1077 ms** | 1415 ms | 1298 ms |

### This reverses a conclusion

**Omafil is slower than Nautilus to a readable list at every size**, by 17 to
22 percent, despite the window-mapped figures putting it ahead. The window maps
early and empty, and the earlier table flattered it exactly as suspected.

**The GTK4 slice is flat**: 1061, 1049, 1077 ms from 1,000 to 50,000 files.
Nautilus grows, from 1155 to 1298 ms, so the slice's advantage widens with
directory size — 8 percent at 1,000 files, 17 percent at 50,000. That is the
paged listing showing up on the metric that matters, and it is the shape that
would keep widening past 100,000 entries.

### Caveats

- **Everything here includes Hyprland's window-open animation.** Frames stop
  changing only once the animation finishes, which adds a constant to every
  figure and compresses the differences between apps. The ordering is sound;
  the absolute values are inflated and the relative gaps are understated.
- "Stopped changing" is a proxy for "readable". An app that paints rows and
  then adjusts them would be scored late.
- Two repetitions per cell, though the spread within each was under 3 percent.

### Net

On window-mapped, omafil beat Nautilus. On time to a readable list, it loses.
The GTK4 slice wins on both, and is the only one of the three whose time does
not grow with the directory.
