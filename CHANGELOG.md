# Changelog

## 0.2.3 — 2026-09-15

### Added

- The sidebar collapses to a rail of icons, which gives a tiled window back most of its width. Hovering an icon still names it, and the choice is remembered.

### Fixed

- 0.2.2's click fix never applied when omafil was opened from the app menu. Omarchy exports `GDK_BACKEND=wayland,x11,*` for the whole session, and the check treated the variable merely being set as a deliberate choice of Wayland, so it stepped aside. Only a value naming no Wayland backend is left alone now; `OMAFIL_BACKEND=wayland` overrules it.

## 0.2.2 — 2026-09-15

### Fixed

- Clicks landed in the wrong place on a fractionally scaled display: WebKitGTK reported pointer coordinates against a viewport far wider than the window, so a click aimed at the settings gear arrived a couple of hundred pixels to its left and nothing responded. omafil now runs under XWayland, which does not have the fault, and takes the display scale from the compositor so everything still draws at the size the rest of the desktop uses.
- A narrow window put the right-hand commands outside it. The command bar wraps instead of scrolling with its scrollbar hidden, the Type and Date columns give way as the window narrows, and the search field yields before the header overflows.
- The breadcrumb disappeared below about 700px, because the search field was sized first and left it no width. Crumbs also shrank to their own padding; they keep their width now and the trail scrolls, held at the end so the folder you are in is the one on show.
- A context menu near the bottom of the window ran off the edge, and a long one had no way to reach its last entries. It stays inside the window and scrolls when it has to.
- Emptying the Recycle Bin asked through the webview's own confirm() box, which ignored the theme. It uses the same dialog the rest of the app does, and those dialogs now sit centred rather than against the left edge.
- `omafil-bin` no longer builds an empty debug package alongside itself.

## 0.2.1 — 2026-09-15

### Added

- Type in a folder to filter it: the list narrows to names containing what you typed, the top match is selected so Enter opens it, Backspace edits and Esc restores. Off while vim keys are on.
- `omafil-bin` on the AUR, so installing no longer needs a Rust toolchain or a compile.

### Fixed

- Ctrl+C and Ctrl+X acted on whatever the type-ahead jump had landed on rather than the selection.

### Changed

- A folder listing now stats only the page it returns when sorting by name or type, instead of every entry in the directory on every page request.
- The AUR build no longer reruns the test suite, which had it compile the whole crate graph a second time.

## 0.2.0 — 2026-09-15

### Added

- Undo the last file operation with Ctrl+Z: moves, copies, renames, bulk renames, new folders and trashing.
- Extract tar, gz, bz2, xz, zst, lz4, 7z, iso, cab and rar, alongside the ZIP support that was already there.
- Expand a drive in the sidebar into a folder tree, as deep as you open it.
- The full set of folder operations on pinned folders, known locations and every folder in that tree, with inline rename.
- Format a removable drive through UDisks2, with a confirmation naming the device and its size.
- Clear the list of recently used files.

### Fixed

- Paste now reads the desktop clipboard, so files copied in another file manager arrive here and files copied here arrive elsewhere.
- Ctrl+C, Ctrl+X, Ctrl+V and Ctrl+A no longer need the folder list to hold focus.
- The titlebar close button and header dragging were blocked by a read-only permission set and now work; minimize and maximize are gone, because Hyprland does neither.
- Show hidden files, default sort and sort direction now apply to every open tab instead of only reloading a view that ignores them.
- Dropped the microphone icon from the search field.

## 0.1.0 — 2026-09-12

First release.

- Follows the active Omarchy theme and icon theme live.
- Terminal and editor from any folder, optional vim keys, command-line path.
- Tabs, split view, search, pins, tags, recent files, recycle bin.
- Cancellable copy, move and ZIP with byte progress and staged publication.
- Open with, default app selection, permissions editing, bulk rename.
- Removable drive mount, unmount and eject.
- Image, text and PDF previews; finish notifications; local error log.
