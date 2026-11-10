# Ruf desktop design

The phone and settings share a compact dark interface inspired by Zed's GitHub
Dark Tritanopia theme. Use flat tonal layers, fine dividers, restrained blue
actions and the bundled IBM Plex Sans. Preserve native window controls.

## Maintained sources

- [src/app.css](src/app.css) owns shared color, spacing, radius and focus tokens.
  Component styles own their local layout. Do not maintain a generated duplicate
  of these values or a separate settings palette.
- [Font assets](public/fonts) supply the real Regular (400) and SemiBold (600)
  faces, using the CSS alias `SIP UI` with platform fallbacks. Do not synthesize
  intermediate weights or fetch fonts from a third-party service.
- [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md) preserves the theme/font
  upstream sources, revision and licenses. The earlier visual reference was
  inspected locally; local screenshots and tool-generated swatch ramps are not
  application dependencies or proof of native SIP/audio behavior.

## Layout and hierarchy

The phone opens at 340 × 500 pixels, minimum 320 × 440. Keep header, scrolling
content and the three destinations Dialer/Contacts/History usable at that
minimum. Contacts/history retain reachable compact call controls during calls.
The centered browser preview frame is separate from native-window layout.

Settings open at 900 × 660 pixels, minimum 740 × 540, with a searchable sidebar,
direct account list and scrolling content. Keep narrow/magnified layouts
scrollable without hiding fields behind the save bar. The eight sections are
General, SIP accounts, Audio, Notifications, Contacts & History, Privacy,
Diagnostics & Help and About Ruf. These English names describe the current
German interface.

Use the shared 4px control radius and 13px body text. Tonal surfaces and 1px
borders provide hierarchy. Avoid decorative gradients, shadows and dashboard
cards. Icons accompany clear labels; color alone never carries state.

## Interaction and accessibility

- Use concise German labels, accessible account-specific action names, semantic
  heading order, visible focus and the existing reduced-motion behavior.
- Keep number entry and in-call DTMF separate. Search results open details;
  calling remains an explicit action. Shortcuts/native menus reuse guarded
  controls; plain letters remain available for typing.
- Preserve unsaved account, sound and preference drafts. Navigation, closing
  Settings and Quit use the existing continue/discard flow. Failures remain
  recoverable; loading one settings area must not block unrelated areas.
- Distinguish selected accounts, actual registration, queued operations and
  confirmed call/security outcomes. Keep mute/hangup reachable during supported
  pending states. Show unavailable/empty/error states without invented data.
- Keep account management and security in SIP accounts, contact permissions in
  Privacy, display/history policy in Contacts & History, and technical
  checks in Diagnostics & Help. Test notifications remain anonymous and separate
  from real call actions.
- Keep the existing Ruf icon and monochrome macOS tray template. Native icon
  assets and their regeneration commands are documented in
  [development](../../docs/development.md#ruf-icon-assets).

Feature scope is maintained in [PRODUCT.md](PRODUCT.md); detailed keyboard and
search behavior is in the [API reference](../../docs/api/keyboard-navigation.md).
