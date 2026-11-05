# Application reliability audit and repair

This pass examined application implementation across the frontend, desktop
services, platform bridges and native SIP/audio adapter, including unchanged
files. It preserved all existing working-tree changes from earlier repairs.
The focus was event loss, async ordering, blocked workers, startup restoration,
failure visibility and failed-operation rollback. It is not a claim that every
possible runtime fault or dependency implementation has been certified.

## Reproduced and repaired

| Area                   | Defect and verified correction                                                                                                                                                                                                                                                   |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Contact favorites      | A delayed focus read could revert a successfully saved favorite. A post-write read now invalidates older responses using the existing generation guard.                                                                                                                          |
| History scope          | Older same-contact reads could revert current contact name/number filters. Scope requests now require the current generation, including failure paths.                                                                                                                           |
| History pagination     | Calls inserted between offset requests caused duplicate keyed rows and an actual Svelte exception. Merge by stable IDs. Track raw consumed server offsets separately so a completely overlapping page cannot trap load-more at the same offset.                                  |
| Settings consistency   | Cleanup after failed rollback could replace newer durable accounts with stale memory. Writes/cleanup compare the expected durable state; mismatches preserve metadata and referenced credentials rather than overwriting them.                                                   |
| Startup accounts       | A malformed saved password could abort restoration of every account. Preserve that account as failed, continue other accounts and consistently classify unusable retained credentials as credentialFailure.                                                                      |
| Removal rollback       | A failed metadata write could change outgoing selection. Restore the original selected identity; reset unusable saved secret contents to a safe placeholder before unavailable-account restoration, retaining the failed account even in the combined failure case.              |
| Metadata validation    | Invalid account fields and a selected key referring to no saved account were accepted. Validate these before starting native telephony or reading secrets, preserve corrupt bytes and return storageFailure. Existing legacy defaults remain.                                    |
| Native call errors     | A real malformed-SDP decoder failure closed outgoing setup silently. Zero-status non-local setup closure now exposes nativeFailure with null sipStatus. Existing errors remain; incoming cancellation, local cancellation, shutdown and established hangup do not invent errors. |
| Contacts authorization | A missing OS consent completion retained the contacts gate forever. Add a three-minute explicit-consent deadline; retained callback state stays safe after return and later requests work.                                                                                       |
| Notification recovery  | After the user granted notification access in OS settings, an exhausted still-ringing offer stayed latched. A foreground settings read can re-enable bounded delivery for a failed unclaimed offer, preserving its token and avoiding duplicate successful notices.              |
| Recovery guidance      | Unusable stored-password errors now explain retry and password replacement rather than only repeating access to the same malformed secret. No server authentication result is invented.                                                                                          |

The final ordering review also reproduced a pre-engine state defect: successful
desktop snapshots before the first engine used revision zero, so a delayed old
response could overwrite newly saved sound state. The existing process-wide
counter now belongs to the shared snapshot contract and orders both pre-engine
and engine reads, including engine recreation. Cached error fallbacks retain
their earlier revision. The focused pre-engine regression failed on zero == zero
before this correction.

Each defect had a failing focused regression before its correction. Independent
review additionally found the combined malformed-secret/removal failure and the
full-page pagination stall; both were reproduced, fixed and re-reviewed with no
remaining required finding.

## Coverage and changed files

- Full frontend coverage and browser evidence:
  [frontend audit](2026-11-05-audit-ui.md). Changed `App.svelte`,
  `lib/components/ContactsPanel.svelte`, `HistoryPanel.svelte`, `lib/api.ts`,
  browser contact/history regressions and the onboarding error test.
- Full account/session/storage coverage:
  [account audit](2026-11-05-audit-accounts.md). Changed desktop `phone.rs`,
  `store.rs`, and `credential_recovery_tests.rs`; read all command handlers.
  The late read-ordering fix also changes the core snapshot method, adapter
  snapshot stamping and existing native lifecycle ordering assertions.
- Full native SIP/audio/core phone contract coverage:
  [native audit](2026-11-05-audit-native.md). Changed adapter `ffi.rs` and a
  native regression in `sip_event_tests.rs`; retained upstream/native ABI.
- Coordinator reviewed complete desktop `contacts.rs`, `history.rs`,
  `history/database.rs`, `desktop.rs`, `shell.rs`, `shell_policy.rs`, `lib.rs`,
  `call_notifications.rs`, core contacts/history/desktop contracts, adapter
  contacts and macOS/Windows notification implementations and payload helpers.
  Changed desktop `call_notifications.rs`/`desktop.rs` and native
  `contacts.m`/`contacts.h`. Added `native/tests/contact_timeouts.m`,
  `scripts/check-macos-contact-timeouts.mjs` and its `package.json` command.
- Desktop build wrapper, Tauri window/security/capability configuration,
  generated command permissions, usage descriptions/entitlements, Windows
  installer hooks, existing queue/deadline/native smoke and history database
  regressions were inspected. No additional production change was warranted.

Existing session/credential locks, deletion journal, native restoration,
UserNotifications worker, action tokens, request generations, stable history IDs,
AccountConfig validation, ts-rs contracts and existing test runners were reused.
No new dependency, parallel SIP/audio pipeline, command, permission or DTO field
was introduced by this audit.

## API documentation

- `docs/api/README.md`: validated persisted metadata, expected-state replacement
  and recovery of a failed ringing notification after observing OS authorization.
- `docs/api/credential-recovery.md`: malformed retained secrets, consistent errors,
  per-account restoration and selection/unavailable-account rollback.
- `docs/api/contacts.md` and native C header: authorization deadline, failure and
  late callback semantics; isolated test command.
- `docs/api/sip-events.md`: zero-status local setup failure versus SIP responses,
  cancellation, interruption and established termination.

## Executed verification

- All five browser suites passed, including delayed response, duplicate-key,
  complete page overlap, permission and draft-preservation regressions.
- Frontend unit tests: 42 passed. Svelte/TypeScript check and production build
  passed. Versions agree and repeat type generation was byte-identical.
- Portable workspace tests: 87 passed, one real credential-store test ignored.
  Portable and native workspace Clippy, all targets, warnings denied: passed.
- Native account/credential recovery regressions: nine passed without accessing
  the actual credential store. Native adapter tests: 46 passed, six opt-in lab
  cases excluded from that command; native lifecycle test passed.
- Final native workspace run: 120 passed (46 adapter, 59 desktop, one native
  lifecycle and 14 core contract tests), ten opt-in tests ignored there. The six
  ignored adapter interop cases passed separately in the final Asterisk run.
  Native lifecycle assertions also verified ordering before engine startup and
  across shutdown/recreation. Real OS-keychain write tests remained opt-in.
- SDK rebuilt successfully. Asterisk: all six interop cases passed separately,
  including cold-start incoming before any outgoing call. RTP echo measured
  4.04 seconds, RMS 5811 and approximately 440.1 Hz. Network recovery: six passed.
- Native Contacts stalled/late authorization and notification timeout/withdrawal
  tests passed using fake services. Isolated macOS OS notification delivery,
  categories, synthetic action responses and withdrawal passed.
- Formatting and `git diff --check` passed. The initial integrated formatter
  identified the newly expanded frontend report; it was formatted and rechecked.
- `bun run desktop:build --features native --bundles app`: passed on the final
  source, producing `target/release/bundle/macos/Ruf.app` (19.56 MiB, arm64).
  `codesign --verify --deep --strict` passed. The contact and audio bundle checks
  verified signed entitlements and usage descriptions; they do not claim actual
  contact access or microphone capture. The bundle is ad-hoc signed and not notarized.
- At the final read-only comparison, the executable in `/Applications/Ruf.app`
  matched the final bundle byte-for-byte. This audit did not replace or restart
  the installed application; restart Ruf fully before acceptance so a previously
  running process cannot retain older code through the single-instance behavior.

## Remaining runtime boundaries

No personal account calls, keychain entries, address book records, microphone
capture, installed-app replacement or process restart were used. There is no
hosted-CI or Windows runtime claim. Hardware audio/device hot unplug, machine
sleep and the user's provider/VPN/NAT path still need direct acceptance.

The reported incoming-only-after-outgoing symptom still did not reproduce in the
fresh-registration lab test; no speculative NAT/keepalive/proxy policy was added.
The isolated notification smoke has a different OS identity from installed Ruf.
Full source/test review does not replace installed Ruf permission and incoming
call acceptance. Conflicting externally edited settings are preserved and require
reload/restart; the consistency guard is not cross-process filesystem CAS.
