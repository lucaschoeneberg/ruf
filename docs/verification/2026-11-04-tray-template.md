# macOS status icon template fix

The supplied screenshot shows the Ruf r rendered black against the dark menu bar.
The initial tray builder already used icon_as_template(true) on macOS, but each
state update replaced the image through set_icon. In the installed tray-icon
0.25.1 macOS implementation, set_icon calls the image setter with template=false.
That explains why the initial template setting did not survive the first observer
update. A native Swift check also confirmed that template state belongs to the
replacement NSImage rather than being inherited from its predecessor.

## Minimal patch

apps/desktop/src-tauri/src/shell.rs now uses the existing Tauri 2.12.5
set_icon_with_as_template API for status updates, with the macOS flag. Image and
template state are applied together, preserving OS contrast and avoiding an
intermediate untinted frame. The OS renders a light glyph on dark menu bars and
a dark glyph on light menu bars. Windows retains its existing blue image through
the API's documented fallback. No icon geometry, call state, badges or dependencies
changed. No new tests that mirror this one-line API selection were added.

docs/api/README.md clarifies that adaptive tint persists after status updates.
Commands, DTO fields, validation and errors are unchanged.

## Verification status

TypeScript/Svelte check, 21 UI tests, version check and git diff --check passed.
Initial Rust checks were blocked by concurrent history work: first dead-code
warnings, then the missing contacts::cached_contacts helper. Whole-workspace
format checks also reported files in that separate in-progress history work.
Native build/install verification is recorded below once completed.

## Completed checks and delivery

After the concurrent contact helpers became available, both existing shell tests
passed with Rust 1.98.1. Workspace Clippy (-D warnings) and workspace tests also
passed. The native release app was rebuilt, copied to /Applications/Ruf.app,
strictly signature-verified and restarted. Signed microphone and contacts
entitlement checks passed for the installed app. Generated-contract output was
unchanged around types:generate. Local build logs and menu-bar captures are kept
under ignored native/build. The macOS menu-bar manager hides the Ruf status item
in the collapsed bar, so the captures alone do not certify its final pixels.

The native SDK and existing Tauri APIs were reused; no SIP/audio logic changed.
No Windows runtime, hosted CI or new Asterisk execution was performed. The pinned
release toolchain still emits the known local rust-objcopy/libLLVM stripping
warning; app build and signature verification succeed. The latest whole-workspace
format check was blocked only by concurrently authored history documentation.
The targeted Rust source was formatted and git diff --check passed.

A subsequent build by the concurrent history task changed the target bundle after
installation, so a byte-for-byte comparison against that moving target was not
used as final evidence. The installed Ruf executable retained the atomic template
API symbols, its source update remained present, its strict code signature was
valid and the installed app was running.
