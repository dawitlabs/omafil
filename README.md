# omafil

A fast file manager for [Omarchy](https://omarchy.org). Tauri 2, Rust and Svelte 5.

## What makes it Omarchy's

- **Follows the active theme live.** Every colour comes from the current theme's `colors.toml`, and `omarchy theme set` retints the app on the spot. Off Omarchy it falls back to its own light and dark palettes.
- **Terminal and editor here.** F4 opens a terminal in the current folder through `xdg-terminal-exec`; Edit opens a file with `omarchy launch editor`.
- **Vim keys.** Optional in Settings: `j`/`k` move, `h` goes up, `l` opens, `gg`/`G` jump, `/` searches.
- **Command-line path.** `omafil ~/Downloads` opens straight there, so it works as the `inode/directory` handler and with the Hyprland cwd binding.

## Files

- Tabs, search, pins, tags, recent files, recycle bin.
- Cancellable copy, move and ZIP operations with byte progress and staged publication (a cancelled copy never leaves half a file).
- Open with any installed app for the file's type, default first.
- Permissions editing in Properties.
- Rename many items at once with find/replace and a `{name}{ext}{n}` pattern, previewed before anything changes.
- Removable drives mount, unmount and eject through `udisksctl`; plugging in a stick refreshes the list.
- Drag and drop inside the app and from other apps; image and text previews.

## Develop

```sh
cd src && bun install
cd ../src-tauri && cargo tauri dev
```

Tests: `cargo test` in `src-tauri`, `bun test` and `bun run check` in `src`.
`src/preview.html` runs the UI under plain `vite` with a mocked backend for screenshots.

## Install

```sh
cd src-tauri && cargo tauri build --no-bundle
install -Dm755 target/release/omafil ~/.local/bin/omafil
```

Then point the Hyprland file-manager bindings at `omafil` in `~/.config/hypr/bindings.lua` and make it the folder handler:

```sh
xdg-mime default omafil.desktop inode/directory
```
