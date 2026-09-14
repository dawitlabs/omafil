# Changelog

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
