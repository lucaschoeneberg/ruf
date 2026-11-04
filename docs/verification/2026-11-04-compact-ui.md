# Compact softphone verification — 2026-11-04

The user confirmed a dark, compact first iteration around 360 × 560 pixels,
with real contacts/history navigation and honest empty states until their data
connectors exist. The supplied classic native softphone image sets density and
familiarity; its names/numbers are not application data.

## Implementation and scope

The existing Svelte 5/Tauri application now has an accessible dialpad, number
editing, separate account settings, three navigation destinations and persistent
call controls on other views. Native commands/generated DTOs are unchanged. No
runtime dependencies, fake contacts/history data or unsupported call controls
were added. The old warning-marker detector exception was removed with that CSS.

The native window uses dark OS chrome, defaults to 360 × 560 and permits
320 × 500. The web preview explicitly identifies itself and keeps telephony
unavailable. Account passwords clear on submission/navigation. Numeric input
keeps the native 64-character limit; there is no DTMF transmission API.

## Observed checks

- Bun helper tests: expected missing-module failure first, then four passing
  tests for caret insertion/selection replacement, deletion and length/key limits.
- Svelte/TypeScript: zero errors or warnings. Frontend build and formatting pass.
- Isolated Chromium UI exercise: caret editing, backspace, long-press 0 → plus,
  invalid input, all navigation destinations, disabled browser registration,
  Escape/back, 320px layout, and no horizontal overflow. Stable shell width
  across routes is explicitly checked.
- Mock-IPC UI fixtures cover outgoing/active/incoming/error states and visible
  answer/hangup while contacts is selected. These prove UI behavior only, not SIP.
- The first visual pass found root-container shrink-to-fit and a narrow-height
  keypad overlap. Both were fixed; the final screenshots and repeat checks pass.
- Captured the real native macOS window at measured bounds 360 × 560; all idle
  keypad/call/navigation controls fit. This is an actual app capture, not a mockup.
- Native macOS app packaging succeeds. This host emitted nonfatal rust-objcopy
  debug-info stripping warnings and the expected ad-hoc/notarization warning;
  the resulting app launches. Windows/hosted execution remains unverified.
- Real Asterisk regression passes: authenticated registration, echo call,
  4.04 seconds of received audio, RMS approximately 5811, approximately 439.8 Hz;
  stale actions also preserve a replacement real call. The lab is stopped after
  the check. This is separate from browser fixtures; hardware/incoming SIP
  interoperability still requires its existing manual/platform tests.
- Native workspace Clippy/tests, lockfile installation, toolchain/version and
  generated-contract drift checks, plus GitHub Actions actionlint pass.

## Independent review

A fresh code/visual reviewer inspected the supplied reference and nine required
screenshots: compact 360 × 532 client area, narrow 320 × 472, desktop preview,
contacts, history, browser settings and active/incoming/error fixtures. The
reviewer read the actual code/contracts and independently ran the four Bun tests,
Svelte/TypeScript and whitespace checks. Disposition: **ship**, no material fixes.
Native packaging/audio were verified by the implementer, not that reviewer.

Design documentation includes actual tokens in `apps/desktop/DESIGN.md` and a v2
sidecar in `apps/desktop/.impeccable/design.json`. YAML/JSON/token references were
validated; the installed Impeccable CLI has no formal sidecar schema validator.
Generated tonal-ramp previews are explicitly metadata, not additional UI colors.

Screenshots and diagnostic logs stay in ignored `.impeccable/review/` and
`native/build/`. The front end stores no new contacts, credentials or history.
