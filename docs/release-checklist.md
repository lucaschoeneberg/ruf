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
