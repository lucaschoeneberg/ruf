# Legacy SIP cleanup

The user explicitly requested deletion of the old app's saved settings,
credentials and other persisted state. This supersedes the storage-compatibility
choice in the Ruf branding work.

## Code and documentation

- apps/desktop/src-tauri/src/store.rs now uses Tauri's own app_data_dir directly
  and the org.rufapp.desktop credential service. The legacy redirect/helper and
  its obsolete compatibility test were removed. No fallback/import remains.
- docs/api/README.md documents the independent Ruf namespace and fresh account
  setup. IPC fields, validation, errors and the versioned saved-settings format
  remain unchanged. The existing AccountStore, credential boundary and Tauri
  path API are reused; no dependency was added.
- docs/verification/2026-11-04-ruf-branding.md links this superseding decision.
- Current generated contracts were refreshed through the existing types:generate
  workflow during concurrent contact/history work; no DTO was changed for cleanup.

## Observed cleanup

- Deleted three generic-password entries matching exactly the old credential
  service org.sipsoftphone.desktop. No password values were requested or printed.
- Removed 22 identified legacy paths: the application-data directory, four cache
  directories, four WebKit stores, SIP.plist LaunchAgent, eleven old app/preview
  bundles and the old SIP disk image. Build artifacts were already ignored.
- The SIP LaunchAgent was not loaded (bootout returned 3); its plist was removed.
- tccutil reset All succeeded for org.sipsoftphone.desktop and the audit, smoke
  and design-preview IDs. The old defaults domain was cleared.
- Rebuilt and installed /Applications/Ruf.app; its bundle ID is
  org.rufapp.desktop. The installed executable contains the new namespace and
  no org.sipsoftphone.desktop string.
- Launched the installed Ruf. It has zero saved accounts. After launch, the old
  data/cache/WebKit/etc. paths and SIP LaunchAgent remained absent; an exact-service
  keychain query returned not found (44).

The cleanup was limited to identified app-owned legacy namespaces and bundles.
Ruf's own cache/WebKit data and unrelated application credentials/settings were
not selected. Supplementary Autosave Information enumeration was denied by macOS;
no evidence of a legacy SIP file there was found. Shared OS notification/TCC
history databases were not modified directly; supported scoped tools were used.

## Verification

Passed with explicitly selected Rust 1.98.1 compiler/rustdoc: workspace Clippy
with -D warnings and workspace tests, both portable and sip-desktop/native;
normal ignored keychain/Asterisk tests stayed ignored. Native desktop app build
passed. Installed-app strict signature checks and signed microphone/contact
entitlement checks passed. Format check, versions, TypeScript/Svelte checks and
17 UI tests passed. Generated types were refreshed after concurrent additions
initially caused a missing ContactReference declaration. No new SIP/audio media,
Windows runtime or hosted CI execution is claimed.

The local pinned release toolchain emitted the already observed rust-objcopy /
libLLVM debug-stripping warnings; build and code-signature checks succeeded.
New accounts must be configured in Ruf. The deleted old settings/passwords were
not backed up because the user requested their removal.

The last whole-workspace format check, after these successful checks, reported
new concurrent history work requiring formatting: history-api.ts, history.ts and
docs/superpowers/plans/2026-11-04-contacts-history.md. These unrelated in-progress
files were left to their author; this does not change the observed cleanup or
installed bundle verification.
