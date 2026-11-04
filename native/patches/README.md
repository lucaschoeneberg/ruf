# Local libre SIP NAT extension

`re-sip-nat.patch` applies only to re revision
`65555dad439de9d6a81b3ee6092b9c096585249b` (4.12.0). The SDK builder verifies
the original checkout is pristine, copies it into an isolated build tree,
checks/applies the patch and runs both native regression cases. The SDK manifest
contains the exact patch text; the Rust adapter rejects any different/missing
patch until the SDK is rebuilt. Original upstream BSD-3-Clause notices remain.
No Telephone/PJSIP source is incorporated.

The extension learns a **local** numeric SIP/UDP mapping from a successful
REGISTER response accepted by the existing transaction layer. It also checks
the expected source peer, actual destination socket, Call-ID, transmitted CSeq
and echoed Via sent-by. `received` must be numeric, unicast and of the local
address family; `rport` must be decimal in 1–65535. Missing/invalid data leaves
the mapping unchanged. This is not server authentication or encryption: the
underlying transport remains SIP/UDP.

A changed mapping triggers re-registration with the previous Contact at
`expires=0` and the new Contact, before registration success is published. Port
changes count even if the address is unchanged. Each fresh cycle allows two
immediate mapping corrections; further instability fails with ELOOP and uses
normal registration failure backoff. A subsequent fresh cycle can recover.
Contact expiry selection matches the full numeric address and effective
transport port, so another binding on the same IP cannot supply its expiry.

Contact and request Via serialization use the learned address. The actual UDP
socket, `sipreg_laddr()`, RTP address selection and remote dialog target remain
unchanged. This implements signaling adaptation, not media STUN/ICE/TURN or a
general guarantee of all NAT topologies. Exact local/peer IP+port mappings take
precedence. For a different source port on the same peer IP, a mapping is used
only when every live mapping for that exact local socket and peer IP agrees.
Conflicting mappings fall back to the local endpoint. Other peer IPs and local
sockets never inherit a mapping.

## Native extension API

These C functions are internal to the statically linked SDK. They do not add
frontend commands or DTOs. Call them only on the SIP event-loop thread. Addresses
are copied by value; no caller pointers are retained.

- `bool sip_nat_validate(local, peer, sentby, callid, cseq, msg, mapped)` takes
  non-null expected transaction fields and a decoded response. Returns false on
  invalid/unmatched data and leaves `mapped` untouched; true writes the numeric
  observed endpoint. The SIP transaction layer must already have matched the Via
  branch. No network or DNS operations occur.
- `int sip_nat_set(sip, local, peer, mapped, lifetime)` installs/refreshes an
  already validated mapping. All pointers are required. Lifetime is 1–604800
  seconds; invalid arguments return EINVAL, 32 live tuples return ENOSPC for a
  new tuple. Expired slots are reusable; existing tuples remain updateable.
  REGISTER uses twice the negotiated expiry, clamped to 120–604800 seconds.
- `void sip_nat_addr(sip, tp, local, peer, advertised)` requires non-null
  `local`/`advertised`. For SIP/UDP it prefers an exact local/peer IP+port
  match, then an unambiguous live mapping for the same local socket and peer IP.
  Ambiguous, expired, unrelated-peer or non-UDP lookups use the local endpoint. It
  never changes the socket address passed to it. A null SIP instance or peer
  also selects that fallback.

## Regression coverage

`test_sip_nat` checks validation/rejection, numeric port/address boundaries,
peer/socket isolation, peer-port variation, exact-match precedence, ambiguous
fallback suppression, expiration and bounded mapping storage. `test_sip_nat_registration`
uses a real UDP registrar exchange: CSeq matching, port-only translation,
obsolete Contact removal, another binding's differing expiry, bounded immediate
retries, normal backoff and recovery after a fresh cycle. The test harness also
initializes memory-stat structures when Release builds do not supply statistics,
and supports selecting a single test on Windows without getopt.

`bun run test:asterisk` adds authenticated outgoing and incoming calls through
Docker's translated UDP path with account 1003 and `rewrite_contact=no`. Remote
termination must clear the call slot, and the next call must work without any
local hangup. `bun run test:sip-diagnostics` requires a transmitted 200 response
to remote BYE in that scenario, and keeps the restricted trace opt-in.

The non-ignored native Rust test `incoming_nat_peer_port_variation_routes_ack_and_bye`
uses a synthetic UDP registrar and mapped-port relay on loopback. The incoming
peer uses another port; numeric response Contact must target the relay. Real
incoming → answer → ACK → Active → remote BYE → Idle exchanges repeat, stale
frontend tokens and duplicate answers are rejected, CANCEL releases the slot,
and missing ACK exercises the real native timeout before another call succeeds.
Run with `cargo test --locked -p baresip-adapter --features native`.
