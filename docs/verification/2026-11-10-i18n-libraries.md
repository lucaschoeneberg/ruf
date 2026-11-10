# Migration to i18next and rust-i18n, 10 November 2026

Local observations for the uncommitted `codex/i18n-de-en` working tree based on
`2e7fca0`, macOS arm64, Bun 1.4.4 and pinned Rust 1.98.1. This record verifies the
library migration following [the initial localization implementation](2026-11-10-localization.md).

## Implementation

- Frontend lookup, interpolation, plural selection and number formatting use
  **i18next 27.1.1**. Browser preview language negotiation uses i18next's language
  utilities; desktop system detection continues through **sys-locale 0.3.2**.
- Native lookup and interpolation use **rust-i18n 4.2.4** with compile-time
  resources and an explicit locale for every lookup. No second global language
  state, runtime catalog reader or custom template replacement loop remains.
- Both engines consume `locales/de.json` and `locales/en.json`. The 859 original
  messages map to 866 flat entries per catalog using i18next JSON V4 plural
  suffixes. `%{name}` uses rust-i18n's normal syntax and i18next's configurable
  interpolation delimiters; `%{count, number}` uses i18next's number formatter.
  Translation content was preserved, apart from locale-aware number grouping.
- Existing Svelte message references/stores, persisted `system | de | en`
  preference, Rust authority, IPC/events/revisions and native menu refresh remain.
  Manual changes apply after successful saving. Both windows retain drafts and
  active-call DOM identity. No preference/DTO/event shape changed in this migration.
- Catalog parity and malformed-template validation remain build-time authoring
  checks in `scripts/i18n-catalogs.ts`, outside the frontend runtime.

Changed areas are the frontend/native localization modules, shared catalogs,
dependency manifests/locks, native catalog rebuild tracking, catalog checker,
localization tests, English browser expectations and localization/development docs.
See [the API documentation](../api/localization.md) for contracts and limitations.

The Cargo resolution adds rust-i18n and its transitives. Its TOML minimum also
requires `toml` 0.8.2 → 0.8.23 and `toml_datetime` 0.6.3 → 0.6.11; resolving the
older exact datetime constraint changes `proc-macro-crate` 2.0.2 → 2.0.0. Other
existing dependency versions were retained. Bun adds only i18next.

## Observed checks

`bun run format:check` and `git diff --check` also passed.

| Check                                                            | Result                                                                                                                                                                          |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bun install --frozen-lockfile`                                  | Passed.                                                                                                                                                                         |
| `bun run check`                                                  | Catalog validation, strict tooling/frontend TypeScript and Svelte passed; zero Svelte errors/warnings.                                                                          |
| `bun run test:ui`                                                | 126 passed, zero failed, 1,540 assertions. Covers regional language negotiation, manual overrides, stale revisions, plurals, grouped numbers and literal interpolation values.  |
| `bun run test:ui:flows`                                          | All 19 completions passed: DE9, EN9 and multiwindow localization. Includes successful/failed/discarded saves, retained drafts/live-call DOM and stale changelog responses.      |
| `bun run build`                                                  | Frontend production build passed. Vite reports the existing chunk-size advisory.                                                                                                |
| `bun run check:versions`                                         | Passed with pinned Rust first on PATH.                                                                                                                                          |
| `bun run types:generate`                                         | Passed; generated contract SHA-256 unchanged on repeat generation.                                                                                                              |
| `cargo test --locked --workspace`                                | 138 passed, zero failed, one ignored.                                                                                                                                           |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed.                                                                                                                                                                         |
| Native workspace Clippy                                          | Passed with `--features sip-desktop/native`.                                                                                                                                    |
| `bun run test:native`                                            | All three Cargo partitions passed: 312 passed, zero failed, 20 ignored, including protocol timeout scenarios.                                                                   |
| Native catalog rebuild tracking                                  | Changing only the English catalog timestamp caused Cargo to recompile the desktop crate.                                                                                        |
| `bun run desktop:build`                                          | Portable App and DMG built; SBOM context/hash checks passed.                                                                                                                    |
| `bun run desktop:build --features native --bundles app`          | Native App built; SBOM context/hash checks passed.                                                                                                                              |
| `bun run test:macos-audio <bundle>`                              | Signed microphone entitlement and purpose-string checks passed.                                                                                                                 |
| `bun run test:macos-contacts <bundle>`                           | Signed Contacts entitlement, purpose-string and signature checks passed.                                                                                                        |
| Packaged localization resources                                  | Both packaged `InfoPlist.strings` files match source; declared languages de/en and English development region verified.                                                         |
| Independent review                                               | No actionable findings. Reviewed library delegation, preserved catalogs, explicit native locale, borrowed static resources, catalog rebuild tracking and state synchronization. |

The browser CLI's isolated callback VM cannot dynamically import packages. English
expected labels are therefore resolved with the actual i18next engine in the Bun
runner before serializing the callback; no substitute interpolation engine is used
in the browser harness. The complete suite passed after that harness adjustment.

## Limits

Desktop builds completed with the existing local `rust-objcopy`/`libLLVM.dylib`
debug-stripping warning and ad-hoc signing, without notarization. Bundle checks
verify configuration/signatures, not permission prompts or physical audio devices.
The preceding implementation's Asterisk and macOS notification timeout checks were
not repeated for this translation-engine change; SIP/audio/adapter code was not
modified by this migration.

Windows builds/runtime and hosted CI were not executed for this unpushed working
tree. The previous CI attempt was blocked before runners by account billing/limits.
The pre-existing staged release-checklist edit was compared with its saved patch
and remains unchanged. No commit or push was made for this migration.
