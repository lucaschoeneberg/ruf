# Notification permission, delivery and observer reliability

The installed macOS Ruf process was inspected without changing its settings,
accessing secrets, restarting it or placing personal-account calls. A process
sample showed the phone observer polling, native SIP event loop servicing events,
notification action receiver waiting and delivery worker waiting for new state.
The launch context and bundle identity were valid. This did not reproduce an
App Nap failure or prove real-provider reachability.

## Fixes and regression evidence

- Foreground notification authorization can hold the preferences-update mutex.
  Automatic first-registration login setup now defers instead of blocking the
  phone observer behind that mutex. The held-lock regression first timed out,
  then passed after switching to the existing mutex's nonblocking operation.
- `DesktopSettings.notificationPermission` preserves notDetermined, denied,
  authorized and unsupported. The compatibility boolean remains unchanged.
  Settings requests only undecided permission, explains OS denial accurately and
  refreshes permission on focus without discarding drafts. Browser regressions
  reproduced the ineffective repeated prompt, wrong unsupported feedback and
  stale status before the patch, and passed afterward. Default-enabled notices
  retain a separate first-authorization action even when Save is disabled.
- A failed show previously consumed the only offer for that call. The existing
  registry now retries twice with two-/four-second backoff and keeps the same
  bound token. Success or withdrawal cancels retries. The new regression failed
  on the missing second offer before the retry branch and passed afterward.
- macOS service reads and delivery acknowledgements now have five-second
  deadlines; an explicit permission decision has a separate three-minute limit.
  The deterministic fake-daemon test first hung until its process deadline,
  then passed. It does not request permission or contact the notification daemon.
- Independent review identified late successful acceptance after timed-out show
  and withdrawal. A new regression failed with a stale visible token (exit 8).
  Completion now serializes token-checked cleanup, while preserving live retries
  with the same token. Both orderings pass. Final review has no important findings.

The implementation reuses the UserNotifications delegate, latest-state delivery
worker, preferences mutex, token registry, scoped commands and generated ts-rs
contracts. No dependency, SIP pipeline or persistent notification record added.
Public API documentation is in `docs/api/README.md`; native timeout semantics are
also documented at the C boundary.

Changed implementation files: `apps/desktop/src-tauri/src/{call_notifications,
desktop,shell}.rs`, `apps/desktop/src/Settings.svelte`,
`crates/phone-core/src/desktop.rs`, `crates/phone-core/examples/export_types.rs`,
the generated `apps/desktop/src/lib/generated/AppInfo.ts`, and
`crates/baresip-adapter/native/call_notifications.{m,h}`. Regression files are
`crates/baresip-adapter/native/tests/notification_timeouts.m`,
`scripts/check-macos-notification-timeouts.mjs`, adapter `src/engine.rs`, and the
browser settings/contacts/layout fixtures. `package.json` adds the isolated
timeout-test command.

## Checks observed

- `bun run test:macos-notification-timeouts`: passed stalled settings/delivery,
  late callbacks, withdrawal and live retry cases with a fake native service.
- `bun run test:macos-call-notifications`: passed actual OS acceptance, delivery,
  categories, three synthetic action responses and withdrawal in the isolated
  `org.rufapp.notification-smoke` app identity. No permission prompt was needed.
- `bun run test:ui`: 41 passed. `bun run test:ui:flows`: all five suites passed,
  including the new permission and draft-preservation scenarios.
- `bun run check`, `bun run build`, `bun run format:check`,
  `bun run check:versions`, `git diff --check`: passed.
- `bun run types:generate`: passed; a repeat generation was byte-identical.
- Portable workspace Clippy with warnings denied and workspace tests: passed.
- Native SDK rebuilt; native workspace Clippy with warnings denied: passed.
- Native workspace tests: passed (adapter 43 passed / 6 opt-in interop tests
  skipped there; desktop 49 passed / 4 opt-in tests skipped). The six Asterisk
  cases were executed separately below. Real credential-store write tests remain
  opt-in and were not executed.
- `bun run test:asterisk`: all six integration tests passed, including a fresh
  registration followed by an incoming call before any outgoing call. Echo audio
  measured 4.04 seconds, RMS 5811 and approximately 440.1 Hz.
- `bun run desktop:build --features native --bundles app`: passed. The arm64
  bundle is `target/release/bundle/macos/Ruf.app` (19.52 MiB).
  `codesign --verify --deep --strict` passed; this is ad-hoc signed and not notarized.
  A byte comparison confirmed that `/Applications/Ruf.app` still contains a
  different executable. It was not replaced or restarted by this work; fully
  quit the old instance before installing/launching the new bundle, because the
  single-instance plugin otherwise routes a launch to the already-running app.

## Limits

The smoke app has its own OS authorization; its successful delivery is not proof
that Ruf's installed identity is authorized or that a particular Focus/display
policy will show Ruf's banner. No interactive real notification button was clicked;
the smoke test supplies synthetic responses to real delivered notifications.
Windows runtime, hosted CI and machine-sleep incoming calls were not tested.
The reported provider-specific outgoing-first behavior was not reproduced; see
`2026-11-05-startup-incoming.md`. Fresh-launch and idle incoming acceptance with
the installed new Ruf build and the user's provider remains outstanding.
