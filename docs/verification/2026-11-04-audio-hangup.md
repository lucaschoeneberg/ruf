# Microphone authorization and remote termination

## Scope and findings

Reported: the other party cannot hear the microphone, and ending a call on the
other device leaves this app in a call.

The running macOS native bundle had hardened runtime (`adhoc,runtime`) and a
microphone purpose string, but no signed audio-input entitlement. The new bundle
check failed against that original bundle and passed against a freshly built
native bundle after adding `com.apple.security.device.audio-input`. This fixes a
confirmed packaging defect. It does **not** prove that this was the sole cause
of the reported one-way audio: hardware capture and the user's PBX need retesting.

Remote termination could not be reproduced in the isolated Asterisk lab. The
new test passed on the existing C event handler and Rust callback before any
call-handling change. Incoming and outgoing calls transition to idle on the
PBX's hangup without a local hangup. No speculative timeout or forced UI reset
was added. The reported PBX-specific termination fault remains open.

## Changes

- `apps/desktop/src-tauri/Entitlements.plist`: minimal native audio-input grant.
- `apps/desktop/src-tauri/tauri.conf.json`: use that entitlement when signing.
- `scripts/check-macos-audio.mjs`, root `package.json`: verify actual code
  signature, embedded entitlement and microphone purpose string with
  `bun run test:macos-audio /absolute/path/to/SIP.app`.
- `.github/workflows/desktop-build.yml`: run that check before publishing macOS
  installer artifacts; the release workflow already reuses this build job.
- `crates/baresip-adapter/src/engine.rs`: real remote-hangup interop coverage on
  synthetic account 1001, including call-slot reuse after both call directions,
  cleared call account, no lingering ringtone and no error on normal BYE.
- `tests/interop/asterisk/extensions.conf`: extension 601 answers, waits one
  second and terminates from the PBX.
- `scripts/asterisk-smoke.mjs`: reload the dialplan in reused lab containers.
- `docs/development.md`, `tests/interop/asterisk/README.md`: permission and test
  instructions.

Existing Engine operations, native event mapping, snapshot DTOs, serial interop
runner and Asterisk fixtures were reused. No dependency, command or DTO changed;
API documentation therefore needs no contract change.

## Observed checks

- Original signed bundle: permission check **failed** (missing entitlement).
- Fixed native bundle at ignored `native/build/audio-fix/SIP.app`: permission
  check **passed**, including signature verification.
- `bun run desktop:build --features native --bundles app`: **passed**, macOS arm64
  ad-hoc bundle. Local rust-objcopy emitted a missing-libLLVM warning while
  stripping debug information; build/signing and subsequent verification passed.
- `bun run test:asterisk`: real SIP registration, G.711 RTP echo (4.04 seconds,
  RMS 5811, 439.8 Hz) and native interop tests passed. Test-tone audio is synthetic.
- Additional local Chromium exercise of the real Svelte UI with synthetic IPC:
  active/incoming/ringing to idle in all three tabs clears the call controls
  without issuing a local answer/hangup. This is diagnostic evidence only, not
  a committed browser test or proof of native signaling from the user's PBX.
- `bun run format:check`, `bun run check:versions`, `bun run test:ui` (11 tests),
  `bun run check`, `bun run build`, `bun run types:generate`: passed.
- Workspace Clippy with warnings denied and Cargo tests: passed both portable
  and native configurations. Opt-in OS keychain tests were not run for this fix.
- Independent scoped code review: no critical or important findings.

## Remaining verification

The running user's app and saved accounts were left untouched. The fixed bundle
must be restarted and granted microphone permission by macOS before a real
headset/microphone call can verify speech. Windows runtime and hosted CI were
not executed here. Identify the affected PBX and call direction, then reproduce
termination there and distinguish an undelivered BYE from native audio teardown
or snapshot delivery. Collect only sanitized signaling metadata, never passwords,
personal numbers or raw audio. NAT/contact routing is a hypothesis, not a finding.

## Follow-up: UniFi Talk over VPN

The affected setup was identified as Ubiquiti over VPN. A read-only route check
selected a tunnel interface; the running native process had a SIP socket on the
OS-selected source address. No packets or credentials were captured and no real
calls were placed. This checks availability of a suitable socket, **not** the
Contact/SDP addresses actually transmitted or reverse reachability.

Added an opt-in `SIP_DIAGNOSTICS=1` native trace in `bridge.c`, its test in
`scripts/check-sip-diagnostics.mjs`, the corresponding package script and interop
CI step. Existing libre tracing is reused with a replacement callback that prints
only static, allowlisted markers; upstream raw tracing remains disabled. No API,
DTO or dependency changed. The test first failed because no remote BYE marker
existed; with instrumentation it passed against real local Asterisk signaling.
It also verifies disabling and rejects fixture headers/passwords in output.
See development instructions for a single-instance diagnostic launch. The user
PBX's termination failure is still unverified; instrumentation is not a fix for it.

## Interactive VPN call: first restricted trace

The caller confirmed that the other party now heard their microphone. This is
manual user feedback on a real call, separate from synthetic media checks.

The captured markers showed call establishment, no received BYE, then a local
BYE and close-event dispatch with repeated local BYE transmissions. The caller
confirmed clicking local hangup after the other party had already hung up. Thus
the observed close marker is accounted for by that local action; it is not a
successful remote-termination result. The original trace omitted response status,
CSeq methods and endpoints, so it cannot prove where the remote signal was lost,
whether the local BYE was acknowledged, or what Contact/SDP was advertised.

Extended `bridge.c` tracing with bounded, owned SIP decoding through existing
libre APIs: status, allowlisted method, anonymous interface/address categories and
numeric INVITE Contact/source comparison. No routing behavior changed. A new
diagnostic assertion first failed without packet summaries, then passed with
real Asterisk status/method and matching loopback Contact assertions. The trace
remains opt-in and header/credential exclusion checks remain enabled. Further
VPN testing is required; a differing Contact alone is not a SIP implementation
defect or authorization to rewrite a remote target.

## Interactive VPN call: extended trace and Tailscale translation

The next trace showed INVITE authentication (407), successful call setup (200),
and ACK/BYE on a tunnel socket toward a private peer. UniFi Talk's INVITE Contact
matched the numeric IP and port of its response sender. No received BYE or
response with CSeq method BYE was captured. A successful REGISTER refresh was
captured during the local BYE retransmissions. Multiple call attempts and old
transaction retries coexist in this session log; the trace has no dialog IDs,
so individual retransmissions cannot reliably be assigned to a specific call.
There is no captured evidence for the earlier public-WAN-Contact hypothesis.

The client advertised a shared-address-space Contact matching its local socket.
A single read-only UDP OPTIONS probe to the saved registrar, using a random
synthetic identity and no account password, received 200 OK. Its reflected Via
`received` address differed from the probe's local VPN source, while reflected
`rport` matched the source port. This establishes address translation for that
probe path; it does not establish the actual BYE destination chosen by the PBX.
No registration or call was initiated by the probe and no raw headers or
addresses were retained in this report.

The user identified Tailscale as the VPN. A filtered, read-only Tailscale status
check found an online Linux peer actively advertising the registrar's subnet;
no peer names, addresses, keys or complete status dump were recorded here.
[Tailscale's subnet-router documentation](https://tailscale.com/docs/features/subnet-routers?tab=macos)
states that subnet routers use SNAT by default. Disabling SNAT requires a return
route from the LAN to the tailnet through that subnet router. This is consistent
with the observed translation, but is not proof of the precise packet drop.

Working hypothesis: the endpoint advertises its original tailnet Contact while
the PBX sees a translated sender; PBX-originated requests cannot necessarily use
that advertised address. The current SIP/UDP bridge has no SIP Outbound/proxy
configuration or general SIP NAT traversal. The Asterisk fixture's symmetric
RTP and Contact rewriting do not establish compatibility with this deployment.
Do not silently rewrite SIP targets or disconnect on an arbitrary timeout.

Next verification should examine a sanitized PBX-side BYE destination, or a
controlled fully routed VPN path with valid return routing. For a Linux subnet
router, the documented network-side option is to disable subnet-route SNAT and
provide a return route to the relevant tailnet client through that router's LAN
address. Plan the return route and preserve existing router settings before
changing anything; disabling SNAT alone can remove working connectivity.
No network, Tailscale, firewall or PBX settings were changed in this session.
No additional app restart or user test call was requested for this analysis.

## Control client: Telefon.app works on the same connection

The user reports that Telefon.app works in this deployment. This is an important
working control: observed NAT is not evidence that the network must be changed
for a SIP client to work. Prefer investigating the client compatibility gap over
making subnet-router changes. No Tailscale/PBX settings were changed.

The installed public app metadata reports Telefon version 1.6. macOS denied
access to its sandboxed preferences, so effective per-account transport/NAT
settings were **not** verified and no privacy protection was bypassed.

The upstream Telephone repository, inspected at
`72bbb66dda0ad7a3568cf78ce349c7a61a4bf936`, provides concrete behavioral evidence:

- [DefaultAppSettings.swift](https://github.com/64characters/Telephone/blob/72bbb66dda0ad7a3568cf78ce349c7a61a4bf936/Telephone/DefaultAppSettings.swift)
  defaults to UDP and enables Contact, Via and SDP address updates.
- [AKSIPUserAgent.m](https://github.com/64characters/Telephone/blob/72bbb66dda0ad7a3568cf78ce349c7a61a4bf936/Telephone/AKSIPUserAgent.m)
  forwards these choices to PJSIP's `allow_contact_rewrite`, `allow_via_rewrite`
  and `allow_sdp_nat_rewrite` options.
- [PJSIP documentation](https://docs.pjsip.org/en/2.15.1/api/generated/pjsip/group/group__PJSUA2__ACC.html#_CPPv4N2pj16AccountNatConfig17contactRewriteUseE)
  describes learning the externally observed transport address from REGISTER
  responses, updating Contact and registering again. This is SIP signaling NAT
  handling, distinct from media-only STUN/ICE.

Our pinned bare SIP/UDP integration generates Contact/Via from its local
transport, and does not implement equivalent learned-address updates. Its
`rport` support alone does not provide the same behavior. The captured local
Contact and the translated source reported by the OPTIONS control are consistent
with this gap. This comparison identifies a specific implementation difference,
not final proof of the PBX's actual BYE destination or the user's effective
Telefon settings.

The next client regression must put the endpoint behind address translation and
leave the PBX's Contact rewriting disabled, then require remote termination and
its 200 response without a local hangup. The existing symmetric Asterisk fixture
is insufficient for this claim. Any client fix must learn only validated local
mapping information, preserve per-account/transport isolation and RTP binding,
and must not blindly overwrite a PBX's remote Contact. This source comparison
does not introduce PJSIP or copy Telephone code into the MIT project.
