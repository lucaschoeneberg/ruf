# SIP audit corrections implementation plan

> **For agentic workers:** Use superpowers:executing-plans for continuous implementation, followed by one independent code review.

**Goal:** Correct all ten demonstrated SIP defects and the three additional interoperability issues reported against b8baa10.

**Architecture:** Keep SIP, SDP, transaction ownership and media inside the existing baresip/libre boundary. Extend the tracked SDK patch chain and its manifest checks; reuse native control recovery, call observers, per-dialog snapshots and Svelte components. No new dependency or public IPC DTO is needed.

**Tech stack:** Rust 2024, pinned baresip/libre 4.12.0, C11, Svelte/TypeScript, Bun, native loopback SIP and Asterisk tests.

**Spec:** The user's accepted audit findings in this chat and docs/api/call-controls.md. Manual hold intent survives transfer failure; explicit return from consultation resumes the original.

## Constraints and review focus

- Preserve existing account security and mandatory-SRTP failure behavior.
- Never construct an alternate SIP transaction or media pipeline.
- Invalid SDP must preserve accepted parameters and media; accepted but invalid answers require confirmed recovery or termination.
- Fork cleanup must retain the selected dialog, authenticate peer authority and replay ACKs for duplicates without duplicate BYEs.
- Busy history must preserve privacy and record each rejected transaction once.
- SDK patch contents must be checked by Cargo and recorded in the software bill.
- Existing build/check commands and platform limitations remain applicable.

## Task 1: Durable regressions

- [x] Port the demonstrated native audit probes into sip_reliability_tests.rs, using the existing Lab fixture through a child test module.
- [x] Add the background attended-transfer failure rendering regression.
- [x] Observe the failures against the unchanged SDK/product before implementing fixes.

## Task 2: Adapter and UI corrections

Files: native/call_controls.h, native/bridge.c, src/ffi.rs, CallPanel.svelte, tests/ui/call-controls.test.ts.

- [x] Reset the existing receive watchdog after confirmed local resume.
- [x] Encode phone-number URI users with the existing libre URI escaping formatter for dial, consultation and blind transfer.
- [x] Preserve earlyMedia when a later ringing indication has no new media semantics.
- [x] Surface transfer/control errors with their owning background conversation.
- [x] Run the corresponding native and rendering regressions.

## Task 3: Native protocol corrections

Files: native/patches/re-reliability.patch, native/patches/baresip-reliability.patch and native SDK build/manifest consumers.

- [x] Preserve accepted remote SDP across decoder failures and rejected media offers; reject replacement of the only audio stream with incompatible codecs.
- [x] ACK and reconcile an accepted invalid hold answer using existing recovery; preserve terminal 408/481 semantics.
- [x] Reject initial successful setup without completed SDP negotiation.
- [x] Reuse sip_dialog_fork and the native ACK/transaction machinery to ACK and BYE unwanted confirmed branches, with bounded duplicate handling.
- [x] Use stateful INFO replies and validate exactly one DTMF signal with a bounded duration.
- [x] Export busy rejections at the native dialog limit through the existing sanitized call observer path.
- [x] Rebuild the SDK and run all new regressions and existing control/security tests.

## Task 4: Documentation and complete verification

- [x] Document corrected protocol outcomes, private native API contracts, SDK patches and unchanged public DTOs in docs/api and native/patches/README.md.
- [x] Run format, TypeScript/Svelte, frontend build, version check and type generation; confirm generated output remains unchanged.
- [x] Run portable and native workspace Clippy/tests, Asterisk interop, network recovery and desktop packaging checks.
- [x] Obtain an independent review of the complete working-tree patch and resolve actionable findings with regression coverage.
- [x] Record actual commands/results and remaining platform/runtime limits in docs/verification/2026-11-05-sip-audit-fixes.md.

## Execution notes

The current checkout is reused at the user's request to fix the reviewed project. No merge, push or release is part of this work. Previously red temporary probes establish the initial evidence; durable regressions are run before their fixes. Test output and intermediate native source copies stay outside tracked source.

Progress: all thirteen audit issues are corrected and all planned verification is complete. Durable regressions reproduced the original defects before implementation. Independent review identified three additional edge cases; each was reproduced and corrected with behavioral coverage. The final serial native workspace suite passed, including all 21 new SIP regressions, 116 adapter tests and 87 desktop tests. SDK, frontend, Asterisk/security, network, portable workspace, Clippy and both desktop packaging checks passed. Explicitly ignored platform/credential tests and deployment limits are recorded in the verification report.

Implementation ruling: append two reliability patches after the existing control/security patch chain rather than rewrite their reviewed history. Both new patch texts participate in SDK identity validation and SBOM hashes. Initial outgoing fork cleanup reuses sip_dialog_fork, native ACK caches and session BYE teardown; it is bounded to 32 discarded branches per original ACK lifetime. No public IPC shape changes.
