# Reliable Operations Implementation Plan

**Goal:** Deliver cancellable, observable file transfers and ZIP operations with safe publication and clear failures.

**Architecture:** Rust operation I/O owns cancellation and byte accounting; staging owns publication/cleanup; transfers and archives use those primitives. Queue execution publishes one typed event stream. Svelte reconciles events by operation ID and renders a dedicated component with external CSS.

**Tech Stack:** Tauri 2, Rust, Svelte 5, TypeScript, existing ZIP crate, tempfile and rustix.

## Execution

The user approved implementation in the existing checkout. Preserve unrelated uncommitted changes; execute inline without additional approval or commits.

- [x] Add `src-tauri/src/operation_io.rs`: cancellable 256 KiB reads/writes, throttled cumulative bytes, current filename, preparation phase, safe staging publication. Tests interrupt after a chunk and assert no published partial data.
- [x] Update `src-tauri/src/operations.rs`: prepare complete copies before replacement, preserve symlinks, retain source on failure, return each completed transfer independently. Tests cover copy/move, replacement cancellation, collisions, keep-both and symlinks.
- [x] Update `src-tauri/src/archive.rs`: stream ZIP data through the same context, stage creation/extraction, reject unsafe names/symlinks, extract to a new folder. Test round trip, cancellation, existing destination and traversal.
- [x] Update `src-tauri/src/operation_queue.rs`: serialize jobs, emit byte progress and meaningful terminal errors, retain partial batch results; wire modules in `lib.rs`.
- [x] Add a pure frontend operation reconciliation module and tests; adapt `fileOperations.svelte.ts` so command responses cannot reset terminal events or duplicate jobs, and cut entries clear only on successful moves.
- [x] Extract the operations panel from `App.svelte` into `components/OperationQueue/`, with byte progress, cancelling feedback, persistent error detail and external CSS.
- [x] Run `cargo test -q` in `src-tauri`; run `bun test`, `bun run check` and `bun run build` in `src`; run Svelte autofixer on edited components. Review `git diff --check` and document runtime limits.
