# Linux provider delivery

This delivery adds GIO/GVfs network and device discovery, a URI-based remote
browser, native GTK authentication, address-only saved servers, cancellable
provider requests, and copies between remote locations and local paths. It also
adds optional LocalSearch document-content search using bound TinySPARQL query
parameters, folder scoping, hidden-file handling, type/date result filters,
filename-search fallback and Show in folder.

## Native evidence

- Seven Python/GIO fixture tests passed on disposable directories: URI validation,
  hidden names, Unicode paths, bounded listings, recursive byte-preserving copies,
  no overwrite, self-copy/symlink rejection, safe error text and the JSON protocol.
- A disposable localhost FTP server and a private D-Bus session exercised real
  GVfs connection, listing, remote-to-local and local-to-remote copies. Destination
  bytes matched the sources. Repeating an upload refused the existing destination.
- An isolated TinySPARQL endpoint using the real Nepomuk ontology proved that a
  query finds text inside a document whose filename does not contain that text.
  Scope, hidden files, stale entries and parameter binding were checked.
- Test orchestration now uses private runtime/config/cache/state directories and
  permits only GVfs service activation, avoiding the user's keyring service.
  FUSE is disabled in these fixtures: the provider uses GIO URIs directly.
- Rust: 108 tests passed, with two environment-dependent tests excluded from the
  default run. The new native Rust-to-Python-to-GIO bridge test was then run
  explicitly and passed. CI now runs that test with the GI fixtures.
- Frontend: 41 unit tests passed, Svelte/TypeScript reported zero errors and
  warnings, and the production frontend build passed. The focused provider and
  keyboard browser run passed 27 checks against mocked IPC.

The FTP provider emitted a GLib diagnostic during upload; subsequent content
comparisons and conflict checks passed. This does not establish clean behavior
for every backend or connection condition.

These are separate from the browser harness, which mocks Tauri IPC. They do not
prove that every network protocol, authentication dialog or physical phone works.

## Current limits

- Tested protocol: FTP on localhost. SFTP, SMB, WebDAV, FTPS, Android MTP, cameras
  and Apple AFC access use installed GVfs providers but need separate native
  protocol/device acceptance runs. Password dialogs and disconnect/reconnect under
  load are not yet verified.
- Network browsing is currently a dedicated view. Remote URI tabs, previews,
  external file launching, moves, rename, trash, undo and richer conflict choices
  remain future provider work.
- Copies never overwrite or merge existing destination names. Interrupted copies
  may leave partial destination files/folders; the UI explicitly says so. Directory
  transfer limits are 10,000 visited entries and 64 levels. Links and special files
  are excluded. Downloads currently target the configured Downloads folder.
- Listings show at most 1,000 visible entries with an explicit incomplete-list
  notice. Pagination remains necessary for larger remote folders.
- Content search covers only LocalSearch's index, up to 500 matches. It is not an
  all-files scanner. Type/date filters apply to the returned matches. Index setup,
  coverage management and streamed continuation are not implemented.
- Keyboard checks and semantic controls do not replace a native screen-reader
  audit. This delivery does not establish overall Nautilus parity.

## Reproduction

```sh
/usr/bin/python3 src/tests/linux-services.py
/usr/bin/python3 src/tests/linux-services.py --private-indexed
# Install pyftpdlib into a disposable test directory, not the application:
OMAFIL_FTP_MODULES=/path/to/pyftpdlib/modules /usr/bin/python3 src/tests/linux-services.py --private-ftp
cd src-tauri && cargo test --lib
cargo test native_service_bridge -- --ignored
cd ../src && bun test && bun run check && bun run build
```

Architecture references: [GIO File](https://docs.gtk.org/gio/iface.File.html),
[GTK mount authentication](https://docs.gtk.org/gtk3/class.MountOperation.html),
[LocalSearch endpoint](https://gnome.pages.gitlab.gnome.org/localsearch/endpoint.html),
[TinySPARQL examples](https://gnome.pages.gitlab.gnome.org/tinysparql/examples.html).
