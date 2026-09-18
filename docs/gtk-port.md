# GTK4 port

Omafil's Tauri build reaches a readable list 17-22% slower than Nautilus and
holds 2.4x its memory. A GTK4 front end on the same backend beats Nautilus on
both, and its time to a readable list does not grow with directory size. The
measurements are in [the performance record](verification/2026-09-18-performance.md).

The port is therefore a second binary, not a rewrite in place. `omafil-core`
holds everything toolkit-independent; `src-tauri` and `gtk` are two front ends
over it. The Tauri build stays shippable and stays the default until the GTK
build reaches parity.

## Layout

| Crate | Contents |
| --- | --- |
| `core` | Listing, operations, search, previews, thumbnails, archives, drives, clipboard, desktop integration, FileManager1, Omarchy theming. No UI. |
| `src-tauri` | Tauri commands, the operation queue, the GIO service bridge, AirDrop stubs. |
| `gtk` | GTK4 front end. |

## Screens

| Screen | Tauri | GTK | Notes |
| --- | --- | --- | --- |
| File list | done | **done** | ColumnView with icon, name, size, modified. |
| Navigation | done | **done** | Breadcrumbs, Back, Up, activation. |
| Selection | done | **done** | Multi-select, rubber band, keyboard range from ColumnView. |
| Context menu | done | **partial** | Open, Rename, Open Terminal Here. No copy, move, trash, compress, Open With. |
| Sidebar | done | **partial** | Home, Filesystem, pins, XDG directories, drives, tags, icons. No folder tree, no drive mount actions, no pin or tag editing. |
| Theming | done | **done** | `colors.toml` as GTK CSS, applied at startup. Does not yet retint live. |
| Drives | done | **partial** | Listed with free space. No mount, unmount, eject or format. |
| FileManager1 | done | **done** | Served on a background thread, name requested with DoNotQueue. |
| Icons | done | **done** | Names from `core::file_icons`, resolved by GTK against the Omarchy icon theme. |
| Thumbnails | done | not started | `core::thumbnail` is unused by this build. |
| Tabs | done | not started | |
| Split panes | done | not started | |
| Search | done | not started | Filename and indexed. |
| File inspector | done | not started | Properties, permissions. |
| Operations UI | done | not started | Copy, move, ZIP with progress and cancellation. |
| Operation queue | done | not started | Lives in `src-tauri`; needs a home in `core` first. |
| Undo | done | not started | |
| Recycle bin | done | not started | |
| Bulk rename | done | not started | |
| Open With | done | not started | |
| Drag and drop | done | not started | Within the app and from other applications. |
| Command bar | done | not started | |
| Settings | done | not started | |
| Network and devices | done | not started | Needs the GIO bridge moved out of `src-tauri`. |
| Share dialog | done | not started | AirDrop is a stub in both. |
| Dictation | done | not started | `core::dictation` is unused by this build. |

## Rules while porting

- No logic moves into `gtk`. If a front end needs something, it belongs in
  `core` where both can use it.
- The Tauri build keeps passing CI and keeps shipping. It is the default until
  this table has no gaps that matter.
- Every ported screen keeps the states the Svelte build has: loading, empty,
  error, denied, and the keyboard path.
- Destructive operations stay unwired here until undo and the recycle bin are
  ported. A half-ported file manager should not be able to lose data.
