# Zuverlässige Anrufe und Hintergrundbetrieb

Approved implementation: repair incoming SIP dialog NAT routing with a synthetic wire
regression, reserve an answering state and call token; initialize and observe the
existing phone session independently of windows; add native tray/menu controls,
hide-on-close, single instance, silent installed-app login start, and notification
preferences. Preserve native audio, keychain storage, compact Zed UI and network
configuration. Verify native interop, Rust/Bun checks and installed platform behavior
where available; never claim Windows runtime or real UniFi acceptance without it.

## Execution ledger

- Existing feature checkout reused: `chore/project-foundation`; uncommitted prior
  work must remain intact. No commit, push, or new worktree requested.
- Independent native SIP, network recovery and frontend tasks delegated using
  Superpowers parallel execution; desktop lifecycle and integration remain here.
- Shared interfaces: `CallState::Answering`, `EngineSnapshot.callId: string|null`,
  guarded answer/hangup IPC, `DesktopSettings`, `phone-state-changed` and
  `settings-section` events. Canonical types remain in phone-core/ts-rs.
- Native incoming RED: synthetic learned-port mapping is not used when incoming
  PBX port differs from registrar port. Exact-peer precedence and ambiguous mapping
  rejection must remain covered.
- Native quit confirmation requires the official Tauri dialog plugin in addition
  to the three planned plugins; reuses the platform UI instead of a second dialog
  implementation.
- Implementation and local verification complete; evidence and remaining platform
  acceptance are recorded in `docs/verification/2026-11-04-background-calls.md`.
- Review fixes: noninteractive macOS keychain restoration, authoritative Quit
  state, and Builder-owned native menu initialization. New menu call actions also
  require a currently published call token.
- Desktop notification authorization remains unknown rather than being reported
  as granted; macOS login-start status means registered LaunchAgent, subject to
  external OS background-item policy. Real UniFi/Windows/login acceptance remains
  explicitly open.
