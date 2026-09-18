# Desktop opening verification — 2026-09-17

## Implemented scope

Local URI decoding and CLI opening; single-instance argument forwarding;
queued delivery before frontend readiness; FileManager1 and opt-in cold
activation; reversible per-user default registration; selected-file reveal
across hidden files and pagination; tab-limit handling; safe navigation while loading.

## Automated evidence

- Rust library suite: **100 passed**. The isolated-bus test is intentionally ignored
  in the ordinary suite and run separately.
- `dbus-run-session -- cargo test desktop_bus -- --ignored`: passed. Real D-Bus
  ShowFolders, ShowItems and ShowItemProperties calls; unsupported remote URIs,
  multiple-property requests and an already-owned service name are covered.
- `bun test`: 41 passed.
- `bun run check`: zero errors and warnings.
- Svelte autofixer: no issues in changed components and state modules. Existing
  advisory suggestions about lifecycle effects and nonreactive timer maps remain.
- `bun run build`: passed, approximately 55 KB gzipped application JavaScript.
- Native embedded-assets debug build: `cargo build --locked --features tauri/custom-protocol` passed.
- Browser preview suite: **107/107 checks passed**, with mocked IPC. Checks cover
  desktop opening, hidden selection, repeated requests, background tabs, visible
  startup errors, tab overflow, sidebar opening at the limit, delayed navigation,
  default setup/restore, installation requirements and setup failures.

## Native smoke evidence

Executed the actual Tauri binary on Hyprland, with a private D-Bus session and
temporary XDG configuration/state/cache directories. A temporary `xdg-mime`
query shim returned `omafil.desktop` to exercise registration; this is not a
test of installing or changing the user's actual default.

- Opened a `file://` URI for a real folder containing spaces, Unicode, percent
  and hash characters.
- Started a second invocation with relative `--select` arguments and verified
  successful forwarding to one native Omafil window.
- Checked that the FileManager1 service owner's PID matched the native Omafil
  process before testing its methods.
- Visually inspected both requested entries selected at the top of a 352-item
  folder: one dotfile and one otherwise beyond the 300-item first page.
- Visually inspected the native properties dialog requested over D-Bus, showing
  the fixture's correct path and size.
- Confirmed live Omarchy colors in the native window. Pointer accuracy across
  all scaling settings was not audited.
- Closed the test process after each smoke run. The actual directory association
  remains `org.gnome.Nautilus.desktop`.

Service readiness is probed with NameHasOwner and owner PID checks, which do
not activate a different installed file manager. Captures are accepted only
when the test window is focused and the Omafil contents have been inspected.

## Remaining release limits

No remote URI support or combined multi-item properties.
No clean-system package install/upgrade or upstream Omarchy adoption proof.
Omarchy's explicit file-manager shortcuts remain separately configured.
Browser checks do not establish native clipboard, network,
device or filesystem-operation interoperability. See the
[parity matrix](../nautilus-parity.md) for the broader release work.

## Reversible registration and cold activation

`python3 src/tests/desktop-integration.py --native` passed using the compiled
native executable, disposable XDG directories and a private D-Bus session:

- CLI setup wrote a desktop-specific MIME preference and an activation service;
  real `xdg-mime query default inode/directory` resolved Omafil.
- Repeated setup preserved the original recovery record.
- A ShowItems call with no running provider activated the real Omafil binary.
  Its PID was verified through D-Bus and `/proc`, and subsequent CLI opening
  reused that process.
- Restoration recovered the prior fixture handler and prior service contents,
  preserved a later unrelated text-editor association, and removed the recovery
  record. Unit coverage additionally preserves a later external provider choice
  and verifies rollback after activation-service publication fails.

The non-graphical registration/restore scenario is also configured in CI.
The live workstation's default remains Nautilus; these tests do not switch it.
