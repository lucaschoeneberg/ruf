# Release cleanup verification

Date: 5 November 2026. Scope: prepare the existing 0.1.0 preview for a reproducible
release draft. The working tree already contained extensive application/native
changes and new source files at the start; those changes were preserved. This
cleanup does not commit, tag, push, publish, install or replace the running app.

## Changes

- `.github/workflows/quality.yml`: move the existing portable and browser jobs
  into one reusable workflow. `ci.yml` and `release.yml` both invoke it; release
  draft creation now depends on its success.
- `.github/workflows/desktop-build.yml`: run the existing deterministic macOS
  Contacts/notification timeout checks before packaging. They request no OS consent.
  Request both `app,dmg` on macOS, retaining Ruf.app for the following checks;
  the old DMG-only invocation deleted it and broke the permission-check steps.
- `.github/workflows/release.yml`: require version-matched notes before building,
  create a draft prerelease, require exactly one inventory/evidence pair per target,
  and attach SHA256SUMS for exactly one DMG and one NSIS
  installer. Checksums contain plain filenames for verification after download.
- `scripts/check-versions.mjs`: additionally enforce the pinned Rust version for
  both actual `rustc` and `cargo`, and all workspace package `rust-version` values.
  Previously this check accepted Homebrew Rust 1.99.1 despite the 1.98.1 pin.
- `README.md`, `docs/architecture.md`, `docs/ci-cd.md`, `docs/development.md`,
  `CONTRIBUTING.md`, `SECURITY.md`, `.github/ISSUE_TEMPLATE/bug_report.yml` and
  `THIRD_PARTY_NOTICES.md`: remove obsolete scaffold/functionality descriptions.
- Replace `docs/releases/foundation.md` with `docs/releases/0.1.0.md`; add
  `docs/release-checklist.md` and the cleanup plan under `docs/superpowers/plans`.
- `apps/desktop/src/lib/components/AboutPanel.svelte` and `docs/licensing.md`:
  describe the existing inventory as a project inventory instead of a complete SBOM.
- `tests/ui/software-bill.test.ts`: skip the native macOS SDK integration test
  when that SDK is absent. A source-only temporary checkout verified four tests
  pass and that one is skipped without `native/install`; normal macOS runs include it.
- `tests/ui/browser/settings.mjs`: verify the central portable components and
  variant-specific inventory, including the corrected inventory disclosure label.

Existing Bun/Cargo scripts, generated ts-rs contracts, Vite inventory, Tauri
packaging, native SDK/interop scripts and pinned GitHub Actions were reused. No
dependencies were added. This cleanup changes no IPC command, DTO, validation,
error or status-code contract; no API documentation change was required.

## Observed checks

Final Rust commands use Rustup Rust/Cargo 1.98.1 explicitly ahead of Homebrew on
PATH. Bun is 1.4.4. Earlier exploratory portable checks also passed with Homebrew
1.99.1; those are not the pinned-toolchain evidence.

| Check                                                                      | Observed result                                                                                                                                                                                            |
| -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bun install --frozen-lockfile`                                            | Passed; no dependency changes.                                                                                                                                                                             |
| `bun run format:check`                                                     | Passed.                                                                                                                                                                                                    |
| `bun run check`                                                            | Passed; Svelte reported zero errors/warnings.                                                                                                                                                              |
| `bun run test:ui`                                                          | 53 tests passed in the refreshed run, zero failures.                                                                                                                                                       |
| `bun run test:ui:flows`                                                    | contacts, history, phone, settings and layout passed using synthetic IPC.                                                                                                                                  |
| `bun run build`                                                            | Passed, including the project inventory asset.                                                                                                                                                             |
| `bun run check:versions` and matching `RELEASE_TAG=v0.1.0`                 | Passed with pinned toolchain.                                                                                                                                                                              |
| Version failure paths                                                      | Actual Homebrew 1.99.1, synthetic mismatched Cargo, and wrong tag rejected.                                                                                                                                |
| `bun run types:generate`                                                   | Passed; generated files byte-identical to the pre-generation working tree.                                                                                                                                 |
| Portable workspace Clippy with `-D warnings` and tests                     | Passed under Rust 1.98.1; 89 tests passed, one OS-keychain integration test intentionally ignored.                                                                                                         |
| `bun run native:build`                                                     | Passed, including patched libre NAT registration checks.                                                                                                                                                   |
| Native workspace Clippy with `-D warnings`                                 | Passed under Rust 1.98.1.                                                                                                                                                                                  |
| Native workspace tests                                                     | Passed under Rust 1.98.1: 132 passed, 10 intentionally ignored. Six lab adapter cases also ran through `test:asterisk`.                                                                                    |
| macOS Contacts/notification timeout regressions                            | Passed with no permission request or personal-data access.                                                                                                                                                 |
| `bun run test:network-roam`                                                | Six state-machine cases and native call-action cases passed.                                                                                                                                               |
| `bun run test:asterisk`                                                    | Passed: 4.04 seconds returned audio, RMS 5811, 440.1 Hz; all six native integration tests passed.                                                                                                          |
| `bun run test:sip-diagnostics`                                             | Passed: 64 allowlisted markers; diagnostics absent by default.                                                                                                                                             |
| `bun run desktop:build --features native --bundles dmg`                    | Passed initially and after the parallel SBOM changes; `--bundles app,dmg` produced both bundles under ignored `target/aarch64-apple-darwin/release/bundle/`.                                               |
| `test:macos-audio` and `test:macos-contacts` against built Ruf.app         | Passed signature/entitlement/purpose-string checks.                                                                                                                                                        |
| `go run github.com/rhysd/actionlint/cmd/actionlint@v1.7.12 -color`         | Passed.                                                                                                                                                                                                    |
| Release shell exercised locally with fake installers and mocked `gh`       | Correct draft/prerelease/tag/notes arguments and seven assets; installer checksums verified; extra/missing installers, mismatched per-target inventory counts and missing notes rejected. No GitHub write. |
| Local Markdown link targets, tracked artifact paths and `git diff --check` | Passed. No tracked installer/recording/database/private-key paths found; this is not a full secret scan.                                                                                                   |

The generated-contract comparison uses before/after hashes because intentional
DTO changes were already present in the working tree. It does not claim a clean
diff against HEAD. All intended untracked source files must be included when the
maintainer commits the complete change, especially `apps/desktop/build/software-bill.ts`.

## Remaining limits

The parallel chat “Über-uns-Seite überarbeiten” replaced the inventory
generator/tests during this cleanup, as confirmed by the user. Its implementation
was preserved. After the integration fixes, fresh typechecking passed, all 53 UI
tests passed, and all five browser scenarios passed. Native workspace Clippy and
the desktop tests were refreshed too (61 passed, four intentionally ignored).
The updated native DMG built successfully with the ordinary wrapper, without
manually supplied SBOM environment variables, after the parallel wrapper fix.
Its artifacts now live under `target/aarch64-apple-darwin/release/`. A read-only mount of the generated DMG passed both signature/permission scripts against the shipped Ruf.app and was detached afterward. The `app,dmg` build then completed with both bundles listed. Parallel desktop builds can replace these shared local outputs; hosted platform jobs have separate checkouts.

The pinned local Rust toolchain emitted `rust-objcopy` warnings about loading
`libLLVM.dylib` during release stripping. Compilation and DMG packaging still exited
successfully. Notarization was skipped because production credentials are not
configured. Neither warning was suppressed or worked around in repository code.

No hosted GitHub run, Windows build/runtime execution, real microphone/headset
acceptance, installed notification click, login/upgrade acceptance or private
security-reporting setup was observed in this cleanup. The running installed app
was left in place, and the previously running Asterisk lab remains running.
System-keychain integration tests that write actual credentials were not enabled.

Production signing, complete distribution license/product-SBOM inventory and
platform/provider acceptance remain open in the [release checklist](../release-checklist.md).
The earlier inventory's omissions were not resolved by renaming its UI label;
the parallel implementation has its own substantive changes and verification.
This work does not establish public production readiness.
