# Ruf branding implementation plan

**Goal:** Apply the approved Ruf name and blue r/handset icon to the desktop app,
notifications and status menu. Use `org.rufapp.desktop` as the new app identifier.

**Approach:** Reuse Tauri bundle icons, native notification identity, existing tray
state and Svelte views. Create a clean SVG rendition of the approved icon and
generate platform assets with the existing Tauri CLI. Use a transparent monochrome
tray mark with the existing ready/busy indicators. Preserve the legacy data and
credential namespaces explicitly so existing accounts and favorites remain usable.
SIP protocol terminology, Cargo package names and IPC contracts stay unchanged.

- [x] Rename visible strings, bundle identity, install paths and current docs.
- [x] Add SVG masters and generated desktop/tray assets; wire native and web icons.
- [x] Check existing lifecycle tests and installation paths; verify tray states.
- [x] Run format, versions, UI tests, typecheck, build, generated-contract stability,
      workspace Clippy/tests and native desktop bundle checks.
- [x] Review the patch against the pre-existing working tree and record limits.

**Review focus:** Existing account access after identity change; release install
recognition for login start; readable template icons in macOS light/dark menus;
passive notifications retaining one-per-call behavior and bundle icon identity.
OS consent may need renewal under the new ID. Existing login items for SIP are
separate OS registrations and may need removal when installing Ruf.
