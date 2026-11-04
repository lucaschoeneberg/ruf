# Desktop IPC API

Tauri commands are allowed only for the `main` window through explicit generated
command permissions. There is no HTTP server or HTTP status-code mapping.
Frontend wrappers in `apps/desktop/src/lib/api.ts` reject browser preview explicitly.
Rust `phone-core` types are the contract source; run `bun run types:generate` and
commit the resulting `AppInfo.ts`. CI checks generated-contract drift.

## Commands

| Command               | Input                                 | Result and meaning                                                                                                                                                       |
| --------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `get_app_info`        | none                                  | `AppInfo { version: string, telephony: "notLinked" \| "linked" }`. Reports compiled linkage only; does not start the engine or imply registration.                       |
| `configure_account`   | required `{ account: AccountConfig }` | `EngineSnapshot` after the registration request is queued. Starts the engine on the first request. Idle account replacement reuses that engine; calls block replacement. |
| `get_engine_snapshot` | none                                  | Current password-free `EngineSnapshot`; before configuration, `unconfigured`/`idle`/null. Safe to poll.                                                                  |
| `dial`                | required `{ destination: string }`    | Snapshot after reserving the only outgoing call slot. Requires successful registration and idle call state.                                                              |
| `answer`              | none                                  | Snapshot after queueing acceptance; requires `incoming`.                                                                                                                 |
| `hangup`              | none                                  | Snapshot after queueing rejection/termination; requires a non-idle call.                                                                                                 |

Except for `get_app_info`, commands reject with `PhoneError` on application errors.
Tauri may separately reject malformed arguments, runtime/permission failures or
unknown enum values. Missing required fields and unknown account fields fail
deserialization. Success means queued; poll state for actual native completion.

## AccountConfig (input only)

| Field       | Required/default   | Validation and meaning                                                                                                                                                                                                                                  |
| ----------- | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `registrar` | required string    | DNS hostname or IPv4 spelling, optionally `:port`; total <=255 bytes, host <=253, labels 1–63 ASCII alphanumeric/hyphen, no leading/trailing hyphen. Port contains ASCII digits only and is a nonzero u16. No URI, userinfo, parameters or IPv6 syntax. |
| `username`  | required string    | 1–80 ASCII letters/digits or `_ . + -`. Also used for authentication.                                                                                                                                                                                   |
| `password`  | required string    | 1–1024 UTF-8 bytes, no NUL. Punctuation is preserved through the dedicated native setter. Not returned, serialized, logged or stored in configuration. Debug output redacts it.                                                                         |
| `audioMode` | optional, `system` | `system`: platform microphone/speaker; `testTone`: send 440 Hz and record incoming audio in app cache `received.wav`. Choose before first registration; switching the backend currently requires application restart.                                   |

Only SIP/UDP is enabled for this local development milestone. No silent fallback
from an encrypted transport is offered. Desktop test recordings overwrite the
previous recording and remain on disk until the app cache is cleared; do not use
test-tone mode for private calls. Upstream requires the full recording path to
fit 127 UTF-8 bytes; longer paths return `invalidInput`.

## EngineSnapshot (output)

All fields are required. `lastError` is explicitly nullable, never omitted.

```ts
{
  registration: "unconfigured" | "registering" | "registered" | "failed";
  call: "idle" | "calling" | "ringing" | "incoming" | "active";
  lastError: { code: ErrorCode; message: string } | null;
}
```

`active` means the SIP call is established; it does not prove media. The
[echo test](../../tests/interop/asterisk/README.md) verifies the RTP/audio path.
Dial destinations are 1–64 ASCII digits or `+ * #`; arbitrary SIP URIs are not
accepted yet. Only one account/one call is supported. Answer/hangup capture an internal call generation together with their state check.
If another call replaces it before native execution, the stale action is rejected
as `invalidState` without affecting the replacement call. This identifier remains
internal and does not change the public DTO. Shutdown is tied to app exit;
there is no public IPC filesystem, shell or raw native command interface.

## PhoneError

| `code`              | Meaning                                                                                                     |
| ------------------- | ----------------------------------------------------------------------------------------------------------- |
| `invalidInput`      | Invalid/unsupported field, recording path or backend change.                                                |
| `notRegistered`     | No successful registration before dialing.                                                                  |
| `busy`              | A call occupies the slot or another native engine owns the singleton.                                       |
| `invalidState`      | No configured engine, incoming call or call to end; or the intended call changed before native execution.   |
| `engineUnavailable` | Native SDK feature was not compiled in, or engine stopped.                                                  |
| `nativeFailure`     | Initialization, bounded-queue rejection, native operation, registration, internal state or teardown failed. |

`message` is a safe human-readable explanation. Native failures may include a
numeric native/SIP code, without protocol text or credentials. Error enums are
application codes, not SIP response codes. Asynchronous failures appear in
`lastError`; a new registration/dial clears the preceding error. Invalid new
input does not replace the current account or call.
