# Zed design across the app

## Reference and implementation

The user requested the clearly recognizable compact Zed design throughout the
entire application and explicitly authorized local Zed inspection. Local Zed's
native Open Settings menu opened its actual900×750 settings window; this was
captured as `.impeccable/review/zed-local-reference.png`. No settings values or
project files were edited. The local theme selection is GitHub Dark Tritanopia
from GitHub Zed Themes0.1.9. Palette roles were read from the installed theme.

Sources consulted:

- [Zed Appearance](https://zed.dev/docs/appearance), including interface fonts and visual controls.
- [Zed Themes](https://zed.dev/docs/themes), including theme roles and overrides.
- [Zed UI-density source](https://github.com/zed-industries/zed/blob/main/crates/settings_content/src/theme.rs).
- [GitHub Zed Themes](https://github.com/PyaeSoneAungRgn/github-zed-theme), the installed community theme's source and MIT license.
- [Pinned Zed font assets](https://github.com/zed-industries/zed/tree/e21ff1b368b074586e4dae0d9bbf15d5dd8a1476/assets/fonts/ibm-plex-sans).

The implementation reuses Svelte components, SVG icons and existing Tauri/Rust
contracts. It replaces the split mint-phone/blue-settings palette with shared
root colors, IBM Plex Sans, thin separators,4px controls, compact setting rows
and horizontal phone navigation. The phone defaults to340×500, minimum320×440.
Settings remain a separate native window. Call/error/onboarding/empty states
use the same visual system. No SIP, audio, IPC or DTO behavior changed.

Unmodified Regular/SemiBold TTF files and their SIL OFL1.1 notice are bundled
under public/fonts. Font source commit:
`e21ff1b368b074586e4dae0d9bbf15d5dd8a1476`. SHA-256:

- Regular: `975dcda37d80f038dcd143c22e33ca2d97a0cc5a929aace1c749153b0fe1afa5`
- SemiBold: `a20caf8286023a6a7a85e40b1d2a4ae9fc3e3b1f9eda8f4c542dd4986af67bb1`

The MIT theme notice is bundled through existing license resources. Own code
remains MIT; no runtime dependencies were added.

## Visual and interaction evidence

`native/build/test-zed-ui.mjs` uses synthetic IPC fixtures, not real SIP proof.
It checked font loading, navigation, account selection, call actions, visible
primary actions, lack of horizontal overflow and all supplied state captures:

- Phone340×468 and minimum320×408 client areas.
- Settings900×638 and minimum740×518 client areas, including scrolled actions.
- Contacts/history empty states and first-run/returning onboarding.
- Incoming/active/compact calls, global error and account-field validation.
- Sound settings and its scrolled action area.

All fixtures passed without browser exceptions. The actual bundled font loaded.
The independent visual review compared these captures with the actual local
Zed reference, found no material defects/fidelity blockers, and returned ship
at the full visual scope. The minimum phone's help line may scroll below the
fold; keypad, call actions and navigation stay visible.

A native macOS build with a temporary isolated preview identifier was launched
for capture, avoiding the user's saved SIP accounts and keychain. Its actual
window measured340×500; `native/build/zed-native-phone.png` confirms the native
onboarding presentation and bundled typography. Only the capture PID was
terminated. The distributable application is rebuilt with the normal identifier.

## Verification and limits

Passed: Bun format/version checks,11 UI unit tests (45 assertions), zero-error/
zero-warning Svelte and TypeScript checks, frontend build, portable/native
workspace Clippy (-D warnings) and Rust tests, generated-type stability, native
macOS app bundle and browser state matrix. The production bundle was rebuilt
with org.sipsoftphone.desktop and native linkage markers confirmed.

Token-bearing DESIGN.md and its sidecar now match the approved shared world,
replacing obsolete mint/settings metadata. A single mechanical detector pass
reported two advisory catalog mismatches and no non-advisory findings: the
20px duration role was added to typography, and the legacy3px summary-focus
radius now uses the shared4px token. No ignores/suppressions were added.
Mechanical output and validation logs remain ignored local artifacts.

No new SIP/media execution is claimed for this visual change; prior Asterisk and
sound verification records remain separate. Windows presentation and hosted CI
were not executed here. Native window chrome remains the operating system's,
with no imitation traffic-light controls or editor features added to the phone.
