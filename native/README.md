# Native baresip SDK

`bun run native:build` fetches the exact baresip/re 4.12.0 revisions in
[sources.json](sources.json), verifies clean source checkouts, builds static
libraries and stages `native/install/<Rust target>/`. Re-running is supported.
Sources, build trees, SDKs, recordings and diagnostic logs are ignored by Git.
The SDK also applies the tracked baresip REFER-policy patch in an isolated source
copy. Local patch texts are recorded in sdk.json; older staged SDKs must be
rebuilt before adapter checks. Unsolicited inbound REFER is rejected, including the
out-of-dialog no-subscription form, and never silently accepted. See the
[event/state contract](../docs/api/sip-events.md) for progress, hold, DTMF and errors.
The call-control patches add a per-session modify-result hook and cancellation
preserving the original transaction and its ACK responsibilities, plus native local-hold rollback
and explicit final-transfer-result hooks. They suppress automatic glare retries
for application-managed controls and retire REFER subscriptions on cancellation.
Timed-out hold retains transaction ownership through a bounded recovery period;
late acceptance is ACKed and followed by a compensating current-intent offer.
Unconfirmed recovery ends the uncertain dialog with an explicit safe error.
The libre patch also repairs cached ACK replay through an explicit opt-in
completed-wire callback. Existing header callbacks keep their semantics; both
request entry points share the same va-list builder. The completion callback
runs synchronously on the SIP event thread after assembly, borrows the message
only for that invocation, and the ACK cache takes an owned byte copy. No refcount
sentinel or second SIP message builder is used.
`build.rs` verifies both exact patch texts against the staged SDK manifest.
The native SBOM hashes each library’s ordered patch texts and rejects stale
manifests, so control hooks remain part of the exported build identity.
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

### OpenSSL and the macOS minimum

Ruf targets macOS 14. A Homebrew bottle can require a newer system even when the
SDK itself is compiled with `CMAKE_OSX_DEPLOYMENT_TARGET=14.0`. The SDK probe now
rejects OpenSSL archive members with a newer minimum before building/staging
libraries. Changing `OPENSSL_ROOT_DIR` re-resolves headers and archives rather
than retaining the previous CMake selection.
The macOS desktop/release workflow builds the checksum-pinned OpenSSL 3.5.9
source for that target instead of depending on a runner's Homebrew bottle.

If the installed bottle is incompatible, build a compatible static OpenSSL.
This example uses the same 3.5.9 release verified locally, with its source checksum
from the Homebrew formula. Run from the repository root:

```sh
mkdir -p native/build/openssl-macos14
curl --fail --location --output native/build/openssl-3.5.9.tar.gz \
  https://github.com/openssl/openssl/releases/download/openssl-3.5.9/openssl-3.5.9.tar.gz
echo '603f5602e2eef00d77fbd429d34dcd5822bb301757a1bc9cdb24c670f1eb859a  native/build/openssl-3.5.9.tar.gz' | shasum -a 256 -c -
tar -xzf native/build/openssl-3.5.9.tar.gz -C native/build/openssl-macos14
export OPENSSL_ROOT_DIR="$PWD/native/build/openssl-macos14/install"
(
  cd native/build/openssl-macos14/openssl-3.5.9
  MACOSX_DEPLOYMENT_TARGET=14.0 perl ./Configure darwin64-arm64-cc no-shared no-zlib \
    --prefix="$OPENSSL_ROOT_DIR" --openssldir="$OPENSSL_ROOT_DIR/ssl" \
    --libdir=lib -mmacosx-version-min=14.0
  make -j4
  make test HARNESS_JOBS=4
  make install_sw
  cp LICENSE.txt "$OPENSSL_ROOT_DIR/LICENSE.txt"
)
bun run native:build
```

Follow [OpenSSL's build instructions](https://github.com/openssl/openssl/blob/openssl-3.5.9/INSTALL.md)
when updating the version or target. All example outputs remain ignored under
`native/build`; no system OpenSSL or trust settings are replaced. Checking object
deployment metadata does not replace running the actual app on macOS 14.

## Build and run

```sh
bun install --frozen-lockfile
bun run native:build
bun run dev:native
```

The desktop starts with empty account fields; enter your own test-PBX credentials.
For the optional isolated lab, install Docker with Compose, Python 3 and the
`openssl` CLI, then run
`bun run test:asterisk`. It provisions fresh synthetic TLS certificates before
recreating the container; a prior `lab:up` is unnecessary and can fail without
those certificate files. The synthetic account is documented in
[the lab guide](../tests/interop/asterisk/README.md). The test leaves the lab running
for desktop work; use `bun run lab:down` afterward.
Select audio before the first registration. Changing the
audio backend of an existing engine currently requires restarting the application.
Account replacement with the same audio backend is supported only while idle.

Native packages: `bun run desktop:build --features native --bundles app`
on macOS, or `--bundles nsis` on Windows. CI selects explicit target triples.
Without `native`, no SDK is needed and telephony reports `notLinked`.

## Selected functionality

Static modules: `g711`, `auconv`, `auresamp`, `ausine`, `aufile`, `srtp`, plus `coreaudio`
on macOS or `wasapi` on Windows. Audio stays entirely within baresip. Test-tone
mode uses 440 Hz input and writes decoded incoming PCM to WAV; system mode uses
the platform microphone/speaker and requires manual device/permission testing.
The file player's upstream path buffer allows at most 127 UTF-8 bytes; a longer
path is rejected explicitly. Each test call overwrites its selected WAV.

The native adapter supports up to eight accounts, per-account UDP/TCP/verified TLS,
G.711, mandatory SDES-SRTP with TLS, separate digest usernames and structured
outbound proxies. Existing two-dialog controls remain in baresip. IPv6 registrar
literal syntax and general media NAT traversal are not implemented. Provider
interoperability still requires verification. See [account security](../docs/api/account-security.md)
for trust, observations, migration, safe failures and isolated native tests.

The security patches bind TLS connections and in-dialog continuation to verified
host identity, enforce a sticky TLS floor and block mandatory media before valid
keys and during renegotiation. Native OS trust evaluation uses no exported root
bundle or runtime Homebrew CA path. Matching OpenSSL headers are staged alongside
the static libraries by the existing CMake probe. Build-time Homebrew OpenSSL is
still an external input. The SDK rejects static OpenSSL archives whose object
minimum exceeds Ruf's macOS 14 target; actual older-macOS runtime compatibility
still needs platform acceptance.

The tracked [libre NAT extension](patches/README.md) learns the observed local
SIP/UDP address from validated REGISTER responses and uses it for Contact/Via,
without changing socket or RTP binding. It re-registers and removes the previous
Contact after address/port translation changes. The Docker/Asterisk regression
disables server Contact rewriting and requires both call directions to terminate
remotely. Successful refreshes omitting `received` can renew only a live exact
mapping whose advertised endpoint and numeric `rport` still match; initial
discovery and malformed data remain strictly rejected. See the
[SDK validation contract](patches/README.md#libre-nat-patch).
An active registration is reported only after a matching own Contact with
positive expiry is confirmed. If a combined old/new Contact correction receives
200 without that binding, one immediate retry sends only the new Contact; another
unconfirmed response fails with the existing registration backoff. Explicit
unregister/retirement never takes that recovery path.
Real UniFi Talk over Tailscale still needs deployment verification;
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
- [Recorded verification and limits](../docs/verification/README.md#native-sip-and-audio)

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
