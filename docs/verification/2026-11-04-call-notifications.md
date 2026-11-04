# Actionable incoming call notifications

The native background observer delivers caller name/number and account with
Annehmen/Ablehnen actions. Existing Engine/PhoneSession state and native ringing
remain authoritative. No window is focused by arrival or those actions. Tapping
the body opens the phone. Stale/duplicate tokens and rejection after establishment
are guarded in Rust and on the SIP thread.

Implementation uses UserNotifications.framework on macOS and WinRT/COM on Windows.
The old Tauri notification plugin was removed because its underlying desktop
backends cannot provide the required combination of reliable action identity,
Notification Center activation and withdrawal. Native/unsafe code stays in
baresip-adapter. No SIP packets, passwords or caller details are added to logs.
API details, privacy rules and launch limitations are in [the API guide](../api/README.md).

Observed on macOS arm64:

- Rust portable workspace tests and native-feature workspace tests passed.
- Native synthetic SIP/NAT test passed with caller identity, privacy, rejection,
  ACK/BYE lifecycle and following calls. Existing Asterisk echo/media test passed;
  the first combined run timed out in the existing NAT interop case. That case
  passed individually, then the full Asterisk run passed all four interop cases.
- Notification registry tests passed. Temporarily removing the token comparison
  made the stale-action regression fail; restoring it passed. Nine controller tests
  also cover latest-state handoff, concurrent delivery/withdrawal and token failures.
- Svelte/TypeScript checks, Bun UI tests and browser phone/settings/layout flows
  passed, including caller display and permission changes preserving dirty drafts.
- Portable workspace strict Clippy and Windows-target adapter strict Clippy passed.
- A native release Ruf.app bundle was built and signed ad hoc locally.
- `test:sip-diagnostics` passed its 60-marker allowlist.
- `test:macos-call-notifications` passed actual OS delivery, action category,
  silent audio configuration and withdrawal. Its responses are synthetic delegate
  calls on real delivered requests; this is not an interactive click test.

Reproduce the isolated native notification test:

```sh
bun run test:macos-call-notifications
# Only if this test identity has not been authorized yet:
bun run test:macos-call-notifications --request-permission
```

It builds a separate accessory app under ignored `native/build`, uses public fake
caller data, and touches no SIP account, keychain or contacts. Permission prompts
require the explicit flag. No running installed Ruf/SIP app was restarted.

Remaining acceptance: click both actions on a real incoming call in the installed
macOS app, including while hidden; test native notification display, COM activation
and NSIS uninstall on Windows. Focus/lock-screen policy remains an OS decision.
Cross-compilation does not verify Windows runtime behavior.

Parallel contacts/history/branding edits in this checkout are separate work.
They were preserved and are not part of this feature's verification claim.
