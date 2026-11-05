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
| `send_dtmf`           | `call_id: &str`, `digit: &str`       | Queue one ASCII digit, star or hash on the selected established unheld conversation.                                                                                      |

Every argument is required; there are no optional control arguments. Destinations
reuse formatted-number validation: 1–64 ASCII dial characters after removing
supported visual separators, with a leading plus only. They are interpreted
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

## Desktop IPC

All eight controls are exposed as main-window-only Tauri commands with exactly
these required camelCase inputs and a password-free `EngineSnapshot` result:

| Command               | Input                                                         |
| --------------------- | ------------------------------------------------------------- |
| `set_call_muted`      | `{callId: string, muted: boolean}`                            |
| `set_call_held`       | `{callId: string, held: boolean}`                             |
| `transfer_call`       | `{callId: string, destination: string}`                       |
| `start_consultation`  | `{callId: string, destination: string}`                       |
| `switch_call`         | `{callId: string}`                                            |
| `cancel_consultation` | `{callId: string}` — the child consultation token             |
| `complete_transfer`   | `{callId: string}` — the established child consultation token |
| `send_dtmf`           | `{callId: string, digit: string}`                             |

No optional control input or HTTP status codes exist. Deserialization rejects
missing arguments and invalid types; Engine/native guards enforce token ownership,
1–64 numeric/`+*#` destinations and establishment/conflict rules described above.
Handlers reuse `with_session` and `PhoneState::execute` on the blocking session
worker, enrich the returned snapshot, and never expose handles or SIP headers.
Errors use existing `PhoneError` classifications above. Settings has no capability
for these controls. Acceptance only queues work; confirmed state arrives through
snapshots/events. There is no API to cancel a submitted REFER.

The UI keeps a conversation-owned destination draft through queue acceptance,
rejection, timeout and switches. Mute has a per-token pending acknowledgment guard
against repeated stale clicks; errors/call changes release that guard without
inventing confirmation. `holdPending` does not expose desired direction or a
precise recovery phase, so UI status remains generic. `lastCallFailure` is rendered
with its own account owner, independently of a surviving selected dialog, and can
be dismissed for that exact failure. A retained `transferSucceeded` event confirms
terminal success even if both dialogs disappear between reads.

Contact targets use read-only already-authorized `get_contacts` results and exact
non-null `ContactPhone.destination`; opening the form never requests permission.
Native [account security](account-security.md) observations alone drive the call
badge; saved preferences and registration cannot prove encrypted media.

## Snapshot additions

`EngineSnapshot.calls` is a required array containing zero, one or two live
`CallSnapshot` objects in allocation order. Existing `call`, `callId`,
`callAccountId`, `caller`, `callConnectedAtMs`, `callEvents`, and
`callEventsDropped` project the selected conversation. Background-dialog events
never replace its identity or event buffer. Each dialog keeps its own owning
account and ordered event FIFO; the existing call journal observes both dialogs
synchronously even when no frontend poll occurs.

`EngineSnapshot.lastCallFailure` is a required nullable `CallFailure` output:
`{ callId: string, accountId: number, error: PhoneError }`. All three fields are
required. It retains the most recent actual terminal failure with its dialog and
account owner after dialog removal, independently of foreground selection. No
caller/contact data, credentials or native handles are included. It is memory-only,
survives window reloads while the engine remains alive, and resets when a new
incoming/outgoing/consultation call starts or the engine is recreated. Rejected
busy offers do not reset it. Another actual terminal failure replaces it.

Fatal audio failures, failed setup/native dial attempts, final SIP failures and
explicit failed-recovery termination can populate this field. A refused/recovered
control followed by normal remote termination or intentional hangup cannot
manufacture a terminal failure. A background failure does not replace the surviving
selected dialog's `lastError`. Engine-wide native errors without a live dialog
owner remain separate from per-dialog errors and can be returned through the
existing global `lastError` fallback.

All `CallSnapshot` fields are required, including nullable fields:

| Field           | Type                                     | Meaning                                                                                                                |
| --------------- | ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `id`            | string                                   | Opaque, engine-recreation-safe action token.                                                                           |
| `accountId`     | number                                   | Owning account, independent of outgoing account selection.                                                             |
| `state`         | `CallState`                              | Existing establishment states; `held` continues to indicate remote hold.                                               |
| `destination`   | string or null                           | Validated requested outgoing number; null for incoming calls.                                                          |
| `caller`        | `CallerIdentity` or null                 | Existing sanitized, unverified incoming identity; null for outgoing calls.                                             |
| `contact`       | `ContactReference` or null               | Current authorized address-book identity for this dialog; memory-only and nullable when missing, revoked or ambiguous. |
| `connectedAtMs` | number or null                           | Native initial connection time; null before establishment.                                                             |
| `muted`         | boolean                                  | Confirmed suppression of outgoing audio.                                                                               |
| `localHeld`     | boolean                                  | Confirmed accepted local hold; independent of remote hold.                                                             |
| `remoteHeld`    | boolean                                  | Remote hold observation; both hold flags may be true.                                                                  |
| `holdPending`   | boolean                                  | Hold/resume is queued for SDP, negotiating, or recovering.                                                             |
| `transfer`      | `idle`, `pending`, `succeeded`, `failed` | Outgoing transfer lifecycle. Success requires final 2xx sipfrag NOTIFY.                                                |
| `consultation`  | boolean                                  | This is the explicitly requested second dialog. Becomes false when the original ends.                                  |
| `events`        | `CallEvent[]`                            | Last 64 significant events, oldest first, independently sequenced per dialog.                                          |
| `eventsDropped` | number                                   | Saturating count of evicted events.                                                                                    |
| `lastError`     | `PhoneError` or null                     | Safe dialog-specific error. No native headers/reasons.                                                                 |

Added `CallEventKind` tags are `muteChanged` (required `muted: boolean`),
`localHoldChanged` (required `held: boolean`), `controlFailed`, `transferStarted`,
and `transferSucceeded` (no fields). Existing `held`/`resumed` remain remote-only.
Existing `transferFailed` also covers rejected unsolicited inbound transfer.

## Protocol outcomes and cleanup

If another native SDP exchange temporarily owns the dialog (for example an
offerless remote re-INVITE awaiting ACK), hold/resume stays queued with
`holdPending: true` for at most 10 seconds. A native tick retries it when SDP is
available. Queued timeout reports a safe control error and cancels any dependent
consultation/switch continuation; it never silently abandons automatic return.
It leaves the confirmed hold state intact and allows a later explicit retry.
Once sent, hold/resume has a separate 10-second command deadline. The adapter confirms local hold
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
explicit cancellation/return can intentionally resume it. Setup failure, including native failure without any SIP status, is also
reported as a safe operation error on the surviving original, preserving the SIP
status (for example busy/486). Automatic return/resume does not erase that error;
the next accepted explicit action or new call clears it. Explicit cancellation
is not reported as a setup failure. Original
termination leaves the surviving consultation as the ordinary selected call.
Unsolicited second incoming calls remain busy-rejected; raising the native dialog
limit does not accept them. Inbound REFER remains disabled at the account and
out-of-dialog policy boundaries. Outgoing transfer does not authorize remote
instructions to dial arbitrary destinations.

A refused local hold (including SIP 488/491) restores both the previous SDP hold
direction and the existing native receive/player pipeline before reporting the
recoverable control error. Mute and remote hold remain independent. If restarting
that media pipeline fails, only that dialog closes and its native failure remains
in the owned `lastCallFailure`; the application does not report an active usable
conversation. Mandatory-SRTP policy failures are fatal even during hold: see
[account security](account-security.md).

## In-call telephone keypad

`send_dtmf` requires `{callId: string, digit: string}`; `digit` must be exactly
one ASCII 0–9, `*` or `#`. It returns `EngineSnapshot` after queue acceptance,
not confirmation that an IVR received the tone. Only the main window is granted
this command. `invalidInput` rejects malformed/multiple characters;
`invalidState` rejects stale, background or held dialogs; `busy` rejects pending
hold/transfer controls. Engine/unavailable errors retain the existing contract.

Native baresip `call_send_digit` selects negotiated RFC4733 RTP events (through
the existing SRTP path) or SIP INFO. A bounded 32-key queue preserves order, holds
each key for 160 ms and releases it before an 80 ms inter-key gap. Ending/switching/
holding a call or stopping the engine releases the key and discards its remaining
queue. Every execution revalidates the owning generation and selected dialog.
Failures remain native control errors. No browser audio, outgoing DTMF history,
PIN logging or durable digit buffer is introduced. The UI keypad can be shown or
hidden and physical keys operate it only while expanded and outside editable fields.

Dialing and transfer destinations now share `contact_dial_target` validation:
formatting spaces/parentheses/hyphens/dots/slashes are removed, at most 64 dial
characters remain, at least one digit is required and plus is leading only.
International numbers cannot combine plus with star/hash; ambiguous optional
international `(0)`, control characters, letters, arbitrary SIP URIs and post-dial
text are rejected without silently extracting a different number. This does not
change the single-character DTMF command syntax.
