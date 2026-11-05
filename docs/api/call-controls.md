# Native call controls and consultation

These Rust `baresip_adapter::Engine` APIs queue work to the native event loop and
return `Result<(), PhoneError>`. `Ok(())` confirms queue acceptance. Native
snapshots and per-dialog events confirm application or protocol outcomes. No
native call pointers or credentials cross the adapter boundary.

| API                   | Required arguments                   | Behavior                                                                                                                                                                  |
| --------------------- | ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `set_call_muted`      | `call_id: &str`, `muted: bool`       | Suppress microphone/test-tone transmission for this dialog; received audio remains available.                                                                             |
| `set_call_held`       | `call_id: &str`, `held: bool`        | Request local hold/resume through re-INVITE. Resume of a parked dialog first holds the audible conversation.                                                              |
| `transfer_call`       | `call_id: &str`, `destination: &str` | Send blind REFER. A 202 response or provisional NOTIFY does not confirm success.                                                                                          |
| `start_consultation`  | `call_id: &str`, `destination: &str` | Confirm hold on the original, then dial one consultation using its owning account.                                                                                        |
| `switch_call`         | `call_id: &str`                      | Hold the audible conversation, select the target, then resume it.                                                                                                         |
| `cancel_consultation` | `call_id: &str`                      | End the consultation (including while dialing/ringing), select and resume the original. Requires the current consultation token, including when the original is selected. |
| `complete_transfer`   | `call_id: &str`                      | Send REFER/Replaces from the original using the established consultation dialog. Requires the current consultation token, including when the original is selected.        |

Every argument is required; there are no optional control arguments. Destinations
reuse dialing validation: 1–64 ASCII digits or `+`, `*`, `#`. They are interpreted
on the owning account; raw URIs, SIP headers, control characters and arbitrary
remote targets are rejected as `invalidInput`. Opaque `call_id` must match a live
dialog. Cancellation/completion require the consultation token; the original
token cannot authorize an action on a later replacement consultation. Stale tokens and invalid establishment/session states return
`invalidState`. A third dialog or a pending hold/transfer returns `busy` for conflicting controls. Mute and hangup remain available during renegotiation/recovery.
Unlinked builds report `engineUnavailable`. Queue/engine failures report
`nativeFailure`. Native repeats generation, state and pending-operation guards
when executing accepted commands. A racing rejected command reports a safe error
without clearing another operation's pending state. Hangup is available during
pending controls and supports either dialog's token.

## Snapshot additions

`EngineSnapshot.calls` is a required array containing zero, one or two live
`CallSnapshot` objects in allocation order. Existing `call`, `callId`,
`callAccountId`, `caller`, `callConnectedAtMs`, `callEvents`, and
`callEventsDropped` project the selected conversation. Background-dialog events
never replace its identity or event buffer. Each dialog keeps its own owning
account and ordered event FIFO; the existing call journal observes both dialogs
synchronously even when no frontend poll occurs.

All `CallSnapshot` fields are required, including nullable fields:

| Field           | Type                                     | Meaning                                                                               |
| --------------- | ---------------------------------------- | ------------------------------------------------------------------------------------- |
| `id`            | string                                   | Opaque, engine-recreation-safe action token.                                          |
| `accountId`     | number                                   | Owning account, independent of outgoing account selection.                            |
| `state`         | `CallState`                              | Existing establishment states; `held` continues to indicate remote hold.              |
| `destination`   | string or null                           | Validated requested outgoing number; null for incoming calls.                         |
| `caller`        | `CallerIdentity` or null                 | Existing sanitized, unverified incoming identity; null for outgoing calls.            |
| `connectedAtMs` | number or null                           | Native initial connection time; null before establishment.                            |
| `muted`         | boolean                                  | Confirmed suppression of outgoing audio.                                              |
| `localHeld`     | boolean                                  | Confirmed accepted local hold; independent of remote hold.                            |
| `remoteHeld`    | boolean                                  | Remote hold observation; both hold flags may be true.                                 |
| `holdPending`   | boolean                                  | Local hold/resume is awaiting a SIP result.                                           |
| `transfer`      | `idle`, `pending`, `succeeded`, `failed` | Outgoing transfer lifecycle. Success requires final 2xx sipfrag NOTIFY.               |
| `consultation`  | boolean                                  | This is the explicitly requested second dialog. Becomes false when the original ends. |
| `events`        | `CallEvent[]`                            | Last 64 significant events, oldest first, independently sequenced per dialog.         |
| `eventsDropped` | number                                   | Saturating count of evicted events.                                                   |
| `lastError`     | `PhoneError` or null                     | Safe dialog-specific error. No native headers/reasons.                                |

Added `CallEventKind` tags are `muteChanged` (required `muted: boolean`),
`localHoldChanged` (required `held: boolean`), `controlFailed`, `transferStarted`,
and `transferSucceeded` (no fields). Existing `held`/`resumed` remain remote-only.
Existing `transferFailed` also covers rejected unsolicited inbound transfer.

## Protocol outcomes and cleanup

Hold/resume has a 10-second command deadline. The adapter confirms local hold
only after an accepted SIP answer. Refusal and glare (491) restore the previous
local media state immediately. Every final 2xx is ACKed even if SDP is absent or
invalid; malformed answers fail without wedging subsequent renegotiation.

A command timeout rolls back local media but does not pretend that the remote
session is restored. `holdPending` remains true during recovery, and `lastError`
records the timeout with null `sipStatus`. The original SIP transaction retains
ownership, non-2xx transaction ACKs, Contact/route processing and cached 2xx ACKs.
A late accepted obsolete offer is ACKed without applying its SDP, then a native
event-loop tick sends a compensating offer for the latest retained local hold
intent after old transaction cleanup. Accepted compensation clears pending state.
A final refusal of the old request also resolves the uncertainty. Conflicting
hold/transfer actions stay blocked; mute and hangup remain available.

Recovery waits at most another 12 seconds for the old transaction; compensating
renegotiation has its own 10-second deadline. If recovery cannot be confirmed,
the adapter explicitly ends the uncertain dialog and retains a safe final error
explaining that recovery failed. It never reports an unconfirmed restored
conversation. Native session teardown clears call-owned hooks before replacing
callback arguments, and retains upstream transaction cleanup. No alternate
transaction stack, ad hoc ACK builder or detached request ownership is added.
SDP and received audio for the parked call remain disabled while locally held.

Transfer has a 15-second deadline. Final failure, REFER refusal and timeout retain
the original dialog. The native subscription is retired on timeout, and a late
success cannot close that recovered call. Final 2xx NOTIFY confirms success before
local cleanup ends the original and any consultation. Confirmed attended success
is forwarded to both dialogs' event FIFOs before each cleanup, so the final idle
legacy `callEvents` retains `transferSucceeded` even if both close between polls.
This forwarding does not mean that each dialog sent its own REFER. Actual final SIP failures
retain their numeric `sipStatus` and existing classification; local deadlines and
native failures use `nativeFailure` with null `sipStatus`.

A consultation cannot be dialed before original hold succeeds. An already-held
original proceeds using that confirmed state. Failed setup or unsolicited remote
consultation termination returns to the original's pre-consultation hold intent;
explicit cancellation/return can intentionally resume it. Setup failure is also
reported as a safe operation error on the surviving original, preserving the SIP
status (for example busy/486). Automatic return/resume does not erase that error;
the next accepted explicit action or new call clears it. Explicit cancellation
is not reported as a setup failure. Original
termination leaves the surviving consultation as the ordinary selected call.
Unsolicited second incoming calls remain busy-rejected; raising the native dialog
limit does not accept them. Inbound REFER remains disabled at the account and
out-of-dialog policy boundaries. Outgoing transfer does not authorize remote
instructions to dial arbitrary destinations.
