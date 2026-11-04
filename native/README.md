# Native baresip SDK

`npm run native:build` fetches the exact baresip/re 4.12.0 revisions in
[sources.json](sources.json), verifies clean source checkouts, builds static
libraries and stages `native/install/<Rust target>/`. Re-running is supported.
Sources, build trees, SDKs, recordings and diagnostic logs are ignored by Git.
Do not edit the fetched checkouts; update the pins deliberately instead.

Supported build hosts: macOS arm64, Windows MSVC x64; Linux x64/arm64 is available
for headless interoperability tests. Cross compilation is not implemented.

## Prerequisites

- All platforms: Git, Node 24, CMake 3.22+, platform C/C++ compiler, pinned Rustup toolchain.
- macOS: Xcode Command Line Tools and `brew install cmake openssl@3`.
- Windows: Visual Studio 2022 C++ tools/Windows SDK, CMake and vcpkg.
  Set `VCPKG_INSTALLATION_ROOT` to vcpkg's checkout. GitHub's Windows runner
  supplies it. The manifest uses a pinned baseline and `x64-windows-static-md`
  OpenSSL, matching the dynamic MSVC CRT used by Rust.
- Linux: CMake, GCC/Clang, pkg-config and OpenSSL development/static libraries
  (`sudo apt-get install cmake libssl-dev pkg-config` on Ubuntu).

`OPENSSL_ROOT_DIR` can select a compatible static OpenSSL installation explicitly.
The generated SDK manifest records resolved OpenSSL version/prefix, source pins,
target and modules. Source pins are reproducible; host compiler and macOS/Linux
OpenSSL packages remain external inputs, so this is not a bit reproducible build.
CMake imported targets resolve configuration-specific OpenSSL library paths on
Windows; archives are normalized for Cargo linking.

## Build and run

```sh
npm ci
npm run native:build
npm run lab:up
npm run test:asterisk
npm run dev:native
```

The lab's synthetic account is documented in [the lab guide](../tests/interop/asterisk/README.md).
The test leaves the lab running for desktop work; use `npm run lab:down` afterward.
The desktop initially offers the same registrar/user/echo extension, but requires
entering the password. Select audio before the first registration. Changing the
audio backend of an existing engine currently requires restarting the application.
Account replacement with the same audio backend is supported only while idle.

Native packages: `npm run desktop:build -- --features native --bundles app`
on macOS, or `--bundles nsis` on Windows. CI selects explicit target triples.
Without `native`, no SDK is needed and telephony reports `notLinked`.

## Selected functionality

Static modules: `g711`, `auconv`, `auresamp`, `ausine`, `aufile`, plus `audiounit`
on macOS or `wasapi` on Windows. Audio stays entirely within baresip. Test-tone
mode uses 440 Hz input and writes decoded incoming PCM to WAV; system mode uses
the platform microphone/speaker and requires manual device/permission testing.
The file player's upstream path buffer allows at most 127 UTF-8 bytes; a longer
path is rejected explicitly. Each test call overwrites its selected WAV.

This milestone supports one in-memory account, SIP/UDP, G.711, one call, answer,
reject/hangup and observable registration/call states. It does not implement
TLS/SRTP, IPv6 registrar syntax, outbound proxies, NAT traversal outside the local
lab, persistence, transfers, account switching or full SIP interoperability.
OpenSSL is linked as an upstream dependency; this does not enable encrypted calls.

The bridge uses a dedicated libre thread and message queue. A bounded pending
queue rejects overload, and a timer observes an allocation-free shutdown flag.
Answer/hangup are bound to an internal call generation, so delayed actions
cannot target a replacement call. Rust joins the thread before freeing callbacks
or C state. See
[the C boundary](../crates/baresip-adapter/native/README.md).

## Licenses and references

The SDK stages upstream license files. Tracked [license texts](licenses/) are
bundled by Tauri and attributed in [THIRD_PARTY_NOTICES](../THIRD_PARTY_NOTICES.md).
Complete transitive/optional-module distribution review is still required before
production releases.

- [Upstream build and source](https://github.com/baresip/baresip)
- [Library integration](https://github.com/baresip/baresip/wiki/Using-baresip-as-a-Library)
- [Pinned public API](https://github.com/baresip/baresip/blob/9cfab81550acc4bd38d6580d79b45cc6eea65cd6/include/baresip.h)
- [Current verification evidence](../docs/verification/2026-11-04-native.md)
