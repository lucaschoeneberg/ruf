# Development

Use the Bun version in `.bun-version` and Rustup with `rust-toolchain.toml`.
The exact Bun version is also pinned in `package.json`; install with `bun install --frozen-lockfile` from the root.
Commit `bun.lock` and the root `Cargo.lock`. Cargo defaults to the two
portable crates; use `--workspace` to include Tauri.

On macOS install Xcode/Command Line Tools. On Windows install MSVC C++ Build Tools,
the Windows SDK and WebView2 as described in the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
Current release targets are macOS 14+ arm64 and Windows 11 x64. Windows hosted
builds use Windows Server 2022; that is a build environment, not a runtime test
on Windows 11. Intel macOS, ARM Windows and Linux are not release targets yet.

`bun run native:build` builds the pinned SDK; `bun run dev:native` starts a
real native phone. See [native prerequisites](../native/README.md) and the
[Asterisk lab](../tests/interop/asterisk/README.md). `bun run dev` starts Tauri and a loopback-only Vite server. `bun run dev:web` is
for layout work only. Native API access is unavailable in that browser mode.

Before a change is complete:

```sh
bun run types:generate
git diff --exit-code -- apps/desktop/src/lib/generated
bun run check:versions
bun run format:check
bun run test:ui
bun run check
bun run build
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
bun run desktop:build
```

After intentional DTO changes, review and commit the generated diff before using
the clean-diff command as a verification gate. `bun run format` uses the built-in
Bun formatter and `cargo fmt` for source and
documentation. The `desktop:build` wrapper always supplies a concrete Rust target
(defaulting to the host), so release bundles are under
`target/<triple>/release/bundle`. The macOS examples below use the supported
Apple Silicon target, `aarch64-apple-darwin`.

For native changes, also run workspace Clippy/tests with
`--features sip-desktop/native`, `bun run test:asterisk`, and
`bun run desktop:build --features native`. Test-tone mode needs no audio
hardware; microphone/speaker and Windows runtime require separate platform tests.

macOS bundles require both `NSMicrophoneUsageDescription` in `Info.plist` and
the `com.apple.security.device.audio-input` hardened-runtime entitlement in
`Entitlements.plist`. Tauri applies the entitlement when signing the bundle.
After building, run `bun run test:macos-audio target/aarch64-apple-darwin/release/bundle/macos/Ruf.app`.
Desktop CI runs this check against the signed bundle before uploading the
installer (including tag releases). It verifies the signature, entitlement and
purpose string; it never opens a microphone. A user must still grant microphone
access in macOS System Settings → Privacy & Security → Microphone. Do not reset
system privacy permissions automatically. Development binaries and shipped
bundles can have different permission identities; test the actual `.app`.
See [Apple's audio-input entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.device.audio-input)
and [Tauri macOS entitlements](https://v2.tauri.app/distribute/macos-application-bundle/#entitlements).

macOS Contacts also needs `NSContactsUsageDescription` and the signed
`com.apple.security.personal-information.addressbook` resource entitlement when
using Hardened Runtime. Run `bun run test:macos-contacts target/aarch64-apple-darwin/release/bundle/macos/Ruf.app`
after building; desktop CI verifies the shipped signature before artifact upload.
The check never requests access or reads any contact. In Ruf open Einstellungen →
Datenschutz to see the OS permission and explicitly request access. After denial,
the app opens System Settings → Privacy & Security → Contacts instead of trying
to prompt again. Only the user changes the system permission; never reset TCC
automatically. See [Apple's Address Book entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.personal-information.addressbook)
and the [contact IPC contracts](api/contacts.md).

For remote hangup diagnosis, quit the existing app after ending calls, then
launch the **built native bundle** from a terminal with the restricted trace:

```sh
SIP_DIAGNOSTICS=1 target/aarch64-apple-darwin/release/bundle/macos/Ruf.app/Contents/MacOS/sip-desktop 2>&1 | rg '^sip\.diag '
```

Do not launch a second process with the same SIP account while another instance
is registered. The restricted trace contains static markers only, unlike raw SIP
logging. After the other device hangs up, `rx=BYE` followed by `event=closed`
establishes delivery to the native stack and dispatch of its close callback;
it does not prove that Rust or the UI consumed the update. A received BYE
without a close marker warrants investigating dialog handling/audio teardown;
absence of a marker alone does **not** prove packet loss (a blocked event loop or
different remote signaling can also explain it). These markers do not establish
what was advertised in SDP/Contact or what reached the remote PBX.

Additional `packet=rx` / `packet=tx` summaries show `status`, the allowlisted
request/CSeq `method`, local socket interface `tunnel`/`other`/`unknown`, peer address scope and
INVITE `contact` scope plus `contact-match=yes/no/unknown`. A Contact match means
both numeric IP and port match that packet's sender, not that it is reachable.
An omitted SIP port uses libre's standard UDP port resolution. The local socket
interface category does not independently prove the actual egress route.
Different Contact addresses can be legitimate proxies or PBX deployments;
do not rewrite them automatically. `other-ipv4` is a catch-all category, not
proof of a routable public IP. IPv6 scope is intentionally reported only as
`ipv6`. Addresses, ports, SIP users, headers and audio stay out of the trace.
For example, a VPN-delivered INVITE response advertising `other-ipv4` with
`contact-match=no`, followed by ACK/BYE through `local=other` to `peer=other-ipv4`,
would support the hypothesis of a PBX target outside the intended VPN path.

With a VPN, inspect the OS route to the registrar and whether the running process
has a SIP socket on its selected source address before forcing a network
interface. Registration alone does not prove two-way media or reverse SIP
reachability. Compare SIP Contact/Via, SDP media addresses and the PBX's BYE
destination only in an explicitly sanitized signaling capture. Do not infer the
client's SIP port from the PBX's port, or open unrelated WAN port forwards.
Ubiquiti documents [remote UniFi Talk phones over Teleport VPN](https://help.ui.com/hc/en-us/articles/29488591687959-UniFi-Talk-Remote-Workforce-with-Teleport-VPN);
this is not a compatibility certification for this third-party softphone.

Version changes update the root JavaScript package, frontend package and Cargo workspace
version together. Tauri reads the root JavaScript package version directly. Run
`bun run check:versions`; release tags must be exactly `v<version>`.

## Bun tooling

Bun owns installation, workspaces, JavaScript execution and formatting. Run
`bun install --frozen-lockfile` from the repository root after checkout. Commit
`bun.lock`; do not generate a second package-manager lockfile. Both `.bun-version`
and `package.json` pin Bun 1.4.4; `bun run check:versions` verifies the pins and
actual runtime as well as application/release versions. It also checks that both
`rustc` and `cargo` on PATH match `rust-toolchain.toml` and workspace `rust-version`.
If system/Homebrew Rust shadows Rustup, put the pinned Rustup toolchain first on
PATH before running checks/builds. `rustup which cargo` identifies that toolchain.

Workspace tools explicitly run with `bun --bun` so their Node shebangs do not
select a separately installed Node runtime. Existing `node:` standard-library
imports in build scripts are supported by Bun and remain unchanged. GitHub-hosted
actions may use their own bundled runtime; project commands use Bun.

`bun run format` runs the built-in `bun fmt` for Svelte, TypeScript, JavaScript,
CSS, JSON, YAML, TOML and Markdown, followed by `cargo fmt` for Rust. There is no
Prettier or Svelte formatter plugin dependency. Configuration and generated-file
exclusions live in `bunfig.toml`; `.editorconfig` remains authoritative for basic
whitespace. Native C/CMake formatting is unchanged.

Pass arguments directly: `bun run desktop:build --features native --bundles app,dmg`
on macOS, or `--bundles nsis` on Windows. Requesting both `app,dmg` retains Ruf.app
for the signature/permission checks; a DMG-only build removes its temporary app
after packaging. Do not insert npm's argument separator.

References: [Bun formatter](https://bun.com/docs/format),
[lockfile migration](https://bun.sh/docs/pm/lockfile),
[GitHub Actions setup](https://github.com/oven-sh/setup-bun), and
[Dependabot Bun support](https://docs.github.com/en/code-security/reference/supply-chain-security/dependabot-options-reference#package-ecosystem).

## Compact desktop UI

The native window defaults to 340 × 500 pixels (minimum 320 × 440). UI components
live in `apps/desktop/src/lib/components`, with shared styles in `src/app.css`.
The desktop [product record](../apps/desktop/PRODUCT.md) and
[design system](../apps/desktop/DESIGN.md) describe the confirmed dark direction.

`bun run test:ui` covers keypad insertion/deletion, selection replacement and the
64-character native destination limit using Bun's existing test runner. CI runs
it before the Svelte/TypeScript check. No browser/test dependency is added to the
application. Browser preview allows keypad/navigation/layout work but never
claims native readiness or enables registration/calling. Real calls still use the
existing Tauri API and the independent Asterisk echo regression.

## First-run onboarding

The initial introduction is skippable and replayable through the header info
action. The webview stores only `sip.introduction.v1 = seen` in local storage;
blocked storage falls back to an in-memory dismissal. Own SIP credentials start
blank and system audio is the default. Setup opens the separate settings window.
Test tone and local Asterisk instructions remain secondary paths.

## Settings and saved accounts

The gear or Cmd/Ctrl+, opens/focuses one settings window (900 × 660, minimum
740 × 540). `src/Settings.svelte` owns that surface, selected via
`index.html?window=settings`; it never mounts the compact phone. Both windows use
the same managed Rust PhoneSession and poll its password-free snapshot. Closing
settings does not reset the engine. Per-window capabilities keep account writes
in settings and call controls in the phone.

Up to eight accounts register independently; the phone header selects the next
outgoing account. Incoming calls retain their own account ID. One concurrent call
is supported. Save/delete/switch are blocked during a call and the native event
loop rechecks mutations for incoming-call races. Saves wait for application of
the queued native configuration, then poll for registration success.

`accounts.json` under the Tauri application-data directory stores nonsecret
metadata and selected account. Passwords use macOS Keychain Services / Windows
Credential Manager through keyring. No plaintext fallback or browser password
storage exists. Obsolete credential deletion is journaled and retried after
access failures. See the [API contract](api/README.md) for schema/error details.
The actual engine-wide audio mode is returned in snapshots, so webview reloads
and new windows do not infer it from local form defaults.

`bun run test:asterisk` now tests two simultaneous registrations, outgoing
selection/calls, an incoming call to the other account, removal and stale call
actions serially (the native engine is a process singleton). In addition to
normal workspace checks, the opt-in desktop integration test exercises real OS
credential storage and restores a fresh PhoneSession using isolated temporary
metadata and synthetic credentials:

```sh
cargo test --locked -p sip-desktop --features native saved_accounts_survive -- --ignored --test-threads=1
```

It requires the running Asterisk lab and OS credential-store access; it does not
read or change a user's saved app accounts. UI browser fixtures remain separate
from real native registration/audio evidence. See
[settings verification](verification/2026-11-04-settings-accounts.md).

## Ton settings

`SoundPanel.svelte` uses the existing Zed-style setting rows. The parent keeps a
separate sound draft/baseline, protects it on navigation and never overwrites it
with status polling. Device refresh/preview errors remain recoverable. Audio
source/test-tone selection is secondary developer content. The native routing
contract and four settings-only commands are documented in docs/api.

The saved-account integration test also checks sound restoration. A separate
opt-in startup regression verifies that an unavailable device cannot be saved
before an engine exists, without opening audio input:

```sh
cargo test --locked -p sip-desktop --features native first_sound_save -- --ignored --test-threads=1
```

## Zed visual system

All views use shared tokens in `src/app.css`, matching the inspected local Zed
GitHub Dark Tritanopia palette. There is no settings-only color override. The
phone has horizontal compact navigation; settings retain a dense sidebar and
28px controls. IBM Plex Sans Regular/SemiBold are self-hosted frontend assets
with their OFL notice, not runtime packages. No API/DTO changes accompany this
redesign. See DESIGN.md and the Zed-design verification record for provenance,
state captures and checks.

## Background calling and desktop integration

The native process owns account restoration, call state observation and ringing.
`--background` suppresses all startup windows; normal launch opens the existing
phone through the single-instance plugin. Closing the phone hides it. Explicit
Quit confirms occupied calls and performs native shutdown before exit.

Login start is automatically configured only once after successful registration
in an installed release build, never from `native/build` or debug binaries.
Use General settings to change it. OS policy can independently suppress login
items or notifications. Native notification authorization is queried from the OS;
unsupported launch contexts report null. Permission is requested only by explicit
foreground settings actions. macOS startup keychain operations
suppress UI; repair failed accounts interactively in settings.

Run `bun run test:network-roam` after building the native SDK. The standalone
C tests cover late interfaces, removal, active-call deferral, debounce, retries
and enumeration failures. `cargo test --locked -p baresip-adapter --features native`
also runs an isolated synthetic UDP registrar/NAT relay, with a real missing-ACK
timeout (about 32 seconds). No VPN or host network changes are needed.

Installed-platform acceptance must cover silent login, tray/menu actions,
hide/reopen, second-launch handling, denied notifications, delayed VPN availability
and real incoming/outgoing media. A successful native build alone does not verify
these OS behaviors.

## Browser state and layout regressions

Run `bun run test:ui:flows` for the phone, Settings and compact-layout scenarios.
The runner starts its own loopback Vite server, uses pinned Playwright CLI 0.1.23
through Bun, and closes its browser/server afterward. All SIP/account/device IPC is
synthetic; this check cannot register accounts, place calls or access a keychain.
Captures go to the ignored `output/playwright/` directory.

History regressions cover refresh of more than two loaded pages, retained detail
and keyboard position, unchanged contact scopes, recoverable refresh failures,
and debounced searches that keep prior results while rejecting obsolete replies.
Settings regressions cover the eight sections, concrete field-search targets,
nested-disclosure focus, long account names, named account actions and protected
drafts. They separate permission reads from notification/open-action failures and
cover native revision ordering across events, reads and saves. Audio test controls
use synthetic IPC here; native ownership/preemption and real hardware require their
separate checks. The dialer exposes its correction hint through the input's
accessible description.

Install Chromium when no browser is available:

```sh
bunx --package @playwright/cli@0.1.23 playwright-cli install-browser chromium
bun run test:ui:flows
```

Linux CI uses `--with-deps` during browser installation. Existing Playwright
Chromium caches are detected; `SIP_UI_BROWSER_EXECUTABLE` can select another
installed Chromium executable explicitly. The scenario modules under
`tests/ui/browser/` are also usable through the CLI's `run-code --filename` after
wrapping their default function. UI unit tests remain `bun run test:ui`.

## Ruf icon assets

The approved blue r/handset concept is redrawn as a crisp SVG in
`apps/desktop/public/ruf.svg` with transparent space outside the dark tile.
`apps/desktop/src-tauri/icons/tray.svg` is the matching 32px monochrome template.
The existing Tauri CLI generates the PNG, ICNS and ICO assets, without a new
runtime dependency. Native notifications inherit the release bundle's identity
and icon. The native bridge rejects unbundled macOS delivery instead of attributing
notices to Terminal; temporary Windows builds do not register COM activation.
Verify installed release notifications separately from browser fixtures.

To regenerate, invoke the CLI through its JS entry point (as the desktop build
script does) to avoid Bun resolving the `tauri` package script recursively:

```sh
cd apps/desktop
bun -e 'const {run}=require("@tauri-apps/cli"); await run(["icon", "public/ruf.svg", "--output", "../../native/build/ruf-icons"], "bun");'
bun -e 'const {run}=require("@tauri-apps/cli"); await run(["icon", "src-tauri/icons/tray.svg", "--output", "../../native/build/ruf-tray", "--png", "32"], "bun");'
```

Copy the five configured desktop assets plus `icon.png` from `ruf-icons` into
`src-tauri/icons`, and `ruf-tray/32x32.png` to `src-tauri/icons/tray.png`.
Mobile and Appx output is unused. See [desktop identity](api/README.md#ruf-desktop-identity)
for storage compatibility and OS consent/login behavior after the ID change.

## Actionable call notification test

`bun run test:macos-call-notifications` builds an isolated accessory app with a
separate bundle identity. It checks actual OS delivery, silent notifications,
Annehmen/Ablehnen categories, synthetic response callbacks, and removal from
Notification Center. To request permission for that test identity explicitly, add
`--request-permission`. It uses only public fake caller data and does not start
SIP or read saved accounts, keychain entries or contacts. Real notification clicks
and Windows COM activation remain installed-platform acceptance checks.

Notification delivery runs on its own worker, coalesces the latest phone state,
and never blocks the phone observer. A separate delivery mutex orders native
show/withdraw/reset calls; it is never held together with the phone-session lock.
See [verification evidence](verification/2026-11-04-call-notifications.md).

## Contacts and observed call history

The native adapter emits ordered call facts to a synchronous nonblocking observer.
The desktop observer only queues to one SQLite worker; no I/O occurs under native
call state locks. Shutdown joins SIP before draining history writes. SQLite
`history.sqlite3` lives in the existing application-data directory. It stores
password-free SIP/account facts, never resolved address-book names or IDs.
`callerContact` and history `contact` are projections of the authorized in-memory
contact cache. Native Contacts revision notifications and independent permission
checks invalidate that cache without a new consent prompt. See
[contacts API](api/contacts.md) and [history API](api/history.md) for validation,
permission scope, error behavior and recovery.
