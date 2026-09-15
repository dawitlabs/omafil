# omafil

[![ci](https://github.com/dawitlabs/omafil/actions/workflows/ci.yml/badge.svg)](https://github.com/dawitlabs/omafil/actions/workflows/ci.yml)

A fast file manager for [Omarchy](https://omarchy.org). Tauri 2, Rust and Svelte 5.

```sh
yay -S omafil-git
```

## What makes it Omarchy's

- **Follows the active theme live.** Every colour comes from the current theme's `colors.toml`, and `omarchy theme set` retints the app on the spot. Off Omarchy it falls back to its own light and dark palettes.
- **Terminal and editor here.** F4 opens a terminal in the current folder through `xdg-terminal-exec`; Edit opens a file with `omarchy launch editor`.
- **Vim keys.** Optional in Settings: `j`/`k` move, `h` goes up, `l` opens, `gg`/`G` jump, `/` searches.
- **Icon theme too.** File and folder icons come from the theme named in `icons.theme`, following its inheritance chain.
- **Command-line path.** `omafil ~/Downloads` opens straight there, so it works as the `inode/directory` handler and with the Hyprland cwd binding.
- **Desktop notifications.** A copy or ZIP that finishes while another window has focus reports through `notify-send`.
- **Launcher entry.** `~/.config/omarchy/extensions/omarchy-menu.jsonc` gets a Files row.

## Files

- Tabs, search, pins, tags, recent files, recycle bin.
- Cancellable copy, move and ZIP operations with byte progress and staged publication (a cancelled copy never leaves half a file).
- Extract ZIP, tar, gz, bz2, xz, zst, lz4, 7z, iso, cab and rar.
- Split view: two panes on one folder, F6 switches, drag between them.
- Open with any installed app for the file's type, default first; make any of them the default.
- Permissions editing in Properties.
- Rename many items at once with find/replace and a `{name}{ext}{n}` pattern, previewed before anything changes.
- Removable drives mount, unmount and eject through `udisksctl`; plugging in a stick refreshes the list.
- Drag and drop inside the app and from other apps; image, text and PDF previews (`pdftoppm`, cached under `~/.cache/omafil`).
- Panics and unhandled frontend errors land as JSON lines in `~/.local/state/omafil/errors.log`.

## Undo

`Ctrl+Z` reverses the last operation: a move goes back where it came from, a copy
is trashed, a rename reverts, a new folder is removed, and trashed items come
back out of the recycle bin. The last 50 operations are kept.

## Develop

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

Arch and other Arch-based systems, from the AUR:

```sh
yay -S omafil-git          # or: paru -S omafil-git
omarchy pkg aur add omafil-git   # on Omarchy
```

Or build the package from this repo. It installs the binary, the
desktop entry and the icons system-wide, so omafil appears in the launcher:

```sh
git clone https://github.com/dawitlabs/omafil
makepkg -si -p omafil/packaging/PKGBUILD
```

Or install into your home directory without a package:

```sh
make install    # ~/.local/bin, plus the desktop entry and icons
make uninstall  # removes them again
```

Either way, make omafil the folder handler and point the Hyprland
file-manager keys at it in `~/.config/hypr/bindings.lua`:

```sh
xdg-mime default omafil.desktop inode/directory
```

```lua
hl.unbind("SUPER + SHIFT + F")
hl.unbind("SUPER + ALT + SHIFT + F")
o.bind("SUPER + SHIFT + F", "File manager", { launch = "omafil" })
o.bind("SUPER + ALT + SHIFT + F", "File manager (cwd)", { launch = "sh -c 'omafil \"$(omarchy-cmd-terminal-cwd)\"'" })
```

There is no graphical installer. A tagged `v*` release also carries an
AppImage, which runs standalone but registers no desktop entry or file
associations, so it never becomes the system file manager.

CI runs the tests on every push and attaches the AppImage to tagged releases.
