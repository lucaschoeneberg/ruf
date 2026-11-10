# Account transport, authentication and media security

`AccountConfig` remains credential-bearing input only (`Deserialize`, never
`Serialize`). Existing `save_account`, `configure_account`, enable and reconnect
commands reuse it; no new production command or trust override is introduced.
All fields below also appear in password-free `AccountSnapshot` and saved account
metadata. Unknown input fields are rejected.

`transport` and `mediaSecurity` may be omitted but cannot be null.
`authUsername` and `outboundProxy` accept omission or null; an empty string is
invalid for either field.

| Field           | Input/default                             | Validation and meaning                                                                                                                                                                                                                   |
| --------------- | ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `transport`     | Optional `udp` (default), `tcp`, `tls`    | TLS requires certificate/host validation and never falls back to TCP/UDP. Plain TCP does not encrypt signaling.                                                                                                                          |
| `mediaSecurity` | Optional `disabled` (default), `required` | Required means mandatory SDES-SRTP and requires `transport: "tls"`; otherwise `invalidInput`. Disabled requests ordinary RTP.                                                                                                            |
| `authUsername`  | Optional string/null, default null        | Null uses `username`. Present values require 1–80 ASCII letters/digits or `_ . + -`, like the existing SIP username. Empty values are invalid.                                                                                           |
| `outboundProxy` | Optional string/null, default null        | DNS/IPv4 host with optional nonzero decimal u16 port. Same 255-byte/label bounds as registrar. No URI, userinfo, whitespace/control bytes, parameters, path, query, headers or IPv6 literal. Its selected transport follows the account. |

Existing registrar, SIP username, password and audio-mode rules remain. Passwords
never enter metadata, snapshots, Debug output or security events. The SIP username
continues to identify the account; an authentication override does not change its
identity. Existing version 1/2 metadata loads with UDP/disabled/null/null defaults;
saves preserve atomic replacement and the existing v1 backup. Startup, disabled
and unavailable-credential placeholders, enable, reconnect and edits retain all
four fields. An edit with an empty password reuses the existing credential key
and does not rewrite its secret; a supplied replacement password follows existing
journaled rotation/cleanup. No automatic migration enables encryption.

## Native observation

Each live `CallSnapshot.security` is a required `CallSecurity` object:

| Field                | Meaning                                                                                                                                                                                                                                                                   |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `signalingTransport` | Actual observed dialog transport `udp`, `tcp`, `tls`, or null before evidence.                                                                                                                                                                                            |
| `tlsPeerVerified`    | True only when the actual live message connection completed outbound TLS certificate/identity verification for this account's registrar/proxy hostname. Fresh inbound TLS without client authentication, unavailable evidence and another verified hostname remain false. |
| `media`              | `unknown` until native evidence/while renegotiating; `unprotected` for an observed ordinary RTP stream; `sdesSrtp` only after native SRTP contexts are ready.                                                                                                             |

Configured settings are not negotiation evidence. Observations belong to a call
and its opaque generation; stale events cannot change a replacement/other-account
call. New calls reset observations. Hold/switch retain the dialog's ownership;
renegotiation clears media evidence until native confirmation. Fatal security
failures remain in `lastCallFailure` after the call closes. A secure context does
not prove successful returned audio, and PBX-hop TLS/SRTP does not guarantee
end-to-end encryption or the identity of the remote person.

## Trust and no downgrade

macOS uses Security.framework server trust policy, including deliberately trusted
private CAs and distrust settings. Windows uses the default CryptoAPI chain engine
and SSL server policy. Both independently check the expected DNS name or IP SAN;
copying a platform root list into OpenSSL is not treated as equivalent OS policy.
Linux loads explicit conventional distro CA bundles and hashed OS CA directories;
the OS CA package is required. `SSL_CERT_FILE` and `SSL_CERT_DIR` do not add trust.
Packaged runtime trust never reads a Homebrew path.
Static OpenSSL build selection remains a separate build-time input.

Private-PBX operators intentionally configure trust through OS trust settings.
There is no per-account CA field: baresip shares its TLS context, so adding one
account's CA there would silently broaden other accounts' trust. No production
CA environment switch or certificate-verification disable option exists.
TLS resumption is disabled. A reconnect/restart is required after trust changes
for already established connections. macOS intermediate network fetch is disabled;
Windows uses cached URL retrieval. No full online revocation guarantee is claimed.

New host-directed connections are cached by endpoint and expected TLS identity.
An established dialog retains its authenticated peer hostname/endpoint when a
valid Contact changes to an IP, so ACK, hold, transfer and BYE can remain on that
verified TLS connection. Another account/hostname cannot borrow this identity.
A changed endpoint requires fresh identity validation. The TLS floor is sticky:
Contact/Route transport parameters, DNS selection, retries, in-dialog plaintext
requests/responses, ACK/CANCEL and transfer NOTIFY cannot downgrade it. Explicit
insecure next-hop parameters fail before signaling payload is sent.
Incoming messages must also match the dialog/transaction's actual verified peer
hostname, before CSeq, SDP or termination processing. A fresh unverified TLS socket
or another verified hostname cannot acquire authority using matching SIP IDs.
An initially unverified inbound TLS dialog accepts messages only on its original
live connection and never reports verified peer evidence.
Its outgoing controls and their replies also remain bound to that original live
channel and endpoint. Endpoint changes or loss of that channel fail closed;
unverified channel continuity cannot establish trust for a new connection.

Mandatory SRTP blocks outgoing and incoming RTP/RTCP before keys are ready,
rejects absent/malformed/unsupported crypto and closes the gate during rekey or
renegotiation. Invalid negotiation propagates as a fatal policy error; the app
never falls back to plaintext media. An established local re-INVITE answer,
remote re-INVITE offer or ACK answer that fails mandatory crypto destroys rejected
contexts and closes only that dialog with owned `mediaSecurityRequired`. Received
final 2xx is ACKed, or an invalid remote offer receives 488, before deferred session
termination/BYE. The callback's session stays alive until its transaction work is
finished. The failed dialog cannot remain active with permanently gated media;
other dialogs and registration remain available. Ordinary peer 488/491 refusal
without a local policy failure retains its SIP classification and recovery path.
Unsolicited inbound REFER remains rejected.

## Safe errors

- `invalidInput`: invalid fields or required SRTP without TLS; rejected before
  native configuration or credential access.
- `certificateRejected`: native TLS certificate chain/expected identity failed.
- `transportFailure`: native TLS/network signaling failure or forbidden downgrade.
- `mediaSecurityRequired`: native mandatory-SRTP negotiation failed; plaintext was
  blocked. Distinct from final SIP `sipNotAcceptable` when no native policy error
  was observed.

The last three have null `sipStatus`; native errno and localized peer text are not
SIP statuses. Actual final SIP replies retain their existing classifications.
A failed account does not alter another account's registration or call ownership.

## Native harness

The explicit `baresip-adapter/native-test-hooks` feature adds the Rust harness-only
`Engine::start_with_test_ca` constructor. It loads exclusive generated test roots
for an isolated engine, never changes OS stores and has no desktop IPC command.
`start_with_test_ca_and_dns` additionally selects a nonzero loopback nameserver;
`start_with_test_dns` changes only that endpoint and retains production OS trust.
These constructors leave the production resolver algorithm unchanged. Invalid
DNS endpoints return `invalidInput`; no production DNS or trust override is added.
Production builds do not enable this feature. Test keys/build output remain ignored.
Run `bun scripts/check-native-security.mjs` after the SDK build for packet gates,
invalid keys/rekey, TLS dialog/CSeq and transaction checks, and
`cargo test --locked -p baresip-adapter --features native-test-hooks security_tests`
for real TLS/SIP fixtures. `--upstream-red` on the C harness verifies the pinned
unpatched source fails the five original policy checks; the additional authority
predicate has a separate pre-fix real-wire regression. The Python fixture requires
Python 3 and the OpenSSL CLI, generates one-day roots/keys under ignored
`native/build`, and runs private loopback TLS/DNS endpoints without OS changes.
`bun scripts/check-native-trust.mjs` checks the exact Linux verifier branch rejects
environment-injected CAs; running that C branch on macOS does not establish Linux
runtime or OS store integration. Native Windows runtime,
Linux distro trust and older macOS runtime require separate observed verification.
