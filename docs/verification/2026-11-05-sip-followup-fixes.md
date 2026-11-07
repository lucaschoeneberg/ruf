# SIP follow-up corrections — 5 November 2026

## Scope

The user requested correction of all eight findings from the follow-up SIP audit
and permanent regression coverage. Base HEAD is
`b8baa10b97ad4ff5975f03094e8638eb786d622d`, including the preceding uncommitted
SIP corrections already present in this checkout. This report describes the
additional work; the preceding report remains a record of its own observed runs.

The existing native call controller, libre transactions, call-generation guards,
SDP compensation and SIP Lab are reused. No dependency, public Rust API, command
input or generated DTO field changes. `phone-core` and frontend media remain
unchanged. The two existing reliability patches are extended and participate in
the exact SDK manifest and desktop software-bill identity checks.

## Eight findings and corrections

| Finding                                                     | Corrected behavior                                                                                                                                               | Permanent scenario coverage                                                                                            |
| ----------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Saved return state overrode later manual hold/resume        | Confirmed explicit hold/resume and successful selection update the retained manual intent; temporary parking does not                                            | `manual_hold_during_consultation_must_survive_child_bye`, `explicit_switch_to_preheld_original_must_survive_child_bye` |
| Unsupported SDP answers falsely confirmed setup/hold/resume | Validate common media codecs after mandatory security checks; restore accepted SDP before managed compensation; initial invalid negotiation never becomes active | Three `unsupported_codec_*` cases, including explicit compensation and confirmation of the previous hold state         |
| Late hold 200 after hangup/BYE lost its ACK                 | Retain established re-INVITE transaction ownership until final response; ACK after application teardown without running old callbacks or restoring media         | Four late-200 variants with local/remote end and with/without 100 Trying, including exact duplicate ACK replay         |
| Original BYE discarded an outstanding REFER outcome         | Keep one native call/UA owner bounded by the original transfer deadline; deliver outcome to the exact surviving peer generation                                  | Attended success/failure, blind success/failure, timeout, replacement call, shutdown and delayed REFER refusal         |
| Interrupted switch left its only survivor held              | Keep a per-dialog return continuation through outstanding hold/recovery, then reconcile to retained intent                                                       | `target_bye_during_switch_must_not_park_survivor`, `child_bye_during_original_hold_must_restore_original`              |
| Malformed NOTIFY text caused false transfer success         | Require `message/sipfrag`, initial three-digit SIP status 100–699 and complete CRLF Status-Line; empty reason is allowed                                         | Wrong type returns 415; prefixed/truncated/lone-LF status returns 400; valid empty reason completes successfully       |
| Malformed answer in ACK was silently ignored                | Fail an established offerless renegotiation through the existing deferred error/BYE path                                                                         | `malformed_offerless_reinvite_ack_must_not_be_silently_accepted`, with terminal failure assertion                      |
| Repeated final NOTIFY received 481 after initial 200        | Use the existing stateful native server reply for both success and rejection                                                                                     | `duplicate_final_transfer_notify_must_replay_200`; ordinary duplicate BYE remains covered                              |

The scenario module also retains the audit's 14 hold/resume response cases,
16 transfer-failure/hold-intent cases, real INVITE/UPDATE glare, stale Refer event
IDs during retry, and pending-control rules for mute and hangup.

## Native ownership and cleanup

`retired_transfer` is private event-thread state. It borrows no freed application
objects: its call and UA have native references. It is not an application call,
does not allocate media, and cannot increase `EngineSnapshot.calls` beyond two.
The original 15-second deadline is retained, rather than starting another timeout
on BYE. Its peer is identified by the generation reserved for that REFER.

Success is emitted before ending that paired peer. Failure or timeout clears
pending transfer and leaves the peer usable with its error. Mute and hangup stay
available. Result, deadline and shutdown release the owner. New incoming/outgoing
work or account configuration after associated calls have ended retires the old
subscription; an old result cannot affect the new dialog. A last closed blind
call can retain its result event only if its generation is still current and no
live application conversation exists.

Explicit hold intent changes only on confirmation. A failed manual hold cannot
become the future automatic return target. Outstanding automatic return waits
until existing offers/recovery complete; it does not overwrite them.

## Independent review

One fresh-context reviewer inspected the incremental controller changes, both
native patches/applied SDK sources and permanent tests. Three actionable review
findings were resolved:

1. A late REFER refusal could call back into the adapter, release the retained
   call and then read that freed call in `sipsub_close_handler`. The handler now
   returns immediately after the callback. The real late-refusal scenario
   crashed with SIGSEGV under macOS GuardMalloc before the correction and passes
   afterwards. The permanent `test:sip-lifetimes` runner covers this path and two
   additional owner-release scenarios with protected freed allocations.
2. The 408/481 fast path skipped retained-session release after teardown. It now
   applies only to live sessions; terminated sessions take normal cleanup. Four
   added regressions observed missing final local BYE before the fix and completed
   teardown after it, covering both statuses with and without a provisional reply.
3. The NOTIFY regex accepted a missing CRLF and rejected a valid empty reason.
   It now matches a complete line, anchored by the captured status position.
   Truncated, lone-LF and empty-reason regressions were all red before correction.

No review findings are deferred. No second review loop was used.

## Test evidence

Permanent tests reuse the existing real loopback registrar/peer and native
serialization guard. The initial durable 52-case baseline reproduced the audit:
35 passed and 17 failed. Two additional blind-result tests were also red before
the generation-safe closed-call event projection was added. The final focused
suite contains 66 cases and passed after review corrections. Stronger assertions
require actual accepted compensation, a released pending flag, exact late ACK
replay, one transfer-success event and explicit 400/415 responses.

A mutation inverting the explicit-intent condition was detected by the switch
and return regressions. The manual-hold case alone did not detect that combined
mutation; the four-case intent group did. The production file was restored
byte-for-byte and the unmodified intent group was rerun successfully.

`bun run test:sip-lifetimes` discovers the executable through Cargo JSON output,
runs exact test names under system GuardMalloc and fails on missing cases,
injection failure, timeout or nonzero/signal exit. All three guarded cases passed.
This is an additional macOS diagnostic; ordinary native tests cover the behavior
on other platforms without claiming those platforms were executed here.

## Changed files

| Files added or additionally changed                                                | Purpose                                                                          |
| ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `crates/baresip-adapter/native/bridge.c`, `native/call_controls.h`                 | Confirmed intent, deferred return and bounded generation-owned transfer lifetime |
| `crates/baresip-adapter/src/ffi.rs`                                                | Closed blind-call result without resurrecting or contaminating a new dialog      |
| `native/patches/re-reliability.patch`, `native/patches/baresip-reliability.patch`  | SDP validation, late ACK/teardown and NOTIFY validation/replay                   |
| `crates/baresip-adapter/src/sip_scenario_tests.rs`, `src/sip_reliability_tests.rs` | Permanent scenario module using the existing Lab                                 |
| `scripts/check-sip-lifetimes.mjs`, `package.json`                                  | Repeatable macOS native lifetime diagnostic, without new dependencies            |
| `docs/api/call-controls.md`, `docs/api/sip-events.md`, `native/patches/README.md`  | Behavioral outcomes, errors, required wire validation and ownership contracts    |
| `docs/superpowers/plans/2026-11-05-sip-followup-fixes.md`, this report             | Scope, execution and observed verification                                       |

## Verification

All listed checks completed successfully. The final serial native adapter suite
finished in 574.69 seconds with 182 passed and 15 explicitly ignored lab tests.
The native desktop suite passed 87 tests with four explicitly ignored OS/device,
credential-store or credential-specific PBX cases. Adapter lab cases were run
separately by the Asterisk runner; those four desktop cases were not executed.

| Check                                            | Observed result                                                                                   |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| `bun run native:build`                           | Passed for final SDK, including native NAT tests                                                  |
| New permanent scenarios                          | 66 passed, including tightened 400/415 assertions in the final full workspace run                 |
| `bun run test:sip-lifetimes`                     | Three exact GuardMalloc cases passed                                                              |
| Intent mutation and restoration                  | Mutation caught; restored production source                                                       |
| `bun run check`, `bun run build`                 | Passed; Svelte zero errors/warnings                                                               |
| `bun run test:ui`                                | 88 passed                                                                                         |
| `bun run test:ui:flows`                          | All seven browser flow groups passed                                                              |
| `bun run check:versions`                         | Passed with pinned Rust 1.98.1 first on PATH                                                      |
| `bun run types:generate` and generated diff      | Passed; generated TypeScript unchanged                                                            |
| Portable workspace Clippy `-D warnings`          | Passed                                                                                            |
| Final native workspace Clippy `-D warnings`      | Passed                                                                                            |
| `cargo test --locked --workspace`                | Passed; portable desktop device test explicitly ignored                                           |
| Final native workspace tests, serial             | Passed: 182 adapter tests, core tests and 87 desktop tests; explicit ignored cases recorded above |
| Final `bun run test:asterisk`                    | Passed: 16 lab tests and 13 TLS/SRTP tests; fresh post-resume/post-transfer PCM measured          |
| `bun run test:network-roam`                      | Passed: six recovery cases and production C queue fixture                                         |
| Portable and native desktop app/DMG              | Both app/DMG builds passed; software bill and hashes exported                                     |
| Final `bun run format:check`, `git diff --check` | Passed; generated TypeScript diff also unchanged                                                  |

## Limits

Executed locally on macOS arm64 with synthetic SIP identities, test-tone/recorded
PCM and exclusive lab certificate roots. No physical microphone/speaker testing,
Windows runtime, real WLAN/VPN/IP changes, sleep/wake, older macOS runtime,
notarization or hosted CI is claimed. Explicitly ignored desktop OS/keychain/lab
tests are reported with the final workspace result. GuardMalloc is not a complete
leak or concurrency stress test.

The existing Homebrew OpenSSL archive can emit a newer-host deployment-target
warning; successful local native linkage does not qualify it on macOS 14.
Desktop packaging uses the pinned toolchain and its existing LLVM library path;
no toolchain install, system trust change or real credential-store write is part
of these corrections. Build output, captures and test certificates remain ignored.
