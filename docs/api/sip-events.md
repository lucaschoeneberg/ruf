# SIP event and state contract

Existing commands and `phone-state-changed` return the canonical Rust
`EngineSnapshot`; ts-rs generates the frontend types. The additive guarded
`reject({callId})` command is documented in the [IPC API](README.md).

See [native call controls](call-controls.md) for independent local hold, mute,
outgoing transfer and the two-dialog consultation snapshot.

## Required snapshot fields

| Field               | Type and meaning                                                                                                                                                                                                                                                                                     |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `revision`          | Number; process-monotonic snapshot-read revision, including reads before native startup and across engine recreation. Zero denotes default or unobserved state. Ignore deliveries older than the applied revision; equal revisions may be replayed. Not persisted or comparable across app restarts. |
| `call`              | Existing states plus `earlyMedia` (provisional SDP, not established) and `held` (established, remotely held). Both occupy a live dialog. Resume returns to `active`.                                                                                                                                 |
| `callConnectedAtMs` | Number or null; native initial establishment time in Unix milliseconds. Null before establishment/while idle; unchanged through hold/resume or window restoration.                                                                                                                                   |
| `callEvents`        | Last 64 significant `CallEvent` objects for the most recent call, oldest first. Preserved after close; reset on a new call. Memory-only, never logged or persisted.                                                                                                                                  |
| `callEventsDropped` | Number of oldest events evicted from this call's buffer, saturating at u32::MAX. Allows consumers to detect retention gaps.                                                                                                                                                                          |

Existing nullable `caller` / `CallerIdentity` and contact matching are reused.
Incoming identity arrives atomically with the incoming offer, survives answering,
active and held states, and clears on termination/new outgoing calls. Restored
windows infer incoming identity from the snapshot rather than a local flag.
Existing privacy, Unicode and length restrictions remain in force.

Each event has required `sequence` (number, increasing modulo 2^32 within the
call), `callId` (opaque string) and `event` (tagged object below). Deduplicate by
callId plus sequence; filter against the current call for current-call display.
This bounded buffer preserves short signals between snapshot reads, not unlimited
or durable history. Stale native generations cannot alter another call's state,
identity or events. The independent existing call journal remains ordered.

| `event.kind`       | Additional required fields | Meaning                                                                     |
| ------------------ | -------------------------- | --------------------------------------------------------------------------- |
| `progress`         | none                       | Provisional SDP/early media.                                                |
| `held` / `resumed` | none                       | Remote hold/resume.                                                         |
| `dtmfStart`        | `digit: string`            | One validated character from `0–9`, `*`, `#`, `A–D`; RTP-event or SIP INFO. |
| `dtmfEnd`          | none                       | Received signal ended.                                                      |
| `transferFailed`   | none                       | Native transfer failure/refusal; see safe lastError.                        |
| `redirected`       | `sipStatus: number`        | Actual 300–399 response; no target URI exported or dialed automatically.    |

## PhoneError

Required nullable `sipStatus` accompanies required `code` and `message`. It holds
an actual final SIP response only. Native errno, validation, keychain, storage and
queue errors have null sipStatus. No raw peer reason text or headers are returned.
The native stack's internal media-failure sentinel (701) remains a visible
nativeFailure with null sipStatus; it is never treated as a SIP response.

| Code                      | SIP statuses                                                 |
| ------------------------- | ------------------------------------------------------------ |
| `sipRedirect`             | 300–399; automatic redirect disabled                         |
| `sipAuthenticationFailed` | 401, 407 when authentication finally fails                   |
| `sipForbidden`            | 403                                                          |
| `sipNotFound`             | 404, 410                                                     |
| `sipTimeout`              | 408, 504                                                     |
| `sipUnavailable`          | 480, 500, 502, 503                                           |
| `sipBusy`                 | 486, 600; distinct from the local `busy` precondition        |
| `sipNotAcceptable`        | 488, 606                                                     |
| `sipRejected`             | 603                                                          |
| `sipFailure`              | Other final 400–699 responses except normal 487 cancellation |

Success/provisional responses and 487 cancellation do not become call errors.
Accepted local rejection/hangup does not become a remote busy/decline error.
New calls clear the previous error. Registration uses the same classification
per AccountSnapshot.lastError; failures without a SIP response use nativeFailure.
An outgoing attempt closed during calling, ringing or early media without a SIP
status or accepted local cancellation reports `nativeFailure` with null
`sipStatus`; for example, native SDP decoding can fail before establishment.
Any previously reported setup error is preserved. Remote incoming cancellation,
termination after establishment and intentional engine shutdown do not acquire
this setup-failure error.

An incoming answer actually sent by native SIP that closes before ACK also
reports `nativeFailure` with null `sipStatus`; its history outcome is `failed`.
Merely queuing an answer does not prove transmission: a racing remote CANCEL
remains normal cancellation. Local hangup before ACK remains intentional.
If native answering fails while the offer is still live, state returns to
`incoming`, the ringtone remains available and the user can retry or reject.

## Media-loss recovery

Established bidirectional (`sendrecv`) audio uses baresip's existing 60-second
RTP/RTCP inactivity watchdog. Sustained loss of received RTP and RTCP closes the native call,
releases its token and the global call slot, and reports `nativeFailure` with
null `sipStatus` (native media sentinel 701). This also permits the existing
idle-only transport recovery to run after a network interruption. New incoming
calls can be accepted without recreating the engine once the path is available.
An outage remains unable to deliver calls while the network is unreachable.

Negotiated hold suspends this watchdog. Resuming restores a full receive grace
interval; prolonged hold must not consume it. The timeout measures received
RTP/RTCP activity, not microphone silence. Continuing RTCP keeps the call alive
even if audio RTP is missing. A peer that stops transmitting both for more than
sixty seconds without negotiating hold will also trigger recovery. No new audio
pipeline, media worker, IPC fields or dependencies are introduced.

## Native SIP endpoint refresh

Successful REGISTER refreshes may renew an already validated SIP/UDP endpoint
even when a peer omits the Via `received` parameter. This compatibility path
requires a live exact local-socket/registrar-endpoint cache entry, matching
numeric advertised Via address, matching numeric `rport`, and the existing
response source/destination, Call-ID, transmitted CSeq and echoed Via checks.
It cannot learn a new address, revive an expired entry or accept malformed
`received`. This keeps an otherwise successful registration from advertising its
untranslated local Contact between cache expiration and the next REGISTER.

Initial endpoint discovery still requires valid `received` and `rport` values.
The private SDK function now takes its owning SIP instance as the first required
argument: `sip_nat_validate(sip, local, peer, sentby, callid, cseq, msg, mapped)`.
Its validation, pointer ownership, failure behavior and lifetime limits are
documented in the [native SDK API](../../native/patches/README.md#native-extension-api).
No frontend command, DTO, error classification or HTTP status code changes.
The SDK manifest records the exact patch and rejects stale SDK linkage.

An active registration requires the successful response to confirm the account's
own Contact, transport, advertised numeric endpoint and positive remaining
lifetime. A `200 OK` without that binding must not become `registered` merely
because the request asked for a positive expiry. This follows the registrar's
[binding-list response contract in RFC 3261](https://www.rfc-editor.org/rfc/rfc3261.html#section-10.2.4).
If a peer drops the binding when the NAT correction combines old-Contact removal
and new-Contact registration, the native client retries once immediately with
only the new Contact. Repeated missing, unrelated or zero-expiry confirmation
fails through the existing native registration error and retry/backoff path.
The retry never uses a wildcard Contact and is disabled for unregister/shutdown.
No command or DTO is added; callers continue to observe the existing registration
states and error model.

## Transfers and background delivery

Accounts disable the existing call_transfer setting and remove REFER from Allow
and unsupported replaces/norefersub capabilities. In-dialog REFER returns 501 via
the existing session handler. Out-of-dialog REFER, including Refer-Sub:false,
returns 405 with Allow excluding REFER. Neither path returns 202, creates a
transfer subscription or dials Refer-To; an established call remains usable.
The minimal tracked baresip-refer-policy.patch enforces the account flag in the
pinned out-of-dialog handler. Exact patch text is recorded in the SDK manifest;
adapter builds reject a stale SDK. A 3xx redirect is surfaced, never auto-followed.

The Rust observer runs independently of window visibility. Failed reads retain
the last valid call/account/notification state and add a safe error. Pending main
thread deliveries coalesce to the latest snapshot; revision-only changes do not
republish. OS authorization, Focus and sleep policies still apply. Window event
delivery is not evidence that the OS displayed a notification.

## Verification

Native adapter tests use actual loopback UDP/libre/baresip for early media,
re-INVITE hold/resume, INFO DTMF, registration/call failures, redirects, REFER,
caller privacy, CANCEL and missing ACK. Test guards serialize native engines.
`bun run test:asterisk` verifies authenticated PBX/RTP echo, multiple accounts,
remote BYE/NAT and stale actions. `bun run test:sip-diagnostics` checks BYE 200
and restricted output. Browser tests verify delivery ordering and restored
caller/duration with synthetic IPC; they do not replace native SIP execution.
