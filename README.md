# omafil

[![ci](https://github.com/dawitlabs/omafil/actions/workflows/ci.yml/badge.svg)](https://github.com/dawitlabs/omafil/actions/workflows/ci.yml)

A fast file manager for [Omarchy](https://omarchy.org). Tauri 2, Rust and Svelte 5.

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
- Split view: two panes on one folder, F6 switches, drag between them.
- Open with any installed app for the file's type, default first; make any of them the default.
- Permissions editing in Properties.
- Rename many items at once with find/replace and a `{name}{ext}{n}` pattern, previewed before anything changes.
- Removable drives mount, unmount and eject through `udisksctl`; plugging in a stick refreshes the list.
- Drag and drop inside the app and from other apps; image, text and PDF previews (`pdftoppm`, cached under `~/.cache/omafil`).
- Panics and unhandled frontend errors land as JSON lines in `~/.local/state/omafil/errors.log`.

## Develop

```sh
cd src && bun install
cd ../src-tauri && cargo tauri dev
```

Tests: `cargo test` in `src-tauri`, `bun test` and `bun run check` in `src`.
`src/preview.html` runs the UI under plain `vite` with a mocked backend for screenshots; `src/tests/shot.mjs` drives it with Playwright.

## Install

```sh
make install
```

Builds the release binary, installs it to `~/.local/bin` with the desktop entry and icons, and makes omafil the `inode/directory` handler. Then point the Hyprland file-manager keys at it in `~/.config/hypr/bindings.lua`:

```lua
hl.unbind("SUPER + SHIFT + F")
hl.unbind("SUPER + ALT + SHIFT + F")
o.bind("SUPER + SHIFT + F", "File manager", { launch = "omafil" })
o.bind("SUPER + ALT + SHIFT + F", "File manager (cwd)", { launch = "sh -c 'omafil \"$(omarchy-cmd-terminal-cwd)\"'" })
```

`make uninstall` removes it again. Arch users can build `packaging/PKGBUILD` with `makepkg -si`.

CI runs the tests on every push and attaches an AppImage and a .deb to tagged `v*` releases.
