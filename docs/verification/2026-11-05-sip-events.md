# SIP events and background state verification

Observed locally on macOS arm64 with the pinned Rust/Bun toolchain and rebuilt
baresip/libre SDK. No real account was used to initiate an automated external
call. No running user app was restarted or personal credentials changed.

## Delivered behavior

- Progress, remote hold/resume, received DTMF and redirects reach canonical Rust
  snapshots through the private borrowed C event payload. The last 64 signals are
  sequenced with explicit eviction counts and generation ownership.
- Early media and remotely held states retain the single call slot. Caller and
  initial native connection timestamp survive hold/resume and restoring windows.
- Both windows reject older native read revisions. The background Rust observer
  retains its last successful state on read failures and coalesces pending
  main-thread delivery instead of replaying obsolete states.
- SIP failures retain actual status codes with safe typed categories. Native
  errors stay separate, including internal RTP failure 701 and transfer failures
  without a SIP response. Normal cancellation/local rejection is not a remote
  failure.
- Unsupported REFER is not advertised or accepted. Dialog requests receive 501;
  out-of-dialog requests receive 405. Neither dials the transfer target or drops
  an established call. A minimal tracked SDK patch enforces the existing policy.
- Existing CallerIdentity privacy/sanitization, call journal, contact projection,
  native notification delivery and audio stack are reused. No dependency added.

## Main files

- Contracts: `crates/phone-core/src/phone.rs`,
  `crates/phone-core/examples/export_types.rs`, generated
  `apps/desktop/src/lib/generated/AppInfo.ts`.
- Native boundary/state: `crates/baresip-adapter/native/bridge.c`,
  `src/ffi.rs`, `src/engine.rs`, `src/lib.rs` under the adapter.
- SDK policy/build: `native/patches/baresip-refer-policy.patch`,
  `native/build.mjs`, `crates/baresip-adapter/build.rs` and attribution/docs.
- Desktop state: `apps/desktop/src-tauri/src/shell.rs`, `shell_policy.rs`.
- UI: `apps/desktop/src/App.svelte`, `Settings.svelte`, `src/lib/api.ts`,
  `src/lib/components/CallPanel.svelte`.
- Tests: core `tests/sip_contract.rs`; adapter `src/sip_event_tests.rs`,
  `src/nat_tests.rs`, callback regressions in `src/ffi.rs`; UI
  `tests/ui/background.test.ts`, `sip-errors.test.ts`, browser phone/settings and
  updated snapshot fixtures. Native test guards also cover existing interop tests.
- API/behavior details: [SIP event contract](../api/sip-events.md),
  [desktop API](../api/README.md), native boundary/build documentation.

## Evidence

Before the fix a real UDP REFER regression returned 202 Accepted. Four callback
regressions reproduced missing early-media state, missing native connection time,
generic busy errors and stale close events overwriting a replacement call.
Background tests reproduced fabricated idle state on read failure and repeated
obsolete main-thread deliveries. Browser tests reproduced stale delivery ordering,
caller/timer loss on restoration and unsafe fallback error messages. Additional
RED-to-GREEN regressions cover inventing SIP 501 and suppressing native error 701.

Completed commands:

- `bun run native:build`: staged SDK with both exact patch texts; SDK NAT tests passed.
- `bun run format:check`, `git diff --check`: passed.
- `bun run check`, `bun run build`, `bun run check:versions`: passed.
- `bun run types:generate`: passed; repeat generation was byte-identical.
- `bun run test:ui`: 33 passed.
- `bun run test:ui:flows`: contacts, history, phone, settings and layout all passed,
  including stale event/read/command ordering and dirty-draft protection.
- Portable workspace Clippy with warnings denied and Cargo tests: passed.
- `cargo clippy --locked --workspace --all-targets --features sip-desktop/native -- -D warnings`: passed.
- `cargo test --locked --workspace --features sip-desktop/native`: passed;
  adapter 41 passed plus five explicitly ignored Asterisk tests, desktop 39 passed
  plus four explicitly ignored OS/lab tests, and core/lifecycle tests passed.
- `bun run test:asterisk`: RTP echo and all five PBX interop tests passed. Fresh
  PCM echo: 4.04 seconds, RMS 5811, approximately 440.1 Hz. Synthetic audio only.
- `bun run test:sip-diagnostics`: 60 allowlisted markers, remote BYE acknowledged
  with 200, diagnostics default off.
- `bun run test:network-roam`: all six native state-machine tests passed.
- `bun run desktop:build --features native --bundles app`: Ruf.app built and
  ad-hoc signed at `target/release/bundle/macos/Ruf.app` (19.49 MiB).

Independent scoped review found no important issue. Its missing-response transfer
status finding was fixed and covered by a failing-then-passing regression. Focused
follow-up review of both FFI corrections and Settings ordering found no important
issue.

## Execution limits

No hosted CI, Windows runtime or fresh real-provider background-call/notification
acceptance was executed. Actual OS banner presentation, microphone/headset audio,
Focus/sleep behavior and the user's telephone system still need manual acceptance
with the rebuilt app. Existing OS/keychain opt-in tests remained ignored. The
macOS app is ad-hoc signed, not notarized. These local results do not claim full
SIP-extension or every provider/NAT topology compatibility.
