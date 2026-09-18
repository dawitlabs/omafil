# Desktop integration implementation plan

**Goal:** Complete a testable desktop-opening slice of the approved Nautilus parity design, preserving the existing working tree.

**Architecture:** A Rust desktop request module validates CLI paths and local file URIs, groups requested selections by parent, and queues requests until the frontend drains them. Tauri single-instance forwarding reuses the running process. A D-Bus FileManager1 adapter exposes standard desktop opening methods only while Omafil is the configured directory handler. Svelte navigation consumes requests, opens tabs and highlights requested items.

**Scope:** Local desktop opening, selection, properties, packaging and reversible default selection. Remote protocols, indexing and official Omarchy adoption remain separate workstreams. Do not change this workstation's default applications during development.

## Tasks

- [x] Add parser and queue regression coverage for escaped URIs, relative paths, Unicode, literal percent signs, symlinks, multiple targets, invalid hosts/schemes/options and bounded pending requests.
- [x] Implement `desktop_requests.rs`; expose CLI help/version before GTK initialization. Initialize request state before registering single-instance callbacks; use wake events plus a drain command so startup requests cannot disappear before listeners exist.
- [x] Implement `file_manager_service.rs` with ShowFolders, ShowItems and ShowItemProperties. Return D-Bus errors for unsupported input, preserve another running file manager's bus ownership, and verify calls on an isolated session bus.
- [x] Consume queued requests in `desktopOpening.ts`. Open additional targets in tabs, report overflow explicitly, and keep a selection attached to its navigation. Include explicitly revealed hidden or later-page entries without loading an entire large directory into the frontend.
- [x] Update the desktop entry to accept multiple URIs. Document CLI commands and the limits of service activation. Record the installed Nautilus baseline (50.2.2) and remaining feature/evidence gaps.
- [x] Run Rust tests, frontend tests, Svelte checks, production frontend build, browser integration scenarios and isolated D-Bus tests. Record native GUI checks separately from browser mocks.

## Acceptance

`omafil 'file:///tmp/Project%20Files'` opens the decoded folder. `omafil --select /tmp/a.txt /tmp/b.txt` selects both entries in their parent. A subsequent launch routes to the current app without losing early requests. Invalid targets produce a visible error. D-Bus clients can request folders/items/properties on an isolated bus; unsupported remote URIs fail explicitly. Existing operation tests continue passing. No workstation default is changed by development or tests.

## Continued default-handler setup

The user requested continued work. Complete the default-candidate workflow with
`desktop_integration.rs`, a Settings component, CLI setup/restore commands and
a disposable-XDG integration test. Registration writes the current desktop's
per-user MIME preference and FileManager1 service, recording the previous MIME
entry and prior service bytes before changing either. Restore merges only the
owned MIME key and owned service; subsequent external choices are retained.

- [x] Implement locked registration, atomic writes, rollback records, idempotence,
  and tests for unrelated edits, changed providers and managed symlinks.
- [x] Add `--desktop-service`, `--make-default`, `--restore-default`, Settings status,
  setup and restore actions. Require the installed executable and desktop entry.
- [x] Make local installation opt-in for default changes; restore registration
  before local uninstallation when a saved setup exists.
- [x] Verify real MIME lookup and cold activation from a private bus in temporary
  XDG directories, and run frontend settings scenarios and the full regression suite.
