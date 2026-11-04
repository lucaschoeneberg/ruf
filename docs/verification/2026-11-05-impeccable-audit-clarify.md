# Ruf application audit corrections and interface clarification

Verified locally on macOS arm64 on 2026-11-05. The pre-existing Ruf, contacts,
history, notification and SIP-event work was first committed as `f8c07ba`.
Audit corrections and Settings ordering were then committed as `dc0b449`.

## Audit corrections

- Editable fields and search controls reuse a semantic `border-control` token.
  Computed sRGB contrast is 3.24:1 against the raised field fill, 4.12:1 against
  the main surface and 4.47:1 against the panel surface. Decorative dividers keep
  their existing colors.
- History refresh preserves the loaded window, open details, keyboard focus and
  reading position. A 130-entry browser regression exercises multiple 50-entry
  requests without exceeding the API's 100-entry per-request limit. Failed refresh
  retries preserve the same window. Unchanged contact scopes no longer reset on
  focus.
- History search waits for a 200 ms typing pause, retains previous results and
  invalidates obsolete replies immediately. Typing “Testanruf” produces one
  search request in the browser fixture.
- Continuing an unsaved draft restores its last focused field. Escape follows
  the same path, including while Settings search is populated.
- The dialer associates its correction hint with the number input and announces
  changes politely. The About section describes the implemented local history.
- Settings reuses the phone window's native snapshot revision ordering. Older
  events cannot replace newer state or suppress a newer read/save response;
  obsolete read failures do not overwrite current call state.

Browser regressions were observed failing before the relevant fixes. A faulty
assertion on an input's own `disabled` property was corrected to `:disabled`,
which includes the existing parent fieldset's disabled state.

## Clarify pass

The German interface now distinguishes early connection progress, remote hold,
received keypad signals and automatic redirect policy. Destructive actions name
the call or history being deleted. Empty history describes call records rather
than suggesting audio recording. Contact actions accurately name the macOS app
they launch. Device errors avoid asserting an unverified disconnection. New-account
requirements are explicit; the Name input matches its existing required validation.
Shared terminology is recorded in `apps/desktop/PRODUCT.md`.

The audit and clarification reuse Svelte state/effects, existing Tauri wrappers,
canonical generated DTOs, the shared palette and existing Playwright fixture
runner. They add no dependency and change no IPC command or DTO. The concurrently
completed SIP contract documentation is in `docs/api/sip-events.md`.

## Observed checks

- `bun run format:check`, `bun run check:versions`, `bun run check`: passed.
- `bun run build`: passed; JavaScript bundle approximately 49.95 kB gzip.
- `bun run test:ui`: 33 passed.
- `bun run test:ui:flows`: contacts, history, phone, settings and layout passed.
- `bun run types:generate`: passed and byte-identical to the saved generated file.
- Portable workspace Clippy with warnings denied and Cargo tests: passed.
- Native workspace Clippy with warnings denied and Cargo tests: passed. The
  adapter's 41 tests passed; five explicit Asterisk tests were separately run by
  the smoke runner. OS/keychain opt-in tests remained ignored.
- `bun run native:build`: passed before native checks; both local SDK patches
  were applied to isolated copies of the pinned sources.
- `bun run test:asterisk`: passed with real synthetic RTP echo and all five PBX
  interop tests. This is audio evidence, separate from browser fixtures.
- `bun run desktop:build`: portable app and DMG build passed.
- `bun run desktop:build --features native`: native app and DMG build passed.
- Impeccable detector: no deterministic findings.
- Axe on 15 synthetic browser views: no automatically confirmed violations;
  manual-review items were not counted as automatic passes. Screens included
  phone minimum 320 × 440, settings minimum 740 × 540, new-account setup, call
  states and a separate 200% text-size simulation. Standard-size captures also
  come from the repository browser suite. No horizontal document overflow was
  observed. Captures/data remain ignored under `output/playwright/`.
- Independent reviews found and resolved the scoped-history and Escape edge
  cases; final code/copy/contract review found no blocker.

## Limits

These checks do not establish hosted CI, Windows runtime, VoiceOver behavior,
installed notification activation, personal-provider calling or physical headset
audio. The 200% simulation enlarges computed text sizes; it is not an OS zoom test.
The macOS build is ad-hoc signed and not notarized. The local SDK build emitted
linker warnings about Homebrew OpenSSL objects built for macOS 27 while the app's
deployment target is macOS 14; compatibility with the oldest supported macOS
version was not executed here. No build output, personal data or credentials are
included in these commits.
