# Saved-password recovery verification

Observed on local macOS arm64. The user's exact combined message is produced by
AccountSetup rendering a per-account storage error followed by an unconditional
SIP-credentials hint. Startup credential reads previously converted every keyring
error to storageFailure before any SIP registration could occur.

Read-only diagnostics found valid/readable/writable current account metadata and
matching keychain-entry metadata. The running app was ad-hoc signed. No personal
password was requested by these diagnostic queries. The old app discards the
underlying OS read error, so its exact denial is unknown; ad-hoc identity changes
are a possible authorization trigger, not a proven diagnosis.

## Repair and reuse

- CredentialAccessRequired, CredentialMissing and CredentialFailure distinguish
  OS authorization, absence and other credential errors from file/SIP failures.
  Known macOS errors are classified using the existing Security.framework error
  type and SDK status definitions; raw messages/secret payloads never escape.
- Explicit settings-only reconnect_account(id) reads the same saved password
  interactively outside the session lock. It revalidates account ID/key/metadata,
  shutdown and call state afterward and reuses native save_account application.
  No metadata or credentials are rewritten/rotated/deleted by reconnect.
- The account form removes the unconditional wrong-password hint, offers explicit
  retry for local access failures, and requires a new password only when absent.
  Success/cancel preserve dirty drafts. Startup remains silent.
- Existing CredentialStore, native account guards, snapshot revisions, ts-rs,
  Tauri command permissions and keyring service identity are reused. No dependency
  added, no legacy data imported and no credential protections relaxed.

Main files: core phone.rs and generated AppInfo.ts; desktop store.rs, phone.rs,
commands.rs, lib.rs, build.rs, settings capability/generated permission; Settings,
AccountSetup and api.ts; native recovery and UI tests. API contract:
[saved credential recovery](../api/credential-recovery.md).

## Verification

- Existing authorization-contention test first failed: expected access-required,
  received storageFailure. It passes after classification repair.
- UI regressions first reproduced the misleading suffix, missing recovery action
  and unsafe native-message fallback; all now pass.
- Five native recovery tests passed with actual loopback UDP/libre/baresip and
  synthetic in-memory credentials: silent startup then explicit recovery/REGISTER,
  accessible live state during the prompt, cancellation without mutation, incoming
  call winning during authorization, shutdown and unknown-ID rejection. File bytes
  remain identical and credential writes/deletes remain zero.
- Store tests passed for missing/invalid/unavailable credentials, OS authorization
  and cancellation, native payload exclusion and metadata persistence.
- Native SDK rebuilt successfully before native workspace checks.
- Portable and native workspace Cargo tests and Clippy with warnings denied passed.
- bun format:check, check, build, check:versions and git diff --check passed.
- Type generation succeeded; repeat generation was byte-identical.
- All 41 UI unit tests and five browser suites passed, including id-only reconnect,
  no automatic prompt, dirty-draft preservation and call-state disabling.
- Asterisk RTP echo and all five PBX interop tests passed; diagnostics verified
  remote BYE acknowledgment and privacy allowlisting. Only synthetic accounts/audio.
- Native Ruf.app built and ad-hoc signed at target/release/bundle/macos/Ruf.app
  (19.55 MiB). Independent scoped review found no important issue.

## Limits

The actual OS authorization dialog for the user's existing credential has not
been approved/tested by automation. Start the rebuilt app and explicitly select
Erneut verbinden in the saved account, then respond to the OS dialog. Existing
opt-in real-keychain write tests were not run; personal credentials and settings
were not altered. Windows runtime and hosted CI were not executed. The local
bundle is ad-hoc signed, not notarized.
