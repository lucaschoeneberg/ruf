---
name: SIP Desktop
description: Compact dark native softphone with a familiar keypad and explicit runtime state.
colors:
  surface: "#191c20"
  surface-raised: "#24282e"
  surface-hover: "#30363d"
  border: "#343a42"
  text: "#f0f1f3"
  muted: "#a9b0bb"
  accent: "#a0dec0"
  accent-ink: "#123526"
  danger: "#eeafb4"
  caution: "#eacb99"
  error-surface: "#42292d"
  error-copy: "#ffd1d4"
  field-border: "#444c56"
  secondary-text: "#e2e7ec"
  nav-selected: "#303c37"
  nav-selected-text: "#b9e4cf"
  danger-surface: "#5e2e34"
  danger-text: "#ffd8dc"
typography:
  number:
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "27px"
    fontWeight: 450
    lineHeight: 1.3
    letterSpacing: "-0.025em"
  digit:
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "24px"
    fontWeight: 450
    lineHeight: 1.15
  title:
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "17px"
    fontWeight: 650
    lineHeight: 1.45
    letterSpacing: "-0.02em"
  introduction:
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "20px"
    fontWeight: 600
    lineHeight: 1.25
    letterSpacing: "-0.025em"
  body:
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.45
  button:
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "13px"
    fontWeight: 650
    lineHeight: 1.45
  label:
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "12px"
    fontWeight: 550
    lineHeight: 1.45
rounded:
  field: "6px"
  icon: "7px"
  control: "8px"
  key: "9px"
  preview-shell: "12px"
spacing:
  xs: "4px"
  small-gap: "6px"
  gap: "8px"
  control: "10px"
  group: "12px"
  section: "16px"
  inset: "20px"
  dial-inset: "22px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.accent-ink}"
    typography: "{typography.button}"
    rounded: "{rounded.control}"
    padding: "10px 16px"
  button-secondary:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.secondary-text}"
    rounded: "{rounded.control}"
    padding: "10px 16px"
  button-danger:
    backgroundColor: "{colors.danger-surface}"
    textColor: "{colors.danger-text}"
    typography: "{typography.button}"
    rounded: "{rounded.control}"
    padding: "10px 16px"
  button-icon:
    backgroundColor: "transparent"
    textColor: "{colors.muted}"
    rounded: "{rounded.icon}"
    padding: "7px"
    size: "34px"
  input-settings:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    typography: "{typography.body}"
    rounded: "{rounded.field}"
    padding: "7px 10px"
  navigation-selected:
    backgroundColor: "{colors.nav-selected}"
    textColor: "{colors.nav-selected-text}"
    typography: "{typography.body}"
    rounded: "{rounded.control}"
    padding: "5px 8px"
  dial-key:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    rounded: "{rounded.key}"
    padding: "3px"
  inline-note:
    backgroundColor: "{colors.surface-raised}"
    typography: "{typography.body}"
    rounded: "{rounded.icon}"
    padding: "10px"
---

# Design System: SIP Desktop

## Overview

**Creative North Star: "The compact native telephone"**

A compact, dark softphone shaped around the familiar telephone keypad. Charcoal surfaces, system typography and a restrained mint accent keep the number, registration state and available call action easy to scan in a small desktop window.

The system uses tonal layers and fine dividers rather than ornamental depth. German labels accompany the navigation and call actions; settings stay behind their own action, while the current call remains reachable when another view is open.

**Key Characteristics:**

- Compact keypad and persistent three-item bottom navigation.
- System typography with larger numbers and quiet supporting labels.
- Charcoal tonal layers with mint for primary actions and available state.
- Explicit empty and unavailable states grounded in actual capabilities.

## Colors

The palette combines cool charcoal neutrals with a pale mint primary accent.
The frontmatter records the actual reused values from `src/app.css`. The sidecar
adds synthesized tonal ramps for swatch previews; those ramps are documentation
metadata, not additional application color tokens.

### Primary

- **Mint:** primary call and registration actions, available registration state,
  keyboard focus and the focused number-entry underline.
- **Deep green ink:** text on mint primary actions.

### Neutral

- **Charcoal:** the application surface; **raised charcoal** distinguishes keys,
  form fields and informational notes; **hover charcoal** responds to interaction.
- **Soft white:** primary labels and entered numbers; **cool gray:** supporting
  explanations, inactive navigation and registration information.
- **Slate borders:** structural dividers; the lighter field border outlines
  secondary controls and settings fields.
- **Muted green selection:** background and text of the current navigation item.

### State colors

- **Pale rose:** failed registration status.
- **Dark rose with pale rose text:** the hangup or reject action. Error notices
  use a related rose surface and remain distinct from normal content.

**The State Before Decoration Rule.** Use accent and danger color to identify
available actions, selection and real runtime state.

## Typography

**Font:** the platform system stack recorded in the frontmatter. There is no
bundled display font or icon font.

Numbers carry the largest hierarchy: the centered number entry uses the number
role, keypad digits use the digit role, and view headings use the title role.
Most navigation, explanatory copy, form fields and runtime states use the compact
body size. Primary action labels are heavier and slightly larger than supporting
copy. Keypad letters are small uppercase annotations with open tracking.

The active call peer uses a larger heading; the compact call panel reduces it
when another view is open. The call timer uses tabular numerals. Empty-state copy
is centered and constrained to a short measure (28ch).

## Layout

The desktop window starts at approximately 360 × 560 pixels, with native window
controls and a dark OS theme. The content is a single vertical stack: account
identity and availability, flexible view content, then a persistent three-item
bottom navigation. Settings are opened separately from the header.

At narrow viewport widths, the shell fills the available width and viewport
height, capped at 420px wide and 680px tall. At viewport widths from 500px, the
browser preview centers a 360 × 560px shell with a thin border and rounded corners,
inside a darker surround. Its height remains bounded by the viewport.

The keypad is a three-column, four-row grid with consistent gaps. View content
scrolls within the shell; the header and navigation do not shrink. Below a 510px
viewport height, header and dialer padding, keypad gaps and navigation height
compress. The compact call panel sits above contacts, history or settings so its
answer/reject or hangup actions remain accessible.

## Elevation & Depth

Depth comes from tonal layering and thin borders. The implemented system has no
box shadows, gradients or floating cards. The browser preview shell is a frame;
it is not a custom native title bar. Settings notes use a raised tonal container.

## Shapes

Controls have modest rounded corners: settings fields are the tightest, icon
buttons and notes slightly softer, action buttons and navigation slightly wider,
and keypad keys slightly rounder. The desktop preview alone has the larger shell
radius. Status dots are circular. Dividers and outlined fields preserve the
compact utility character.

## Components

### Buttons

Mint primary actions use dark ink, with lighter hover and darker pressed states.
Unavailable primary actions use a muted charcoal/gray-green treatment. Secondary
actions use a raised surface and outlined border. Hangup and reject actions use
the dark rose variant. Icon buttons are compact squares with transparent default
backgrounds and a raised hover/selected treatment. Shared keyboard focus uses a
mint outline (2px with 3px offset).

Color and border transitions use a short easing (140ms ease); reduced-motion
preferences remove transitions and animations.

### First run and connection feedback

The first-run introduction prioritizes the user's own SIP account. It can be
skipped and reopened from the header's info action; only a non-sensitive dismissal
flag is stored. Existing configured sessions do not get interrupted by the intro.
The introduction uses the compact introduction type role, not a hero layout.

Account fields start empty and use the normal microphone/speaker source. Inline
field help identifies the SIP server, authentication username and session-only
password. Test audio and the local Asterisk guide are progressively disclosed.
The actual SIP/UDP development limitation stays visible before connecting.

A queued registration is pending. Focus and scroll reveal pending/failed status
without interrupting a user editing a field; confirmed native registration leads
to the keypad with a short first-call hint. Native failures receive actionable
German copy. Contacts/history do not claim their unavailable data is merely empty.

Restored native sessions whose audio source cannot be read are explicitly
unknown: possible test recording is disclosed, new calls/answer/reconfiguration
are blocked until an app restart, and hangup/reject remain available. The draft's
microphone default is never treated as evidence of a restored engine's mode.

### Inputs / Fields

The number input is centered, borderless and large; focus is shown by an underline
on its containing row. A compact backspace action sits alongside it. Settings
fields use raised surfaces, thin slate borders and visible labels above them.
The number hint changes to describe allowed characters when input is invalid.

### Navigation

The bottom navigation gives equal space to Tastatur, Kontakte and Verlauf. Each
item combines an inline line icon with a label. Hover raises its surface; the
current item uses the muted green selection treatment and `aria-current`.

### Keypad

Twelve raised keys form the familiar telephone arrangement. Digits sit above
letter annotations; symbol keys use slightly larger digits. Hover changes both
surface and border; pressing deepens the green-gray surface. Holding zero enters
plus. These are number-entry keys, not an implemented in-call DTMF control.

### Call panel

The dialer view centers the call state, peer, duration when active, and supported
actions. On other views, the same component becomes a compact strip; its icon and
timer disappear while the peer, state and call actions remain available.

### Containers and empty states

A raised inline note explains unavailable native features in settings. Contacts
and history use centered line icons, concise headings, honest missing-data copy
and a secondary action returning to the keypad. They contain no fabricated rows.

## Do's and Don'ts

### Do:

- **Do** keep dialing and the current call action prominent in the compact window.
- **Do** use German text labels alongside navigation icons and call actions.
- **Do** preserve visible keyboard focus and reduced-motion behavior.
- **Do** show real runtime availability and honest empty states.

### Don't:

- **Don't** turn contacts or history into populated lists before their data exists.
- **Don't** add unsupported call controls such as mute, hold or transfer.
- **Don't** replace native window controls with a decorative custom title bar.
- **Don't** expand the compact utility into a dashboard layout.
