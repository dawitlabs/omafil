# Local Filesystem Parity Implementation Plan

**Goal:** Complete phase 1 of the approved Nautilus parity design.

**Architecture:** Keep normal Linux filesystem permissions authoritative. Resolve
XDG locations in a dedicated parser, preserve destructive-operation validation,
and grant preview access to individual supported regular files. Keep directory
traversal independent from search result filters.

**Tech stack:** Rust, Tauri 2, Svelte 5, Bun.

Execution is inline in the existing checkout, preserving unrelated work. The
approved design authorizes implementation; no additional execution approval is needed.

## Tasks

- [x] Add failing regressions to `src-tauri/src/paths.rs` and `search.rs`:
  browsing `/etc` and a temporary directory succeeds; missing paths fail;
  `type:image` and `size:>1mb` find matching files beneath ordinary subdirectories.
  Run `cargo test` in `src-tauri` and record the expected failures.
- [x] Add `src-tauri/src/user_dirs.rs` with a non-executing XDG parser. Cover
  `$HOME`, absolute paths, spaces, escaped characters, disabled home locations,
  malformed/relative values, absent configuration, and missing directories.
  Wire known locations through it and remove the navigation allowlist from
  `paths.rs`. Preserve final-component symlinks for entry operations.
- [x] Preserve I/O error categories in `error.rs` and `listing.rs`; ensure root
  breadcrumb navigation reaches `/`. Audit archive, copy/move, delete and rename
  callers before changing shared resolution. Add denied-access and root-operation
  regression tests using disposable fixtures only.
- [x] Fix search traversal ordering, count unreadable/skipped entries, and display
  partial search coverage in `navigation.svelte.ts` and `Home.svelte`.
- [x] Add Filesystem navigation and hide unavailable XDG shortcuts in
  `Sidebar.svelte`. Resolve previews outside home using per-file asset grants in
  `preview.rs`/`lib.rs` and a shared frontend helper. Reject special files; text
  previews remain escaped and bounded. Do not broaden the static asset scope to `/`.
- [x] Run Rust tests, `bun test`, `bun run check`, `bun run build`, and the Svelte
  autofixer on changed components. Record native runtime limitations separately
  from source/build and mocked-browser evidence. Update README/changelog and
  annotate the design with delivered and outstanding work.

## Completion evidence

- Before the fix: 62 Rust tests passed and the 3 new navigation/search regressions
  failed for the intended reasons.
- After implementation: `cargo test` passed all 76 Rust tests and the binary/doc
  test targets. A final `cargo test --lib -q` after error-propagation edits also
  passed all 76 tests. Tests ran as the normal user, so denied-permission fixtures
  exercised real permission failures rather than root bypass behavior.
- `bun test`: 41 passed, 0 failed.
- `bun run check`: 0 Svelte errors and warnings; TypeScript checks passed.
- `bun run build`: production frontend build passed.
- Existing Playwright preview suite: 50/50 checks passed. Added scenarios for
  filesystem navigation, disabled XDG shortcuts and partial search coverage:
  9/9 passed together after correcting test expectations for the existing Files
  breadcrumb and the search input's `searchbox` role. Both runs use mocked Rust
  commands. Run the added scenarios with
  `E2E_SCENARIO='filesystem navigation|disabled standard folder|partial search coverage' PLAYWRIGHT_DIR=<node_modules-with-playwright> node tests/e2e.mjs`
  from `src` while the Vite preview harness is running on port 1420.
- Svelte autofixer inspected all changed Svelte components and the navigation
  module. The unkeyed properties tabs were corrected. Remaining suggestions concern
  async effects and existing patterns; cancellation cleanup is retained because
  media URLs require asynchronous backend permission grants. One CLI invocation
  timed out after returning its analysis; the compiler checks above passed.
- `git diff --check`: passed.

### Native runtime limitation

The session has no DISPLAY or WAYLAND_DISPLAY, Xvfb or WebKitWebDriver. Actual
Tauri window interaction, fractional-scale behavior and asset-protocol media
rendering outside home remain unverified. Rust tests exercise real filesystem
permissions and path/media validation; mocked browser checks do not prove native
IPC, compositor behavior or media rendering. Native validation is still required
before calling phase 1 release-ready.

Network connections, phone access, indexed content search, extension compatibility
and the exhaustive Nautilus baseline matrix are not implemented in this slice.
No application installation, default-handler change, commit or release was performed.
