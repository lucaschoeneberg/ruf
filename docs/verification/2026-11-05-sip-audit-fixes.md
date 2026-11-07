# SIP audit corrections — 5 November 2026

## Scope and implementation

Base revision: `b8baa10b97ad4ff5975f03094e8638eb786d622d`. The user requested
correction of all ten demonstrated defects and the three additional
interoperability issues in the preceding SIP audit. Work reuses the existing
checkout, pinned Rust 1.98.1, baresip/libre 4.12.0 and their native event loop.
No dependency, public command, DTO field or second media pipeline is added.

| Audit issue                                                 | Correction                                                                                                                          | Durable behavioral evidence                                                                                                                 |
| ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Local resume after long hold immediately timed out          | Renew the existing RTP receive deadline after confirmed local resume                                                                | `long_local_hold_resume_gets_full_media_grace_interval`: 65 seconds held, then full resumed grace                                           |
| Rejected SDP corrupted subsequent hold/resume               | Keep the last accepted remote SDP body; restore on parser/media rejection; emit remote hold events after accepted media application | `rejected_offer_does_not_break_next_remote_hold_detection`, `rejected_offer_does_not_break_next_remote_resume_detection`                    |
| Additional successful fork dialog was ignored               | Reuse native dialog forking, authenticated ACK/BYE ownership and cached duplicate ACKs                                              | `forked_2xx_acknowledges_and_ends_extra_dialog`: exact ACK replay, no duplicate BYE, selected dialog survives                               |
| Accepted invalid hold answer only rolled back locally       | ACK, enter bounded existing recovery and require accepted compensation; invalid compensation ends the uncertain dialog              | `missing_sdp_accepted_hold_requires_reconciliation`, updated existing invalid-answer/retry test                                             |
| Attended-transfer failure on background original was hidden | Render dialog-owned transfer/control failure in its conversation row                                                                | `attended transfer error is visible while consultation is selected`                                                                         |
| Hash in phone URI was unescaped                             | Reuse libre URI-user formatting in dial, consultation and blind REFER                                                               | `service_code_hash_is_uri_escaped`, `transfer_hash_is_escaped_in_refer_to`                                                                  |
| INFO retransmission repeated a digit                        | Use the existing native server transaction for replies                                                                              | `duplicate_info_does_not_repeat_digit`                                                                                                      |
| re-INVITE 481 retained a lost dialog                        | Preserve terminal 408/481 handling before managed hold-refusal handling                                                             | `hold_481_does_not_keep_nonexistent_dialog`                                                                                                 |
| Third busy incoming offer was absent from history           | Project the pre-allocation native busy event through the same bounded, privacy-filtered identity/journal boundary                   | `third_incoming_offer_is_recorded_during_consultation`                                                                                      |
| Initial final 200 without required SDP became active        | ACK and close as a failed negotiation before establishment                                                                          | `final_200_without_sdp_does_not_claim_active`                                                                                               |
| Incompatible codec reoffer silently removed all audio       | Reject with 488 and restore accepted SDP/media; mandatory SRTP checking retains precedence                                          | `incompatible_codec_update_preserves_existing_session`, combined invalid-crypto/codec tests                                                 |
| 183 followed by SDP-less 180 lost early media               | Preserve native early-media playback and the snapshot                                                                               | `early_media_followed_by_ringing_keeps_early_media`, `early_media_pcm_continues_after_sdp_less_ringing`: fresh nonzero 500 Hz PCM after 180 |
| Malformed INFO signal was silently shortened                | Validate one signal and decimal duration of 1–5000 ms, stateful 400 on malformed input                                              | `invalid_multi_digit_info_is_rejected`, `malformed_info_fields_are_not_silently_truncated`                                                  |

Existing transfer intent is retained: failed blind transfer preserves manual
hold; failed consultation setup restores the pre-consultation intent; explicit
cancel/return resumes the original. The corresponding native regressions remain
in the suite, including retry after failed attended transfer.

## Independent review and resulting fixes

A fresh-context independent reviewer inspected the working-tree patch and both
applied native SDK trees. All three actionable findings were reproduced and
corrected:

1. `sip_dialog_fork` lacked To/From/route bookkeeping offsets. Populate those
   existing offsets; the regression exercises early branch A, selected answer B,
   discarded answer C and subsequent hold/resume requests on B without duplicate
   dialog headers.
2. Early codec rejection skipped mandatory-SRTP validation. Preserve media
   security validation before recoverable codec rejection. Real TLS regressions
   combine incompatible codecs or downgraded media transport with missing crypto,
   require owned fatal media-security failure, BYE and a reusable account.
3. Retaining the early-media snapshot alone did not retain playback. The native
   progress handler now keeps accepted media when a subsequent 180 has no SDP;
   fresh RTP-derived PCM verifies continuity rather than inferring it from state.

No review findings were deferred. The fixes use the existing SDK security and
transaction boundaries; the bounded fork cache permits 32 discarded branches
within its original 64*T1 retention interval.

## Test infrastructure repairs

The network/C queue-handler fixture still referred to the pre-consultation
single-call structure. It now uses the actual selected native dialog and links
the same native security implementation as the bridge.

The Asterisk command helper waited for process exit before reading piped output.
Large channel diagnostics could fill the pipe and cause a false 15-second
timeout. Both streams are drained while the command runs, retaining the existing
deadline. A one-megabyte stdout plus stderr subprocess regression reproduced the
deadlock before the correction and passes afterwards. The full Asterisk run then
passed, including both remote-end consultation variants.

## Changed files and reused mechanisms

| Files                                                                                                                          | Responsibility                                                                                              |
| ------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------- |
| `native/patches/re-reliability.patch`, `native/patches/baresip-reliability.patch`                                              | Corrections appended after existing NAT/control/security patches; pinned upstream checkouts remain pristine |
| `native/build.mjs`, `crates/baresip-adapter/build.rs`, `apps/desktop/build/software-bill.ts`, `tests/ui/software-bill.test.ts` | Exact patch identity in SDK validation and software-bill hashes                                             |
| `crates/baresip-adapter/native/bridge.c`, `native/call_controls.h`, `src/ffi.rs`                                               | Existing native call control, safe identity projection and selected snapshot                                |
| `apps/desktop/src/lib/components/CallPanel.svelte`, `tests/ui/call-controls.test.ts`                                           | Owner-aware background failure display and render regression                                                |
| `crates/baresip-adapter/src/sip_event_tests.rs`, `src/sip_reliability_tests.rs`                                                | Existing real loopback SIP Lab plus durable audit regressions                                               |
| `crates/baresip-adapter/src/security_tests.rs`, `tests/native/tls_peer.py`                                                     | Existing isolated TLS/SRTP peer with combined invalid-offer cases                                           |
| `crates/baresip-adapter/src/asterisk_tests.rs`, `native/tests/call_actions.c`, `native/tests/CMakeLists.txt`                   | Real PBX/media validation, bounded subprocess helper and current C fixture                                  |
| `docs/api/call-controls.md`, `docs/api/sip-events.md`, `docs/api/history.md`, `native/patches/README.md`                       | Outcomes, validation/errors, privacy, ownership and private SDK helper contracts                            |
| `docs/superpowers/plans/2026-11-05-sip-audit-fixes.md`, this report                                                            | Scope, implementation decisions and observed evidence                                                       |

Rust public APIs and generated TypeScript DTOs are unchanged. New SDK helpers
remain private; their required arguments, borrowed ownership, native failure
behavior and retention limits are documented in the patch README.

## Verification observed

All listed checks completed successfully. The final serial native adapter suite
finished in 533.95 seconds; all 21 new SIP regressions passed in that run.

| Command                                                                             | Observed result                                                                                                                                              |
| ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `bun run native:build`                                                              | SDK rebuilt successfully; native NAT tests passed; manifest includes both exact reliability patches                                                          |
| `bun run check`                                                                     | Zero Svelte errors/warnings; TypeScript passed                                                                                                               |
| `bun run build`                                                                     | Production frontend build passed                                                                                                                             |
| `bun run test:ui`                                                                   | 88 passed                                                                                                                                                    |
| `bun run test:ui:flows`                                                             | All seven browser flow groups passed                                                                                                                         |
| `bun run check:versions`                                                            | Passed with pinned Rust 1.98.1 on PATH                                                                                                                       |
| `bun run types:generate` plus generated-file diff                                   | Passed; generated TypeScript unchanged                                                                                                                       |
| `bun run format:check`                                                              | Passed                                                                                                                                                       |
| Portable workspace Clippy with `-D warnings`                                        | Passed                                                                                                                                                       |
| Native workspace Clippy with `-D warnings`                                          | Passed                                                                                                                                                       |
| `cargo test --locked --workspace`                                                   | Passed, including 72 desktop unit tests; the OS device test remains explicitly ignored                                                                       |
| `cargo test --locked --workspace --features sip-desktop/native -- --test-threads=1` | Passed: 116 adapter tests, core tests and 87 desktop tests; 15 adapter lab tests and four desktop OS/credential/lab tests explicitly ignored by this command |
| `bun run test:asterisk`                                                             | Passed: 16 ignored/lab tests executed, then 13 TLS/security tests; fresh echo, hold/resume, mute, IVR and post-transfer PCM measured                         |
| `bun run test:network-roam`                                                         | Six recovery state-machine cases and production queue-handler fixture passed                                                                                 |
| `bun run desktop:build`                                                             | Portable app and DMG built successfully                                                                                                                      |
| `bun run desktop:build --features native`                                           | Native app and DMG built successfully; software bill and executable/installer hashes exported                                                                |

Initial shell PATH selected Homebrew Rust 1.99.1. Final Rust checks, version/type
checks and desktop builds use the installed pinned toolchain directory first.
The pinned toolchain's objcopy needed its existing `libLLVM.dylib` on
`DYLD_FALLBACK_LIBRARY_PATH` for packaging. An initial native DMG attempt hit an
occupied temporary image; after verifying and detaching only that run's own
image, normal native packaging succeeded. No toolchain install, system trust
change or unrelated mounted-image cleanup was performed.

## Remaining limits

Tests ran locally on macOS arm64, with synthetic SIP identities and exclusive
test certificate roots. No hosted CI, Windows runtime, physical microphone/
speaker qualification, sleep/wake, arbitrary provider/VPN topology or macOS 14
runtime execution is claimed. The Homebrew OpenSSL archive emits a deployment
target warning (built for the newer host); successful local linkage does not
qualify that archive on older supported macOS versions. Bundles use ad-hoc signing
and were not notarized. Build output, recordings and certificates stay ignored.

The 15 ignored adapter lab tests were executed separately by the Asterisk runner.
The four explicitly ignored native desktop tests require OS device metadata,
actual credential-store writes or credential-specific PBX scenarios; those were
not executed. No hardware/keychain coverage is inferred from unit or PBX tests.
