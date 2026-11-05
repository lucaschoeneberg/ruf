# Incoming calls before the first outgoing call

The reported behavior is that an external incoming call is missed after opening
Ruf, but works after placing an outgoing call. Native notification presentation
is a separate reported failure. The provider/network-specific incoming failure
has not been reproduced against the isolated lab.

## Observed native evidence

Added `initial_registration_accepts_incoming_before_any_outgoing_call` in the
adapter's existing Asterisk integration tests. A fresh engine registers synthetic
account 1003, whose server endpoint has `rewrite_contact=no`, and the lab
immediately originates the first call. The test never dials. It requires
Incoming, a bound call/account identity, answer, Active after ACK, and Idle after
remote hangup without a native error.

The isolated diagnostic run observed REGISTER 401, REGISTER 200 and mapped-Contact
re-registration 200 by 17 ms; first incoming INVITE at 178 ms; ringing response at
180 ms; answer at 196 ms; ACK/Active at 198 ms; remote BYE and successful response
at approximately 1200 ms. There was no preceding outgoing call.

Executed:

- `bun run native:build`: passed, including SDK NAT regression cases.
- The new native test alone with the synthetic fixture password and
  `SIP_DIAGNOSTICS=1`: passed.
- `bun run test:asterisk`: passed all six native interop tests, including the
  new startup test. Tone echo measured 4.04 s, RMS 5811 and 440.1 Hz.

The prior NAT/remote-hangup test starts with an outgoing call, so it could not
establish the absence of an outgoing-first dependency by itself. The existing
synthetic loopback incoming tests and the new translated Docker path both accept
an incoming call immediately after registration.

## Source findings and boundaries

Saved-account restoration starts the engine and queues `ua_register` without a
dial or webview listener. Unavailable saved credentials instead create a failed
account for explicit foreground recovery. Snapshot/account mutation and native
callbacks are serialized; a callback cannot race initial account insertion.
Hidden mode starts the same Rust observer. Call state and native notification
handoff do not wait for the webview to subscribe; a short call may finish between
snapshot polls, while the separate native journal retains its lifecycle.

The normal UDP account requests `regint=60`. Libre schedules refresh at 90% of
the returned matching Contact expiry, otherwise the response Expires/default
expiry. A 60-second negotiated binding therefore refreshes after 54 seconds;
the requested interval is not a guarantee of the server's accepted lifetime or
of a router's UDP mapping lifetime. The account does not enable SIP Outbound;
libre's separate registration keepalive is conditional on an outbound registration
and the server requiring that extension.

The existing NAT extension corrects the advertised Contact before reporting
registration success when a validated REGISTER response contains usable numeric
`received` and `rport` values. Missing or invalid values leave the advertised
address unchanged. Mapping reuse remains scoped to the local socket and registrar
peer IP, with an unambiguous same-IP port fallback. No arbitrary external peer
inherits another peer's mapping.

These mechanisms do not establish that a particular provider sends an INVITE to
the correct reachable address after idle time, from a particular ingress edge,
or across a particular NAT/VPN topology. No keepalive, STUN, proxy or Contact
rewriting policy was changed without evidence identifying that failure.

## Remaining acceptance

The installed application still needs a user-driven external incoming call after
fresh launch, before any outgoing call, and another after remaining idle. Distinguish
whether an INVITE reaches the native trace from whether Incoming state, ringtone,
and OS notification appear. This verification did not access personal account
data, place real-account calls, install/restart Ruf, or prove Windows behavior,
OS notification delivery, machine-sleep availability or arbitrary-provider NAT
reachability.
