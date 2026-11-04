# Isolated Asterisk echo lab

Requires Docker Engine/Desktop with Compose, and the matching native SDK.
The digest-pinned `andrius/asterisk:22` community image reports Asterisk 22.11.0
in the verified macOS arm64 run. This container is a separate test tool; it is
not linked into or distributed inside the MIT application.

```sh
bun run native:build
bun run lab:up
bun run test:asterisk
bun run dev:native
# When finished:
bun run lab:down
```

Desktop fields: registrar `127.0.0.1:5070`, user `1000`, password
`sip-lab-only-not-a-secret`, number `600`. This is an intentionally public,
synthetic fixture password, never a real account credential. Choose test-tone
mode for headless verification. System audio is available for a separate manual
microphone/headset check. Use headphones to avoid feedback with Echo.

Only loopback host ports are published: SIP/UDP 5070 and RTP/UDP 30000–30010.
The client uses an ephemeral SIP port and RTP ports 40000–40100. The endpoint
advertises `media_address=127.0.0.1` explicitly: Docker Desktop's gateway rewriting
otherwise makes NAT heuristics advertise an unreachable container address.
Symmetric RTP learns the client's actual source. These settings are intentionally
for a client on the same host, not a network deployment template.

The automated test registers with digest authentication, calls Answer/Echo on
extension 600, rejects a second outgoing call/account replacement while active,
waits four seconds, hangs up and joins the engine thread. It then parses a newly
created WAV and requires mono PCM16 at 8 kHz, duration >=3 seconds, RMS >500 and
returned frequency between 420 and 460 Hz. Stale files, silence and a successful
SIP response without media fail the test. A second native regression test creates
two successive real calls, replays stale hangup/answer actions from the first,
and requires the second call to remain active with an `invalidState` error. Output recordings stay in ignored
`native/build`. The lab is left up for interactive development; the CI job always
stops its own Compose project afterward.

For direct debugging, run the example with the native feature and provide
`SIP_TEST_PASSWORD` in its environment. Optional variables are
`SIP_TEST_REGISTRAR`, `SIP_TEST_USERNAME`, `SIP_TEST_EXTENSION` and
`SIP_TEST_RECORDING`. Never enable raw SIP logging with real credentials or share
unsanitized packet captures. The normal app returns only safe error codes/messages.

Useful lab-only diagnostics:

```sh
docker compose -f tests/interop/asterisk/compose.yml exec asterisk asterisk -rx 'pjsip show contacts'
docker compose -f tests/interop/asterisk/compose.yml logs --tail 50
```

Evidence and limits are recorded in [native verification](../../../docs/verification/2026-11-04-native.md).
Configuration follows the official
[Asterisk NAT guidance](https://docs.asterisk.org/Configuration/Channel-Drivers/SIP/Configuring-res_pjsip/Configuring-res_pjsip-to-work-through-NAT/).
