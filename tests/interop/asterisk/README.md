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

The fixture also defines synthetic accounts 1001 and 1002 with the same public
lab password for the multi-account regression. The smoke runner executes native
interop tests serially because the native engine is a process singleton. They
cover simultaneous registrations, outgoing account selection, an incoming call
to the other account, manual answer/hangup, independent removal and native call
guards. These accounts are local test identities, never production credentials.

Extension `601` answers, waits one second and hangs up from the PBX. The remote
hangup test uses reserved lab account `1001`: two successive outgoing calls and
one answered incoming call must return to `idle`, clear `callAccountId`, stop
ringing and leave no error, without invoking local `hangup`. The second outgoing
call also verifies that the native call slot was released. A further outgoing
call proves slot reuse after the incoming remote hangup too. The smoke runner
reloads the bind-mounted dialplan when reusing an existing lab. These tests use
synthetic audio; they do not prove microphone permission or microphone capture.

Synthetic account `1003` deliberately sets `rewrite_contact=no`, with the same
public lab password. Docker translates the host client's UDP source on the way
to Asterisk. The NAT regression requires the client to learn that mapping,
re-register, accept outgoing and incoming remote hangup, clear the call account,
and place a subsequent call without any local hangup. This test failed before
the client fix with the call stuck active for 15 seconds. The fixture does not
provide a SIP proxy or repair the Contact on the client's behalf. Its AOR allows
four bindings so it does not conceal stale Contacts by forcibly replacing them.
The native SDK's controlled-registrar test separately verifies old Contact
removal and port-only rebinding. The smoke runner reloads PJSIP and the dialplan
only in this isolated lab.
