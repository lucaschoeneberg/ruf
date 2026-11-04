---
name: Ruf Desktop
description: Compact native softphone in the shared Zed GitHub Dark Tritanopia visual world.
colors:
  surface: "#0d1117"
  surface-panel: "#010409"
  surface-raised: "#212830"
  surface-hover: "#262e38"
  surface-selected: "#1a2737"
  border: "#30363d"
  border-strong: "#3d444d"
  text: "#f0f6fc"
  text-secondary: "#c9d1d9"
  muted: "#9198a1"
  disabled: "#8b949e"
  accent: "#58a6ff"
  accent-fill: "#1f6feb"
  accent-hover: "#1a60cf"
  accent-active: "#174ea6"
  accent-ink: "#ffffff"
  info-surface: "#142235"
  danger: "#ffa198"
  danger-surface: "#2b191e"
  danger-hover: "#3d2025"
  danger-border: "#77413f"
  caution: "#e3b341"
typography:
  number:
    fontSize: "24px"
    fontWeight: 400
    lineHeight: 1.3
    letterSpacing: "-0.02em"
    fontFamily: '"SIP UI", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
  digit:
    fontSize: "22px"
    fontWeight: 400
    lineHeight: 1.15
    fontFamily: '"SIP UI", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
  call-peer:
    fontSize: "22px"
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: "-0.02em"
    fontFamily: '"SIP UI", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
  call-timer:
    fontSize: "20px"
    fontWeight: 400
    lineHeight: 1.4
    fontFeature: '"tnum"'
    fontFamily: '"SIP UI", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
  introduction:
    fontSize: "18px"
    fontWeight: 600
    lineHeight: 1.25
    letterSpacing: "-0.01em"
    fontFamily: '"SIP UI", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
  settings-title:
    fontSize: "17px"
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: "normal"
    fontFamily: '"SIP UI", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
  view-title:
    fontSize: "15px"
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: "normal"
    fontFamily: '"SIP UI", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
  label:
    fontSize: "14px"
    fontWeight: 600
    lineHeight: 1.4
    fontFamily: '"SIP UI", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
  body:
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.4
    fontFamily: '"SIP UI", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
  button:
    fontSize: "13px"
    fontWeight: 600
    lineHeight: 1.4
    fontFamily: '"SIP UI", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
  help:
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.4
    fontFamily: '"SIP UI", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
  caption:
    fontSize: "11px"
    fontWeight: 400
    lineHeight: 1.4
    fontFamily: '"SIP UI", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
  key-letters:
    fontSize: "10px"
    fontWeight: 400
    lineHeight: 1.3
    letterSpacing: "0.06em"
    fontFamily: '"SIP UI", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
rounded:
  control: "4px"
  preview-shell: "6px"
spacing:
  xs: "4px"
  small-gap: "6px"
  gap: "8px"
  control: "10px"
  group: "12px"
  phone-inset: "14px"
  section: "16px"
  settings-gap: "20px"
  settings-compact-inset: "24px"
  settings-inset: "32px"
components:
  button-primary:
    backgroundColor: "{colors.accent-fill}"
    textColor: "{colors.accent-ink}"
    typography: "{typography.button}"
    rounded: "{rounded.control}"
    padding: "5px 10px"
    height: "30px"
  button-secondary:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text-secondary}"
    typography: "{typography.button}"
    rounded: "{rounded.control}"
    padding: "5px 10px"
    height: "28px"
  button-danger:
    backgroundColor: "{colors.danger-surface}"
    textColor: "{colors.danger}"
    typography: "{typography.button}"
    rounded: "{rounded.control}"
    padding: "5px 10px"
    height: "30px"
  button-icon:
    backgroundColor: "transparent"
    textColor: "{colors.muted}"
    rounded: "{rounded.control}"
    padding: "5px"
    size: "28px"
  input-settings:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    typography: "{typography.body}"
    rounded: "{rounded.control}"
    padding: "3px 8px"
    height: "28px"
  button-settings-primary:
    backgroundColor: "{colors.accent-fill}"
    textColor: "{colors.accent-ink}"
    typography: "{typography.button}"
    rounded: "{rounded.control}"
    padding: "4px 10px"
    height: "28px"
  settings-navigation-selected:
    backgroundColor: "{colors.surface-hover}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "4px 8px"
    height: "28px"
  settings-account-selected:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text-secondary}"
    typography: "{typography.body}"
    rounded: "{rounded.control}"
    padding: "6px 8px"
  navigation-selected:
    backgroundColor: "{colors.surface-selected}"
    textColor: "{colors.accent}"
    typography: "{typography.help}"
    rounded: "{rounded.control}"
    padding: "4px 6px"
    height: "30px"
  dial-key:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "3px"
  settings-feedback:
    backgroundColor: "{colors.info-surface}"
    textColor: "{colors.text-secondary}"
    rounded: "{rounded.control}"
    padding: "9px 10px"
---

# Design System: Ruf Desktop

## Overview

**Creative North Star: "The compact Zed workspace"**

A compact native telephone with Zed’s dark workspace character across the whole application. The inspected local Zed settings use GitHub Dark Tritanopia: near-black panels, slate controls, fine dividers and blue actions. Phone, onboarding, calls, empty states, account settings and sound settings share this palette and the bundled IBM Plex Sans interface face.

Density serves operation: short horizontal navigation, flat keypad cells and restrained setting rows leave the current identity, real runtime state and available action easy to scan. Native window controls remain native. Settings open in their own window, with a persistent searchable sidebar and a scrolling document.

The Ruf brand mark combines a lowercase r with a handset shoulder. App, Dock and notification icons use the existing blue accent on a dark rounded tile with transparent outer margins. The 32px native status mark omits the tile: macOS uses a monochrome template, Windows the same blue silhouette. Small and large badges describe readiness and non-idle calls; tooltip text supplies the explicit state.

**Key Characteristics:**

- One shared palette and font across phone and settings.
- Compact 340 × 500px phone with a flat keypad and horizontal navigation.
- Blue actions and focus; explicit text accompanies registration and error states.
- Searchable settings sidebar, multiple account identities and thin setting rows.
- Honest capability, empty and unavailable states.

## Colors

The frontmatter records the shared `:root` values in `src/app.css`. Phone and
settings resolve the same tokens; settings have no local palette overrides.

### Primary

Blue `accent` marks focus, selected phone navigation and connected runtime state.
The stronger `accent-fill` carries primary actions with white `accent-ink`;
`accent-hover` and `accent-active` give the button states. `surface-selected`
marks a selected phone destination, while settings section selection uses the
quieter `surface-hover` and account selection uses `surface-raised` plus a border.

### Neutral

Near-black `surface-panel` separates the header, navigation and settings sidebar
from the dark `surface` canvas. `surface-raised` and `surface-hover` distinguish
controls and interaction. `border` divides regions; `border-strong` outlines
controls. `text`, `text-secondary`, `muted` and `disabled` establish hierarchy.
`info-surface` contains operation and unsaved-change feedback.

### State colors

`danger` identifies validation, failed registration and destructive actions.
`danger-surface`, `danger-hover` and `danger-border` support notices and call
termination. `caution` identifies explicit test-audio disclosures. State labels
remain readable independently of a colored dot.

**The State Before Decoration Rule.** Use accent and danger color to identify available actions, selection and real runtime state; selection alone never proves registration.

The sidecar's eight-step OKLCH ramps are synthesized swatch metadata derived
from these current colors, not additional application tokens.

## Typography

**Font:** `SIP UI` is the local CSS alias for unmodified IBM Plex Sans Regular
(400) and SemiBold (600), bundled under SIL OFL 1.1 in `public/fonts/`. The
existing platform stack remains the fallback. `font-synthesis: none` prevents
synthetic faces; there is no icon font.

Body and fields use 13px with a 1.4 base line height. Setting labels use 14px/600;
section navigation uses 14px/400. Settings titles are 17px/600, collection titles
15px/600 and first-run titles 18px/600. Number entry is 24px/400, keypad digits
22px/400 and letter annotations 10px with 0.06em tracking. Phone identity and the
outgoing-account picker use 14px/600. Helpers are 12px, captions 11px. Descriptions
use 1.4 line height; feedback and longer introduction copy use 1.45. Setting
descriptions are bounded to 60ch, empty-state copy to 38ch.

Call peers use 22px/600; compact call strips reduce them to 14px. The active
20px timer uses tabular numerals. Empty states are left-aligned.

## Layout

The native phone starts at 340 × 500px and has a 320 × 440px minimum. A fixed
header, flexible scrolling content and persistent three-item navigation form
one vertical stack. Navigation pairs icons and labels horizontally. The dialer
uses 14px side insets and a three-column, four-row keypad with 4px gaps and
44px minimum rows. At viewport heights up to 450px, rows reduce to 40px and
padding compresses while the content remains scrollable.

Native windows always fill their available content area, including when enlarged.
Only the browser preview receives the centered 340 × 500px frame. The runtime
`native-document` marker keeps that preview decoration out of macOS/Windows windows.

The unframed browser shell fills its viewport, capped at 420px wide and 680px
high. From 500px viewport width the browser preview centers a 340 × 500px shell
inside 24px padding, with height bounded by the viewport. This preview frame is
not a custom native title bar.

Settings open at 900 × 660px, minimum 740 × 540px, filling their own viewport.
A 222px sidebar holds search, section links, the scrollable account list and a
bottom version footer. A fixed 44px minimum breadcrumb sits above the scrolling
main document. Main insets are 32px. Setting rows pair descriptions and
180–230px controls across a 20px gap, with 16px vertical padding and thin rules.
Fields, search and settings buttons have a 28px minimum height.

At widths up to 820px the sidebar is 200px, main insets 24px and controls
160–190px. At up to 620px the browser fallback has a 160px sidebar, 18px insets
and stacked row controls and actions. Native settings cannot shrink to this
fallback width. Compact call panels sit above contacts or history so the
current supported call actions remain reachable.

Text selection, carets and scrollbars share the global palette across phone and
settings. Use the actual bundled 400/600 font faces; do not introduce intermediate
weights for the same role. Disabled controls retain their disabled appearance on hover.

Settings drafts survive failed saves and incoming-call transitions. Native window
close and app Quit reuse the same inline discard panel as section navigation;
explicit Quit first reveals minimized settings. Loading one settings subsystem must
not disable unrelated sections. A failed account switch keeps the authoritative
outgoing identity visible, and unavailable phone state always offers a retry path.

## Elevation & Depth

Depth comes from tonal layers and one-pixel borders. There are no box shadows,
gradients or floating cards. Feedback receives a tonal container; sidebar and
setting rows remain flat.

## Shapes

Actions, fields, navigation and keypad cells share 4px corners. The browser
preview frame alone has 6px corners. Status dots are circular; dividers and
outlined flat keys preserve the workspace character.

## Components

### Buttons and keyboard feedback

Primary buttons use blue fill, white labels and darker hover/pressed states.
Their usual minimum height is 30px, with 5px 10px padding; the full-width call
action is at least 34px. Settings buttons use 28px minimum height and 4px 10px
padding. Secondary buttons use raised slate, secondary text and a strong border;
destructive buttons use the danger surface and border. Icon actions are 28px
squares. Disabled actions expose actual capability or busy state.

Shared focus is a 2px blue outline with 3px offset. Color transitions use
140ms ease and disappear with reduced-motion preferences. Search and number
entry use focused border treatments instead of a second nested outline.

### Inputs, keypad and navigation

Number entry is centered and borderless above an underline; backspace sits
alongside. Its hint explains invalid characters. Settings fields are raised,
outlined and paired with visible labels and descriptions. Errors appear below
the relevant field, with focus moved to the first invalid control. Sidebar
search is transparent inside an outlined control and uses a blue focused border.

Twelve flat outlined keys retain telephone order; hover raises the surface and
pressing selects it. Holding zero enters plus. These enter numbers and do not
implement in-call DTMF. Tastatur, Kontakte and Verlauf receive equal navigation
space, horizontal icon/label pairs and `aria-current` for the selected page.

### First run, accounts and errors

The first-run introduction prioritizes the user's SIP account, can be skipped
and reopened, and stores only a non-sensitive dismissal flag. Configured
sessions are not interrupted. Setup opens settings with empty account fields.
Up to eight accounts persist; passwords stay in the OS credential store and are
never returned to the form. A blank password preserves a saved credential;
new accounts require one. Saving starts registration and does not prove success.
Dots and labels follow native pending, connected, failed and removing snapshots.
Native errors receive actionable German copy.

Settings search filters section labels and account identities. Cmd/Ctrl+F focuses
it; Escape and a labeled clear action clear it. Section/account buttons expose
`aria-current`. Account names truncate above their SIP identity. The outgoing
account picker is independent of the account being edited. The plus action
opens an empty form and disables at eight accounts.

Unsaved account or sound edits reveal an inline continue/discard choice before
navigation; the notice scrolls into view and focuses continue editing. Removal
requires inline confirmation and deletes the stored password. Account mutations
are disabled during calls or busy operations. Browser and portable modes explain
unavailable native operations. About retains storage details, SIP/UDP limits
and the collapsed local Asterisk guide.

### Sound

Ton uses the same setting rows for Mikrofon, Lautsprecher and independent
Klingelton-Ausgabe. Systemstandard follows the OS. Refresh, missing devices and
ambiguous duplicate names receive explicit feedback; an unavailable saved device
stays visible until replaced. Ringtone enablement uses a native 16px checkbox
with blue accent and an adjacent Ein/Aus label. Disabling ringing preserves the
incoming-call display.

Klingelton anhören explicitly previews two seconds on the selected output without
saving, changes to Hörprobe stoppen while native playback is reported, and has
a live status. Ton speichern requires unsaved changes and applies to all SIP
accounts. Device changes and preview are disabled during calls, operations or
unavailable native support. Audio für Entwicklung remains initially collapsed:
it selects the shared source before the first account, then shows and locks the
engine snapshot's `audioMode` for the session. Test-tone mode discloses its
440-Hz source, received-audio recording and suppressed automatic ringing.

### Calls, containers and empty states

The dialer call panel centers state, peer, active duration and supported actions.
Other views show its compact strip without the icon or timer. Contacts and
history keep left-aligned icons, concise missing-data copy and a return-to-keypad
action. No fabricated rows or unsupported call actions appear.

The actual local Zed reference is repository-root
`.impeccable/review/zed-local-reference.png` (900 × 750px). The `zed-*` app review
captures use synthetic IPC fixtures to inspect layout and states; they are not
evidence that native SIP registration, calls or audio succeeded. Font assets
come from `zed-industries/zed` commit
`e21ff1b368b074586e4dae0d9bbf15d5dd8a1476`; their license ships with the files.

## Do's and Don'ts

### Do:

- **Do** reuse the shared root palette for every application surface.
- **Do** keep dialing and current call actions reachable at the native minimum size.
- **Do** use German labels, visible keyboard focus and reduced-motion behavior.
- **Do** preserve real runtime availability, explicit save actions and unsaved-change protection.

### Don't:

- **Don't** introduce a separate settings palette or font stack.
- **Don't** populate contacts or history before their data exists.
- **Don't** add unsupported call controls such as mute, hold or transfer.
- **Don't** replace native window controls or expand the utility into a dashboard.
