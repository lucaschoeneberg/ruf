# Settings audit fixes · 2026-11-05

Implemented the approved sequence **Impeccable harden → adapt → polish** in the
existing settings-redesign worktree. The original
[technical audit](2026-11-05-settings-impeccable-audit.md) remains the record of
its initial observations; all six reported issues are addressed below.

## Findings and resulting behavior

| Finding                                                            | Fix                                                                                                                                                                                                                                                     | Verification                                                                                                                                                                               |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| A01: delivery/open errors masquerade as failed authorization reads | Separate permission-read, authorization/open-action and test-delivery errors. Failed test delivery clears obsolete pending feedback and retains the real OS permission and caller draft.                                                                | Failed send/open scenarios, successful retry, existing unknown/denied/authorization/draft cases pass. Confirmation shows `authorized` and `Erlaubt` alongside the separate delivery error. |
| A02: About search target missing                                   | Existing About heading receives its registered ID and programmatic focus.                                                                                                                                                                               | Version search places focus on `about-heading`.                                                                                                                                            |
| A03: incomplete field search                                       | Reuse the existing typed route/field register with concrete persistent field labels, IDs and synonyms. Search focuses actual caller, volume, company, retention and password controls; existing parent disclosures open before focusing nested targets. | Six new search unit regressions, rendered visible-label contract across all eight sections and nested SBOM focus pass. No account values or secrets enter the static index.                |
| A04: valid long account names overlap actions                      | Apply `overflow-wrap: anywhere` to existing overview-name headings; preserve full identity and existing grid/actions.                                                                                                                                   | Eight-account fixture with 64-character unbroken Latin and CJK names has equal content/client widths: 678/678 at 900px and 540/540 at 740px. Actions remain alongside the name column.     |
| A05: repeated account actions lack account context                 | Existing rows become named groups; action names include the current account while retaining visible action words.                                                                                                                                       | Browser verifies group `Büro`, action `Deaktivieren · Büro` and native-state activation behavior.                                                                                          |
| A06: section heading hierarchy inverted                            | SIP-Konten is h1, saved-account overview and embedded account details are h2, detail fields are h3. Reuse AccountSetup through an optional embedded presentation prop; standalone headings retain their prior levels.                                   | Browser asserts h1/h2/h3 structure; existing standalone account SSR tests pass.                                                                                                            |

Temporary deletion-confirmation steps remain explicit follow-up controls rather
than global search destinations. Retention and history deletion are searchable
through their existing permanent controls; searching never selects retention,
confirms deletion or executes an action.

## Reused patterns and scope

The patch uses existing Svelte 5 state, AccountSetup, permission status rendering,
CSS grid/tokens, settings routes, guarded drafts, inline errors and native IPC
wrappers. No new dependency, native functionality, persistence schema, command,
DTO or capability was introduced. API documentation and generated IPC contracts
remain unchanged. PRODUCT.md/DESIGN.md context drift noted in the audit was not
silently rewritten as part of these fixes.

## Observed checks

- RED: the six new exact-field search assertions failed on the pre-fix registry.
  New browser cases reproduced misleading permission state, missing About focus,
  unassociated account actions and inner overflow **877/540**.
- GREEN: `bun run test:ui` — **67 passed, 0 failed**.
- `bun run test:ui:flows` — contacts, history, phone, settings and layout passed.
  Settings cases cover preserved drafts, failed send/open, successful retry,
  concrete search IDs, disclosure focus, account action names/hierarchy and valid
  long names. The existing native-window minimum and effective zoom layout checks
  also pass with synthetic IPC.
- `bun run check` — **0 errors, 0 warnings**.
- `bun run build`, `bun run check:versions` and `bun run format:check` passed.
- `bun run types:generate` leaves the generated file byte-identical:
  `49a694dfcd48236125311558e1575fbea281a49245cdef33ab12f1f4cbb7f9ab`.
- Pinned Rust 1.98.1: `cargo clippy --locked --workspace --all-targets -- -D warnings`
  and `cargo test --locked --workspace` passed. The real-keychain portable test
  remains ignored as before.
- `bun run desktop:build --features native --bundles app` produced Ruf.app with
  matching SBOM/build evidence, using the already-built native SDK.
- `bun run test:macos-audio <bundle>` verified the signed audio-input entitlement
  and microphone purpose string; it does not prove actual audio capture.
- Final Impeccable detector over settings/components/CSS: **0 findings**.
- Independent focused review approved the patch. Reviewer independently ran all
  67 UI tests and demonstrated in an isolated copy that breaking search matching
  makes all six new field-search regressions fail.
- Two bounded visual groups: standard settings/layout screenshots, then explicit
  long-name, notification-error and About-focus confirmation. Captures use only
  synthetic data and are ignored by Git under `output/playwright/`.

## Final confirmation artifacts

- `output/playwright/settings-audit-fixed-accounts-900.png`
- `output/playwright/settings-audit-fixed-accounts-740.png`
- `output/playwright/settings-audit-fixed-notification-error.png`
- `output/playwright/settings-audit-fixed-about-focus.png`

The original six findings have no remaining open correction in this patch.
This targeted confirmation is not a new formal WCAG compliance assessment.

## Changed files

- `apps/desktop/src/Settings.svelte`
- `apps/desktop/src/app.css`
- `apps/desktop/src/lib/settings-navigation.ts`
- `apps/desktop/src/lib/components/NotificationSettings.svelte`
- `apps/desktop/src/lib/components/AboutPanel.svelte`
- `apps/desktop/src/lib/components/AccountSetup.svelte`
- `apps/desktop/src/lib/components/ContactsSettings.svelte`
- `apps/desktop/src/lib/components/DataSettings.svelte`
- `apps/desktop/src/lib/components/AudioDiagnostics.svelte`
- `tests/ui/settings-redesign.test.ts`
- `tests/ui/browser/settings.mjs`
- `docs/verification/2026-11-05-settings-impeccable-audit.md` — resolution link added; original audit evidence retained.
- `docs/verification/2026-11-05-settings-audit-fixes.md` — this verification report.

## Limits

No SIP, RTP or native audio logic changed, so Asterisk and the long native SIP
suite were not rerun for this UI patch. Their earlier evidence remains in the
settings-redesign verification report. No real OS notification, clipboard,
permission prompt, production call, keychain write or physical audio capture was
performed. Actual VoiceOver, Windows runtime, WebKit-specific rendering and
hosted CI remain unverified. The local app is ad-hoc signed without notarization;
it was built, not installed. The original checkout and installed version were
not modified, and no commit, merge or push was performed.
