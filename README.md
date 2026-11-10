<div align="center">

<img src="apps/desktop/src-tauri/icons/128x128.png" alt="Ruf app icon" width="80">

# Ruf

**A compact SIP softphone for macOS and Windows.**

[![CI](https://github.com/mardu-systems/ruf/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/mardu-systems/ruf/actions/workflows/ci.yml)
[![Version: 0.1.0 Developer Preview](https://img.shields.io/badge/Version-0.1.0%20Developer%20Preview-3b82f6)](docs/releases/0.1.0.md)
[![Target platforms: macOS and Windows](https://img.shields.io/badge/Platforms-macOS%20%7C%20Windows-64748b)](docs/development.md)
[![License: MIT](https://img.shields.io/badge/License-MIT-22c55e)](LICENSE)

[Get the preview](docs/testing-preview.md#get-the-preview) · [Watch the UI demo](#ui-demo) · [Report a bug](https://github.com/mardu-systems/ruf/issues/new?template=bug_report.yml) · [Help test compatibility](docs/testing-preview.md#help-test-compatibility)

</div>

Ruf connects your desktop to a SIP telephone system. Dial numbers or extensions,
manage multiple accounts, and keep contacts and recent calls in a clear interface.
Mute, hold, the DTMF keypad and transfer controls stay within reach during a call.

**Ruf 0.1.0 Developer Preview is free and has no in-app purchases.** It is intended
for isolated test networks and is not qualified for production use. The first
release is being prepared; public installers appear in
[GitHub Releases](https://github.com/mardu-systems/ruf/releases) only after publication.
You can [build from source](#run-locally) now. Read the
[tester guide](docs/testing-preview.md) and [release notes](docs/releases/0.1.0.md)
before connecting a test account.

## UI demo

![Ruf UI walkthrough: dialer, contacts, call controls, history and audio settings](docs/screenshots/demo.gif)

This 30-second walkthrough uses the real interface with synthetic IPC states,
fictional contacts and no live SIP or audio. German and English are available;
the captures show English. [Media provenance and regeneration](docs/screenshots/README.md).

## Screenshots

<table>
  <tr>
    <th>Dialer</th>
    <th>Active call</th>
    <th>Contacts</th>
  </tr>
  <tr>
    <td><img src="docs/screenshots/dialer.png" alt="Ruf account selection and compact dialer" width="260"></td>
    <td><img src="docs/screenshots/call.png" alt="Active call with hold, mute and transfer controls" width="260"></td>
    <td><img src="docs/screenshots/contacts.png" alt="macOS contacts with search and favorites" width="260"></td>
  </tr>
</table>

<details>
<summary>View audio settings</summary>

<img src="docs/screenshots/settings.png" alt="Ruf microphone, speaker and ringtone output settings" width="900">

</details>

The screenshots show the current English interface with fictional demo accounts
and contacts. They do not establish real calling or operating-system acceptance.

## Features

| Area                  | What Ruf offers                                                                                                     |
| --------------------- | ------------------------------------------------------------------------------------------------------------------- |
| **Multiple accounts** | Up to eight saved SIP accounts, independently enabled, with a selectable outgoing identity.                         |
| **Calls**             | Answer, reject, hang up, mute, hold and send native DTMF tones for digits, star and hash.                           |
| **Transfers**         | Transfer directly or start a consultation, switch between calls and complete the attended transfer.                 |
| **macOS contacts**    | Search after explicit permission, use favorites and identify matching contacts while dialing or on a call.          |
| **Local history**     | Call outcomes and duration, callbacks through the original account, and configurable history storage and retention. |
| **Audio**             | Select microphone, call speaker and ringtone outputs separately, and check them with short audio tests.             |
| **Desktop**           | Tray menu, background operation, keyboard shortcuts and search with ⌘K or Ctrl+K; incoming notifications on macOS.  |
| **Languages**         | German and English across the phone, settings and native presentation; follows the system or your saved selection.  |

Rust and Tauri 2 power the desktop application, with Svelte for the interface.
SIP and audio run natively through baresip. Original project code uses the
[MIT license](LICENSE); dependencies retain their own licenses.

## Status and limitations

- Target platforms are **macOS 14+ on Apple Silicon** and **Windows 11 x64**.
  A successful build does not replace testing on physical audio hardware or
  against your own telephone system. See the
  [verification records](docs/verification/README.md) for results and boundaries.
- Default direct-distribution builds use ad hoc signing on macOS; Windows
  installers are unsigned. Local Developer ID signing is separately documented,
  but it does not establish notarization or the identity of a future hosted
  installer. Check each release's actual asset status. The
  [release checklist](docs/release-checklist.md) tracks qualification steps.
- Accounts support UDP, TCP and TLS, plus required SDES-SRTP with TLS.
  UDP/TCP and ordinary RTP are unencrypted. TLS verifies certificates and hosts
  through the operating system, with no automatic plaintext fallback. Call
  security indicators follow actual native observations. Protection to the
  telephone system is not end-to-end encryption.
- At most two related dialogs share one audible conversation. Passwords are
  stored in macOS Keychain or Windows Credential Manager.
- Windows contacts, calendar integration and automatic updates are not yet available.

## Run locally

Requirements: the Bun version in `.bun-version`, Rustup and the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
(Xcode on macOS; MSVC Build Tools and WebView2 on Windows). Native telephony also
requires CMake and OpenSSL. Platform-specific setup is documented in
[native/README.md](native/README.md).

```sh
bun install --frozen-lockfile
bun run native:build
bun run dev:native
```

A fresh installation starts without a SIP account. Enter your telephone system's
credentials in account settings; the test lab is not required. The preview
remains limited to isolated test networks. `bun run dev` starts Tauri without
native telephony; `bun run dev:web` starts only the web interface without the
native API.

The optional [Asterisk lab](tests/interop/asterisk/README.md) additionally requires
Docker with Compose, Python 3 and the `openssl` command:

```sh
bun run test:asterisk
# After completing the lab tests:
bun run lab:down
```

`test:asterisk` generates synthetic TLS test certificates, starts or recreates the
isolated telephone system, and leaves it running afterward. A prior `lab:up` is
unnecessary on a fresh checkout and can fail without the generated certificates.
The lab guide documents the test account and extensions. The test CA is not
imported into the system trust store.

Rustup installs the version pinned in `rust-toolchain.toml`. If Cargo comes from
Homebrew or another package manager instead of Rustup, explicitly select the
same toolchain version.

```sh
bun run check
bun run build
bun run types:generate
bun run check:versions
bun run format:check
bun run test:ui
bun run test:ui:flows
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
```

After building the SDK, these commands also check the native variant:

```sh
cargo clippy --locked --workspace --all-targets --features sip-desktop/native -- -D warnings
cargo test --locked --workspace --features sip-desktop/native
bun run desktop:build --features native --bundles app,dmg
```

The packaging command above builds the app and DMG on macOS; use
`--bundles nsis` on Windows. Omit `--features native` for portable build checks.
That variant reports `notLinked` and contains no native telephony. `linked`
confirms compiled linkage only.

Cargo uses a shared root `target/` directory. Without `--workspace`, Cargo commands
check only the portable packages by default.

## Project layout

| Path                     | Responsibility                                                   |
| ------------------------ | ---------------------------------------------------------------- |
| `apps/desktop/src`       | Svelte interface                                                 |
| `apps/desktop/src-tauri` | Tauri app, IPC, account storage, history and desktop integration |
| `crates/phone-core`      | Platform-independent Rust contracts and validation               |
| `crates/baresip-adapter` | Safe integration with the native telephony engine                |
| `native`                 | Pinned native builds and license texts                           |
| `tests/interop`          | Local Asterisk system and real audio tests                       |
| `.github`                | CI, cross-platform installers and draft releases                 |

## Documentation

- [Architecture](docs/architecture.md) and [development workflow](docs/development.md)
- [Internal API and generated types](docs/api/README.md)
- [GitHub, CI/CD and signing](docs/ci-cd.md)
- [Verification records and limitations](docs/verification/README.md)
- [Try the preview and report compatibility](docs/testing-preview.md)
- [First release preparation and community task drafts](docs/releases/0.1.0-launch.md)
- [Privacy policy draft](docs/privacy-policy-ruf.en.md)
- [Contributing](CONTRIBUTING.md)
- [Code of conduct](CODE_OF_CONDUCT.md), [security reports](SECURITY.md) and
  [licensing and attribution](docs/licensing.md)

GitHub workflows check source code, browser states, native builds and the Asterisk
lab. A matching version tag creates a draft prerelease with installers and
SHA-256 checksums after successful checks. The
[CI/CD guide](docs/ci-cd.md) covers repository rules and production qualification.

## Help shape the first release

We are looking for controlled Windows 11 headset tests, FreeSWITCH
interoperability reports and first-run setup feedback from a fresh Mac.
The [tester guide](docs/testing-preview.md#first-contributions) explains how to
help; [CONTRIBUTING.md](CONTRIBUTING.md) covers focused code and documentation PRs.
Only observed profiles will be described as tested. Please keep all reports free
of credentials and personal contact/call data.
