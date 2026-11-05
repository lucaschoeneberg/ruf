# Dialing, current contact identities and native DTMF

## Integration and scope

`main` was fast-forwarded from `5bc847a` to `2b28a0c`, retaining the settings
foundation and all 20 call-controls/security commits. `chore/project-foundation`
at `e1512cb` was already an ancestor. No divergent commit was discarded.
The unfinished contacts/dialpad changes were copied to main and independently
preserved in stash commit `3a89465dba78b610572bd9a1f7cda13cd38efeac` before further
edits. The original dirty feature worktree remains untouched as another backup.
No remote push, installation, production-app restart or personal test call was
performed for this change. Accounts remain under Settings.

The dialer now accepts formatted numbers and a leading plus, with explicit plus
insertion, numeric keys and up to three authorized contact suggestions. Choosing
a suggestion selects its existing destination; it never places a call. Typed
letters are prevented; unsupported pasted text is rejected rather than extracted
into another destination. The existing Rust `contact_dial_target` is reused for
dial, transfer and consultation validation; the frontend mirrors its checks.

Each live dialog has a required nullable `CallSnapshot.contact`. Incoming and
outgoing conversations, notifications and historical projections use the current
authorized address book, including later additions/renames. Revocation and failed
reads remove displayed names; multiple equivalent owners do not assert an
identity. Matching never authenticates a caller. The optional
`contacts.matchGermanNumbers` preference defaults to true, including legacy
preferences, and equates national 0… with +49…/0049… for display/search only.
It never rewrites the number sent to the phone system. Other countries retain
conservative matching. Native names remain memory-only.

An expandable conversation keypad sends `0–9`, `*` and `#` through the existing
baresip media path. The new main-only `send_dtmf` IPC uses the existing blocking
session worker and opaque call tokens. It accepts one ASCII key for the selected,
established, unheld conversation. A native 32-key FIFO controls 160 ms key holds
and 80 ms gaps, revalidates ownership, and discards pending keys on hold, switch,
close or shutdown. No browser audio pipeline, new dependency, PIN log or durable
outgoing-digit history was added. Queue acceptance does not prove IVR delivery.

## Code and contracts

- `crates/phone-core/src/{contacts,desktop,phone}.rs`: shared normalization,
  optional regional matching, preference defaults and dialog identity DTO.
- `crates/baresip-adapter/src/{engine,dialogs,asterisk_tests}.rs` and
  `native/{bridge.c,call_controls.h}`: numeric validation, native DTMF ownership,
  timing and lifecycle integration.
- `apps/desktop/src-tauri/src/{contacts,history,call_notifications,commands,preferences}.rs`:
  current authorized projections, notification captions and thin IPC/preferences
  integration. Existing command registration and window capabilities are reused.
- `apps/desktop/src/{App.svelte,lib/components,lib/contacts.ts,lib/dialing.ts}`:
  nonprompting cache refresh, request ordering, number suggestions and expandable
  keypad using existing compact controls/tokens. Settings search includes the new
  matching preference. Generated TypeScript comes from Rust through ts-rs.
- API documentation in `docs/api/{README,call-controls,contacts,history,notifications,permissions,preferences}.md`
  describes required/nullable DTO fields, inputs, normalization, errors, privacy
  and main-only authority. README, product/design docs, changelog and release notes
  were synchronized, including removing obsolete TLS/SRTP omission text.

## Observed checks on macOS arm64

| Check                                                                                  | Result                                                                                                               |
| -------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| SDK rebuild, `bun run native:build`                                                    | Passed; staged SDK used for native checks.                                                                           |
| Native workspace tests, `sip-desktop/native,baresip-adapter/native-test-hooks`, serial | 214 passed, 0 failed, 19 ignored. Adapter 106 passed/15 ignored; desktop 87 passed/4 ignored.                        |
| `bun run test:asterisk`, final serial run                                              | 15 Asterisk tests and 11 controlled TLS tests passed, plus strict UDP echo.                                          |
| Portable `cargo test --locked --workspace`                                             | 120 passed, 0 failed, 1 ignored.                                                                                     |
| Native and portable all-target workspace clippy, `-D warnings`                         | Both passed.                                                                                                         |
| `bun run test:ui`                                                                      | 87 passed, 0 failed, 1,283 assertions.                                                                               |
| `bun run test:ui:flows`                                                                | All seven real-browser suites passed: contacts, history, phone, call-controls, dialer-identity, settings and layout. |
| Svelte/TypeScript check                                                                | 0 errors, 0 warnings.                                                                                                |
| Production web build and version check                                                 | Passed; version 0.1.0 consistent.                                                                                    |
| Type generation twice with SHA-256 comparison                                          | Stable generated contract after the deliberate DTO changes.                                                          |
| Native production desktop prebuild, `--features native --no-bundle`                    | Passed, no test-hook feature selected.                                                                               |

`bun run format:check`, Cargo formatting and `git diff --check` also passed.
The final production bundle command, `bun run desktop:build --features native
--bundles app`, passed and produced Ruf.app (21.12 MiB).

The new Asterisk extension 605 only plays an 880 Hz tone after receiving `5`
terminated by `#`. The test queues both keys rapidly through the real engine and
measures 16,000 returned samples at 880.0 Hz, RMS 5102. It also rejects malformed
and stale tokens, queues tones before immediate hangup, opens a new conversation
and checks that no queued key leaks into it. Existing registration-before-first-
outgoing-call, media, mute, hold, transfer and TLS/SRTP scenarios remain green.
Expected TLS certificate and mandatory-SRTP failure messages come from negative
tests; they are not omitted test failures.

Browser coverage uses synthetic contacts only. It exercises caret-aware plus
insertion, spaces, letter prevention, normalized dialing without country rewrite,
national/international suggestions, rapid ordered `5#`, current outgoing names,
rename/revocation, disabled held-call tones and contacts added after a history row.
No contact permission prompt is initiated by lookup. An existing listener-ordering
test caught an initial refresh-before-subscribe race; subscription now precedes
the initial read. Fixture expectations were updated for intentional input and
notification-caption changes; failed intermediate runs are not counted as passes.

The five-axis review covered correctness, readability, existing architecture,
privacy/authority and bounded work. A leading-plus validation mutation in an
isolated source copy turned the corresponding passing test red; tracked source
was not altered by the experiment. Impeccable review retained the established
compact design. Synthetic captures at 340×500 and 320×440 were inspected; numeric
suggestions and the scrolled in-call keypad remain usable. The detector was run
once on the changed interface and reported no findings. Captures/build output and
temporary test recordings are ignored and not committed.

## Final production artifact

The final package was built from clean implementation commit
`84d20b1ec34c1f9bf634ad5e9088369ef41d87d3` on `main`, with only `native` enabled.
Its local path is
`target/aarch64-apple-darwin/release/bundle/macos/Ruf.app`. The executable in the
signed bundle exports the same CycloneDX inventory as the raw release executable.
The build manifest matches the raw executable and inventory hashes. The final
feature graph excludes `native-test-hooks`; both symbol tables exclude
`sip_engine_set_test_ca` and `sip_engine_set_test_dns`. Local
`codesign --verify --deep --strict` succeeded. A hash snapshot of all 396 tracked
files was retained outside Git for local source binding. This verification text
was subsequently extended without changing the implementation or app assets.

| Artifact                       | SHA-256                                                            |
| ------------------------------ | ------------------------------------------------------------------ |
| Raw release executable         | `b23ae32ce71adb3c8d21bc8e2d6ae83065f1758e74b4858a3ee1c2c9dc8ff03e` |
| Bundled signed executable      | `7d49ef9fc8647ae47f370b9afd7c80bd8a0727be8eee8fb1209e0a75ad61ee69` |
| Exported native CycloneDX SBOM | `f454afff9a802bcd54a5395bce21c4e1702baf5489fbb7042ea022032a1d2c80` |

## Limits

This is observed local macOS/loopback verification. It does not establish physical
microphone/speaker behavior, personal provider interoperability, actual current OS
contact sync or notification banners, Windows/Linux runtime, hosted CI or older
macOS runtime compatibility. OS credential-writing opt-in tests were not run.
The existing newer-macOS OpenSSL object and duplicate-library warnings remain;
they predate this change. Ad-hoc signing is not notarization. Neither system
permissions nor trust settings were changed for these tests.
