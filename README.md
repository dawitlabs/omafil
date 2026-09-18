# omafil

[![ci](https://github.com/dawitlabs/omafil/actions/workflows/ci.yml/badge.svg)](https://github.com/dawitlabs/omafil/actions/workflows/ci.yml)

A fast file manager for [Omarchy](https://omarchy.org). Tauri 2, Rust and Svelte 5.

```sh
yay -S omafil-bin
```

## What makes it Omarchy's

- **Follows the active theme live.** Every colour comes from the current theme's `colors.toml`, and `omarchy theme set` retints the app on the spot. Off Omarchy it falls back to its own light and dark palettes.
- **Terminal and editor here.** F4 opens a terminal in the current folder through `xdg-terminal-exec`; Edit opens a file with `omarchy launch editor`.
- **Vim keys.** Optional in Settings: `j`/`k` move, `h` goes up, `l` opens, `gg`/`G` jump, `/` searches.
- **Dictation in search.** With Omarchy's `voxtype` installed, the microphone in the search box starts and stops it and shows when it is listening; `F9` and `SUPER+CTRL+X` work too. The control is hidden when voxtype is not installed.
- **Icon theme too.** File and folder icons come from the theme named in `icons.theme`, following its inheritance chain.
- **Desktop opening.** Paths and local `file://` URIs open in tabs, including escaped spaces and Unicode. Subsequent launches reuse the running window. `--select` reveals files in their parent folder and `--properties` opens an item's details.
- **Desktop notifications.** A copy or ZIP that finishes while another window has focus reports through `notify-send`.
- **Launcher entry.** `~/.config/omarchy/extensions/omarchy-menu.jsonc` gets a Files row.

## Files

- Browse the whole filesystem under your normal Linux permissions, with a
  Filesystem shortcut and breadcrumbs back to `/`.
- Standard folders follow XDG user-directory settings, including localized names
  and custom paths. Disabled or missing locations do not appear as shortcuts.
- Tabs, search, pins, tags, recent files, recycle bin.
- Recursive search supports names, wildcards, `type:image`, `ext:pdf` and
  `size:>10mb` filters. It reports unreadable locations and result limits; it does
  not yet provide indexed document-content search.
- Type in a folder to filter it: the list narrows to names containing what you typed, Backspace edits, Esc restores. Off while vim keys are on.
- Cancellable copy, move and ZIP operations with byte progress and staged publication (a cancelled copy never leaves half a file).
- Extract ZIP, tar, gz, bz2, xz, zst, lz4, 7z, iso, cab and rar.
- Split view: two panes on one folder, F6 switches, drag between them.
- Open with any installed app for the file's type, default first; make any of them the default.
- Permissions editing in Properties.
- Rename many items at once with find/replace and a `{name}{ext}{n}` pattern, previewed before anything changes.
- Removable drives mount, unmount and eject through `udisksctl`; plugging in a stick refreshes the list.
- Drag and drop inside the app and from other apps; image, text and PDF previews (`pdftoppm`, cached under `~/.cache/omafil`).
- Grid thumbnails for video, audio, office documents and other types with no
  preview of their own. Anything another file manager already rendered is reused
  from the shared cache under `~/.cache/thumbnails`; the rest go through an
  installed system thumbnailer such as `ffmpegthumbnailer`. Files with no
  thumbnailer keep their icon.
- Panics and unhandled frontend errors land as JSON lines in `~/.local/state/omafil/errors.log`.

## Undo

`Ctrl+Z` reverses the last operation: a move goes back where it came from, a copy
is trashed, a rename reverts, a new folder is removed, and trashed items come
back out of the recycle bin. The last 50 operations are kept.

## Develop

The Share dialog reports AirDrop availability, but native iPhone transfers are
**not supported yet**. The backend includes tested transfer-state and safe
receive-staging primitives; a verified AirDrop transport is still required. See
[the implementation status and research](docs/airdrop.md).

```sh
cd src && bun install
cd ../src-tauri && cargo tauri dev
```

Tests: `cargo test` in `src-tauri`, `bun test` and `bun run check` in `src`.
`src/preview.html` runs the UI under plain `vite` with a mocked backend; `src/tests/shot.mjs` takes
screenshots of it and `src/tests/e2e.mjs` checks the golden paths through it:

```sh
cd src && bunx vite --port 1420 &
PLAYWRIGHT_DIR=<a node_modules holding playwright> node src/tests/e2e.mjs
```

## Install

### Opening from other applications

```sh
omafil ~/Downloads ~/Documents
omafil 'file:///home/me/Project%20Files'
omafil --select ~/Downloads/report.pdf
omafil --properties ~/Downloads/report.pdf
omafil --help
```

Explicitly revealed items appear first, including requested hidden files, without
changing the global hidden-file setting. “Restore normal order” clears this
temporary ordering. A maximum of 12 tabs is supported; overflow is reported.

After installation, **Settings → Desktop integration → Make default** configures
folder opening and the standard `org.freedesktop.FileManager1` interface for
“Show in folder” and single-item properties. Its per-user D-Bus launcher starts
Omafil when it is closed. “Restore previous setup” restores the former folder
association and launcher while preserving later unrelated changes.

Another running file manager may still own the session's service. Close it and
restart Omafil to activate this integration; Omafil never kills or replaces the
owner. Changing the folder association in another application does not remove
Omafil's D-Bus launcher: use “Restore previous setup” to remove that registration.
Remote URIs and combined multi-item properties remain unsupported.

See [the delivery matrix](docs/nautilus-parity.md) for remaining work toward an
Omarchy default candidate. Official inclusion requires an upstream Omarchy decision.

### Packages

Arch and other Arch-based systems, from the AUR:

```sh
yay -S omafil-bin          # prebuilt, installs in seconds
yay -S omafil-git          # builds from source, takes several minutes
omarchy pkg aur add omafil-bin   # on Omarchy
```

Or build the package from this repo. It installs the binary, the
desktop entry and the icons system-wide, so omafil appears in the launcher:

```sh
git clone https://github.com/dawitlabs/omafil
makepkg -si -p omafil/packaging/PKGBUILD
```

Or install into your home directory without a package:

```sh
make install          # installs without changing the default
make default          # optional, reversible desktop integration
make restore-default  # restores the previous integration
make uninstall        # restores a saved setup before removing the local install
```

Choose “Make default” in Settings, or use the equivalent commands:

```sh
omafil --make-default
omafil --restore-default
```

These change per-user folder and D-Bus associations. Omarchy's explicit
file-manager shortcuts are configured separately in `~/.config/hypr/bindings.lua`:

```lua
hl.unbind("SUPER + SHIFT + F")
hl.unbind("SUPER + ALT + SHIFT + F")
o.bind("SUPER + SHIFT + F", "File manager", { launch = "omafil" })
o.bind("SUPER + ALT + SHIFT + F", "File manager (cwd)", { launch = "sh -c 'omafil \"$(omarchy-cmd-terminal-cwd)\"'" })
```

There is no graphical installer. Setup requires the installed `omafil` executable
on PATH and its desktop entry; development builds cannot select themselves as
the default accidentally. A tagged `v*` release also carries an
AppImage, which runs standalone but registers no desktop entry or file
associations, so it never becomes the system file manager.

CI runs the tests on every push and attaches the AppImage to tagged releases.

The real registration/restore test uses disposable XDG directories:

```sh
cargo build --manifest-path src-tauri/Cargo.toml --features tauri/custom-protocol
python3 src/tests/desktop-integration.py
# On a graphical desktop, also exercise cold D-Bus activation:
python3 src/tests/desktop-integration.py --native
```

### Clipboard and naming

Use **Ctrl+C / Ctrl+X / Ctrl+V**, or **Paste**, to transfer files and folders.
Desktop clipboard integration requires `wl-clipboard` on Wayland. Copied PNG,
JPEG, and WebP images paste as `Clipboard image.png` (or the matching extension)
in the current folder. Repeated image pastes use a free numbered name; **Ctrl+Z**
undoes a paste. Image clipboard reads are limited to 64 MiB.

After **New folder**, type the name and press **Enter** to save it while staying
in the parent folder. **Escape** cancels the name edit. To rename an existing
item, select it and click its name again after a brief pause, or press **F2**.
Double-click continues to open an item. Adjust text under **Settings → Appearance
→ Font size**; the setting is saved between launches.

### Network, phones and content search

**Network & Devices** connects to SFTP, SMB, WebDAV (`dav://` / `davs://`), FTP
and FTPS using the installed GIO/GVfs providers. Enter an address without login
credentials; native desktop dialogs handle sign-in. Remembered server addresses
exclude credentials. Refresh devices after connecting or unlocking a phone.

Open remote folders, select items and choose **Copy to Downloads**. To upload,
copy local files first, then use **Paste copied files here**. Existing destination
names are never overwritten. Interrupted copies can leave incomplete destination
files; inspect them before retrying. Remote moves, rename, trash and Undo are not
yet available. The remote view currently shows at most 1,000 entries.

On Arch, these features use optional `python-gobject` and `gvfs` packages, plus
`gvfs-smb`, `gvfs-mtp`, `gvfs-gphoto2` or `gvfs-afc` for the corresponding provider.
Actual phone access depends on device support, unlocking/trust and the selected
USB mode. Presence in the device list is not a successful-transfer guarantee.

**Content search** uses optional `localsearch` and `tinysparql` to search indexed
document text and metadata. Choose a local folder and words, then filter the
returned matches by type/date or reveal one in its folder. Coverage is limited to
the existing index and supported extractors, up to 500 matches. **Search filenames
instead** retains the chosen scope and works without the indexer. Omafil does not
silently enable indexing or change indexed folders.

Keyboard improvements include Shift+Arrow range selection, Ctrl+Space selection
toggling, and Left/Right/Home/End navigation when a tab has focus. New network and
search controls expose labels, status messages, cancellation and visible focus.
See [provider verification and remaining limits](docs/verification/2026-09-18-linux-providers.md)
for the distinction between native service tests, browser tests and pending
physical-device/screen-reader checks.
