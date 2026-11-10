# Ruf 0.1.0 Developer Preview: local preparation

Repository entry points, tester onboarding, a compatibility issue form, current
English UI media and the release handoff were prepared locally on 10 November 2026. The preview is free and has no in-app purchases. Nothing was pushed, tagged,
uploaded or published; GitHub settings and community issues remain local drafts
in the [launch package](../releases/0.1.0-launch.md).

The existing feature checkout includes uncommitted German/English localization
and DMG artwork. Those changes and the pre-existing staged checklist edit were
preserved. This task adds no application command, DTO, dependency or SIP/audio
behavior. Its new media command is documented in [tooling](../api/tooling.md),
reusing the installed Playwright dependency, synthetic App Store IPC fixture,
bundled fonts and canonical Ruf artwork.

Changed files in this task:

- Entry points: `README.md`, `CONTRIBUTING.md`.
- Testing: `docs/testing-preview.md`, `.github/ISSUE_TEMPLATE/compatibility_report.yml`.
- Release copy: `CHANGELOG.md`, `CHANGELOG.de.md`, `docs/releases/0.1.0.md`,
  `docs/releases/0.1.0-launch.md`; the title flag in `.github/workflows/release.yml`.
- Media tooling: `scripts/capture-repository.ts`, the `media:repository` script in
  `package.json`, `docs/api/tooling.md`, `docs/screenshots/README.md`.
- Media: `docs/screenshots/{dialer,contacts,call,history,settings,social-preview}.png`
  and `docs/screenshots/demo.gif`.
- Evidence: this report and `docs/verification/README.md`.

## Observed checks

| Check                                                       | Result                                                                                                                                                                                          |
| ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bun install --frozen-lockfile`                             | Passed; no dependency changes.                                                                                                                                                                  |
| `bun run format:check` / `bun run check`                    | Passed; Svelte reported zero errors and warnings.                                                                                                                                               |
| `bun run check:versions`                                    | Version 0.1.0 matches JavaScript, Cargo and Tauri.                                                                                                                                              |
| `bun run types:generate`                                    | Passed; generated contract hash matches the task baseline.                                                                                                                                      |
| `bun run test:ui`                                           | 126 passed. A new leading changelog bullet initially broke an existing ordering assertion; retaining the existing first entry and appending the new bullet fixed it without changing the test.  |
| `bun run test:ui:flows`                                     | Nine German flows, nine English flows and the multiwindow localization flow passed using synthetic IPC.                                                                                         |
| Portable workspace Clippy / tests                           | Passed with warnings denied; 138 tests passed, one ignored.                                                                                                                                     |
| `bun run native:build`                                      | Passed using the existing macOS-14-compatible static OpenSSL installation.                                                                                                                      |
| Native workspace Clippy / `bun run test:native`             | Passed with warnings denied; all three test partitions passed, 312 tests, 20 ignored.                                                                                                           |
| `bun run desktop:build --features native --bundles app,dmg` | Native Apple Silicon app/DMG and SBOM/build manifest generated; frontend production build passed.                                                                                               |
| Signed bundle and DMG                                       | Deep/strict signatures, microphone/Contacts entitlement and purpose checks, matching DE/EN permission resources, DMG verification and read-only mounted executable hash passed. Mount detached. |
| Inventory and source binding                                | Embedded SBOM equals its sidecar; raw executable, installer and SBOM match build manifest. All 420 frozen repository inputs were unchanged during packaging.                                    |
| `bun run media:repository`                                  | Current screenshots, five-frame 30-second GIF/MP4 and 1280×640 social image generated; no page errors or forbidden permission/call/audio actions. Images inspected visually.                    |
| Documentation and release shell                             | Local links resolve; compatibility form IDs are unique; YAML and release shell parse; draft/prerelease flags retained. Scoped independent review found no remaining actionable findings.        |

Native checks selected Rust/Cargo through the pinned Rustup toolchain. No fresh
Asterisk, physical-audio, provider, Windows runtime or hosted workflow execution
was performed for this documentation/media patch. Earlier SIP results remain
bound to their own [verification records](README.md).

## Local candidate and limits

The ignored `artifacts/release-preparation/0.1.0/` directory contains the local
native macOS DMG, CycloneDX SBOM, build-hash manifest, `SHA256SUMS`, copied release
body, frozen-input hashes and `local-readiness.json`. The latter records the base
commit, dirty-tree boundary, exact signed bundle hash and actual signing status.
The source snapshot predates this verification report and its index update;
neither changes executable inputs. This is a local candidate, not a complete two-platform
release asset set.

| Artifact                | SHA-256                                                            |
| ----------------------- | ------------------------------------------------------------------ |
| `Ruf_0.1.0_aarch64.dmg` | `ccee9107c4bb1c4c7c75a02b311bc9979031f0c3ecb8bb7f8bd1f136ef18af1a` |
| Native SBOM             | `9f86fa03f32adbedf7c02fe1378d8ff621806dba464b836d3e37fa50d983dfff` |
| Raw Cargo executable    | `9ae21356d4c8e6c86d9a4dad0f31bf8ee8adb6feb3841127b06e329921080b91` |

The build manifest hashes the raw Cargo executable. Tauri signing changes the
bundled executable bytes; the mounted DMG was therefore checked against the
signed app hash separately, rather than incorrectly comparing it to the raw hash.

This candidate is ad-hoc signed and not notarized. Packaging succeeded with the
existing Vite chunk-size warning and a replayed `rust-objcopy`/`libLLVM.dylib`
stripping warning; this run does not claim a warning-free package build. No
toolchain files or runtime code were changed to hide those warnings.

Before publication, review/freeze one source revision, observe its required hosted
jobs, build and inspect both platform installers, verify their exact distribution
identity and complete the applicable [release checklist](../release-checklist.md).
Windows desktop behavior, real audio devices and provider qualification remain
separate acceptance work. No app was installed or launched into normal calling
mode during this preparation.
