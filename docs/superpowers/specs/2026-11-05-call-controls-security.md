# Ruf call controls and encrypted accounts

Approved scope: the user's 2026-11-05 instruction implements all four proposed
steps in an extra worktree. Accounts remain under Settings. Continue through
implementation and native SIP verification without another planning handoff.

## Behavior

- Mute is per call and suppresses transmitted microphone/test audio in baresip;
  incoming audio remains available. UI state comes from native confirmation.
- Hold/resume distinguishes local hold, remote hold and a pending re-INVITE.
  Failed or timed-out renegotiation restores the prior usable state. A second
  operation cannot race an unfinished hold or transfer.
- Blind transfer sends REFER and waits for final NOTIFY success, failure or a
  bounded timeout. Failure retains the original call and allows recovery.
- Consultation supports an original call plus one consultation call, a single
  audible conversation, switching, cancellation with return to the original,
  and attended transfer using REFER/Replaces. Track each dialog by its own
  opaque token and owning account. Old actions cannot target a replacement call.
- Unsolicited incoming REFER remains rejected; outgoing transfer does not opt
  into accepting remote instructions to dial arbitrary targets.
- Per-account UDP/TCP/TLS and disabled/required SDES-SRTP. TLS plus required SRTP
  is the recommended encrypted profile; no automatic plaintext downgrade.
  Required SDES-SRTP must require TLS. Verify server certificate chain and host.
  Trust roots must work in packaged macOS builds without a Homebrew path;
  explicit trusted private CA configuration may supplement platform roots.
- Distinguish configured encryption from observed SIP transport and media
  protection. Never label a call encrypted based only on a preference. Explain
  that PBX hop encryption does not guarantee end-to-end encryption.
- Account settings can specify a separate authentication username and outbound
  proxy where needed. Validate bounded structured fields, never raw SIP headers.
  Preserve existing account/keychain data through defaulted schema migration.
- Keep the current compact Svelte UI. Mute, hold and transfer belong to the call
  panel. Consultation shows both parties and clear return/complete actions.
  Accounts and security remain under Settings, including edit/save/restart.

## Architecture and constraints

Reuse baresip 4.12.0, libre and existing OpenSSL linkage. No replacement SIP stack
or frontend audio pipeline. Native calls and unsafe code stay in baresip-adapter;
phone-core contains portable documented types. Rust edition 2024 with pinned
toolchain. Generate strict TypeScript contracts using ts-rs. Preserve the legacy
current-call snapshot as a projection while adding per-dialog state. History,
notifications, startup registration and background reachability must retain
correct ownership and termination behavior. Do not change system permissions,
installed Ruf, production accounts, or credentials for tests. No secrets, audio
recordings or build output in Git. Do not merge into main or push.

## Verification

First pin regression tests and observe their failure. Test native state guards,
SIP hold acceptance/refusal/glare, mute/returned audio, blind and attended
transfer with success/failure/timeout, stale actions and consultation cleanup.
Extend the isolated Asterisk lab with generated test certificates and TLS/SRTP
peers. Verify positive encrypted audio and negative trust/hostname/plaintext
downgrade cases. Keep all existing native SIP regressions. Run UI flows, Rust
tests/clippy (portable and native), format, types stability, versions, frontend
build and native desktop build. Record observed results and remaining platform
or provider limitations honestly.
