# SIP event and state contract

Existing commands and `phone-state-changed` return the canonical Rust
`EngineSnapshot`; ts-rs generates the frontend types. No command inputs change.

## Required snapshot fields

| Field               | Type and meaning                                                                                                                                                                                                                                       |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `revision`          | Number; process-monotonic native snapshot-read revision, including engine recreation. Zero before native startup. Ignore deliveries older than the applied revision; equal revisions may be replayed. Not persisted or comparable across app restarts. |
| `call`              | Existing states plus `earlyMedia` (provisional SDP, not established) and `held` (established, remotely held). Both occupy the single call slot. Resume returns to `active`.                                                                            |
| `callConnectedAtMs` | Number or null; native initial establishment time in Unix milliseconds. Null before establishment/while idle; unchanged through hold/resume or window restoration.                                                                                     |
| `callEvents`        | Last 64 significant `CallEvent` objects for the most recent call, oldest first. Preserved after close; reset on a new call. Memory-only, never logged or persisted.                                                                                    |
| `callEventsDropped` | Number of oldest events evicted from this call's buffer, saturating at u32::MAX. Allows consumers to detect retention gaps.                                                                                                                            |

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
