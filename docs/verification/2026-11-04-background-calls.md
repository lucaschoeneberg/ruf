# Incoming calls and native background lifecycle — 2026-11-04

## Implemented

- SIP incoming responses use the learned translated Contact when the PBX uses a
  different source port, provided same-peer-IP candidates are unambiguous. Exact
  mappings, expiry, capacity bounds and unrelated-peer isolation are preserved.
- Opaque call tokens guard UI/native-menu actions; `answering` reserves acceptance
  until native establishment. Duplicate/stale actions cannot act on another call.
- Rust startup restores accounts noninteractively and observes phone state without
  webview polling. Native ringing remains in baresip. Closing hides the phone;
  explicit Quit checks authoritative state and confirms occupied calls.
- Native menus, status icon, settings navigation, single instance, silent login
  start for installed releases, and General preferences reuse existing services.
- Native idle-only address/DNS recovery handles late VPN interfaces without
  restarting the engine or resetting an occupied call's transports.

## Evidence

The synthetic UDP registrar/NAT relay test failed before the patch because the
incoming 200 Contact contained the local port instead of the learned mapped port.
Afterward it completes incoming → answer → ACK → active → remote BYE → idle,
repeats calls, rejects duplicate/stale actions, handles CANCEL, exercises a real
missing-ACK timeout (about 32 seconds), then accepts another call.

The helper's six standalone C state-machine tests cover late interfaces, removal,
active-call deferral, debounce, capped retry and failed enumeration. Address
handling uses the actual SDK's libre implementation. macOS C warnings-as-errors
and ASan/UBSan checks also passed during implementation.

Fresh integrated checks passed on macOS arm64:

- `bun run test:ui`: 12 tests; browser fixtures additionally exercised native-state
  events, delayed initial reads, asynchronous listener teardown, General saving
  and settings draft protection. Screens inspected at 900×660 and 740×540.
- `bun run check`, `bun run build`, `bun run check:versions`.
- `bun run format:check`; repeated type generation is byte-for-byte stable.
- Portable workspace tests/Clippy and native workspace tests/Clippy with warnings
  denied; explicit native desktop tests. OS-keychain-write tests remain opt-in.
- `bun run test:asterisk`: four interop cases and real G.711 RTP echo (~440 Hz).
- `bun run test:sip-diagnostics`: 60 allowlisted markers and diagnostics off by
  default. Native markers include elapsed monotonic milliseconds/local generation.
- `bun run test:network-roam`: all six C tests through the portable CMake runner.
- Native macOS release `.app` build and signed microphone entitlement/purpose check.

An isolated native smoke bundle, using a separate identifier and no real accounts,
was exercised through macOS accessibility and running-application APIs:

- `--background`: no visible windows, Accessory policy, no focus activation.
- A normal second launch preserves the PID and opens the phone with Regular policy.
- Closing the phone keeps the process alive, with no visible windows/Accessory policy.
- An actual mouse click on the status icon opens the expected native menu;
  “Telefon öffnen” restores the phone and Regular policy.
- Native settings open separately; closing all windows leaves the status app running.
- Native “SIP beenden” exits. Tauri's default-menu startup override was observed
  and corrected by supplying the menu through Builder before setup.

The whole-feature read-only review found no remaining must-fix issues after the
startup keychain UI guard and authoritative Quit-state fixes.

## Limits and remaining acceptance

Real incoming UniFi Talk/Tailscale media and ACK routing still need user acceptance;
the synthetic reproduction demonstrates the bug class, not every real topology.
No network/VPN configuration was changed. Windows tray/installer/login behavior
and actual login/logout/wake acceptance have not been executed here; CI includes
the cross-platform native helper test and existing native build matrix.

The desktop notification plugin cannot query actual OS permission (`null` in the
API). macOS login-start querying confirms a LaunchAgent entry, not external OS
background-item authorization. System notification/Focus policies remain effective.

The local release toolchain emitted a rust-objcopy LLVM lookup warning; bundling
and signing succeeded. Host Homebrew OpenSSL also carries a newer deployment
target than the configured macOS 14 minimum; this run does not certify macOS 14.
Production signing/notarization remains outside this local ad-hoc development build.
