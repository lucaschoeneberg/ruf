# Local libre SIP NAT extension

## SIP reliability patches

`re-reliability.patch` and `baresip-reliability.patch` apply after the existing
NAT, transfer policy, control and security patches to the same pinned revisions.
Their exact contents are required by Cargo's SDK check and the desktop SBOM.

On macOS, the libre patch destroys each temporary resolver state with
`res_ndestroy`, including shared cleanup after a server-list or allocation
failure. `res_nclose` only closes sockets and retains the DNS configuration
notification token. Repeated idle DNS refreshes otherwise exhaust libnotify's
registration limit and terminate the process with SIGTRAP. Other platforms
retain their existing resolver cleanup. No public SDK signature changes.

The baresip patch retains only the last accepted remote SDP body in a call-owned
buffer, released with the call. Decoder failures restore that description;
rejected incompatible media offers restore both SDP and the native media path.
Restoration failure ends the dialog through the existing deferred failure API.
Mandatory SRTP errors never restore cleared crypto state. Remote hold events are
emitted after accepted media application. DTMF INFO validates one signal and a
decimal duration of 1–5000 ms, returning stateful 400/200 replies so retransmitted
requests do not repeat application events. Blind numeric transfer targets use
the existing URI-user encoder.

The libre patch preserves terminal 408/481 handling for managed re-INVITEs and
rejects initial successful setup without a required SDP answer, after ACK.
The private `sipsess_ack` helper adds a required boolean `allow_forks` argument;
only ACKs completing an initial locally offered INVITE enable it. Missing cached
ACKs may match that original by Call-ID, local tag, CSeq and existing peer
authority. `sip_dialog_fork` supplies Contact/Record-Route/TLS state; the normal
ACK cache and authenticated session BYE teardown retire the unwanted branch.
Duplicates replay the ACK without repeating BYE. At most 32 distinct unwanted
branches are retained per original within its existing 64*T1 ACK lifetime.
`sipsess_ack_fork` and `sipsess_close_fork` are private event-thread helpers:
arguments are required and borrowed synchronously; they return errno/security
errors and never expose native handles through Rust IPC. Existing security
matching and no-downgrade rules remain in force.

Accepted invalid hold answers reuse bounded compensation rather than reporting
unconfirmed local rollback as recovery. The adapter renews the existing RTP
watchdog on confirmed local resume and journals global-limit busy rejections
through the existing sanitized observer boundary. Public commands and DTOs are
unchanged; their behavioral contract is in `docs/api/call-controls.md` and
`docs/api/sip-events.md`.

The follow-up corrections also validate common codecs in SDP answers, restoring
the last accepted description before managed compensation. Invalid answers in
ACK end established dialogs through deferred failure. Terminated established
sessions retain pending re-INVITEs through final-response ACK and normal cleanup,
including late 408/481; no application callback or media is resurrected. REFER
NOTIFY validates content type and a complete initial Status-Line (empty reason
permitted), with stateful replies. Subscription-close handlers return immediately
after invoking a control callback because it may release their call owner.

The adapter retains at most one closed transfer call and its UA until the original
transfer deadline, using native reference counting and the existing subscription.
Only the originally paired peer generation receives completion/cleanup. The owner
is released on result, timeout, stop or superseding idle work; no native handle is
exported, no media is allocated for it, and no closed application call is retained.
Confirmed explicit hold intent and deferred return remain per-dialog state.
`bun run test:sip-lifetimes` exercises owner release with macOS GuardMalloc;
ordinary native scenario tests cover the same lifecycle on other platforms.

## Baresip REFER policy

`baresip-refer-policy.patch` applies to the pinned baresip 4.12.0 revision. It
enforces the existing disabled account transfer flag for out-of-dialog REFER,
returning 405 with the actual Allow list before a request can be accepted.
In-dialog rejection uses libre's existing 501 path. The app also removes REFER,
replaces and norefersub advertisements. No transfer target is dialed automatically.
The SDK builder preserves pristine upstream checkouts, applies this small patch
in an isolated copy and records exact patch text in sdk.json. Adapter builds
reject older SDKs. Native SIP tests cover both request forms and call survival.
Upstream BSD notices and source pins are unchanged.

## Libre NAT patch

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
address family; `rport` must be decimal in 1–65535. Invalid data leaves the
mapping unchanged. Initial discovery still requires both values.

Some peers omit `received` on subsequent successful REGISTER responses. Although
[RFC 3581 §4](https://www.rfc-editor.org/rfc/rfc3581.html#section-4) requires it
when answering an `rport` request, the client permits a narrow compatibility
renewal: an existing **live exact local-socket/peer-endpoint mapping** must equal
the echoed numeric Via sent-by, and its port must equal the validated `rport`.
The same transaction/source/destination/Call-ID/CSeq checks still apply. This
cannot discover a new address, change an existing endpoint, revive an expired
entry or reuse a different peer port's mapping for validation. A present but
malformed/empty `received` never enters the compatibility path. Without a valid
renewal, expiration and normal discovery remain unchanged. This is not server
authentication or encryption: the underlying transport remains SIP/UDP.

A changed mapping triggers re-registration with the previous Contact at
`expires=0` and the new Contact, before registration success is published. Port
changes count even if the address is unchanged. Each fresh cycle allows two
immediate mapping corrections; further instability fails with ELOOP and uses
normal registration failure backoff. A subsequent fresh cycle can recover.
Contact expiry selection matches the full numeric address and effective
transport port, so another binding on the same IP cannot supply its expiry.

A successful active registration also requires the response to confirm this
client's own Contact with a positive expiry, using that same existing
user/transport/address/port matcher. A 200 response alone is insufficient.
During NAT correction, a response without the new active binding after the
combined old-Contact deletion/new-Contact update triggers one immediate retry
containing **only the new Contact**. No success callback is published before
confirmation. If that retry is again empty, unrelated or expired, the SDK returns
`EPROTO`, keeps registration false and uses its existing failure backoff.
Ordinary active refreshes without an own active binding also fail instead of
remaining green. This follows the complete-binding response semantics in
[RFC 3261 §§10.2.3–10.3](https://www.rfc-editor.org/rfc/rfc3261.html#section-10.2.3).

This compatibility retry is limited to once per fresh registration cycle and
does not reset or increase the two-correction NAT rebind budget. A later fresh
cycle may recover normally. Explicit unregister and terminated/retiring clients
never enter this active-registration retry path; an empty successful unregister
response remains successful. No wildcard Contact deletion is sent.

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

- `bool sip_nat_validate(sip, local, peer, sentby, callid, cseq, msg, mapped)`
  takes the owning SDK SIP instance, non-null expected transaction fields and
  a decoded response. All pointer arguments are required; `cseq` is the sequence
  actually transmitted. Returns false on invalid/unmatched data and leaves
  `mapped` untouched; true writes the numeric observed endpoint or the unchanged
  live endpoint confirmed by the compatibility rule above. It does not mutate
  the cache; the REGISTER handler renews the validated entry with `sip_nat_set`.
  The SIP transaction layer must already have matched the Via branch. No network
  or DNS operations occur. The added `sip` argument changes this private SDK C
  signature; all SDK consumers must rebuild together, as enforced by sdk.json.
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
retries, normal backoff and recovery after a fresh cycle. A successful refresh
omitting `received` must preserve the mapped endpoint beyond the entry's prior
expiry, including an incoming peer's alternate port lookup. Initial/incomplete
discovery, malformed received/rport, unrelated tuples and expired entries stay
rejected. The test harness also
initializes memory-stat structures when Release builds do not supply statistics,
and supports selecting a single test on Windows without getopt.

The same UDP regression simulates a peer that responds to the combined
delete/add with 200 and no remaining Contact. It requires the single-Contact
retry before success; missing, unrelated and zero-expiry retry bindings must
produce `EPROTO` and backoff, followed by recovery on a fresh cycle. It also
rejects a normal active refresh with no own binding and verifies that explicit
unregistration succeeds without triggering another registration.

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

## TLS and mandatory SDES-SRTP security patches

`re-security.patch` is applied after the NAT/control patches; `baresip-security.patch`
is applied after the REFER/control patches. The SDK manifest records every exact
patch. Adapter build checks reject stale identities/modules/headers. The SBOM hashes
the ordered cumulative patch texts separately for libre and baresip and retains the
selected `srtp` module and existing OpenSSL/BSD license inventory; no new native
cryptography library is added.

libre adds documented native helpers for an outbound certificate verifier,
strong verified-peer evidence, borrowed per-message verified hostname, and a sticky
TLS dialog floor. Callbacks run synchronously on the SIP event thread, borrow
OpenSSL/native objects only for their invocation, and return failure on missing
identity/trust. Account and call error getters preserve numeric originating failures
without parsing localized protocol strings. No native pointers enter Rust snapshots.
The adapter's trust helper uses the existing platform libraries and OpenSSL.

The TLS cache includes target identity. Dialog requests keep DNS/Request-URI separate
from their authenticated same-endpoint TLS identity; the control patch's shared
`drequestv` builder covers both request APIs. ACK replay/CANCEL retain that identity.
Plaintext messages cannot match encrypted dialog/transaction/subscription state or
advance CSeq. Explicit transport selection disables DNS protocol fallback.

Mandatory SRTP has TX/RX RTP/RTCP packet gates, valid crypto/key-length requirements,
secure-state reset and consistent TX/RX suite replacement/rekey. Stream update
failures propagate to the session instead of leaving a gated but apparently usable
call. See [security API](../../docs/api/account-security.md) for configured and
observed state, limitations and focused test commands.

## Private SIP header declaration

`re-security.patch` forward-declares `struct udp_sock` in the private SIP
interface. Its keepalive prototypes use that type even when a caller (ACK or
Contact code) does not include the UDP API. The declaration avoids a prototype-
local incomplete type without introducing an unrelated header dependency. Exact
patch text remains the SDK compatibility guard and participates in the existing
ordered patch hashes in the generated native SBOM. Rebuild via
`bun run native:build` after any tracked patch change.
