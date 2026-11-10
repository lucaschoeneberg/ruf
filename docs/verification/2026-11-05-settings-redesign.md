# Settings redesign verification · 2026-11-05

Historical macOS verification from 5 November 2026, based on source baseline
`a0e281a25406c80b8f8035bb1d0532bc954b3949`. Current contracts are maintained in the
[API reference](../api/README.md); these observations do not qualify later builds.

## Behavior and reused patterns

- Eight settings sections, field search and draft protection; the existing direct
  **Deine Konten** block, plus button and eight-account limit remain below navigation.
  Account links focus their existing form. Flat rows, shared font/palette,
  native singleton windows and explicit saves are preserved.
- Independent read-only notification authorization on launch/open/focus; failed
  reads remain unknown. Permission checks do not restore accounts or read secrets.
  App preference saves never request OS authorization. A separate explicit action
  requests undecided permission; denial links to fixed OS settings.
- Anonymous five-second test notifications use their own native identifiers and
  no call actions. Caller-detail privacy updates live call captions without
  altering call action tokens or retry ownership. Enqueue is never described as
  proof of a visible banner.
- Privacy reuses ContactsSettings and adds AVFoundation authorization metadata;
  opening privacy captures no samples or contact records. Diagnostics uses a
  twelve-field whitelist, nullable unavailable readings, cached nonblocking phone
  state and explicit clipboard copying with the existing WebView/native pattern.
- Account activation uses existing native acknowledgments, guarded operations,
  session identities, credential policies and atomic metadata persistence.
  Disabled accounts restore without secrets/native UAs, retain selection, and
  cannot dial or silently switch outgoing identity. Late authorization cannot
  defeat a newer disable request. Login-only visibility preserves manual opening
  and existing explicit background/notification activation. Only configured,
  already-enabled Ruf login entries have their arguments refreshed.
- Native audio tests reuse baresip sources/player/event-loop ownership: microphone
  samples are discarded, peak updates at most 10 Hz, capture at most 10 s, speaker
  playback 2 s; real calls preempt tests. Device selection uses the draft. Three
  ringtones and 0..100 volume preserve original classic/100 PCM exactly. Ringtone
  preview uses saved tone/volume and the selected draft ring output, as labeled.
- Existing contact provider/favorites/search honor display preferences independently
  of metadata failures. The ordered history worker latches recording for future
  calls, prunes completed rows only on startup/configuration/daily intervals, and
  confirms retention shortening before deletion. Schema-v1 bytes are durably backed
  up before v2 replacement; keys, selection and cleanup references are preserved.
- No new package dependencies or second audio pipeline. Public Rust contracts use
  documentation comments and ts-rs; Tauri permissions follow existing generated ACLs.

## Observed checks

All Rust verification below uses pinned Rust 1.98.1 with the Rustup shims ahead of
system Rust on PATH; Bun 1.4.4. Native SDK was built before native checks.

| Check                                                            | Result                                                                                                                                                      |
| ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bun run native:build`                                           | SDK staged successfully for aarch64-apple-darwin.                                                                                                           |
| `bun run format:check`                                           | Passed.                                                                                                                                                     |
| `bun run check`                                                  | Svelte/TypeScript 0 errors, 0 warnings.                                                                                                                     |
| `bun run build`                                                  | Passed.                                                                                                                                                     |
| `bun run check:versions`                                         | JavaScript/Cargo/Tauri versions and pinned tools match.                                                                                                     |
| `bun run types:generate` twice                                   | Identical generated-file SHA-256 on second generation.                                                                                                      |
| `bun run test:ui`                                                | 61 passed,0 failed.                                                                                                                                         |
| `bun run test:ui:flows`                                          | contacts/history/phone/settings/layout all passed with synthetic IPC.                                                                                       |
| Layout                                                           | Eight sections at 900×660 and 740×540, eight-account independent scroll, keyboard search, effective 125% layout at 592×432. Screenshots visually inspected. |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed, repeated after final Rust changes.                                                                                                                  |
| `cargo test --locked --workspace`                                | Passed, repeated after final Rust changes;70 desktop tests passed,1 real-keychain test ignored.                                                             |
| Native workspace clippy with `--features sip-desktop/native`     | Passed, repeated after final Rust changes.                                                                                                                  |
| Native workspace tests with `--features sip-desktop/native`      | Passed:67 adapter tests and83 desktop tests, plus core/integration/doc tests;6 lab and4 desktop opt-in tests ignored.                                       |
| Final desktop native suite after startup helper                  | 84 passed,0 failed,4 opt-in tests ignored.                                                                                                                  |
| Final native audio diagnostic regression                         | Passed after permission-policy alignment; ownership, exclusivity, timing and shutdown.                                                                      |
| `bun run test:asterisk`                                          | 6/6 passed, including fresh initial incoming before any outgoing call, multiple accounts, SIP termination/actions and echo media.                           |
| Native CMake `call_actions_test`                                 | Passed; queued actions/account guards and audio DSP.                                                                                                        |
| `bun run test:macos-notification-timeouts`                       | Passed stalled/late-callback, withdrawal/retry and independent anonymous test-delivery harness.                                                             |
| `bun run desktop:build --features native --bundles app`          | Native Ruf.app plus matching SBOM/build hashes produced.                                                                                                    |
| `bun run test:macos-audio <bundle>`                              | Signed audio-input entitlement and microphone usage description verified.                                                                                   |
| Independent review                                               | Approved; all findings resolved and regression verified.                                                                                                    |

Generated contract SHA-256:
`49a694dfcd48236125311558e1575fbea281a49245cdef33ab12f1f4cbb7f9ab`.
Full command logs and source manifests remain in private task files under `/tmp`;
no generated bundles, credentials or personal call/contact data are added to Git.

## Review corrections

A failed preference read originally had no independent working retry. The final
implementation retains a domain-specific error and retry so concurrent phone
refresh cannot erase it. A real transient-failure browser fixture verifies
recovery without reopening and without replacing drafts. Late microphone
permission callbacks are checked again before starting native capture. A compact
height rule preserves the account list and footer in enlarged layout. Backend and
frontend agree that unsupported permission metadata permits an explicit native
test, while failed queries and known denial do not.

## API documentation

Changed commands, required/optional input, nullable output, validation, errors and
capabilities are documented in [API overview](../api/README.md),
[notifications](../api/notifications.md), [permissions](../api/permissions.md),
[diagnostics](../api/diagnostics.md), [preferences](../api/preferences.md),
[account activation](../api/account-activation.md),
[audio diagnostics](../api/audio-diagnostics.md) and [history](../api/history.md).

## Remaining verification limits

No production-provider call, physical microphone/speaker test, real notification
banner/Focus test, OS permission prompt/reset, keychain write or real clipboard
write was performed for this redesign. Four opt-in desktop tests remain ignored;
Asterisk covers the six adapter lab tests separately. Windows runtime/cross-target
execution and hosted CI were not observed. Windows microphone authorization
metadata remains unsupported; explicit capture relies on its native backend.
WebView clipboard is attempted there, but the native fallback remains macOS-only.
The locally ad-hoc-signed app was built without notarization credentials. The
installed app and production credentials were not changed. Metadata/SQLite saves
roll back observed failures; cross-store power-loss atomicity is not established.
