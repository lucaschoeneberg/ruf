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
The synchronous platform inventory callback copies device IDs, names and selectability; its borrowed strings never escape C.

The native singleton prevents conflicting process-global upstream state. Rust
joins the native thread before destroying the handle and callback context.
Callbacks catch Rust unwinding; raw C pointers remain private. The workspace's
unsafe-code exception applies only to this FFI module.

`Engine` is the safe API. A queued command is not a successful registration or
established call. Portable builds reject startup with `engineUnavailable`.
Real lifecycle tests and the Asterisk tone-echo example exercise the actual
library. Hardware devices and Windows runtime still require platform execution.

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
The diagnostic decoder owns a bounded temporary copy and never mutates the
original SIP packet. Each marker ends in `ms=<elapsed monotonic milliseconds>`
and `call=<local call generation>`, with no SIP Call-ID or clock timestamp. It does
not enable upstream raw SIP tracing. Disabled unless the environment value is
exactly `1`, and read once at engine creation. No IPC/DTO changes are involved.
`bun run test:sip-diagnostics` verifies real remote BYE/closed markers,
allowlisted output, absence of sensitive fixture headers/passwords and disabled
behavior against the existing isolated Asterisk lab. Desktop CI runs it after
the regular interop suite.
