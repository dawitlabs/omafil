# Linux service providers implementation plan

**Goal:** Deliver usable GIO network/device browsing and optional LocalSearch
content search, with accessible controls, cancellation and explicit coverage.

**Architecture:** A bundled Python GI bridge talks to GIO/GVfs and TinySPARQL.
Rust owns isolated child processes, bounds output/time, and cancels by request ID.
Remote locations retain their URI identity and use GIO operations; local paths
remain distinct. Native GTK mount dialogs handle credentials without sending them
to the webview or application settings. Existing local operations remain intact.

**Tech stack:** Rust/Tauri, Python GObject introspection, GIO/GVfs, GTK3,
TinySPARQL/LocalSearch, Svelte5 and external modular CSS.

This continues the approved parity design. It is a first delivery in these gaps,
not a complete-parity or real-phone compatibility claim.

- [ ] Add and test the GI bridge: capability discovery, URI validation, GIO mount
  and device discovery, bounded listings, file/folder copies with no overwrite,
  connection cancellation, and errors without credential-bearing URI echoes.
- [ ] Add the Rust command/process boundary with request ownership, output/time
  limits, cancellation, duplicate request rejection and structured errors.
- [ ] Add Network & Devices navigation: connect, mounts/volumes, browse/back/up,
  selection, copy from local clipboard and copy to Downloads. Explicitly show
  provider failures and partial transfer results; no remote delete/move/undo claim.
- [ ] Add optional indexed document-content search through parameterized
  TinySPARQL queries, scope/hidden filtering, coverage notice and missing-service
  handling. Retain filename search with an explicitly separate mode.
- [ ] Verify keyboard controls, accessible names, live status, focus after
  navigation, cancellation and stale-result suppression in both new surfaces.
- [ ] Package/document optional dependencies and feature limits.
- [ ] Run native disposable-provider fixtures, Rust/frontend checks, browser
  interaction tests and production builds. Mark phone hardware, credentials,
  disconnection and protocol combinations unverified unless actually exercised.
