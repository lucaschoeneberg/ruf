# Saved account activation

`set_account_enabled` requires `{ id: number, enabled: boolean }` and returns
`EngineSnapshot`. `id` is an existing nonzero session account ID; it is not a
credential key. There are no optional input fields. This local Tauri command
uses `PhoneError`, not HTTP status codes.

Disabling retires the native user agent through the existing event queue and
waits for its acknowledgment. It retains saved metadata, credential key,
password and selected outgoing identity. Enabling reads that saved password
only after this explicit foreground action, outside the phone-state lock, then
creates/registers the same account identity. An acknowledgment confirms native
application of activation, not successful SIP registration or an accepted
unregister response. Registration remains a separately observed state.

`AccountSnapshot` adds these required output fields:

| Field                            | Meaning                                                                                                                                                                                                             |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `enabled: boolean`               | Saved activation intent; existing/new accounts default to true. False accounts do not register.                                                                                                                     |
| `enabledChanging: boolean`       | Native account configuration/activation acknowledgment is pending; dialing is blocked.                                                                                                                              |
| `registeredAtMs: number \| null` | Unix milliseconds of the latest real native registration-success event. Null before confirmation and after replacement/disable. Never fabricated from a queue acknowledgment or stored as a reachability guarantee. |

On startup, disabled accounts restore nonsecret display metadata without reading
credentials or allocating native user agents. Their registration is
`unconfigured`, without a manufactured credential/registration error. The saved
selected account remains selected even when disabled; dialing then returns
`notRegistered`. The application does not select another caller identity.
Editing/saving a disabled account preserves its disabled state. Removal remains
a separate, explicitly destructive operation.

`busy` rejects activation during an incoming, queued outgoing, answering or
established/held call, or while another native update is pending. The native
queue repeats the idle guard; a call that wins the race rejects the update and
restores the previous snapshot and durable activation intent. `invalidInput`
rejects unknown/removing accounts. `invalidState` rejects shutdown, changed
account identity during credential authorization, and a superseded activation
request. A newer activation intent is recorded before dispatch; an old
credential callback cannot enable an account after a newer disable request.
Credential errors preserve their existing `credentialAccessRequired`,
`credentialMissing` and `credentialFailure` classifications. Metadata failures
return `storageFailure`. Failure to restore durable metadata after native
rejection is surfaced as a storage warning/error, never treated as success.

Other accounts, selection and preference drafts remain usable while foreground
credential authorization waits. Activation does not rotate, rewrite or delete
credentials. Short native application/metadata commits use the existing session
serialization and atomic account-store replacement. An idempotent activation
request does not read credentials or re-register the account.

`reconnect_account` retains its existing `{ id } → EngineSnapshot` contract and
supports any enabled account with a registration failure. It explicitly
reauthorizes credential reading outside the state lock without saving drafts,
rewriting metadata, rotating keys or changing selection. Disabled accounts
return `invalidState` before any credential read; enable them explicitly first.
