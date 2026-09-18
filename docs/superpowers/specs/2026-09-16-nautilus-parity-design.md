# Nautilus parity for Omafil

Status: approved by the user. Phase 1 is implemented in the working tree;
native desktop validation is still pending. Phases 2–5 remain future work.

Phase 1 implementation and verification are recorded in
`../plans/2026-09-16-local-filesystem-parity.md`.

## Outcome and scope

Make Omafil a dependable everyday replacement for GNOME Files while retaining
its Svelte/Tauri interface, Omarchy themes, Vim navigation, split panes and tags.
Target Linux first, with Omarchy/Arch as the primary integration environment.

“All features” needs a bounded reference: before claiming parity, record an
installed stable Nautilus version and enumerate its menus, settings and supported
workflows in a feature matrix. Each row must record implementation status,
dependencies, automated evidence and native runtime evidence. The comparison
from this conversation identifies initial gaps; it is not an exhaustive inventory.
Third-party Nautilus extensions are a separate compatibility category rather than
an unbounded promise to reproduce every extension.

## Approach

Recommended: retain Omafil and integrate established Linux services through Rust.
Use GIO/GVfs for URI-based remote/device access and LocalSearch for optional
indexed search. Retain ordinary local operations without requiring an indexer.

Alternatives considered:

- Implement protocols and indexing independently: more control, substantially
  greater protocol, authentication and maintenance work.
- Adapt Nautilus itself: closer native integration, but a major change to the
  existing frontend and product architecture.

## Delivery order and completion criteria

### 1. Local filesystem correctness

- Replace the home/mount navigation allowlist with normal operating-system
  access checks. Expose Filesystem in navigation and support readable paths such
  as /etc, /opt and /var/log. Do not elevate the application to root.
- Preserve protections against moving folders into themselves, archive traversal,
  unsafe overwrite and destructive root operations. Audit every caller of shared
  path resolution before widening it; preview access requires a separate review
  of the Tauri asset scope and handling of untrusted file content.
- Resolve standard folders from XDG user-directory configuration without shell
  evaluation. Support spaces, localized names, configured paths outside home and
  disabled entries. Do not silently create missing directories.
- Traverse eligible subdirectories before applying type/size filters to search
  results. Preserve cancellation, hidden-file behavior and symlink-cycle safety.
- Distinguish access denied, missing location, disconnected location and partial
  results in user-facing errors.

Acceptance: regression tests for nested filtered search, custom user directories,
readable paths outside home, denied access and symlink handling; native navigation
and preview checks outside home; existing operation tests remain passing.

### 2. Remote locations and devices

- Introduce a location abstraction distinguishing local paths from remote URIs.
  Listing, navigation, tabs, clipboard, metadata and operation requests use this
  abstraction rather than coercing remote URIs into filesystem paths.
- Add Connect to Server for SFTP, SMB, WebDAV and FTP through installed GVfs
  backends. Support authentication prompts, cancellation, bookmarks and reconnect.
- Use GIO mount/volume discovery for compatible MTP phones and camera/device
  backends. Report missing dependencies and locked/disconnected devices clearly.
- Route remote copies and moves through the provider's supported operations.
  Display capabilities honestly: trash, atomic rename and undo are not universally
  available. Never describe partial remote transfers as atomic local operations.
- Keep credentials out of bookmarks, application state and diagnostics. Use the
  platform credential mechanism where available and session-only handling otherwise.

Acceptance: real local-to-remote and remote-to-local transfers, denied credentials,
disconnect during transfer, overwrite decisions, cancellation, remount and phone
reconnection. Hardware-dependent checks remain explicitly unverified until exercised.

### 3. Search parity

- Add an optional LocalSearch provider for indexed names, supported document
  contents and metadata. Feature-detect service availability and version support.
- Support global configured locations, current-folder scope, type and date filters,
  streamed results, cancellation and explicit index coverage/status.
- Retain recursive filename search when the indexer is unavailable or a location
  is not indexed. Never imply this fallback searches document contents.
- Replace silent incompleteness with visible limits and a continuation or narrowed
  scope workflow. Avoid duplicate results when combining providers.

Acceptance: document-content fixtures, excluded/unindexed folders, date/type
filters, missing service, stale entries and cancellation. Benchmark representative
small and large folders; record measurements rather than assuming Rust is faster.

### 4. Desktop workflows and extensibility

- Audit the chosen Nautilus baseline for remaining preview/thumbnail formats,
  properties, links, permissions, opening behavior, archive workflows, bookmarks,
  recent/starred collections, undo and volume actions. Implement missing behavior
  and record intentional differences individually.
- Add a user scripts menu with documented selected-file and working-directory
  context, safe argument handling and explicit user activation.
- Design Omafil extension points for context actions, metadata and emblems.
  Nautilus binary/Python extension compatibility requires a separate compatibility
  assessment; do not claim it from the presence of an Omafil plugin API.
- Add translatable UI strings and verify keyboard access, focus order, screen-reader
  names, contrast, scaling and touchpad behavior in the native desktop application.
- Package optional backend dependencies and document which features they enable.

Acceptance: each baseline matrix row has an implementation or an explicit remaining
gap; real desktop clipboard, drag/drop, application launch and accessibility checks
are recorded separately from browser mocks.

### 5. Release validation

- Run cargo tests, frontend tests, Svelte checks and production builds.
- Validate disk-full, permission-denied, interrupted operations, symlinks, large
  directories, conflicting names and removable-device disconnection using disposable
  fixtures. Compare content hashes for completed transfers.
- Exercise normal and fractional display scaling on the supported compositor setup.
- Validate installation, upgrade and default-folder-handler behavior in a disposable
  environment. Do not change the user's live default handler during development.
- Publish a supported-feature matrix and known limitations. Only claim parity for
  workflows with recorded evidence; planned or mocked functionality does not count.

## First implementation slice

Implement phase 1 first. Its code boundaries are paths.rs, search.rs, error mapping,
filesystem navigation and narrowly scoped preview changes. Add regression tests
before fixing filtered traversal and path-resolution behavior, then run existing
checks. Subsequent phases depend on these local semantics being stable.

## References

- Current source: src-tauri/src/paths.rs, search.rs, drives.rs, operation_queue.rs,
  src/src/lib/navigation.svelte.ts and src-tauri/tauri.conf.json.
- GIO files: https://docs.gtk.org/gio/iface.File.html
- Mount authentication: https://docs.gtk.org/gio/class.MountOperation.html
- LocalSearch: https://gnome.pages.gitlab.gnome.org/localsearch/overview.html
- LocalSearch queries: https://gnome.pages.gitlab.gnome.org/localsearch/endpoint.html
