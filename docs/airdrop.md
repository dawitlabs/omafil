# AirDrop interoperability research

Research date: **2026-09-16**. Status: **application foundation implemented; native transport stopped at the peer-verification and transport-validation boundary**. Omafil does not currently implement AirDrop discovery or transfers.

**NOT VERIFIED ON REAL IOS DEVICE**

## Evidence labels

- **VERIFIED**: inspected source code, published primary documentation, or a locally executed check. An upstream author's successful transfer is attributed evidence, not an Omafil compatibility result.
- **UNKNOWN**: the reviewed public material does not establish an implementable, secure contract.
- **REQUIRES DEVICE TESTING**: behavior that must be reproduced on identified Apple hardware and Linux networking hardware.

This is an unofficial interoperability investigation, not an Apple API integration. The Share dialog and fail-closed backend commands are implemented, along with isolated session and receive-staging primitives and automated tests. No networking service, privileged helper, wire-protocol parser or invented devices were added. Native transport remains unimplemented under the explicit instruction to stop rather than invent unsupported protocol behavior.

## Protocol findings

| Area | Evidence and remaining boundary |
| --- | --- |
| Discovery | **VERIFIED, historical architecture:** Apple describes BLE discovery followed by peer-to-peer Wi-Fi. OpenDrop browses/advertises `_airdrop._tcp.local.` using DNS-SD/mDNS on an IPv6 AWDL interface. Ordinary LAN mDNS alone does not supply AWDL. BLE wake/discovery behavior on current iPhones **REQUIRES DEVICE TESTING**. |
| Link layer | **VERIFIED:** OWL implements AWDL in userspace and needs compatible active-monitor, injection and acknowledgement behavior. An ordinary Wi-Fi connection or a driver advertising monitor mode is insufficient evidence of compatibility. |
| Identity | **VERIFIED, historical architecture:** Apple describes contact-identifier hashes, Apple Account identity certificates and validation records. Display names and Bonjour records are attacker-controlled labels, not authenticated identities. The complete modern noncontact trust bootstrap is **UNKNOWN** for this implementation. |
| TLS/authentication | **VERIFIED:** OpenDrop disables certificate verification. The inspected Rust alternative additionally bypasses TLS handshake-signature verification. Neither meets this task's security contract. Simply enabling a default public Web PKI verifier is not a demonstrated solution for AirDrop identities. |
| Application protocol | **VERIFIED in inspected OpenDrop code:** HTTPS POST requests to `/Discover`, `/Ask`, `/Upload`; plist metadata; archive upload. This flow does not use WebSockets. It is not established as the only modern AirDrop flow. |
| Metadata | **VERIFIED in that implementation:** sender name/model/ID and file names, type identifiers, directory flags and archive paths appear in the request. Their presence does not authenticate the sender or authorize filesystem access. |
| Payload | **VERIFIED upstream observations:** newer implementations handle chunked metadata, `application/x-dvzip` framing around CPIO, and transfer identifiers. Older OpenDrop assumes `application/x-cpio`. Exact variants, sizes and directory semantics across supported iOS releases **REQUIRE DEVICE TESTING**. |
| Pairing | **UNKNOWN:** a secure, reproducible Linux implementation of current noncontact authentication/code exchange was not established. Extracting somebody's Apple credentials or accepting every certificate is not an acceptable substitute. |

Sources: [Apple platform security, published February 2021](https://support.apple.com/en-ca/guide/security/sec2261183f4/web), [OpenDrop source](https://github.com/seemoo-lab/opendrop/tree/11fe7ba7861093b302bc0637e8cb10adf2d29337), [OWL](https://github.com/seemoo-lab/owl/tree/da255a70f221784c836d943dd3f243bc798f223b), and the version-pinned implementations below. Apple's 2021 description is not a complete iOS 26 specification. The OWL/OpenDrop repositories warn that their former `owlink.org` website is no longer associated with the project.

### Current Apple and Google behavior

**VERIFIED, published product behavior:** Google documents native, bidirectional AirDrop interoperability, direct peer-to-peer transfer, Rust implementation work, receiver consent and independent assessment. Its documented cross-platform mode uses **Everyone for 10 Minutes**, not Contacts Only. Google's May 2026 update describes expansion beyond Pixel. These statements establish feasibility on supported Android products, not availability of a reusable Linux stack. [Google security](https://blog.google/security/android-quick-share-support-for-airdrop-security/), [May 2026 update](https://blog.google/products-and-platforms/platforms/android/new-android-updates/).

**VERIFIED, current Apple documentation:** Apple's September 14, 2026 guide requires Wi-Fi and Bluetooth enabled, describes temporary Everyone visibility, and adds a noncontact AirDrop code when **both devices use iOS 26.2 or later**. **UNKNOWN:** negotiation and fallback behavior with a Linux peer; this conditional instruction must not be generalized into a claim that every cross-platform transfer requires that code. [Apple user guide](https://support.apple.com/en-us/119857).

**VERIFIED, assessment evidence:** NetSPI's public September 2025 report covers Android/iOS/OpenDrop interoperability, session validation, encrypted transfer, parser/path attacks and logging. It records a remediated low-severity logging issue. Its examples contain BLE discovery and HTTPS-style discovery/ask metadata. **UNKNOWN:** a sufficiently detailed certificate/identity/bootstrap specification or licensed Google implementation to reproduce its security properties; the report is not that specification. [NetSPI report](https://www.netspi.com/wp-content/uploads/2025/11/google-feature-review-report.pdf).

The reviewed primary material does not establish which transport variants all current Google devices negotiate. Apple's [Wi-Fi Aware framework](https://developer.apple.com/documentation/WiFiAware) is not itself a specification for native AirDrop. A generic Wi-Fi Aware app, QR upload or cloud link would not satisfy the requested experience.

## Implementation candidates and reuse decision

These are inspected snapshots, not an exhaustive guarantee that no other implementation exists. Recent commits do not by themselves establish maintenance quality or security.

| Project and pinned revision | Assessment |
| --- | --- |
| [OpenDrop](https://github.com/seemoo-lab/opendrop/tree/11fe7ba7861093b302bc0637e8cb10adf2d29337), `11fe7ba7861093b302bc0637e8cb10adf2d29337` | Python; GPL-3.0. Research reference. README admits missing peer authentication; `config.py` sets `CERT_NONE`. Receiving/extraction and acceptance need substantial hardening. Not a safe dependency or subprocess service as-is. |
| [OWL](https://github.com/seemoo-lab/owl/tree/da255a70f221784c836d943dd3f243bc798f223b), `da255a70f221784c836d943dd3f243bc798f223b` | C; GPL-3.0. Link-layer foundation, not a complete AirDrop library. Requires hardware validation and privilege separation. No helper installed. |
| [User-cited matiskov fork](https://github.com/matiskov/opendrop/tree/72afe9afb29a2614542dc0dfe9204b42f9aa437a), `72afe9afb29a2614542dc0dfe9204b42f9aa437a` | Python; GPL-3.0. Inspected revision dated February 2025; retains `CERT_NONE`. Does not establish secure current-iOS support. |
| [airdrop-mt7921](https://github.com/jedbillyb/airdrop-mt7921/tree/3e9800e83647a4b5aff9909b55966f4efe940fdf), `3e9800e83647a4b5aff9909b55966f4efe940fdf` | GPL-3.0 research integration with September 2026 updates. Author reports iOS 26 receiving and sending on MediaTek MT7921, Void Linux/kernel 6.18.33. Send proof is a **68-byte file**; large-send throughput unproven. Standalone mode takes over Wi-Fi; concurrent-Wi-Fi daemon reached **99.1%, not completion**. Useful format/TransferID findings, but retains certificate bypass and includes research TLS key logging. Not production-ready. |
| [opendrop-rs](https://github.com/ayourtch-llm/opendrop-rs/tree/dccc798e244363eb92d35e3c52e9a913188dda91), `dccc798e244363eb92d35e3c52e9a913188dda91` | Rust; GPL-3.0 derivative, with library and binaries. Upstream capture names iPhone 15 Pro Max/iOS 18.6.2 and macOS 15.7.3. Missing BLE wake and large-transfer limitations. `AcceptAnyCert` accepts certificates **and TLS 1.2/1.3 handshake signatures** without verification. Rejected as a secure foundation as-is. |
| [Airdrop-standalone](https://github.com/nineza1337/Airdrop-standalone/tree/32b257ca248ba0f5dc2d163699cafcd54acfc4ca), `32b257ca248ba0f5dc2d163699cafcd54acfc4ca` | GPL-3.0-or-later research tooling, including claimed newer discovery/QUIC paths. Legacy and QUIC paths disable certificate verification. Claims are not locally reproduced. Not adopted. |

Direct security evidence: [OpenDrop TLS configuration](https://github.com/seemoo-lab/opendrop/blob/11fe7ba7861093b302bc0637e8cb10adf2d29337/opendrop/config.py), [Rust verifier](https://github.com/ayourtch-llm/opendrop-rs/blob/dccc798e244363eb92d35e3c52e9a913188dda91/crates/luftlift-rs/src/tls.rs), [iOS 26 send patch](https://github.com/jedbillyb/airdrop-mt7921/blob/3e9800e83647a4b5aff9909b55966f4efe940fdf/patches/opendrop-py314-send.patch).

**Decision:** use these as attributed research, not dependencies, vendored code or subprocess wrappers. Wrapping does not fix trust checks. Reimplementing protocol knowledge may be technically possible, but secure interoperability still needs evidence. GPL code adaptation requires compliance; Rust rewrites of derivative code do not automatically become MIT. Distribution and jurisdiction-specific reverse-engineering/patent questions remain **UNKNOWN**, not blanket legal clearance. See [GNU's licensing guidance](https://www.gnu.org/licenses/gpl-faq.en.html). Omafil remains MIT. The foundation adds **one direct dependency: `sha2 = "=0.10.9"` (MIT OR Apache-2.0)**, already present transitively in Cargo.lock; no new package versions or GPL code are introduced. The lockfile adds only `sha2` to Omafil’s dependency list. The cached crate manifest/licenses and [RustCrypto documentation](https://docs.rs/sha2/0.10.9/sha2/) were inspected: mature incremental hashing API, existing locked implementation, no optional assembly feature enabled. This is a dependency review, not a cryptographic audit or a claim that no future advisories can appear.

## Repository audit and proposed integration

The repository-wide survey covered backend modules, frontend components/state, tests, capabilities/configuration, documentation, CI and packaging. These are integration findings, not a full security certification.

| Existing layer | Integration point and constraint |
| --- | --- |
| `src-tauri/src/lib.rs` | Typed Tauri commands and managed state; blocking filesystem work uses `spawn_blocking`. Add an isolated managed AirDrop service only after protocol gates pass. |
| `operation_queue.rs`, `operation_io.rs` | Local operations use a serial worker, cancellation, progress and private staging with atomic no-replace publication. Network waits must not block this worker. Existing local symlink-copy semantics must not apply to received archives. |
| `paths.rs`, `user_dirs.rs` | Downloads follows XDG configuration. Ordinary local path resolution is not authorization for remote-supplied paths. Missing/disabled Downloads needs an explicit local destination choice. |
| `archive.rs`, `error.rs`, `diagnostics.rs` | Existing archive handling is not a safe DVZIP/CPIO protocol decoder. Use structured errors and bounded parsing; avoid logging metadata, file contents, identities, keys or raw requests. |
| `store.rs`, watchers | Settings storage is not secret storage. Filesystem watcher events are separate from peer discovery. |
| `src/src/App.svelte`, state modules | Existing lifecycle-managed Tauri listeners and Svelte runes support typed sharing events. Network transfers need their own state, not a misleading local copy operation. |
| `FileList`, `ContextMenu`, `CommandBar` | Existing context menu has no submenu model. A Share action can open a dedicated dialog with nearby devices; preserve selection. Add incoming consent and progress components with external modular CSS. |
| Tauri config, CI, packaging | No existing AirDrop daemon or privilege model. AppImage alone cannot guarantee radio capability. Keep the desktop app unprivileged; any future link helper needs narrowly scoped IPC and packaging review. |

Implemented modules under `src-tauri/src/airdrop/`: `mod`, `commands`, `error`, `session`, `storage`, and `storage_tests`. Discovery, identity/pairing, TLS and wire-protocol modules remain blocked; empty substitutes were not created.

Expose typed `DeviceId`, `TransferId`, `Device`, `IncomingOffer`, `TransferProgress` and structured errors through discover/send/accept/reject/cancel commands. Frontend commands never accept endpoints, certificates or remote destination paths. Events cover device discovered/updated/lost and transfer requested/progress/terminal state. A bounded async service owns subscriptions, deadlines and cancellation; shutdown releases listeners, sockets and helper resources. No perpetual polling.

Consent must bind an immutable offer to its verified session. Names remain explicitly untrusted labels. The receiver chooses a destination under configured Downloads, validates archive entries, rejects absolute/parent paths, links and special files, enforces entry/byte/decompression/depth limits, stages privately, then publishes without replacing existing files. Cancellation, disconnect and expiry remove partial output. Never execute received content. Outgoing transfers expose only locally selected entries; retries require a fresh valid session and cannot silently duplicate publication.

TLS integrity, archive completeness and cross-device file checksums are different guarantees. A locally computed hash without an authenticated expected value is not sender-integrity verification. No invented wire checksum field or default trust-on-first-use policy is proposed.

## Follow-up: receiver verification and hardware prerequisites

**VERIFIED source evidence, not current-iOS compatibility:** a deeper review found historical receiver-verification logic in [PrivateDrop Base, revision `43f6bbccba7d78358a673054bbbd0befd0d9ca85`](https://github.com/seemoo-lab/privatedrop-base/tree/43f6bbccba7d78358a673054bbbd0befd0d9ca85). Its TLS helper validates Apple certificate chains; its record decoder verifies CMS signatures; its client can bind the signed account identifier to the TLS leaf and compare signed contact hashes. Therefore OpenDrop's `CERT_NONE` is not proof that a secure sender is impossible. The same framework has an unsafe single-certificate fallback, which must not be adopted. Its checked record path also does not establish a freshness policy. No clear top-level reuse license was found, and Apple-framework code is not directly usable on Linux: treat it as protocol research, not code to vendor.

**UNKNOWN / REQUIRES DEVICE TESTING:** whether iOS 26.6.2 returns the required Apple-issued chain and signed record to a generated Linux sender identity. Chain validation alone does not authenticate a displayed device name; a verified record/contact match identifies an account, not necessarily one specific phone among devices sharing that account. A candidate strict probe must verify the chain and handshake, then bounded signed discovery data and identity binding, before transmitting file metadata. Missing evidence must remain a hard failure. Native code-pairing and record freshness still require research.

**VERIFIED locally:** PCI identifies **Intel Wireless 8260 `[8086:24f3]`**, using `iwlwifi`. `iw phy phy0 info` reports monitor mode but neither active-monitor capability nor NAN among supported interface modes. There is no `awdl0` or installed OWL executable. Stock [OWL's monitor setup](https://github.com/seemoo-lab/owl/blob/da255a70f221784c836d943dd3f243bc798f223b/daemon/netutils.c) requests active monitor; [Linux's capability check](https://github.com/torvalds/linux/blob/master/net/wireless/nl80211.c) rejects that request without the advertised capability. This is source-based evidence against using stock OWL on the present configuration, not a successful radio test or proof all alternative transports are impossible. No adapter mode or network connection was changed.

`bluetoothctl list` and `/sys/class/bluetooth` show no controller; `bluetoothctl show` reports no default controller. The service and `btusb` module are active, with no controller-initialization error found in the inspected boot log. The cause (hardware availability, firmware/BIOS configuration, or another issue) remains unknown. A baseline Bonjour browse on existing interfaces returned no `_airdrop._tcp` services; phone readiness was not confirmed, and this is **not an AWDL or iPhone compatibility test**.

## Unblocking and test contract

Before implementing native transport and the device/transfer UI:

1. Establish a documented, reproducible peer-verification mechanism accepted by native iOS, including certificate/signature verification and session binding. Resolve the current code-exchange/legacy-mode boundary with evidence. Do not use an accept-all verifier, even for an apparent successful demonstration.
2. Validate an isolated Linux link implementation on identified hardware, including BLE behavior, AWDL coexistence and cleanup. No assumption that sharing the same access point supplies the necessary link.
3. Reproduce a complete small transfer both ways with verified bytes and native consent, recording exact OS builds, hardware and visibility settings. Then connect the application foundation to the verified transport and implement device discovery, incoming consent and transfer progress UI incrementally.

Full transport test contract (only the foundation subset described below is implemented): valid/malformed plist and archive fixtures; truncation and decompression limits; discovery updates/expiry; legal and illegal session transitions; stale/replayed consent; certificate and handshake-signature rejection; traversal/symlink/race protection; filename collisions; cancellation during all stages; partial transfers; checksum/completeness failures; concurrent transfer isolation; timeout and shutdown cleanup. Mocks verify service behavior, not iOS interoperability.

**REQUIRES DEVICE TESTING — all cases unrun:** iPhone → Omafil Photos, Files, multiple files, large/small files, interruptions, cancellation and duplicate names; Omafil → iPhone photo, video, PDF, large/multiple files and cancellation. Repeat with same Wi-Fi and isolated/different-network conditions, capturing which peer transport actually operates. Test directories only where native protocol behavior is established. Record exact iOS build and downloaded-file checksums for each case.

## Local environment and verification

Linux inspected: **Omarchy 4.0.2, Linux 7.1.9-arch1-2, x86_64**, `iwlwifi` on `wlp2s0`. No `awdl0`. The advertised supported interface modes include monitor but not NAN; active-monitor injection/ACK compatibility remains unverified. No radio configuration was changed.

`idevice_id -l` failed to retrieve a device list. There is no usable Apple device session established in this environment; this does not prove no iPhone is physically nearby. **Exact iOS versions tested here: none.** Upstream versions listed above are not Omafil test results.

### Available target device

User-reported hardware: **iPhone 13 Pro Max, iOS 26.6.2**. Apple lists [iOS 26.6.2 as build 23G90](https://developer.apple.com/news/releases/?id=09082026a); the device's installed build has not been independently inspected. Device availability is now confirmed by the user, but no native discovery or file transfer has been attempted. This does not change the **NOT VERIFIED ON REAL IOS DEVICE** status. No additional phone identifiers are needed for the current investigation. The remaining prerequisite is a secure Linux transport, not another phone setting.

## Implemented foundation

- `airdrop_availability` returns an explicit unsupported capability. Discover/send/accept/reject/cancel commands return `transport_unavailable` before opening paths or sockets. No simulated discovery result, progress or success is returned. These are command guards, not a transport implementation.
- `session.rs` enforces consent, bounded byte progress, verification before completion, rejection, cancellation and deadlines. An adapter must still bind the state to its authenticated offer and serialize cancellation/publication.
- Linux-only `storage.rs` validates a bounded flat-file manifest (256 entries, 10 GiB total). It rejects traversal, separators, control/bidi characters, duplicates and unsupported metadata. Files are written mode 0600 inside private temporary staging. A pinned destination directory descriptor prevents destination symlink replacement from redirecting publication. A complete batch is published as a new directory using no-replace rename; collision requires a new local name. Incomplete/error/cancelled batches are removed when dropped. Startup recovery after process death is not implemented.
- The streaming storage API enforces declared lengths and computes SHA-256; a supplied expected digest must match. Such a digest is an adapter input, **not an invented AirDrop metadata field**, and is not authenticated merely by being supplied. A future receiver must use the existing XDG Downloads resolver for its locally chosen destination. Directories, archives and network streams are not accepted by this primitive.
- `sharing.svelte.ts` snapshots the selection and ignores stale results after closing/retrying; availability checks time out. File-list context menu and toolbar open a native modal Share dialog with focus containment, Escape dismissal and an explicit AirDrop unavailable message. No incoming-request or transfer-progress UI is shown because no real transport can produce those events yet.

## Foundation verification

The native protocol, radio discovery and real iPhone transfers are not exercised by these tests.

| Gate | Result |
| --- | --- |
| Rust: `cargo test --locked` | **PASS**, 88 tests, including 12 new foundation tests |
| Mocked browser integration: `node tests/e2e.mjs` | **PASS**, 74/74 checks, including 15 sharing checks; not native Tauri/iOS testing |
| Frontend: `bun test` | **PASS**, 41 tests |
| Typecheck: `bun run check` | **PASS**, no Svelte errors/warnings; TypeScript passed |
| Build: `cargo build --locked` and `bun run build` | **PASS**, debug native binary and production frontend |
| New Rust module formatting | **PASS**, `rustfmt --edition 2021 --check src-tauri/src/airdrop/mod.rs` |
| Repository formatter | **FAIL**, existing differences outside the new modules |
| Strict Clippy | **FAIL**, same eight existing diagnostics in `inspect`, `archive`, `user_dirs`, `icon_theme`, `listing`, `clipboard`, `search`; none in the new AirDrop modules |
| Svelte autofixer | No issues in changed components/modules; suggestions about existing effects and optional actions/attachments reviewed |
| Packaging recipe syntax | **PASS**, both PKGBUILDs checked with `bash -n` |
| Release/AppImage for this foundation | **NOT RUN**: previous pre-foundation release binary built, but bundling failed with `No space left on device`; current filesystem still reports 99–100% used. No package for the new code is claimed. |
| Native AirDrop discovery | **UNVERIFIED / NOT IMPLEMENTED** |
| iPhone → Omafil | **UNVERIFIED / NOT IMPLEMENTED** |
| Omafil → iPhone | **UNVERIFIED / NOT IMPLEMENTED** |

The new Rust tests cover fail-closed commands, consent/state transitions, timeouts, cancellation, invalid metadata, traversal, partial/oversized payloads, known SHA-256 output and checksum mismatch, concurrent publication, filename collisions, symlink protection and cleanup. They do not cover packet parsing, peer lifecycle or TLS because those components are not implemented. Global lint/format findings were not suppressed. No application was installed or published.
