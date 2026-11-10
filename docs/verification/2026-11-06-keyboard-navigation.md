# Ruf: search, menus, and keyboard shortcuts

As of November 6, 2026. Implemented in the existing checkout; pre-existing
changes were preserved.

## Behavior and reuse

- The magnifying glass in the phone header and Cmd/Ctrl+K open the action
  search with German search terms, visible shortcuts, and disabled call actions
  with explanations.
- Arrow keys select, Enter executes, and Escape closes and restores focus.
  Browser rendering was checked at 340 × 500 and 320 × 440.
- Cmd/Ctrl+1/2/3 switch views; +L focuses the number field, +F the relevant
  search. Contacts receive focus even after asynchronous loading.
- View, Call, and Help menus make navigation, call actions, and shortcuts
  discoverable. Tauri registers the macOS Help menu for system search.
- Mute, hold, the transfer form, and the in-call keypad use the existing
  `CallPanel` handlers. Stale native menu tokens are ignored; confirmation
  locks remain in place. Transfer first opens the existing form without
  initiating a transfer.
- Existing Tauri menus, `listen`/`emit_to`, `perform`, `open_settings`,
  generated call models, settings navigation, icons, and design tokens
  are reused. No new dependency or second audio pipeline.

## API documentation

[Keyboard navigation](../api/keyboard-navigation.md) documents shortcuts,
menu behavior, internal window events, required primitive payloads,
token checks, state validation, and error behavior. Existing invoke commands
and Rust DTOs were reused. TypeScript generation produced no diff.

## Checks run

Rust checks ran with the pinned toolchain 1.98.1;
`/opt/homebrew/opt/rustup/bin` was placed before Homebrew Rust in PATH. The first
version/desktop attempt correctly detected the differing system version 1.99.1.

Successful:

- `bun run format:check`
- `bun run check` — no errors or warnings
- `bun run build`
- `bun run check:versions`
- `bun run types:generate` and a diff check of generated files
- `bun run test:ui` — 93 tests passed
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace` — including the existing explicitly
  ignored test for real keychain writes
- `bun run native:build`
- `cargo clippy --locked --workspace --all-targets --features sip-desktop/native -- -D warnings`
- `bun run test:asterisk` — registration and media smoke test, 16
  Asterisk scenarios, 13 security checks, and one restart/incoming test
- `bun run desktop:build` — portable macOS app and DMG bundle
- `git diff --check`

The shortcut test experimentally detects removal of the repeat guard.
The temporary mutation was reverted immediately. The browser check was first
observed failing without the search button, then run successfully.
An additional focus check required resetting its synthetic contact permission
so that subsequent permission scenarios remained independent.

Also completed successfully:

- `bun run test:ui:flows` — all eight flows: contacts, history, phone,
  call controls, keyboard shortcuts, dial suggestions, settings, and layout.
- `bun run desktop:build --features native` — native macOS app and DMG bundle
  with confirmed native SBOM build context. The first attempt failed due to the
  local LLVM search path/process limit. Repeating it with
  `DYLD_FALLBACK_LIBRARY_PATH` pointing to the toolchain `lib` directory and
  `CARGO_BUILD_JOBS=2` succeeded; Cargo still printed the cached strip warning
  from the previous build-script build.

## Remaining limitations

Native macOS menu search was integrated through Tauri and compiled, but was
not manually exercised in the system menu. Windows runtime, external SIP
providers, and physical audio devices were not tested in this task.
Local bundles are ad hoc signed and not notarized. No hosted CI result
is claimed.

## Extension: search contacts and history together

At the user's explicit request, contacts and history were integrated into
“Ruf durchsuchen” (“Search Ruf”). Starting at two characters, up to eight results
per source appear; additional results open the existing individual view with
the search text carried over. Contact results open the contact card; history
results open the exact call details. Calling back remains a separate explicit
action. Phone numbers use the existing normalization. Contact display
preferences are respected.

In addition to the previous change list,
`tests/ui/browser/global-search.ts` was added; changes were made to
`ActionSearch`, `App`, `ContactsPanel`, `HistoryPanel`, `shortcuts.ts`, the menu
label in `shell.rs`, the browser runner, its existing shortcut/call-control
checks, `DESIGN.md`, and the API documentation linked above.

Reused: `getHistory`, `searchContacts`, generated `Contact`/`HistoryEntry`
models, `historyTitle`, `historyOutcomes`, `normalizeDialNumber`, contact ID
focus, and existing detail/call controls. No new dependency, no new IPC command,
and no DTO change.

New browser cases check combined results, explicit opening without calling,
formatted phone numbers, contact permission revocation, delayed responses,
source errors, both “Alle Treffer” (“All results”) handoffs, search text/focus
transfer, and IME confirmation. The new browser case was observed failing
before implementation. Final checks for this extension:

- `bun run check`: no errors or warnings.
- `bun run test:ui`: 96 tests passed.
- `bun run test:ui:flows`: all nine browser flows passed, including the new
  global search and a final repeated full run.
- `bun run format:check`, `bun run check:versions`, `git diff --check`: passed.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` and
  `cargo test --locked --workspace`: passed; the existing real keychain write
  test remains intentionally ignored.
- `bun run types:generate`: succeeded, generated files without a diff.
- Web production build and `bun run desktop:build --features native`: passed;
  a current native macOS app and DMG bundle with a matching SBOM were created.
  Two initial packaging attempts failed with `Resource temporarily unavailable`
  (OS error 35). With `CARGO_BUILD_JOBS=1`, `TOKIO_WORKER_THREADS=1`,
  `RAYON_NUM_THREADS=1`, and the LLVM library path described above, the full
  build succeeded. The older cached strip warning remains visible.

The additional search was visually checked with synthetic contacts/call data
and at 320 × 440 pixels. The SIP/audio implementation remained unchanged;
the Asterisk run documented above was from the first extension and was not
repeated for the combined search alone. Windows and manual native menu
interaction remain untested.

## Local installation and signing

On November 6, 2026, the older running installation at
`/Applications/Ruf.app` was updated at the user's request. The new native build
was first signed separately with the valid existing Apple Development identity
and checked with `codesign --verify --deep --strict`. The designated requirement
matches the previous installation.

Ruf was closed through the regular Quit flow, the previous app was retained
locally as a rollback backup, the verified new app was installed and reopened.
The installed binary's SHA-256 matches the previously verified signed update;
signature verification, the embedded native SBOM build context, and the running
new app process were then confirmed. User accounts, keychain data, and history
were not changed manually. This is an Apple Development signature, not a
notarized Developer ID release.
