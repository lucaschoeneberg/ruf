# Saved credential recovery

Local credential access is separate from SIP authentication and settings-file
access. A failure reading a saved password during startup does not establish
that the telephone system rejected the username/password. Startup continues to
suppress OS credential UI and preserves the account as visibly failed.

## reconnect_account

Settings-window command; main and browser previews have no permission.

| Input | Required | Validation                                                                                                                             |
| ----- | -------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `id`  | u32      | Existing saved, non-removing account ID. Zero/unknown IDs reject with invalidInput; malformed/missing arguments are rejected by Tauri. |

Returns password-free `EngineSnapshot` after native account configuration is
applied and acknowledged. Registration remains asynchronous: observe that
account's registration and lastError rather than treating command acceptance as successful SIP login.
No HTTP status mapping exists.

Only an explicit foreground action invokes this command. It reads the existing
saved password with OS authorization UI allowed, outside the phone-state lock.
Live call observation remains available while the system dialog is pending.
Other foreground credential actions are serialized using the existing guard.

The call must be idle both before and after credential access. After a prompt,
the command rechecks shutdown, account ID, saved credential reference and account
metadata. If a call arrived or the account changed, recovery fails rather than
replacing a live call/account. The native event loop repeats its existing call
guard when applying the account.

The saved account must be enabled; disabled accounts return `invalidState` before
credential access. A registration failure is not required: an enabled, already
registered account can also be reconnected.

Recovery never saves form drafts, rewrites settings, rotates/deletes a credential
or changes outgoing account selection. Cancellation/failure leaves saved state
intact. Passwords are never returned to a webview. Existing `save_account` remains
the route for intentionally supplying/changing a password; its empty-password
update still reuses a saved credential.

## Error codes

All have null sipStatus. Raw keychain error payloads, native reason strings and
secrets are never returned or logged.

| Code                                  | Meaning and recovery                                                                                                                                                                                                                                                                                                                   |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `credentialAccessRequired`            | OS authorization/unlocking is required, was denied or cancelled; also used when background access defers during foreground authorization. The user can choose Erneut verbinden and allow the OS dialog. Known macOS auth/interaction errors are classified separately from SIP auth.                                                   |
| `credentialMissing`                   | No matching saved credential exists. Enter the SIP password again and save; reconnect cannot invent a password.                                                                                                                                                                                                                        |
| `credentialFailure`                   | Credential-store initialization, availability, encoding, unusable saved password contents (empty, NUL or beyond the supported bound), or an unknown operation failure. Retry explicit access or enter and save a replacement SIP password; reconnect cannot repair malformed stored contents. This is not proof of a server rejection. |
| `storageFailure`                      | Settings/application-data file access failed; inspect the app-data location. It is no longer the classification for system credential operations.                                                                                                                                                                                      |
| `invalidInput`                        | Unknown saved account ID or invalid explicitly supplied account/password fields. Unusable contents read from an existing credential are classified separately as credentialFailure.                                                                                                                                                    |
| `busy`                                | A call occupies the phone before access or arrived before native application.                                                                                                                                                                                                                                                          |
| `invalidState`                        | The saved account is disabled, account/credential metadata changed during authorization, removal is pending, or shutdown started.                                                                                                                                                                                                      |
| `nativeFailure` / `engineUnavailable` | Native queue/configuration failed or this build lacks native telephony.                                                                                                                                                                                                                                                                |

Other commands using the existing credential boundary can also return the three
credential-specific errors. Actual server authentication failures continue to use
the independent sipAuthenticationFailed/sipForbidden classification and numeric
SIP status from native responses.

## UI and persistence

AccountSetup displays the classified error without appending a universal
wrong-password hint. Saved failed accounts with access/failure errors expose an
explicit retry button, disabled while busy/removing/in a call. A missing saved
password requires entry before saving. Dirty name/server/username/password drafts
survive recovery success and cancellation; reconnect uses the saved account ID,
not draft fields. No new persisted fields, service migration or automatic popup.
An unusable password returned by the OS during startup preserves that account as
failed without preventing restoration of other accounts. Reconnect and an update
that retains the stored password report the same credential-specific failure.
If a removal must roll back after a settings-file failure, even such an account
is restored visibly failed and its original outgoing selection is preserved.

The existing keyring store and service identity are reused. Authorization remains
under OS access controls; recovery does not broaden an item's trusted-app list or
disable credential protection. Development ad-hoc builds may need OS approval
again when their executable identity changes.

A save with a blank retained-password input reads the existing credential with
foreground authorization and keeps its opaque reference without rewriting the
secret. It does not repair that item's code-signing access controls. To store a
replacement credential under a new opaque reference, explicitly supply the SIP
password and save; the existing journaled save path then cleans the old
reference. Reconnect performs no rotation. The app does not rewrite keychain ACLs
or provide a blanket credential-access bypass.
