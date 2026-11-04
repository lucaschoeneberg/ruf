# Zed design throughout the app

User explicitly requests the locally observed Zed design across the entire app,
with its compact density, and permits inspecting local Zed. Local reference:
Zed Settings window900×750, GitHub Dark Tritanopia, IBM Plex Sans/.ZedSans UI.
Official Zed appearance/theme/density sources and local theme tokens inspected.

Implement inline in existing checkout; retain Svelte/Rust/Tauri APIs and all
functional behavior. All existing working-tree changes belong to preceding tasks.

- Unify phone and settings under shared GitHub-dark surfaces, blue accents, thin
  borders,4px controls and compact typography. Bundle unmodified upstream IBM
  Plex Sans regular/semibold with OFL notice; no runtime dependencies.
- Reduce phone default to340×500 (minimum320×440); use horizontal compact tabs,
  smaller header/buttons, flatter keypad, compact call/onboarding/empty states.
  Keep separate settings window and real account/sound/call states.
- Use22–28px navigation/controls where appropriate to desktop,44px dial keys,
  13–14px body/settings labels, tighter setting rows and consistent status/error
  colors. Eliminate divergent mint/blue palettes and decorative large radii.
- Verify browser fixtures across phone min/default, settings default/min,
  onboarding,contacts/history,incoming/active/compact calls, errors and sound.
  Confirm actions/focus remain reachable; maintain appropriate contrast.
- Update DESIGN/sidecar/product/development/licensing evidence; preserve upstream
  licenses and cite research. Run existing tests/typecheck/build/format/codegen
  and native bundle; no protocol/media change or new SIP behavior.
- Independent bounded visual/code review, fix listed material findings and stop.

Completed: local/online research, shared app styling/font licenses, smaller native
window, complete browser matrix, isolated native capture, full visual ship
review, design documentation replacement and all required checks. Native
production bundle restored; API/DTO/protocol behavior unchanged.
