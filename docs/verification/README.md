# Verification evidence

These are recorded observations, not a claim that the current working tree or
every version-0.1.0 installer passed them. Detailed reports retain source/artifact
bindings where those were recorded. Reports without an executable hash or source
revision cannot qualify a newer package. Local runtime-test captures, recordings
and build outputs are not committed. Synthetic public UI illustrations are
documented separately in [media provenance](../screenshots/README.md).

Completed implementation plans and repeated audit transcripts have been removed.
Their relevant results are consolidated below; tracked originals remain in Git
history. Current behavior belongs in the [API reference](../api/README.md),
[architecture](../architecture.md), [product scope](../../apps/desktop/PRODUCT.md)
and [release checklist](../release-checklist.md).

## Detailed records retained

| Record                                                                                    | Evidence and boundary                                                                                                                                                                                                                                                       |
| ----------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Native CI speed, 12 November](2026-11-12-ci-speed.md)                                    | Independent OpenSSL cache and bounded live native runner; local 312-test comparison and native/portable/Asterisk/app checks. Full local OpenSSL qualification blocked by an upstream OCSP pipe deadlock; hosted cache and Windows validation remain separate.               |
| [Published v0.1.1, 11 November](2026-11-11-0.1.1-release.md)                              | Fresh local and tagged hosted checks, both downloaded installers and their inventory/hash bindings, and verified public prerelease publication. Physical-device/provider and unsigned-distribution boundaries retained.                                                     |
| [Developer Preview preparation, 10 November](2026-11-10-developer-preview-preparation.md) | Repository/tester documentation, synthetic marketing media, fresh UI/Rust checks and a locally hashed native macOS candidate. No publication, hosted CI, Windows runtime or hardware/provider qualification.                                                                |
| [German/English localization, 10 November](2026-11-10-localization.md)                    | Local UI/native state, browser regression and packaging evidence; records the original implementation before the established-library migration.                                                                                                                             |
| [i18n library migration, 10 November](2026-11-10-i18n-libraries.md)                       | i18next/rust-i18n shared catalogs, preserved system/manual/native synchronization, local UI/Rust/build checks; no hosted qualification of this feature.                                                                                                                     |
| [DMG design, 10 November](2026-11-10-dmg-design.md)                                       | Distilled English installation background, signed local native app/image and mounted Finder checks; no HTML guide or notarization.                                                                                                                                          |
| [GitHub Actions and installers, 10 November](2026-11-10-github-actions.md)                | All five hosted CI jobs passed at a frozen revision; native macOS DMG and Windows NSIS artifacts downloaded and checked against their SBOM/hash manifests. Includes macOS fixture timing fixes and local background controls; installed-device acceptance remains separate. |
| [Fresh local release checks, 10 November](2026-11-10-release-checks.md)                   | Portable/native Rust, Asterisk PCM/security, real synthetic Keychain round-trips, macOS bundles/DMGs, frozen source/SDK inputs and artifact hashes. Local Apple Silicon results; no hosted CI, Windows or macOS 14 runtime qualification.                                   |
| [Call controls and account security, 5 November](2026-11-05-call-controls-security.md)    | Frozen source revisions, Asterisk PCM/transfer measurements, TLS/SRTP positive and negative cases, production symbol checks and artifact hashes. Synthetic media does not qualify physical devices.                                                                         |
| [SIP lifetime/recovery fixes, 5 November](2026-11-05-sip-followup-fixes.md)               | Reproduced GuardMalloc crash and passing protected-allocation scenarios, ownership/race regressions and final native suites.                                                                                                                                                |
| [Installed external calls, 5 November](2026-11-05-installed-incoming.md)                  | Actual incoming calls, banners, user-reported two-way audio and NAT observations; later failed acceptance remains recorded. Final correction confirmed registration only.                                                                                                   |
| [Startup incoming, 5–6 November](2026-11-05-startup-incoming.md)                          | Two-cycle desktop restoration and negative authentication control; older installed 0.1.0 package rang after cold launch without outgoing priming. That installed call was cancelled before answer; no media claim or exact build binding.                                   |
| [DNS resolver lifetime, 7 November](2026-11-07-dns-resolver-lifetime.md)                  | Actual crash, reproduced SIGTRAP and 65,536 successful real resolver refreshes after correction. The rebuilt app was not installed and no multi-day soak was run.                                                                                                           |
| [Contacts and history, 4 November](2026-11-04-contacts-history.md)                        | Native lifecycle journal, SQLite recovery/idempotence and authorized in-memory contact projections; platform/runtime limits retained.                                                                                                                                       |
| [Settings, 5 November](2026-11-05-settings-redesign.md)                                   | Preferences/migration/permission/activation regressions, browser coverage and package checks.                                                                                                                                                                               |
| [Build-specific SBOM, 5 November](2026-11-05-sbom-build-fix.md)                           | Embedded export/sidecar/manifest binding, native OpenSSL coverage, dependency relationships, schema/negative checks and exact SBOM/DMG hashes; no completeness or compliance attestation.                                                                                   |
| [Dialing, contacts and DTMF, 5 November](2026-11-05-dialer-contacts-dtmf.md)              | Real IVR requires ordered `5#`; fresh 880 Hz PCM and no stale-key leakage. Production source commit, executable/SBOM hashes and absence of test hooks retained.                                                                                                             |
| [Keyboard/search and installed update, 6 November](2026-11-06-keyboard-navigation.md)     | Browser/menu regression evidence, Asterisk rerun and installed Apple-Development-signed update checks; no production-signing or platform-wide certification.                                                                                                                |
| [App Store package processing, 6 November](2026-11-06-appstore-quarantine.md)             | Actual error 91109, quarantine regression, package hash and successful build-2 processing for internal testing; no external TestFlight/public App Store approval.                                                                                                           |

## Native SIP and audio

Earlier 4–5 November checks are consolidated here rather than kept as separate
implementation transcripts:

- The initial native lifecycle/Asterisk run observed 4.04 seconds of returned
  synthetic PCM, RMS 5811 and approximately 440.1 Hz. The stale-call race was
  reproduced before the guarded-operation correction. Later detailed reports
  above retain stronger media and ownership evidence.
- A shortened-lifetime UDP fixture failed when a successful refresh omitted
  `received`, then passed after exact live-mapping renewal was added. Initial
  discovery, expired/unrelated mappings and malformed data stayed rejected.
  A separate binding-confirmation fixture first failed with two requests instead
  of three, then passed with positive-expiry own-Contact confirmation and one
  bounded single-Contact retry. The installed-call report retains the actual
  provider observations, including confirmation 49 ms after an empty response.
- A user-operated UniFi Talk/Tailscale trace observed incoming BYE, outgoing 200
  and native close without a local BYE. This establishes native dispatch, not
  frontend consumption or measured UI latency, and is not VPN/provider certification.
- Native SIP regressions exercised the 32-second missing-ACK path, RTP/RTCP
  inactivity and recovery, 65-second negotiated hold, answer/hangup races and
  subsequent calls. The 120-second unanswered timer was source-traced rather
  than observed for its complete duration. Malformed SDP first failed without a
  visible error, then produced safe `nativeFailure` with null `sipStatus`.
- Native virtual-clock jitter-buffer comparison played all 100 packets with zero
  insertion errors for fixed 100 ms versus adaptive 40 ms initial playout, both
  without jitter and with repeated 0–12 ms variation. The 60 ms difference is a
  buffer-component result, not measured microphone-to-speaker improvement.
- Actual isolated macOS notification delivery, silent categories and withdrawal
  passed. Action responses were synthetic delegate calls on real delivered
  requests, not interactive installed-app clicks. Fake-service timeout/late-
  callback/withdrawal tests passed separately; neither bypasses Focus policy.

## Desktop UI and local data

- On 4 November, an isolated native macOS lifecycle exercise verified hidden
  `--background` startup without focus, a normal second launch reopening the same
  process, close-to-hide with the process alive, actual status-icon/menu reopening
  and explicit Quit. Background activation was Accessory and normal opening
  Regular. A separate isolated native accessibility exercise passed dirty-close
  protection, revealing minimized Settings on Quit, Keep editing, explicit discard
  and final Quit. These smoke bundles used separate identities without real
  accounts; they did not qualify the installed/current Ruf package, login/logout,
  sleep/wake or Windows behavior.
- On 4 November, the opt-in saved-session regression actually used the macOS
  credential store with unique synthetic secrets and isolated metadata/Asterisk.
  It restored two accounts and outgoing selection, exercised retained passwords
  and obsolete-secret cleanup, and removed its test data. Native singleton
  settings-window creation/close/reopen was also exercised. This is distinct from
  later suites that intentionally left OS-keychain write tests ignored.
- The native sound smoke test enumerated devices, applied a selectable output,
  rejected an absent device without changing preferences and checked actual player
  start/stop/completion. It opened no microphone and did not establish subjective
  listening quality or hardware routing acceptance.
- Earlier installed SIP bundles lacked the signed microphone/Contacts entitlement.
  Rebuilt/staged/installed signature checks passed after correction. The older
  SIP installation showed granted Contacts permission and enumerated the address
  book, inspecting only the count. No first-consent/revocation cycle was forced;
  this is not qualification of the current Ruf identity or its fresh permissions.
- Settings audit regressions on 5–6 November resolved permission-read versus
  action-error confusion, stale loading indicators, search focus/field targets,
  long account-name overflow, accessible account names and heading order.
  All eight sections were exercised at 900 × 660 and 740 × 540, including CSS
  zoom 2 and a 370 × 270 layout equivalent. In 34 additional synthetic browser
  measurements, the narrow content changed from 170 px with up to 228 px overflow
  to 234 px content in a 234 px area. This is not observed native WebView zoom,
  VoiceOver acceptance, Windows presentation or full WCAG certification.
- Foundation, branding, tool-migration and repeated design-score reports were
  superseded by maintained docs and later checks. Font/theme attribution remains
  in [THIRD_PARTY_NOTICES](../../THIRD_PARTY_NOTICES.md); generated color ramps
  and local tool screenshots are not additional application tokens or dependencies.

## Latest local checks

The [10 November release-check record](2026-11-10-release-checks.md) supersedes
the cleanup-only check status below with fresh Asterisk, platform and artifact
verification. It also records the OpenSSL deployment-target/cache fixes and the
corrected optional restart fixture. Earlier observations remain historical.

The 9–10 November code cleanup preserved IPC contracts, generated types and lockfiles,
and reused the existing invocation/storage patterns. Its recorded macOS-arm64
working-tree checks were:

| Check                                                  | Recorded result                                                                                                                                                                 |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| UI tests and browser flows                             | 109 tests and all nine synthetic-IPC browser flows passed.                                                                                                                      |
| Svelte/TypeScript, frontend build, format and versions | Passed; Svelte reported zero errors/warnings.                                                                                                                                   |
| Contract generation                                    | No generated diff.                                                                                                                                                              |
| Portable workspace Clippy/tests                        | Passed; 126 tests passed, one OS-keychain write test ignored.                                                                                                                   |
| Native SDK, workspace Clippy/tests                     | Passed; 296 tests passed, 20 lab/OS cases intentionally ignored.                                                                                                                |
| Native macOS app/DMG and signed permission metadata    | Build, signature, entitlement and purpose-string checks passed. No installed-app/hardware test.                                                                                 |
| Independent review and mutation control                | No findings; 3,000 contact-suggestion comparisons matched the prior source. Inverting the status guard failed all seven new cases; restoring it passed all 16 background tests. |

Rust/Cargo 1.98.1 was selected through Rustup, with Bun 1.4.4. The package build
emitted the known missing-`libLLVM.dylib` stripping warning but exited successfully;
signing was ad hoc and notarization was skipped. Asterisk was not rerun in that
cleanup. The working-tree results have no recorded source freeze/installer hash
here and do not qualify every subsequent package.

The 10 November documentation alignment additionally passed formatting, 96 local
links, coverage of all 57 registered commands and all 21 `EngineSnapshot` /
14 `AccountSnapshot` fields, stable type generation, versions, three changelog
tests, actual bundled-changelog parsing, typecheck and frontend build. No native
or platform test was repeated for that documentation change.

## Release limits and upkeep

The [hosted CI record](2026-11-10-github-actions.md) establishes successful jobs
and installer uploads for its frozen source revision. No record establishes
Windows runtime, older-macOS hardware compatibility, complete physical-device
acceptance or a multi-day GUI soak.
Specific historical installed speech/banner or internal App Store processing
results must not be generalized to current packages or public distribution.
Production signing, complete license/source inventory and platform/PBX acceptance
remain in the [release checklist](../release-checklist.md).

Keep concise dated evidence when it adds a new observed platform result,
reproduced regression or artifact binding. Update this index rather than adding
a report for every cleanup, agent handoff or routine successful command. Keep
local tool state, screenshots and generated review metadata ignored.
