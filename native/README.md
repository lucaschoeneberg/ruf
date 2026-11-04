# Native baresip SDK

`bun run native:build` fetches the exact baresip/re 4.12.0 revisions in
[sources.json](sources.json), verifies clean source checkouts, builds static
libraries and stages `native/install/<Rust target>/`. Re-running is supported.
Sources, build trees, SDKs, recordings and diagnostic logs are ignored by Git.
The SDK also applies the tracked baresip REFER-policy patch in an isolated source
copy. Both local patch texts are recorded in sdk.json; older staged SDKs must be
rebuilt before adapter checks. Unsupported REFER is rejected, including the
out-of-dialog no-subscription form, and never silently accepted. See the
[event/state contract](../docs/api/sip-events.md) for progress, hold, DTMF and errors.
Do not edit the fetched checkouts; update the pins deliberately instead.

Supported build hosts: macOS arm64, Windows MSVC x64; Linux x64/arm64 is available
for headless interoperability tests. Cross compilation is not implemented.

## Prerequisites

- All platforms: Git, Bun 1.4.4, CMake 3.22+, platform C/C++ compiler, pinned Rustup toolchain.
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
bun install --frozen-lockfile
bun run native:build
bun run lab:up
bun run test:asterisk
bun run dev:native
```

The lab's synthetic account is documented in [the lab guide](../tests/interop/asterisk/README.md).
The test leaves the lab running for desktop work; use `bun run lab:down` afterward.
The desktop initially offers the same registrar/user/echo extension, but requires
entering the password. Select audio before the first registration. Changing the
audio backend of an existing engine currently requires restarting the application.
Account replacement with the same audio backend is supported only while idle.

Native packages: `bun run desktop:build --features native --bundles app`
on macOS, or `--bundles nsis` on Windows. CI selects explicit target triples.
Without `native`, no SDK is needed and telephony reports `notLinked`.

## Selected functionality

Static modules: `g711`, `auconv`, `auresamp`, `ausine`, `aufile`, plus `coreaudio`
on macOS or `wasapi` on Windows. Audio stays entirely within baresip. Test-tone
mode uses 440 Hz input and writes decoded incoming PCM to WAV; system mode uses
the platform microphone/speaker and requires manual device/permission testing.
The file player's upstream path buffer allows at most 127 UTF-8 bytes; a longer
path is rejected explicitly. Each test call overwrites its selected WAV.

This milestone supports up to eight accounts, SIP/UDP, G.711, one call, answer,
reject/hangup and observable registration/call states. It does not implement
TLS/SRTP, IPv6 registrar syntax, outbound proxies, general media NAT traversal,
transfers or full SIP interoperability.
OpenSSL is linked as an upstream dependency; this does not enable encrypted calls.

The tracked [libre NAT extension](patches/README.md) learns the observed local
SIP/UDP address from validated REGISTER responses and uses it for Contact/Via,
without changing socket or RTP binding. It re-registers and removes the previous
Contact after address/port translation changes. The Docker/Asterisk regression
disables server Contact rewriting and requires both call directions to terminate
remotely. Real UniFi Talk over Tailscale still needs deployment verification;
the automated result is not a blanket certification of VPN/NAT interoperability.

The client configures the existing adaptive audio jitter buffer with a 40–200 ms
delay range, replacing the upstream fixed 100 ms minimum. It can reduce delay on
stable paths while retaining headroom for variable arrival times. This is not a
60 ms guarantee for total microphone-to-speaker latency. Hardware, remote PBX,
packetization and the VPN still contribute. No device-buffer setting is changed.

CPU-only native jitter-buffer comparison (macOS, matching staged SDK required):

```sh
clang -std=c11 -I native/install/aarch64-apple-darwin/include -I native/install/aarch64-apple-darwin/include/re \
  crates/baresip-adapter/native/latency_probe.c -L native/install/aarch64-apple-darwin/lib \
  -lbaresip -lre -lssl -lcrypto -framework CoreFoundation -framework SystemConfiguration \
  -framework CoreAudio -framework AudioToolbox -framework Security -lresolv -lpthread -lm \
  -o native/build/latency-probe
native/build/latency-probe
```

This uses a virtual clock and synthetic RTP timing: fixed 100–200 ms versus
adaptive 40–200 ms, no jitter and a repeated 0–12 ms arrival variation. Each
scenario must play all 100 packets with no insertion errors. It does not test
hardware latency, larger jitter spikes, packet loss or a real VPN call.

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

## Sound routing

Settings → Ton selects microphone, call output and independent ringtone output.
The device list reads platform metadata without opening capture. macOS uses the
pinned upstream CoreAudio module because AudioUnit ignores explicit device names;
WASAPI remains the Windows backend. Rebuild an older macOS SDK with
`bun run native:build`; build.rs rejects a manifest without CoreAudio.

A built-in two-second PCM ringtone is played through baresip, never browser audio.
Incoming calls loop it until answer/reject/close; developer test-tone sessions do
not ring automatically. The explicit preview supports stop and automatic finish.

Run the opt-in audible device check (no microphone capture):

```sh
cargo run --locked -p baresip-adapter --features native --example sound_smoke
```

It queries real devices, routes to a selectable output, rejects an invalid device,
and checks preview start/stop/completion. Windows execution is still required.
