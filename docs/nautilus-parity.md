# Omafil: Omarchy default candidate

Baseline inspected on 2026-09-17: installed GNOME Nautilus **50.2.2**.
The local Omarchy defaults name Nautilus in `inode/directory` and the file-manager
keybindings. This matrix is a delivery tracker, not a claim of complete parity
or a complete audit of every Nautilus menu and extension.

The product direction retains Rust/Tauri/Svelte, live Omarchy themes, Vim keys,
split panes and tags. Reusing Linux GIO/GVfs and LocalSearch is preferred to
implementing network protocols and document indexing from scratch. Forking
Nautilus would trade the current interface and architecture for closer GNOME
integration; it is not the selected approach.

| Workflow | Current Omafil implementation | Remaining acceptance work |
| --- | --- | --- |
| Local navigation and standard folders | OS permissions, XDG directories, root breadcrumbs, tabs and split panes | Native permission, symlink and disconnected-mount scenarios |
| Copy, move, trash, restore, undo, archives | Existing Rust operations, staging, cancellation and frontend queue | Disk-full, cross-filesystem, disconnect and crash-recovery matrix; hash verification |
| Desktop opening | Decoded local URIs, multiple targets, existing-window forwarding, selection and one-item properties | Desktop application interoperability and compositor activation-token support |
| Show in folder | FileManager1, opt-in cold activation, reversible default registration; existing owner preserved | Combined multi-item properties, broad desktop interoperability |
| Remote locations and phones | GIO URI browser, GTK connection prompts, saved addresses, device discovery and copies to/from local paths; native FTP round-trip verified | Other protocol/authentication matrices and real phone tests; URI tabs, previews, remote moves/rename/trash/undo, pagination and conflict choices |
| Search | Recursive names plus optional LocalSearch indexed text/metadata, scope and hidden filtering, type/date result filters, cancellation, reveal and explicit index coverage limits | Index management/status, streamed continuation, large-index benchmarks and per-pane filename-search cancellation |
| Power workflow | Split panes, Vim keys, bulk rename with preview, tags, terminal/editor actions | Session restoration, command palette, saved searches, folder comparison and user scripts |
| Media and previews | Images, text, PDF and supported media | Thumbnail coverage/cache limits, native codec behavior, large-file responsiveness |
| Accessibility and language | Font scaling, keyboard range/toggle selection, roving tabs, semantic network/search controls and live status | Native screen-reader audit, remaining focus restoration, translations and full keyboard walkthrough |
| Performance | Paged directory listing, selective metadata reads | Reproducible cold/warm startup, RAM, 10k/100k folders, search latency and operation-throughput comparisons |
| Omarchy integration | Live colors/icons, terminal/editor launch, Arch packaging, desktop entry and per-user default-handler rollback | Clean-system package install/upgrade/uninstall, shortcut integration, fractional-scale pointer accuracy |
| Sharing | Availability dialog and backend transfer primitives | No native AirDrop transport; no completed phone-transfer claim |

## Delivery order

1. **Dependable desktop opening**: the 2026-09-17 slice, with automated URI,
   request-queue, pagination, D-Bus and UI checks. Preserve the existing local
   operations foundation and unfinished work in this checkout.
2. **Remote/device parity**: implement the location/provider boundary in the
   approved design before adding remote buttons. Keep credentials in platform
   credential storage, and explicitly model unavailable trash/undo/atomic moves.
3. **Search parity**: optional LocalSearch content/metadata provider, recursive
   fallback with honest coverage, cancellation per pane and saved queries.
4. **Power workflow**: session restoration and command palette first; scripts,
   folder comparison and additional archive/preview workflows follow.
5. **Default-candidate release**: measured performance, native accessibility,
   disposable clean-install/upgrade/default-switch tests, supported-feature
   documentation, then an upstream proposal backed by evidence.

## Evidence rules

Unit tests establish backend behavior. The browser suite uses `src/preview.html`
and **mocks Rust IPC**, so it proves UI behavior only. The isolated D-Bus test
exercises real service calls without claiming the user's session-bus name.
Native smoke checks do not replace the full desktop interoperability matrix.
Hardware-dependent tests stay unverified until performed on a real device.

Test commands:

```sh
cd src-tauri
cargo test --lib
dbus-run-session -- cargo test desktop_bus -- --ignored
cd ../src
bun test
bun run check
bun run build
# With Vite running on port 1420 and Playwright available:
PLAYWRIGHT_DIR=/path/to/node_modules node tests/e2e.mjs
```

## References

- [GNOME Files](https://apps.gnome.org/en/Nautilus/): local/network file management, removable media, scripts and app launch.
- [GIO File](https://docs.gtk.org/gio/iface.File.html): provider-based local and remote operations.
- [Tauri single instance](https://v2.tauri.app/plugin/single-instance/): launch forwarding with arguments and caller working directory.
- [Approved parity design](superpowers/specs/2026-09-16-nautilus-parity-design.md).
- [Linux provider evidence](verification/2026-09-18-linux-providers.md).
- [Desktop implementation plan](superpowers/plans/2026-09-17-desktop-integration.md).
