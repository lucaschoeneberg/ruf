# Ruf branding verification

Storage compatibility described below was subsequently removed at the user’s
request. See [legacy cleanup](2026-11-04-legacy-sip-cleanup.md).

## Result

The app is named Ruf, with bundle identifier `org.rufapp.desktop`. The approved
blue r/handset concept is redrawn as a clean SVG with transparent outer margins.
The existing Tauri CLI produced PNG, ICNS and ICO assets. App windows, settings,
menus, permission explanations, notification sender identity and Windows native
notification registration use Ruf. SIP remains the protocol name.

The native status icon reuses the r/handset silhouette without the dark tile.
macOS applies template tint; Windows uses the existing blue. Ready accounts have
a small badge and non-idle calls a larger badge. No ready account/call means no
badge. Native call actions, state tooltips and menu dispatch are reused.

The new bundle ID retains the legacy application-data directory and credential
service for existing accounts, favorites and settings. Other preview IDs remain
isolated. New OS consent and login registration can be needed. Quit the old SIP
app before starting Ruf because their compatible account storage is shared.

No new dependency or IPC/DTO schema change was introduced by branding. Reused:
Tauri platform icons, include_image, notification bundle identity, existing
tray lifecycle and Svelte styles. API behavior and identity compatibility are
documented in docs/api/README.md; contacts documentation and build instructions
use the new display name and bundle path.

## Changed files for this task

- Identity/build: apps/desktop/src-tauri/tauri.conf.json,
  apps/desktop/src-tauri/Cargo.toml, apps/desktop/src-tauri/Info.plist,
  apps/desktop/index.html, .github/workflows/desktop-build.yml,
  .github/workflows/release.yml, scripts/check-macos-contacts.mjs.
- Assets: apps/desktop/public/ruf.svg; apps/desktop/src-tauri/icons/32x32.png,
  128x128.png, 128x128@2x.png, icon.png, icon.icns, icon.ico, tray.svg, tray.png.
- Native shell/compatibility: apps/desktop/src-tauri/src/shell.rs, desktop.rs,
  shell_policy.rs, commands.rs, lib.rs, store.rs and contacts.rs.
- UI: apps/desktop/src/App.svelte, Settings.svelte, app.css,
  lib/components/ContactsSettings.svelte and ContactsPanel.svelte.
- Windows native notification display label:
  crates/baresip-adapter/src/notifications/windows.rs (Ruf instead of SIP).
- Existing browser expectations: tests/ui/browser/layout.mjs and settings.mjs.
- Documentation: README.md, apps/desktop/PRODUCT.md, DESIGN.md,
  .impeccable/design.json, docs/api/README.md, docs/api/contacts.md,
  docs/development.md, docs/superpowers/plans/2026-11-04-ruf-branding.md and this
  verification record.

Pre-existing and concurrent contacts/caller/notification changes were preserved.
Only branding-related edits in those files belong to this task. A small strict
TypeScript issue introduced concurrently was fixed by passing null instead of
undefined for a missing incoming caller number.

## Observed checks

- bun run format:check and bun run check:versions passed.
- bun run check passed with zero Svelte errors/warnings.
- bun run test:ui passed: 17 tests, 65 assertions.
- bun run build passed.
- bun run test:ui:flows passed contacts, phone, settings and layout suites.
- Generated contracts were checked for stability around types:generate.
- Native SDK was rebuilt successfully before native workspace checks.
- Workspace Clippy with -D warnings and workspace tests passed both without and
  with sip-desktop/native. Final checks explicitly selected the pinned Rust
  1.98.1 compiler and rustdoc; target/.rustc_info.json confirmed that compiler.
- The new tray badge regression was observed failing before replacing the old
  drawing and passing afterward. Ruf state labels and compatible/isolated storage
  paths are covered by Rust tests. Existing install-path policy tests now use Ruf.
- bun run desktop:build --features native --bundles app passed with Rust 1.98.1.
- Signed microphone/contact entitlement checks and strict code-signature checks
  passed on target/release/bundle/macos/Ruf.app.
- Bundle plist confirmed Ruf display/name, org.rufapp.desktop identifier and the
  existing sip-desktop executable. Its embedded ICNS exactly matches the asset.
- A separate release preview with org.rufapp.brand-preview was launched and
  captured at 340 × 500 pixels, without accessing the shared account store. The
  native window and application menu identify Ruf. The preview was terminated;
  the final deliverable was rebuilt with org.rufapp.desktop.
- git diff --check passed. The patch was reviewed against source snapshots taken
  before branding to distinguish these edits from existing/concurrent work.

Local captures and build logs remain ignored under native/build and
output/playwright. No credentials or personal contact/call data were added.

## Limits

- No Windows runtime, hosted CI, real incoming banner appearance or new Asterisk
  media execution was observed for this branding task. OS/Focus policies can
  suppress notifications; browser fixtures do not establish notification delivery.
- Several native/keychain/lab tests remain explicitly ignored by the normal suite.
- The local Rust 1.98.1 release build emitted rust-objcopy warnings about locating
  libLLVM while stripping debug information. The bundle build and signature checks
  succeeded. The development bundle is ad-hoc signed and not notarized.
- Initial UI/type failures appeared during concurrent caller work; the final
  observed UI tests and typecheck passed. An early mixed-toolchain doc-test failure
  was superseded by successful checks with explicit pinned compiler/rustdoc.
- OS consent may need renewal; stale SIP login registrations should be removed and
  login start enabled in the installed Ruf app. No installation or login/permission
  setting was changed in the user's OS by this task.

## Concurrent changes after the verified bundle

Immediately after the final successful bundle build, concurrent work introduced
apps/desktop/src-tauri/src/call_notifications.rs and wired actionable banners
through desktop.rs, lib.rs, shell.rs and shell_policy.rs. This replaces the
previous passive banner path. The Ruf bundle identity and icon remain applicable;
those concurrent notification behavior changes are outside this branding review.
The latest whole-workspace format check failed on the newly added notification
module and associated Rust edits. The passing Clippy/tests/build evidence above
precedes that in-progress integration; it does not certify its latest state.
The delivered Ruf.app is the successfully built and signature-checked snapshot.
