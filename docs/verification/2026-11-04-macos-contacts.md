# Native macOS contacts verification

The existing Kontakte tab now reads the system address book after an explicit
permission action. It provides name/company/number search, several labeled phone
numbers, favorites and direct calls through the existing selected-account dial
handler. The system address book stays read-only. Namesakes remain separate;
unsupported/ambiguous phone formats are displayed but cannot be called.

## Changed files owned by this task

- `crates/phone-core/src/contacts.rs`, `src/lib.rs`,
  `examples/export_types.rs`, `tests/contacts_contract.rs`: source-aware DTOs,
  permission states and conservative number conversion.
- `crates/baresip-adapter/native/contacts.h`, `native/contacts.m`,
  `src/contacts.rs`, `src/lib.rs`, `build.rs`, `Cargo.toml`: Contacts.framework
  permission/read bridge, owned UTF-8 copying and macOS 14 deployment target.
  The existing cc dependency also compiles contacts without enabling native SIP.
- `apps/desktop/src-tauri/src/contacts.rs`, `src/lib.rs`, `build.rs`,
  `capabilities/main.json`, `Info.plist` and three generated contact command
  permission files: worker operations, serialized local favorite-ID storage,
  explicit command permissions and system permission explanation.
- `apps/desktop/src/lib/contacts.ts`, `contacts-api.ts`,
  `components/ContactsPanel.svelte`, minimal integrations in `src/App.svelte`,
  and generated `lib/generated/AppInfo.ts`: contact panel and existing dial flow.
- `tests/ui/contacts.test.ts`: Unicode/company/phone/favorite/namesake search.
- `docs/api/contacts.md`, the link in `docs/api/README.md`,
  `apps/desktop/PRODUCT.md`, implementation plan and this verification record.

Other agents concurrently changed account saves, settings, design, build/CI and
test tooling in this checkout. Those edits were preserved and are not attributed
to the contacts feature. No automatic commit or branch integration was performed.

## Reused patterns and API documentation

- Existing Tauri commands/capabilities and blocking worker runtime.
- Existing Rust/serde/ts-rs contracts, PhoneError codes, and generated frontend DTOs.
- Existing selected-account `dial` command and operation/busy/registration checks.
- Existing app-data JSON versioning and same-directory file replacement; favorites
  have a separate mutex and file, so no contact operation holds the phone lock.
- Existing typography, palette, controls and compact call panel. Styles are local
  to ContactsPanel; no replacement design system or UI dependency was added.
- Full required/nullable fields, permissions, validation, errors, refresh cadence,
  local persistence and platform limitations are in `docs/api/contacts.md`.

## Observed automated checks

- Contact normalization tests: 2 passed, including observed failing then passing
  regression for `0049 (0)…`. International `+… (0)…`, post-dial sequences, controls,
  SIP addresses, service codes, internal extensions and length boundaries covered.
- Favorite persistence tests: 3 passed, including malformed/unsupported preference
  preservation and validation before creating storage.
- Native Rust callback test: 1 passed using synthetic borrowed C strings, multiple
  numbers, namesakes and a contact without numbers. No personal contacts accessed.
- Contact search tests: 4 passed. Numeric company search was observed failing
  before its fix; Unicode, spelling separators, favorites and source order covered.
- `bun run format:check`, `bun run check`, `bun run test:ui`, `bun run build`,
  `bun run check:versions`: passed. Typecheck reported zero errors and warnings.
  The shared test command also discovered another agent's ignored-directory audit
  snapshot tests on one run; those passed too and are not extra contact coverage.
- `bun run types:generate`: passed; SHA-256 unchanged after regeneration.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed.
- `cargo test --locked --workspace`: passed; one actual-OS-keychain test is explicitly
  ignored. No hosted CI execution is claimed.
- `bun run native:build`: completed; native workspace clippy with
  `--features sip-desktop/native`: passed.
- `cargo test --locked --workspace --features sip-desktop/native`: passed;
  lab-dependent tests remain explicitly ignored in this command and were run
  separately through the Asterisk script.
- `bun run test:asterisk`: passed. RTP echo and the four selected native interop
  tests completed, including account switching, remote hangup and stale actions.
- `bun run desktop:build --features native --bundles app`: completed with an ad-hoc
  signed macOS app. `codesign --verify --deep --strict` passed and the bundled
  `NSContactsUsageDescription` was checked. Notarization is not configured.

## Rendered interaction verification

Playwright exercised the actual Svelte panel with synthetic Tauri IPC responses
at 340×500 and 320×440. Assertions passed for:

- Entering Kontakte without requesting permission; explicit permission action.
- Namesakes, multiple phone actions and entries without numbers.
- Unicode, formatted phone search and favorite filtering.
- Successful favorite mutation; storage failure preserves the prior flag.
- Keyboard navigation and Enter forwarding the exact `1234` dial target.
- Contact call buttons disabled during a call or when the account is unregistered.
- Revoked permission and read failure clear all displayed contacts; retry recovers.
- Denied permission refresh does not request access again.
- No document-width overflow and visible bottom navigation at the minimum size.

Screenshots and fixture scripts live only in ignored `artifacts/contacts/`.
The native contact data path was not replaced in production code. An independent
read-only review reported no remaining material findings in the contacts scope
after the normalization fix and inspection of all four screenshots.

## Remaining manual validation and scope

The real macOS permission dialog, reading a person's actual address book,
permission withdrawal in System Settings, and source changes have not been
exercised. To validate, run the newly built app, open Kontakte, press
macOS-Kontakte freigeben, then test granted and revoked access. No automatic
permission acceptance or personal-contact logging was used.

Source data is refreshed on panel entry, visible focus/visibility restoration,
manual refresh and a 30-second visible timer; no instantaneous push sync is
claimed. Incoming caller name resolution and saving unknown callers need the
separate peer/history work. Contact editing stays in Apple's Contacts app.
Windows, other providers, vCard import, full contact caching and cloud logins
remain deferred. Windows runtime and hosted CI were not executed.
