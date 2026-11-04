# macOS contact permission repair

User-authorized follow-up: make the native permission flow work and discoverable
in SIP settings, then rebuild and launch the installed app for direct testing.

## Evidence and approach

- The installed app has `adhoc,runtime` signing flags, but only the audio-input
  entitlement. Apple requires Address Book resource access for Hardened Runtime.
- The usage-description plist key is present; the address-book entitlement is not.
- SIP Settings has no contacts section. Denied access only offers a passive retry
  and directions to System Settings; request-completion errors are ignored.
- Keep the existing phonebook and call path. Add permission-only native commands
  for Settings so checking access does not enumerate or persist any contacts.

## Tasks

- [x] Add a verifier for the actual signed app; observe failure against the current
      installed bundle before adding the entitlement.
- [x] Extract shared native authorization, surface request errors, add permission-only
      safe Rust API and regression tests. Native implementation delegated with an
      explicit file boundary; parent owns Tauri/UI and verification.
- [x] Add main/settings-scoped get/request permission and fixed System Settings
      action. Document exact fields, behavior and errors without granting filesystem access.
- [x] Add a Kontakte section in SIP Settings with current status, explicit request,
      open-system-settings recovery and focus refresh; add recovery action to phonebook.
- [x] Run browser regressions for no automatic prompt/enumeration, request/deny/grant,
      failed requests, returning from System Settings and existing phone/settings flows.
- [x] Run format, typecheck, build, generated-contract stability, Rust tests/clippy,
      native SDK/build checks and signed-bundle verification.
- [x] Gracefully replace/restart the installed app. Leave actual consent to the user;
      never reset TCC or automatically accept/alter a system privacy decision.

## Completion evidence

See ../../verification/2026-11-04-contacts-permission-fix.md. The installed app reported Freigegeben and loaded 596 contacts; only the count was inspected. No consent or TCC state was changed.
