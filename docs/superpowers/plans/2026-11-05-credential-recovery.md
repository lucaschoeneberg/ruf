# Credential recovery repair

The reported combined error comes from saved-account credential restoration and
an unconditional SIP credential hint in AccountSetup. Local metadata is valid and
readable/writable and the corresponding credential's metadata exists. No password
was requested by the diagnostic check. The running bundle is ad-hoc signed; its
exact OS password-read denial is not known because the current code discards it.

## Implementation

- [x] Distinguish credential access requiring authorization, absent credentials
      and other credential-store failures from application-data errors and SIP errors.
      Map only allowlisted OS categories, never return raw keychain error payloads.
- [x] Add settings-only reconnect_account(id): explicitly authorize reading the
      existing secret outside the phone-state lock, revalidate account/call state and
      reapply the existing native account. Do not rotate/delete credentials, rewrite
      metadata, save drafts or prompt automatically at startup.
- [x] Remove the unconditional wrong-SIP-password hint. Provide a classified
      recovery action and preserve dirty drafts on reconnect success/cancellation.
- [x] Reproduce the misclassification, add safe native credential recovery tests
      and frontend regressions, document the API, run project checks and rebuild Ruf.

Reuse CredentialStore, existing native account application, generation/call
guards, snapshot ordering, ts-rs and scoped Tauri permissions. No dependency or
credential-service migration. No personal secret or account data in logs/tests.

Completed checks and remaining OS acceptance: [verification](../../verification/2026-11-05-credential-recovery.md).
