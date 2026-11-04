# C boundary

This directory will contain the small C shim for the pinned baresip API. It is
deliberately not populated with pretend FFI functions. No native engine is linked
or initialized by the current foundation.

The native milestone must establish ownership, startup/shutdown, callback lifetimes
and a dedicated libre event-loop thread. All native control calls must be marshalled
to that thread with libre's message queue. Owned event data crosses into Rust;
raw C pointers do not leave this crate. Rust panics must never unwind across C.

The workspace denies unsafe Rust by default. When real FFI is added, allow it only
in the reviewed private FFI module, keeping the public wrapper safe.
