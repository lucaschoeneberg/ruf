# German and English localization, 10 November 2026

These observations precede the migration to i18next/rust-i18n. See
[the library migration record](2026-11-10-i18n-libraries.md) for current engine verification.

Local observations for the uncommitted `codex/i18n-de-en` working tree based on
`2e7fca0`, macOS arm64, Bun 1.4.4 and Rust 1.98.1. These observations do not
establish Windows runtime, hosted CI, notarization or hardware audio behavior.

## Behavior and contracts

Shared local catalogs cover the phone/settings UI, accessible labels, search,
errors, native menus/dialogs/notifications and release notes. Automatic mode uses
the preferred OS languages and falls back to English. A manually selected
German/English language applies only after a successful full preference save.
Both windows and native presentation share a revisioned Rust state, with no
component remount or change to SIP state. See [localization](../api/localization.md)
for commands, input defaults, events, adapter text boundaries and the documented
downgrade limitation. Rust-generated DTO output includes the new preference and
localization types and remains stable on regeneration.

## Observed checks

| Check                                                            | Result                                                                                                                                                                                                                                                                              |
| ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bun run check`                                                  | Catalog checks, strict tooling/frontend TypeScript and Svelte passed; zero Svelte errors/warnings.                                                                                                                                                                                  |
| `bun run i18n:check`                                             | Matching German/English keys, placeholders and plural forms. Malformed-template regression observed failing before the validator fix, then passing.                                                                                                                                 |
| `bun run test:ui`                                                | 125 tests passed, zero failed; 1,432 assertions. Includes resolution, revision ordering, plural/parameter validation, safe error rendering and component localization.                                                                                                              |
| `bun run test:ui:flows`                                          | All 19 completions passed: full existing DE9 regression baseline, EN9 scenarios using the same synthetic native fixtures, and the multiwindow localization flow. The focused flow also passed after strengthening stale-response assertions.                                        |
| Multiwindow browser coverage                                     | Four windows verified held/successful/discarded/failed saves, matching committed languages, preserved account/dial drafts and live-call DOM identity. An expanded changelog aborted the old request, rejected its late response and retained German bundled notes on fetch failure. |
| `bun run build`                                                  | Frontend production build passed.                                                                                                                                                                                                                                                   |
| `bun run check:versions`                                         | Passed with the pinned toolchain first on PATH.                                                                                                                                                                                                                                     |
| `bun run types:generate`                                         | Passed; generated contract hash unchanged on repeat generation with pinned Rust.                                                                                                                                                                                                    |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed.                                                                                                                                                                                                                                                                             |
| `cargo test --locked --workspace`                                | Passed; 136 tests, one ignored.                                                                                                                                                                                                                                                     |
| `bun run native:build`                                           | Passed with the existing macOS-14-compatible static OpenSSL installation.                                                                                                                                                                                                           |
| Native workspace Clippy                                          | Passed with `--features sip-desktop/native`.                                                                                                                                                                                                                                        |
| `bun run test:native`                                            | All three Cargo partitions passed: 310 tests, zero failures, 20 ignored; protocol timeout scenarios included.                                                                                                                                                                       |
| `bun run test:asterisk`                                          | Registration, established/closed call and returned audio passed. Thirty opt-in lab/security/startup tests passed. PCM: 4.04 seconds, RMS 5811, 440.1 Hz. The test lab was stopped afterward.                                                                                        |
| `bun run test:macos-notification-timeouts`                       | Passed stalled settings/delivery, late callbacks, withdrawal, retry, independent anonymous test delivery and updated category-label checks.                                                                                                                                         |
| `bun run desktop:build`                                          | Portable App and DMG built; SBOM context/hash checks passed.                                                                                                                                                                                                                        |
| `bun run desktop:build --features native --bundles app`          | Native App built; SBOM context/hash checks passed.                                                                                                                                                                                                                                  |
| Bundle checks                                                    | Microphone entitlement/purpose and Contacts entitlement/purpose/signature checks passed. Both packaged `InfoPlist.strings` files exactly matched source; declared localizations are de/en with English development region.                                                          |
| Independent review                                               | Native state/save ordering, frontend reactivity, adapter ownership/escaping, catalogs and changelog races reviewed. The untranslated settings tooltip and missing malformed-template guard were corrected.                                                                          |

The original staged release-checklist edit was compared with its saved patch and
preserved. Test artifacts and synthetic recordings remain ignored local files.

## Limits

Desktop builds completed with the existing local `rust-objcopy`/`libLLVM.dylib`
debug-stripping warning and were ad-hoc signed, without notarization.
No Windows runner, actual system notification display, permission prompt or
physical microphone/speaker test was executed for this working tree. The previous
GitHub CI attempt was blocked before runner startup by account billing/limits;
this unpushed feature was validated locally, not by that run.
