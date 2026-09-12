# Reliable file operations

Approved scope: strengthen the existing copy/move and ZIP workflows before broader Explorer fidelity and Linux integration work.

Operations share a cancellable, chunked I/O context and throttled byte progress. Transfers prepare a private staging copy on the destination volume, then publish it without overwriting an unexpected destination. Replacement preserves the old destination until the new content is ready. Completed items remain completed when a later item fails or is cancelled; the event includes those results. Moves retain their source until publication succeeds. Symbolic links are copied as links, never recursively followed.

ZIP creation stages its output and removes unfinished output on error/cancellation. Extraction publishes into a new archive-named folder, never merges over existing files, and rejects traversal and symbolic-link entries. This gives extraction one recoverable publication boundary.

A dedicated operations UI displays byte progress, current item, terminal status and useful error messages. Queue events and command responses reconcile by ID so fast jobs cannot duplicate or regress. Cut clipboard entries clear only after successful moves.

Validation: filesystem regression tests cover interrupted copy, unchanged replacement on cancellation, destination collisions, preserved symlinks, safe ZIP round trips and invalid archives. Frontend reducer tests cover event ordering and partial move completion. Run Rust tests, Svelte/TypeScript checks, frontend build and Svelte analysis. Desktop interaction evidence must be reported separately.

## Delivered behavior

- Native jobs run on one FIFO worker. Cancellation is checked between 256 KiB chunks and directory entries. A short publication/source-removal phase finishes once committed; cancellation never intentionally interrupts that phase.
- Copy and cross-volume move output is staged on the destination filesystem. Same-volume moves use an atomic no-replace rename. Replacement sends the old item to desktop Trash only once the new content is ready, preserving its original name and restore location. If publication then fails, the error explains that the original is in Trash and the source remains available.
- Cancelling or failing a batch keeps its completed results. Skipped and unfinished cut items stay on the clipboard. Copy preserves symbolic links; rename and delete act on the link itself.
- ZIP extraction creates an archive-named folder and uses a numbered name when that folder exists. Traversal, symbolic links and special archive entries fail without publishing a partial folder. ZIP creation rejects symbolic links rather than following them.
- Cancellation/error cleanup handles read-only directories in the private staging tree without following links.
- The operations panel is an independent Svelte component with external CSS, bounded scrolling, a native progress element, current filename, processed bytes, item counts, persistent error text, cancellation feedback and dismiss controls.

## Validation and limits

Rust filesystem and queue tests, frontend reconciliation tests, Svelte/TypeScript diagnostics, production frontend build and Svelte component analysis are the automated gates. A Chromium preview check uses simulated Tauri events to verify progress/error rendering, Cancel feedback and Dismiss. The only preview console error observed was the existing missing favicon.

The existing App.svelte directory-watcher effect receives advisory Svelte analyzer suggestions; it was retained because it intentionally controls a native subscription. The new operations component has no analyzer issues or suggestions.

This milestone does not establish end-to-end native desktop behavior, power-loss recovery, Windows 11 visual parity or completed Omarchy integration. Jobs are not persisted across app restarts. Atomic no-replace publication requires filesystem support; an unsupported filesystem returns an error rather than falling back to an overwrite.

Final automated results: `cargo test -q` — 39 passed; `bun test` — 4 passed; `bun run check` — 0 errors and 0 warnings; `bun run build` — passed; scoped `rustfmt --check` and `git diff --check` — passed. Browser screenshot: `output/playwright/operations-panel.png`.
