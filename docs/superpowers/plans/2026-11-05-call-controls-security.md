# Call Controls and Security Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Track checked steps and native evidence.

**Goal:** Deliver mute, local hold/resume, blind and attended transfer, and verified TLS/SRTP accounts in Ruf.

**Architecture:** Keep baresip as the only SIP/audio engine. Extend native queued operations with guarded per-dialog identities, expose portable session/security DTOs, and preserve existing current-call projections for shell/history compatibility. Persist account security through existing metadata/keychain patterns.

**Tech Stack:** Rust 2024, C/baresip 4.12.0, libre/OpenSSL, Tauri, Svelte 5, ts-rs, Bun, Docker/Asterisk.

**Spec:** docs/superpowers/specs/2026-11-05-call-controls-security.md

## Global Constraints

- Accounts remain under Settings.
- Keep phone-core independent of native handles/Tauri; unsafe/SIP/audio in baresip-adapter.
- No new SIP stack or frontend audio pipeline; reuse existing dependencies.
- Incoming REFER remains explicitly rejected.
- Required SDES-SRTP requires verified TLS and cannot downgrade to plaintext.
- Configured settings are not evidence of negotiated security.
- Preserve current accounts, background registration, history and notification ownership.
- No production credentials, recordings, generated private keys or build output in Git.
- Work only in the new worktree; do not merge, push, install or change macOS permissions.

## Review Focus

- Stale actions and late native callbacks must not mutate a replacement or other-account call.
- Remote hangup during consultation must leave the surviving dialog usable and clean up history/notifications exactly once.
- Rejected/glared hold and failed/expired transfer must not strand or silently terminate the original call.
- Reopened saved accounts must retain TLS/SRTP/auth/proxy settings without exposing passwords or SDP keys.
- Invalid trust/hostname or missing media encryption must fail without plaintext fallback or a misleading lock indicator.

### Task 1: Native call controls and consultation sessions

**Files:** Modify crates/phone-core/src/phone.rs, crates/baresip-adapter/src/{engine,ffi,call_journal}.rs, crates/baresip-adapter/native/bridge.c; add focused call-control helpers/tests where responsibility warrants; update docs/api/phone.md (or existing equivalent).

**Interfaces:** Produce portable per-dialog snapshots and Engine methods `set_call_muted(call_id, muted)`, `set_call_held(call_id, held)`, `transfer_call(call_id, destination)`, `start_consultation(call_id, destination)`, `switch_call(call_id)`, `cancel_consultation(call_id)`, `complete_transfer(call_id)` returning Result<(), PhoneError>. Keep current-call EngineSnapshot projection usable. Exact added DTO fields must be documented and generated before reporting.

- [ ] Write failing focused tests for local/remote hold separation, stale control rejection, two-dialog lifecycle, hold failure rollback, consultation cancellation and transfer final success/failure handling.
- [ ] Run focused tests and capture expected RED output.
- [ ] Implement commands using audio_mute, call_hold, call_transfer and call_replace_transfer on the native event thread. Track native generations per call, confirmed/pending state and bounded deadlines; keep inbound REFER rejected. Handle both call endings and retain ordered lifecycle observations.
- [ ] Run portable/native adapter tests, generate TypeScript contracts, document API semantics and capture GREEN evidence.
- [ ] Commit this independently testable deliverable and write task report.

### Task 2: Verified TLS/SRTP accounts and persistence

**Files:** Modify native/build.mjs, crates/baresip-adapter/{build.rs,native/bridge.c,src/ffi.rs,src/engine.rs}, crates/phone-core/src/phone.rs, apps/desktop/src-tauri/src/{store,phone}.rs; add focused native trust helper if required; update native/SBOM documentation and docs/api.

**Interfaces:** Consume Task 1 per-dialog state. Produce defaulted typed account transport and media-security policy plus authentication-user/proxy options in AccountConfig, saved metadata and safe snapshots; expose observed per-dialog signaling/media security. Use SipTransport (udp/tcp/tls), MediaSecurityPolicy (disabled/required), AccountConfig.transport/mediaSecurity/authUsername/outboundProxy; nullable options default to null. CallSnapshot.security is CallSecurity { signalingTransport: SipTransport|null, tlsPeerVerified: boolean, media: unknown|unprotected|sdesSrtp }. Add CertificateRejected/TransportFailure/MediaSecurityRequired error classifications with native provenance. Maintain API source compatibility where practical; default legacy persisted accounts to UDP/no media encryption.

- [ ] Write and run failing tests for defaults, validation, metadata roundtrip, reconnect retention, and native security observations/reset.
- [ ] Enable and load srtp module in the native SDK; enforce SDK compatibility checks. Configure actual UDP/TCP/TLS per account/destination without accidental UDP override; wire separate auth username and proxy.
- [ ] Implement certificate/hostname verification and portable packaged trust roots; honor platform trust policy including deliberately trusted private CAs. Do not expose per-account CA injection into the shared TLS context. Tests may inject generated CA trust through a private test-only Rust path. Require TLS for required SDES-SRTP, observe media events and fail closed when media policy is unmet.
- [ ] Run account/core/native tests and SDK rebuild; document trust paths, fields, errors, migration and security limits; regenerate types.
- [ ] Commit and write task report with RED/GREEN evidence.

### Task 3: Desktop commands and settings/call UI

**Files:** Modify apps/desktop/src-tauri/src/{commands,phone,lib}.rs as needed; apps/desktop/src/{App,Settings}.svelte, src/lib/{api,phone}.ts and existing call/account/settings components; tests/ui and browser flow script; docs/api, README/PRODUCT/CHANGELOG as appropriate.

**Interfaces:** Consume Task 1 Engine controls and Task 2 account/session DTOs. Produce guarded Tauri IPC for each control, Svelte call controls/consultation flow and editable account-security settings. Use generated Rust TypeScript types, existing error presentation and compact layout.

- [ ] Write/run failing UI behavior tests for mute/hold/pending states, two-call switching/cancel/complete, settings validation/defaults and confirmed-security display.
- [ ] Wire command handlers to existing blocking/session patterns, preserving permission and stale-call guards. Implement complete controls and consultation UI, including distinct per-dialog parties and transfer destination entry or selection from existing authorized contacts (reuse existing permissions; no automatic prompt). Keep account creation/edit/security under Settings and persist all fields through save/reconnect. Show confirmed native security, never a preference-derived lock, and clearly distinguish pending hold/recovery.
- [ ] Update API docs and product/changelog descriptions accurately without claiming unobserved provider compatibility.
- [ ] Run UI tests, browser flows, Svelte check/build and portable desktop Rust tests; capture GREEN evidence.
- [ ] Commit and write task report.

### Task 4: Native SIP interoperability and full verification

**Files:** Modify tests/interop/asterisk/{compose.yml,pjsip.conf,extensions.conf,README.md}, scripts/asterisk-smoke.mjs; add generated test-certificate helper and adapter native call/security integration tests; docs/verification/2026-11-05-call-controls-security.md.

**Interfaces:** Exercise completed public Engine methods against isolated loopback Asterisk and controlled SIP fixtures. Generate trust/key material in ignored native/build only. Existing test:asterisk must include new tests and retain earlier regressions.

- [ ] Extend native tests to prove returned audio survives hold/resume, mute suppresses transmission, blind/attended transfer reaches final completion, and failed/cancelled operations preserve or resume the original call.
- [ ] Add verified TLS plus required-SRTP positive audio test and untrusted, hostname-mismatch and plaintext-policy negative tests. Include remote ending during consultation and stale command isolation; cover bounded hold/transfer failure paths with controlled native peers where appropriate.
- [ ] Run bun run test:asterisk, portable/native Rust suites and clippy, bun run test:ui, bun run test:ui:flows, format:check, check, build, check:versions, types:generate (confirm stable), and desktop:build --features native.
- [ ] Fix actual failures with regression tests, record commands/results/platform limits, commit evidence and deliver the reviewable branch.
