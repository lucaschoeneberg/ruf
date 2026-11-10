# Ruf product scope

Ruf is a compact SIP softphone built with Rust, Tauri 2 and Svelte. Release targets
are macOS 14+ on Apple Silicon and Windows 11 x64. The current 0.1.0 preview is
for isolated test networks; supported provider/platform profiles require observed
qualification. See the [README](../../README.md), [release checklist](../../docs/release-checklist.md)
and [verification evidence](../../docs/verification/README.md).

## Current scope

- Up to eight independently registered SIP accounts, with explicit outgoing
  identity, activation and OS credential storage. Fresh account fields are empty;
  the user's test PBX is the primary setup path and Asterisk is an optional lab.
- One call plus one related consultation, with one audible conversation. Native
  mute, local/remote hold, DTMF and transfer results follow observed state.
- UDP/TCP/TLS and required SDES-SRTP with TLS. Configured policy is distinct from
  observed protection; there is no plaintext fallback for required protection or
  claim of end-to-end encryption.
- Authorized macOS contacts, local favorites and SQLite call history. Contact
  names are current in-memory projections; history retention and collection are
  explicit preferences. Windows contacts are not implemented.
- Separate phone/settings windows, background ringing, tray controls, action/
  contact/history search and accessible keyboard/menu actions. Closing the phone
  hides it; explicit Quit completes the native shutdown path.
- Native audio routing/tests, explicit OS permission actions, anonymous diagnostics
  and notification privacy. Automatic updates and calendar integration are absent.

Exact command, DTO, validation and error semantics live in the
[API reference](../../docs/api/README.md), not in a second product model.

## Product rules

- Keep dialing, current identity and available call actions easy to reach in the
  compact window. Use concise German labels and visible keyboard focus.
- Browser previews and portable builds must describe their missing native
  capabilities. Never fabricate registration, security, contact or media success.
- Preserve dirty settings drafts through failures and incoming calls. Confirm
  destructive history/account actions; selected identity alone is not registration.
- Permission prompts require explicit actions. Passwords stay in the OS store;
  contacts and history stay local. Personal results are never search telemetry.
- Reuse the existing Rust contracts, Tauri IPC and baresip pipeline. See
  [architecture](../../docs/architecture.md) and [design rules](DESIGN.md).
