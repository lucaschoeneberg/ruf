# macOS contacts permission repair

## Evidence and repair

The installed `/Applications/SIP.app` had Hardened Runtime enabled
(`adhoc,runtime`) and the contacts usage-description string, but its actual
signature contained only the microphone entitlement. The missing
`com.apple.security.personal-information.addressbook` resource entitlement was
confirmed by inspecting the signature and by an observed failing bundle check.
Apple documents Address Book access for Hardened Runtime here:
https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.personal-information.addressbook

The app also lacked contacts controls in SIP Settings and ignored native request
completion errors. The fix adds:

- The Address Book entitlement to the signed deliverable.
- Shared native authorization, a permission-only API and sanitized request errors.
- Settings → Kontakte with live status, explicit request and a fixed system
  privacy-settings action, also available after denial in the phonebook.
- Permission-only commands in Settings; that window has no contact-enumeration
  or favorite-mutation capability.
- A signed-bundle regression check, wired into macOS desktop build CI.

## Changed files

- `apps/desktop/src-tauri/Entitlements.plist`
- `crates/baresip-adapter/native/contacts.h`, `native/contacts.m`,
  `src/contacts.rs`, `src/lib.rs`
- `apps/desktop/src-tauri/src/contacts.rs`, `src/commands.rs`, `src/lib.rs`,
  `build.rs`, main/settings capabilities and generated permission files
- `apps/desktop/src/Settings.svelte`, `src/lib/api.ts`, `src/lib/contacts-api.ts`,
  `src/lib/components/ContactsSettings.svelte`, `ContactsPanel.svelte`
- `scripts/check-macos-contacts.mjs`, `scripts/check-ui-flows.mjs`,
  `tests/ui/browser/contacts.mjs`, `package.json`,
  `.github/workflows/desktop-build.yml`
- `docs/api/contacts.md`, `docs/api/README.md`, `docs/development.md`,
  the permission-fix plan and this record

The fix reuses the existing Contacts framework bridge, ContactsPermission DTO,
Tauri worker/capability patterns, existing settings sections and design tokens.
No dependency was added. New commands and the allowed `contacts` settings section
are documented in `docs/api`; no HTTP API or second SIP/audio path was introduced.

## Observed checks

- Signed-bundle check: failed against the old installed app; passed against the
  rebuilt bundle, staged installation and final `/Applications/SIP.app`.
- Both actual signature and usage description verified, with the existing signed
  microphone permission check also passing.
- Native authorization regression: synthetic failure observed before the fix;
  permission mapping/error tests passed. The native implementer also exercised
  an isolated Objective-C fake store without querying personal contacts.
- `cargo test --locked --workspace` and workspace Clippy with warnings denied:
  passed. Native-feature workspace tests/Clippy passed after rebuilding the SDK.
- `bun run test:ui`: passed. Browser flows reported `UI_FLOW_OK` for contacts,
  phone, settings and layout. The new settings flow was observed failing before
  implementation and passing afterward.
- Browser assertions cover no implicit request/enumeration, request failure and
  retry, denial, system-settings action, granted state and focus refresh.
- `bun run check`: zero errors/warnings. Version checks passed. Type regeneration
  was unchanged at the time checked.
- `bun run native:build`, native app build and Asterisk script completed; four
  selected SIP interop regressions passed.
- Formatting passed for this fix. A later workspace-wide recheck overlapped
  another agent's incoming-caller work and reported unformatted changes in
  `ffi.rs`, `nat_tests.rs` and `phone_contract.rs`; those unrelated files were not
  reformatted or reverted by this task.
- Independent bounded review of UI, commands/capabilities and packaging verifier
  found no material issues. Granted/denied screenshots use synthetic fixtures.
  No hosted CI or Windows runtime execution is claimed.

## Installed-app validation

The previous installed process exited through the normal SIP Quit menu. The
verified updated app replaced `/Applications/SIP.app` and was launched there.
A reversible copy of the prior bundle remains in ignored
`artifacts/contacts/SIP-before-permission-fix.app`; account/keychain data was not
modified by installation.

In the actual installed app, SIP Settings → Kontakte reported **Freigegeben**.
The actual phonebook then loaded **596 contacts**. Only the count was inspected;
no contact names/numbers were logged, exported or committed, and no call was made.
The app was left running on Kontakte for the user's direct test.

Permission was already granted at the time of the live check, so a first-time
system consent dialog was not forced. No TCC reset, automatic consent acceptance
or system privacy toggle was performed. First-time/denial behavior is covered by
native and browser fixtures; an actual fresh-consent/revocation cycle remains a
manual platform check if needed. The live system-settings deep link was not
asserted because the user was interacting with the windows concurrently.
