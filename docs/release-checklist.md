# Release checklist

Ruf 0.1.0 is a development preview for isolated test networks. A passing build
allows a reviewable installer draft; it does not establish production readiness.
Use [development commands](development.md) and [CI/release behavior](ci-cd.md).

## Before creating a preview draft

- Review all intended source changes, including currently untracked source files.
  Keep build output, recordings, credentials and personal account/contact/call data
  out of Git. `apps/desktop/build/software-bill.ts` is source code and must be included.
- Run `bun install --frozen-lockfile`, use the pinned Bun/Rust versions and build
  the native SDK before native workspace checks.
- Run formatting, version validation, UI unit tests, typecheck/build and portable
  Rust Clippy/tests. Regenerate TypeScript contracts and verify no unintended diff.
- Run native workspace Clippy/tests, Asterisk SIP/RTP echo, network-roam/call-action
  and sanitized diagnostic regressions, plus browser state/layout regressions.
- On macOS run contact/notification timeout regressions. Build the native DMG and
  verify its signed microphone and contacts permissions. Build the Windows NSIS
  installer in its platform job.
- Keep root/frontend/Cargo versions in sync, update the user-facing CHANGELOG.md
  and add `docs/releases/<version>.md`. Tag exactly `v<version>` after review.
- Observe the actual hosted quality, macOS, Windows and Asterisk jobs before
  treating their artifacts as verified. The tag workflow waits for all of them.
- Download both installers and SHA256SUMS from the resulting draft and verify
  their hashes. Inspect contents and bundled licenses before publishing a preview.

## Public production distribution blockers

- Configure Apple Developer ID signing/notarization and Windows publisher
  signing. Current macOS ad-hoc signatures and unsigned Windows installers do
  not establish publisher trust. Preserve the final app identity for OS consent,
  credential storage, notifications and login items.
- Exercise the eight settings sections with keyboard navigation, field search,
  long account names and protected drafts. Verify account activation preserves
  credentials and outgoing identity, especially when a call wins an update race.
- Test version-1 metadata migration and its original-byte backup. Exercise history
  recording changes for future calls, confirmed retention shortening, completed-only
  cleanup and storage-failure recovery without interrupting live calls.
- Check OS authorization independently of corrupt/missing account metadata. Saving
  the app notification switch must not prompt; explicit test-notification failure
  must not change a known permission state. Verify caller-detail privacy on actual
  banners and that anonymous tests never carry call actions.
- Verify native microphone/speaker test bounds, selected outputs, navigation cleanup
  and real incoming-call preemption on hardware; inspect all three ringtones and
  zero/full ringtone volume. Meter success alone does not establish usable media.
- Qualify installed macOS and Windows behavior: first-run permissions, credential
  recovery, upgrade/restart, silent login, second launch, hide/reopen, tray/quit,
  denied notifications and actual notification clicks. Test real microphones,
  speakers/headsets and incoming/outgoing audio through supported PBXs/networks.
- Document the supported unencrypted SIP/UDP deployment boundary. TLS/SRTP are
  not implemented; do not advertise encrypted/public-network telephony.
- Complete the redistribution/license inventory for the actual shipped variant.
  Review the product SBOM against the selected platform, Cargo features, frontend
  bundle and native SDK, including OpenSSL, local patches and dependency
  relationships. Archive it with the matching installer and verify its contents;
  generating CycloneDX alone does not establish completeness. The
  [earlier SBOM review](verification/2026-11-05-sbom-eu-review.md) records the initial
  inventory's gaps; recheck them against the current implementation and
  [licensing documentation](licensing.md#about-page-and-project-sbom).
- Connect the actual repository, enable branch protection and private security
  reporting, and publish a maintained private reporting contact/support policy.
  Record hosted and Windows runtime results; local macOS checks cannot supply them.

Automatic updates remain disabled. A future updater requires its own endpoint,
signing key, release policy and verification before being enabled.

## Mac App Store preparation

The existing direct-distribution build remains separate from the App Store
variant. `tauri.appstore.conf.json` and `Entitlements.appstore.plist` enable App
Sandbox, inbound/outbound networking, microphone and read-only Contacts access.
The `app-store` Cargo feature selects the sandbox-compatible macOS login-item
implementation instead of writing a LaunchAgent outside the app container.
Native telephony and the existing SBOM/build wrapper are reused.

Use the pinned Rust/Bun toolchains and build the native SDK first:

```sh
bun run native:build
bun run test:macos-login-item
bun run appstore:prepare --unsigned --check
bun run appstore:prepare --unsigned
bun run appstore:screenshots
```

The local candidate is clearly labelled `unsigned` and is **not uploadable**.
The app, package, exact binary/package hashes, SBOM and `readiness.json` are under
ignored `artifacts/app-store/build`. The script checks the actual signed sandbox
entitlements and code signature before packaging. It does not install, overwrite
`/Applications/Ruf.app`, migrate personal account data, upload or submit anything.
The current native build supports the host architecture; this is not a universal
Intel/Apple Silicon build.

For a distribution-signed package, provide these environment variables locally:

- `RUF_APPSTORE_APP_IDENTITY`: exact local Apple Distribution or 3rd Party Mac
  Developer Application certificate name or SHA-1 identity, with private key.
- `RUF_APPSTORE_INSTALLER_IDENTITY`: exact local Mac Installer Distribution or
  3rd Party Mac Developer Installer name or SHA-1 identity, with private key.
- `RUF_APPSTORE_PROFILE`: absolute path to the Mac App Store provisioning profile
  for `org.rufapp.desktop`, including the selected application certificate.
- `RUF_APPSTORE_NON_EXEMPT_ENCRYPTION`: explicit `true` or `false`, after the
  publisher confirms its export-compliance classification. Ruf bundles OpenSSL
  and SRTP; the script does not assume an exemption or decide that classification.
- Optional `RUF_APPSTORE_BUILD_NUMBER`: one to three numeric components; default
  `1`. Increase for subsequent uploads of the same marketing version.

Then run `bun run appstore:prepare --check` followed by
`bun run appstore:prepare`. Preflight rejects missing identities, development or
expired profiles, wrong platform/Bundle ID/team and certificates absent from the
profile. The profile and team/application/keychain entitlements are embedded only
in ignored output. No certificate, profile or private key belongs in Git.
Downloaded profiles are copied into staging and cleared of extended file
attributes before bundling. The staged app is also cleared and re-signed before
packaging; symlinks do not cause cleanup outside the staging tree. The command
rejects `com.apple.quarantine` on the app, installer or expanded installer
payload (App Store error 91109). `readiness.json` records
`quarantineAttributesAbsent` and `expandedPackageChecked` only after these checks
pass. The original downloaded profile and the installed app are not cleaned.
Final distribution-identity sandbox testing and Apple's upload validation remain
required; successful local packaging is not App Review approval. Recheck keychain,
Contacts, incoming SIP/RTP, permissions, notifications and login behavior in that
final signed app. The sandbox uses its own app container; do not silently copy a
user's existing accounts or history into it.

Screenshots are generated from the actual current Svelte components with isolated
synthetic IPC responses, without reading the installed app, user credentials,
address book or call history. Five German PNGs cover dialing, active controls,
Contacts, history and sound settings. Each is 2880×1800, RGB without alpha, under
`artifacts/app-store/screenshots`; the capture also writes a local privacy report.
They show synthetic UI states and do not constitute a live call/audio test.
Review the images before uploading them to App Store Connect.

References: [Tauri App Store distribution](https://tauri.app/distribute/app-store/),
[Apple screenshot specifications](https://developer.apple.com/help/app-store-connect/reference/app-information/screenshot-specifications),
[Apple signing certificates](https://developer.apple.com/help/account/certificates/certificates-overview/).
