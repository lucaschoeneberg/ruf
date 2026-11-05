# Native SIP/audio reliability audit — 2026-11-05

Reviewed the full implementation, including unchanged files, rather than only
the working-tree diff:

- `crates/phone-core/src/phone.rs`: account/sound validation, safe error models,
  registration projection, call reservation and snapshot contracts.
- `crates/baresip-adapter/src/{lib,engine,ffi,call_journal}.rs`: feature fallbacks,
  queue reservations/acknowledgements, generation guards, observer ordering,
  bounded event retention, lifecycle state, startup/shutdown and FFI ownership.
- `crates/baresip-adapter/build.rs`: native feature/linkage, SDK/source/patch
  identity checks and platform libraries.
- `crates/baresip-adapter/native/bridge.c`, `audio_devices.c/.h` and
  `network_roam.c/.h`: request ownership, callback lifetimes, account retirement,
  audio/ringtone ownership, explicit-device validation, network reconciliation,
  call deferral, retry backoff and shutdown signaling.
- Native lifecycle/unavailable, NAT, SIP-event, journal, callback and network
  tests; Asterisk script, example and synthetic fixtures. Ownership assumptions
  were checked against the pinned upstream player, queue, UA and session code.

Contacts/notifications, persistence/session orchestration and frontend delivery
were reviewed separately by the coordinating audit. This report makes no claim
that arbitrary providers or platform runtime behavior have been certified.

## Reproduced defect and minimal correction

A native call setup failure could disappear from the snapshot. The pinned
baresip session close handler does not populate `call_scode` when its local
decoder/transport callback fails. The C bridge consequently emits a close with
status zero. Rust previously moved calling/ringing/early-media directly to idle
without an error, although the independent journal classified the attempt as
failed.

The real loopback regression accepts REGISTER and sends an INVITE response
`183 Session Progress` with unsupported SDP version `v=invalid`. Before the fix,
the actual native decoder reported its error and closed the call, while the
regression failed because `last_error` was absent. The same test passes after
the fix, with `nativeFailure` and null `sipStatus`. No peer reason text or guessed
SIP status is exported.

The patch uses the existing FFI snapshot projection and safe error model.
Zero-status outgoing setup closure fills an absent error; an existing native
error is preserved. Accepted local cancellation and engine interruption suppress
this setup-failure classification. Callback regressions cover calling, ringing,
early media, incoming remote cancellation, local outgoing cancellation, engine
shutdown and remote termination after establishment. Both the native regression
and the new setup-failure callback test were observed failing before the patch.

Changed files for this finding: `crates/baresip-adapter/src/ffi.rs`, the new
targeted regression in `crates/baresip-adapter/src/sip_event_tests.rs`, and the
error semantics in `docs/api/sip-events.md`. Existing earlier changes in these
files were retained. No new dependencies, native ABI, DTO fields, audio pipeline
or NAT policy were introduced.

## Checks observed

- Native malformed-SDP regression: RED for the missing visible error, then GREEN.
- Native callback observation tests: 14 passed after the patch.
- `cargo test --locked -p baresip-adapter --features native -- --test-threads=1`:
  46 library tests passed, six Asterisk-only cases ignored in that invocation;
  the real native lifecycle integration test passed, including three complete
  start/shutdown cycles and singleton rejection.
- `bun run test:asterisk`: all six previously ignored interoperability cases
  passed. Real RTP tone echo measured 4.04 seconds, RMS 5811 and 440.1 Hz. This
  also covers immediate outgoing cancellation, first incoming call before any
  outgoing call, multiple accounts, native history without polling, remote BYE,
  translated signaling without server Contact rewriting and stale call actions.
- `bun run test:network-roam`: all six standalone state-machine cases passed.
- `cargo clippy --locked -p baresip-adapter --features native --all-targets -- -D warnings`: passed.
- `cargo fmt -p baresip-adapter -- --check`: passed after formatting.
- `bun fmt --check docs/api/sip-events.md docs/verification/2026-11-05-audit-native.md`: passed.
- `git diff --check`: passed.

## Remaining runtime boundaries

The checks use synthetic loopback accounts and test-tone media. No personal
account, contact record, OS credential entry, installed application restart or
hardware microphone session was used. Windows runtime, actual OS presentation,
device hot-unplug behavior and real-provider/VPN incoming-call reachability need
their own observed executions. In particular, no speculative keepalive, STUN or
Contact policy change was made for the reported outgoing-first incoming symptom.
