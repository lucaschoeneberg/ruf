# Account and settings reliability audit — 2026-11-05

## Scope and existing boundaries

Read the complete `apps/desktop/src-tauri/src/phone.rs`, `store.rs` and
`credential_recovery_tests.rs`, account/sound command handlers, relevant API docs,
and account configuration/selection/removal interfaces in phone-core and the
native adapter. Existing uncommitted credential-recovery changes were preserved.
No dependencies, IPC fields or command permissions were added by this audit.

The existing `PhoneState` session mutex, foreground credential guard,
`AccountStore`, `CredentialStore`, `AccountConfig::validate`, native account
acknowledgments, and failed-account restoration remain the implementation paths.

## Reproduced failures and changes

1. **Credential cleanup overwrote newer durable account metadata after a failed
   rollback.** A session containing two deletion-journal entries, while the
   metadata file referenced one of them as a replacement account, deleted the
   other obsolete secret and rewrote the file with the stale empty account list.
   `cleanup_does_not_overwrite_newer_durable_accounts_after_rollback_failure`
   failed before the fix, showing the lost account in the resulting bytes.
   Cleanup now stops before deletion or writes when durable and session settings
   differ and surfaces `storageFailure`.
2. **A subsequent preference save could erase that newer durable account too.**
   `preference_save_rejects_stale_metadata_instead_of_erasing_durable_accounts`
   observed an incorrectly successful save before the fix. Normal session writes
   now compare the expected settings with the persisted settings. Account/sound
   rollback compares against the tentative settings before restoring the previous
   version. This protects serialized application writes; it is not a filesystem
   compare-and-swap or a lock against arbitrary external writers.
3. **An invalid saved password aborted restoration of every account.** The native
   test `invalid_saved_password_does_not_discard_other_restored_accounts` used an
   empty in-memory password for one account and a valid synthetic password for
   another. Startup returned `invalidInput` and lost both account snapshots.
   Each restored secret now passes the existing `AccountConfig` validation;
   invalid secrets produce a failed `credentialFailure` account with a safe
   repair instruction, while other accounts continue restoring. Stored metadata
   and secrets are unchanged.
4. **A failed account removal changed the outgoing account.** With two unavailable
   saved accounts, a directory at the temporary-file path deterministically
   rejected the metadata replacement. Native removal/restore ran, but outgoing
   selection moved to the other account. The native regression
   `failed_removal_restores_the_outgoing_account_selection` failed on the wrong
   selected credential key before the fix. Rollback now reselects the restored
   account when it was selected in durable metadata.
5. **Corrupt nonsecret metadata passed the storage boundary.** Empty account
   names/registrars/usernames and a dangling selected key were accepted by
   `AccountStore::load`. The regression
   `metadata_rejects_invalid_account_fields_and_dangling_selection` first failed
   on the empty persisted name. Load now validates names and reuses
   `AccountConfig::validate` with a placeholder password, then validates selected
   account references. Invalid metadata returns `storageFailure` unchanged,
   before starting native telephony or reading credentials. Existing schema-v1
   defaults for absent sound/desktop preferences remain covered.
6. **The same malformed secret received inconsistent error classifications during
   explicit reconnect and an empty-password save.** Native reconnect and portable
   retained-password regressions observed `invalidInput` before the fix. The
   saved-password read/validation path is now shared by startup, reconnect,
   retained-password save, and removal rollback, returning `credentialFailure`
   with an instruction to enter and save a replacement password. Invalid newly
   entered request fields still use `invalidInput`.
7. **Malformed credentials prevented failed-removal rollback from preserving the
   account.** A separate native regression combined the temporary-file write
   failure with a malformed in-memory saved password. Restoration failed with
   `invalidInput`. The unavailable-account fallback now resets its password
   placeholder before validation, keeps the failed account and selected
   credential reference, and returns the original `storageFailure`.

## Executed checks

- Each newly reported regression was observed failing against the prior
  implementation before its production fix.
- `cargo test --locked -p sip-desktop credential_tests -- --nocapture`:
  **6 passed** after metadata consistency changes.
- `cargo test --locked -p sip-desktop store::tests -- --nocapture`:
  **5 passed, 1 ignored** after persisted-field validation.
- `cargo test --locked -p sip-desktop --features native credential_recovery_tests -- --test-threads=1 --nocapture`:
  **9 passed** in the final run, including startup/removal and malformed-secret
  reconnect/rollback regressions, and existing real native cancellation,
  incoming-call, shutdown and unknown-ID cases.
- `rustfmt --edition 2024` ran on the three audited Rust files.
- Final `cargo test --locked --workspace`: **85 passed, 1 ignored**, including
  all account/storage regressions and the notification recovery regression.
- Final `cargo clippy --locked -p sip-desktop --all-targets -- -D warnings`:
  **passed**, including the shared saved-password validation follow-ups.
- An initial `cargo test --locked --workspace` during an independently owned,
  intentional notification RED phase passed the adapter/core/account/store tests
  but failed
  `call_notifications::tests::observing_authorized_permission_recovers_a_failed_live_offer_only`.
  The subsequent complete workspace rerun passed after the root fix.

The native tests used in-memory synthetic credentials, temporary metadata files,
loopback SIP, and the existing test-tone backend. No actual keychain item was
read/written, microphone capture was started, installed app was restarted, or
personal account was used. Native testing was serialized with the native-adapter
agent. These checks do not establish Windows runtime, hosted CI, or physical
microphone/speaker behavior. The root audit records complete application build,
lint, type generation and Asterisk checks separately.

## Follow-up: ordered snapshots before native startup

The settings UI applies snapshots in revision order and accepts equal-revision
replays. Before the first account/engine exists, `PhoneSession::snapshot` returned
revision zero for every read, allowing a delayed earlier response to replace newer
sound preferences. The portable regression
`snapshots_before_native_startup_are_ordered_for_delayed_delivery` observed this
failure before the fix.

The existing read-revision counter moved unchanged from the native adapter into
`EngineSnapshot::with_read_revision` in phone-core. Engine reads and no-engine
session reads now share that counter. A portable adapter regression verifies
ordering across both read paths; the existing native lifecycle/recreation test
also gained assertions before startup, during native ownership, and after
shutdown. Cached error snapshots are unchanged. Zero denotes default or
unobserved state; a fresh successful no-engine read now receives a positive
revision. No IPC field changed. The Rust API comment, generated TypeScript comment,
and `docs/api/sip-events.md` document the ordering contract.

The post-fix portable workspace run passed **87 tests, 1 ignored**. The root
owns execution of the updated real native lifecycle test and rebuilt bundle.

## Remaining operational limits

A settings conflict is preserved and rejected rather than automatically merging
account, sound and desktop state with a running native engine. Restarting reloads
the intact durable file. A new failure reading/writing that file may still require
repairing filesystem access first. OS credential authorization remains governed
by the OS. Live calls retain the existing native guards during any recovery.
