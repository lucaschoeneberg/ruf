# Private C boundary

`bridge.c` is compiled against the pinned SDK headers only with Cargo feature
`native`. It owns libre/baresip initialization, platform audio selection, up to eight UAs,
one borrowed active-call pointer, the message queue and native cleanup. No
password appears in a SIP account URI; dedicated upstream authentication setters
receive it after UA allocation. Copied queue passwords are wiped before free.

Rust's private `ffi` module owns an opaque engine handle and a boxed callback
context. One dedicated thread runs libre; all UA operations execute there via
its message queue. Queue creation/destruction, pending-request ownership and
shutdown signaling are protected by the C mutex. The bounded pending list keeps
overload explicit. Shutdown sets a flag checked every 50ms, so pipe saturation
or allocation failure cannot prevent requesting exit. Answer/hangup requests also carry the call generation captured under the Rust
state lock; the native thread rejects stale generations before touching a call.
The public snapshot exposes that generation as an opaque call token; guarded
answer/hangup require a matching token before queueing. Answer reserves
`answering` under the same lock until ACK/establishment, close, or an explicit
native answer failure. Queue rejection restores `incoming` immediately.
Engine callbacks copy numeric state/error information into a synchronized Rust snapshot.
Incoming rejection uses the same generation checks plus native unanswered-offer
validation. An already answered call cannot be ended by a stale reject control.
Already queued hangups of the same closed generation are idempotent. An answer
queued for an already closed call reports the existing stale-action event instead
of EINVAL/nativeFailure. A failed dial emits only its attempt-scoped failure;
it must not also publish a generic
error against a different call that won the slot.

Private event `EV_ANSWER_CLOSED` (34) uses the existing event payload: code zero,
the owning account/generation and null borrowed text fields. It is emitted only
when a successfully transmitted answer closes while still native `INCOMING`
without a final SIP status, before establishment. This distinguishes missing ACK
from remote CANCEL before native answering. Rust records a failed history outcome,
reports `nativeFailure` with null SIP status, and clears the slot. Stale generations
cannot close another call; the restricted diagnostic marker remains `event=closed`.
The public DTO and error enum are unchanged.

Ringing stops after native answer success; recoverable answer failure keeps the
existing play object so the still-offered call remains audible. No hardware
audio path is replaced. The existing RTP watchdog is configured to 60 seconds
for established bidirectional media. Native hold suppresses receive checks;
resume rearms the existing audio stream timer with the configured interval.
This releases a lost-media call without a user hangup and lets idle transport
recovery proceed. See [media-loss recovery](../../../docs/api/sip-events.md#media-loss-recovery).
The private `sip_event` / repr(C) `NativeEvent` payload also borrows bounded caller
arrays and, only for DTMF-start, a two-byte NUL-terminated digit array. Rust copies
them synchronously; no native pointer escapes. Progress, hold/resume, DTMF and
redirect/transfer-failure signals enter a bounded sequenced snapshot buffer.
SIP response statuses are interpreted only on protocol-specific event kinds;
native errno remains a separate failure. See [the SIP contract](../../../docs/api/sip-events.md).
The synchronous platform inventory callback copies device IDs, names and selectability; its borrowed strings never escape C.

The native singleton prevents conflicting process-global upstream state. Rust
joins the native thread before destroying the handle and callback context.
Account replacement/removal releases the old UA through its existing destructor.
Do not call `ua_unregister` before releasing it: libre owns one pending REGISTER
pointer per registration. Its destructor waits for that request to complete or
time out, then unregisters a confirmed binding without overlapping requests.
Retired UAs leave the engine account table immediately, so their late events
cannot update the replacement account. A rejected replacement keeps the previous
UA and its registration unchanged.
Callbacks catch Rust unwinding; raw C pointers remain private. The workspace's
unsafe-code exception applies only to this FFI module.

`Engine` is the safe API. A queued command is not a successful registration or
established call. Portable builds reject startup with `engineUnavailable`.
Real lifecycle tests and the Asterisk tone-echo example exercise the actual
library. Hardware devices and Windows runtime still require platform execution.

`bun run test:network-roam` also builds the standalone `call_actions_test` target.
It includes the production queue handler and uses real libre ringtone ownership;
only `ua_answer` failure/success and external inventory/network effects are
substituted. It does not open audio hardware. Actual UDP regressions separately
verify successive incoming calls, pre-ACK termination, missing ACK, dropped
signaling/registration packets, delayed old BYE transactions and media recovery.

Sound routing/preview operations execute on the same event loop, acknowledge
application/rejection, and recheck the single call slot. `audio_devices.c` reads
platform metadata only, independently of engine lifetime. Explicit devices are
validated again before media startup; duplicated macOS backend names are rejected.
The ringtone uses baresip play_tone with owned PCM and is stopped before module
teardown. Playback callbacks cannot outlive the joined engine.

`SIP_DIAGNOSTICS=1` enables a deliberately restricted native trace on stderr.
It prints only static `sip.diag` markers for INVITE/ACK/BYE/CANCEL, generic
responses, active/closed state and audio errors. Packet bytes, headers, URIs,
IP addresses, credentials and audio are never printed by this trace. Packet
summaries add a bounded SIP status, an allowlisted CSeq method, local interface
category (`tunnel`/`other` on macOS, otherwise `unknown`), peer address category
and an INVITE Contact comparison against the packet sender. Only numeric Contact
hosts are compared; DNS hosts remain `unknown`, without extra DNS requests.
An address mismatch is legal SIP and must not trigger automatic rewriting.
Successful REGISTER responses also report a bounded Contact count
(`none`/`one`/`many`) and whether a numeric Contact without explicit zero expiry
matches the endpoint described by the response Via (`present`/`missing`/`unknown`).
This is a diagnostic comparison, not registrar authentication or a reachability
guarantee. It prints no endpoint, account identity or expiry values.
Successful received REGISTER responses additionally report a
`register-via received=present|missing rport=valued|empty|missing` marker.
Only parameter presence is exposed, never the parameter values, addresses or
ports. This distinguishes complete initial mapping evidence from incomplete PBX
refresh responses without raw packet capture. Presence alone does not validate
an address or prove that a NAT mapping was renewed.
The diagnostic decoder owns a bounded temporary copy and never mutates the
original SIP packet. Each marker ends in `ms=<elapsed monotonic milliseconds>`
and `call=<local call generation>`, with no SIP Call-ID or clock timestamp. It does
not enable upstream raw SIP tracing. Disabled unless the environment value is
exactly `1`, and read once at engine creation. No IPC/DTO changes are involved.
`bun run test:sip-diagnostics` verifies real remote BYE/closed markers,
allowlisted output, absence of sensitive fixture headers/passwords and disabled
behavior against the existing isolated Asterisk lab. Desktop CI runs it after
the regular interop suite.
