# AirDrop foundation implementation plan

Goal: implement the application boundary and independently testable storage/session components while preserving the documented protocol stop boundary.

Architecture: typed, fail-closed Tauri commands; standalone Rust transfer state and safe batch staging; a Share dialog reports native availability. No network adapter or simulated peers. Use the current checkout to preserve existing filesystem changes.

- [x] Add `src-tauri/src/airdrop/{mod,error,commands,session,storage}.rs`. Unknown transport returns a structured error before filesystem or networking work. Unit-test status and every command guard.
- [x] Test then implement consent, cancellation, expiry, bounded progress and terminal-state invariants in `session.rs`.
- [x] Test then implement flat-file batch staging with strict names, manifest limits, SHA-256 checks, cleanup and atomic no-replace publication in `storage.rs`; keep tests in `storage_tests.rs`. Do not implement unknown archive formats.
- [x] Register commands in `src-tauri/src/lib.rs`. Add `sharing.svelte.ts` and `ShareDialog` with external CSS, native modal focus handling, loading/error/unavailable states and immutable selection snapshots. Add Share actions to FileList and CommandBar and mount the dialog in App.
- [x] Add mocked browser coverage for selection, unavailable status, errors and dismissal. Run Rust tests, Bun tests, Svelte/TypeScript checks, Svelte autofixer, frontend build and existing browser suite. Check Rust formatting for new modules, report global formatting/lint baseline separately.
- [x] Update docs/airdrop.md and README with actual implemented scope, dependency changes, verification and still-unverified hardware behavior. Do not label the feature working AirDrop.
