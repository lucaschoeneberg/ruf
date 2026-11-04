# Private C boundary

`bridge.c` is compiled against the pinned SDK headers only with Cargo feature
`native`. It owns libre/baresip initialization, platform audio selection, one UA,
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
C callbacks copy only numeric owned state/error information into a synchronized Rust snapshot.

The native singleton prevents conflicting process-global upstream state. Rust
joins the native thread before destroying the handle and callback context.
Callbacks catch Rust unwinding; raw C pointers remain private. The workspace's
unsafe-code exception applies only to this FFI module.

`Engine` is the safe API. A queued command is not a successful registration or
established call. Portable builds reject startup with `engineUnavailable`.
Real lifecycle tests and the Asterisk tone-echo example exercise the actual
library. Hardware devices and Windows runtime still require platform execution.
