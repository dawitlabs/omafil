# Clipboard, naming and font fixes

The reported Enter bug was reproduced in the browser harness: committing a new
folder name also navigated away. The rename handler cleared its state before the
key bubbled into the parent list, which then treated Enter as Open. Input events
now stop propagation, list navigation ignores editable targets, and a cancelled
or already committed rename cannot be committed again by blur.

A second, separate click on a selected item's name starts renaming after the
double-click interval. Double-click still opens, dragging cancels pending rename,
and F2 remains available. Escape cancels the draft.

Clipboard fixes include fresh reads before toolbar, shortcut and folder-menu
paste, GNOME copied-files and URI-list formats, KDE cut markers, publication
ordering for immediate Copy/Paste, and clearing stale paths when the clipboard
changes or fails. PNG, JPEG and WebP clipboard payloads can be saved into the
chosen directory with collision-safe names and Undo. Reads have a five-second
timeout and a 64 MiB bound; temporary image files are published without replacing
an existing file or symlink.

The root text baseline increased from 16px to 18px, retaining the saved percentage.
At 100%, file text increases from 13px to 14.625px. At 125%, it is 18.28125px.

## Evidence

- Rust library tests: 104 passed, one separate desktop-bus integration test ignored.
  Clipboard tests use real temporary files for byte preservation, collision handling,
  symlink protection, invalid-image rejection and URI decoding. MIME selection tests
  inject clipboard offers; they do not read the user's live clipboard.
- Frontend unit tests: 41 passed.
- Svelte/TypeScript check: zero errors and warnings.
- Frontend production build passed. The native desktop binary was built with
  `cargo build --locked --features tauri/custom-protocol` and reports its version.
- Browser checks: 132/132 passed using the existing mocked Tauri preview harness. New scenarios
  cover Enter, click-to-rename, Escape, double-click, fresh file clipboard state,
  copy/cut switching, empty/unavailable clipboard, image naming and Undo,
  immediate Copy/Paste, and font-size persistence across reload.

The installed `/usr/bin/omafil` remains the older September 15 package. The rebuilt
executable is `src-tauri/target/debug/omafil`. A live cross-application clipboard
transfer through the native desktop has not been verified in this pass.
