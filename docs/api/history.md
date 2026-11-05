# Local call history

The main phone window can read and manage the local call history. History is
recorded from native call lifecycle observations, independently of frontend
polling. No PBX import or retrospective call reconstruction is performed.
Portable builds cannot produce native call evidence.

## Commands

All failures use the existing `{ code, message }` `PhoneError` contract.

### `get_history { query } → HistoryPage`

`query` is required; its fields are:

| Field            | Required/default    | Meaning and validation                                                                                                                                                                                                                          |
| ---------------- | ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `search`         | Optional, `""`      | Case-insensitive literal substring in number, SIP peer name, original account name/registrar/username, or currently authorized matched contact name. At most 200 Unicode characters; controls are rejected. `%` and `_` are literal characters. |
| `filter`         | Optional, `"all"`   | `all`, `missed` (incoming unanswered), `incoming`, or `outgoing`.                                                                                                                                                                               |
| `offset`         | Optional, `0`       | Unsigned 32-bit zero-based matching-record offset.                                                                                                                                                                                              |
| `limit`          | Optional/null, `50` | Integer 1–100.                                                                                                                                                                                                                                  |
| `contactNumbers` | Optional, `[]`      | Restrict to any of these original contact numbers; at most 100 supported numbers. Uses conservative number matching, including explicit `00`/`+` prefixes; no country or suffix guessing. Unsupported numbers are rejected.                     |

Unknown query fields and unknown enum values are rejected by deserialization.
Invalid bounds, control characters or unsupported contact numbers return
`invalidInput`. Malformed invocation arguments fail IPC deserialization.

`HistoryPage` contains `entries`, matching `total`, `hasMore` and nullable
`recordingError`. Entries are newest first by start time with a stable opaque-ID
tiebreaker. Offset pagination is not a snapshot: calls arriving between requests
can shift page boundaries. Ordinary paging uses SQLite indexes and LIMIT/OFFSET;
contact and text searches filter ordered candidates against current projections.

Each `HistoryEntry` contains:

| Field                                 | Meaning                                                                                                                                                            |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `id`                                  | Stable opaque random record ID, unrelated to a SIP call token.                                                                                                     |
| `accountName`                         | Original display name; empty if the native account was unavailable.                                                                                                |
| `accountRegistrar`, `accountUsername` | Nullable original identity; no password or credential key.                                                                                                         |
| `direction`                           | `incoming` or `outgoing`.                                                                                                                                          |
| `number`, `remoteName`                | Nullable original selected target/sanitized SIP caller facts. These names are unverified peer input.                                                               |
| `startedAtMs`                         | Observed UTC Unix start milliseconds.                                                                                                                              |
| `answeredAtMs`                        | Nullable actual observed connection time.                                                                                                                          |
| `endedAtMs`                           | Nullable actual observed end time. Unknown crash ends remain null.                                                                                                 |
| `durationMs`                          | Nullable monotonic connected duration supplied by native observation; never calculated from wall-clock timestamps.                                                 |
| `outcome`                             | `inProgress`, `completed`, `missed`, `rejected`, `busy`, `cancelled`, `failed`, or `interrupted`. Missed never asserts that another device answered.               |
| `sipStatus`                           | Nullable actual SIP response status in 100–699; native errno is never stored as a SIP code.                                                                        |
| `contact`                             | Nullable current `{ id, source, name }` authorized address-book match. Ambiguous owners and unavailable access yield null. Never persisted.                        |
| `callbackAccountId`                   | Nullable current registered account ID matching the original registrar (ASCII case-insensitive) and exact username, with a supported dial target. Never persisted. |

History is available without creating or starting a SIP engine. Reading current
account snapshots does not prompt for credentials or contacts. Storage failures
return `storageFailure`; a prior write failure also remains visible through
`recordingError` and engine snapshot `historyError` until application restart.
Telephony continues independently of history storage availability.

### `delete_history_entry { id } → null`

`id` is a required history ID. Unknown IDs return `invalidInput`. An ongoing
`inProgress` row returns `busy` and is preserved. Completed records are deleted
transactionally. The frontend requests confirmation before invoking deletion.
Storage failures return `storageFailure`.

### `clear_history → null`

Deletes completed records transactionally. Ongoing calls are preserved and can
finish normally into their existing rows. The frontend explains this behavior
and requests confirmation. Storage failures return `storageFailure`.

### `callback_history { id } → EngineSnapshot`

`id` is required and must still exist (`invalidInput` otherwise). The phone must
be idle (`busy` otherwise). The original number must normalize with the existing
contact-number/dial rules (`invalidInput` if unavailable/unsupported). The
original registrar and exact username must match exactly one existing,
registered, non-removing account (`notRegistered` otherwise). Renaming an account
does not prevent a match; deleting it or changing its identity does.

The matching account is selected through the existing persisted account-selection
path, then the existing engine dial command reserves the single call slot and
performs its normal validation. No fallback to another account occurs. Existing
selection-storage, registration, engine/native and dial errors propagate. The
returned snapshot includes current `historyError`.

## Events and storage lifecycle

`history-changed` has a null payload. It invalidates cached pages after durable
native start/connect/end changes and deletions. Duplicate native observations
are idempotent. A storage failure also invalidates pages so clients can expose
the recording warning. Contact cache changes invalidate derived projections
through the contacts event; clients should reread history when those change.

A dedicated ordered worker owns `history.sqlite3` in the application data folder;
native callbacks only enqueue owned lifecycle facts and never perform disk or
frontend work. SQLite `user_version=1` schema and indexes are created only for a
new empty database. Corrupt databases and unsupported schema versions are left
in place and reported as failures; they are never silently replaced.

At startup only unfinished records become `interrupted`; observed connection
facts are retained, but unknown end time and duration remain null. No call is
fabricated from a UI snapshot. The worker is attached before account restoration
and drained after native engine shutdown. History stores SIP/account facts only;
current contact identities/names and current callback IDs are never written to
the database. History is local user data and must not be committed or logged.

## Native observation boundary

`Engine::set_call_observer` receives owned password-free `ObservedCallEvent` facts
synchronously in native lifecycle order. The desktop observer only queues them;
it never performs database or UI work under the engine mutex. Observation IDs
are process-unique across engine recreation and independent of persistent history
IDs. Busy additional incoming offers have their own lifecycle without replacing
the occupied call's snapshot. Account facts use the effective native identity
until a queued replacement is acknowledged. Native operation errno values are
separate from SIP response codes. Local reject/cancel intent is recorded only
when the native generation-guarded action was accepted. Observer panics are
isolated from engine state. There is no second audio pipeline or SIP implementation.

An incoming answer actually transmitted by native SIP that fails before ACK is
recorded as `failed`, with no connection time or SIP status. An answer only queued
in Rust followed by remote cancellation remains an unanswered incoming call.
This distinction uses native evidence rather than the optimistic `answering`
snapshot. Established calls ended by the media watchdog retain their observed
connection duration and existing `completed` history classification; the current
engine snapshot exposes the separate `nativeFailure` media error.

## Collection and retention preferences

[Application preferences](preferences.md) expose explicit metadata collection and
retention settings with current-behavior defaults. The ordered history worker
latches collection at each native start event, excludes ignored calls permanently,
and prunes only completed rows strictly before the selected start-time cutoff.
Startup recovery still marks unfinished prior-process rows `interrupted` before
retention. Daily cleanup and configuration failures use the existing independent
history storage warning; SIP continues. `clear_history` keeps its existing
completed-only semantics and confirmation remains in the frontend.
