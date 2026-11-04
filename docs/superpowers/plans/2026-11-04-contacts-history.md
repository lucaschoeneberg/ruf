# Contacts and complete call history implementation plan

> Use Superpowers task execution with explicit subagent file ownership in the
> existing shared checkout. Preserve the other agents' caller/notification and
> branding work; no commits, resets, broad formatting or automatic consent changes.

**Goal:** Authorized contact-name resolution on incoming calls, useful contact
actions/keyboard navigation, native address-book invalidation, and complete local
call history with safe callback, search/filter, contact links and deletion.

**Spec:** User's explicit request in this chat plus the Contacts/History sections
of ../specs/2026-11-04-sip-mvp-design.md. macOS is the native contact source; history
works wherever the existing phone engine is supported. Other providers remain out.

**Architecture:** Native lifecycle observations are sent to a dedicated SQLite
worker independently of UI polling. phone-core owns event/IPC contracts. The
desktop caches only currently authorized contacts in memory and projects matches
into caller/history views and system banners. Svelte uses existing generated
DTOs, command wrappers, compact list patterns and the current dial path.

## Decisions

- Use the already proposed SQLite/rusqlite design for durable transactions,
  crash recovery, indexed queries/pagination and deletion. JSON settings/favorite
  files are reused unchanged; a single versioned history database is new.
- Record actual native start/connect/end events, including short calls and busy
  second offers. Outcomes: inProgress, completed, missed, rejected, busy,
  cancelled, failed, interrupted. Native errno must never become a SIP code.
- Timestamps are UTC Unix milliseconds; duration comes from monotonic observed
  connection time. Startup marks unfinished records interrupted without inventing
  an end time or duration. No PBX-history import or retroactive fabricated calls.
- History stores SIP/account facts. Current macOS contact names/IDs are runtime
  projections, never stored in the database; revoked access clears the projection.
- Account association uses original registrar/username identity and display name;
  callback requires that registered account and never silently uses another.
- Contact matching is conservative: preserve extensions/service codes, normalize
  visual separators and explicit international prefixes, never guess a country
  or match short suffixes; ambiguous owners produce no match.
- Delete completed entries with explicit UI confirmation. An ongoing record
  cannot be deleted; clearing history preserves ongoing calls and says so.
- Native contact changes invalidate the memory cache; focus/manual refresh and
  permission checks remain a fallback. No repeated consent prompts.

## Shared interfaces (canonical in phone-core/src/history.rs)

- `ObservedCallEvent { observation_id: u64, at_ms: u64, kind: ObservedCallKind }`
- `ObservedCallKind::Started { account: Option<AccountSnapshot>, direction,
number: Option<String>, remote_name: Option<String> }`, `Connected`,
  `Finished { outcome, sip_status: Option<u16>, duration_ms: Option<u64> }`.
- Adapter API: `Engine::set_call_observer(Option<Arc<dyn Fn(ObservedCallEvent) + Send + Sync>>)`.
  Sink only queues a message; no disk, Tauri or frontend work in native callbacks.
- DTOs: `HistoryEntry`, `HistoryQuery`, `HistoryFilter`, `HistoryPage`,
  `CallDirection`, `CallOutcome`, `ContactReference`; definitions already scaffolded.
- New snapshot projections: `callerContact` and `historyError`, both nullable.
- IPC: `get_history { query: HistoryQuery } -> HistoryPage`,
  `delete_history_entry { id } -> null`, `clear_history -> null`,
  `callback_history { id } -> EngineSnapshot`. Main only.
- `history-changed` event has null payload and invalidates frontend pages.
- Contacts helper parent supplies `contacts::cached_contacts(&AppHandle) -> Vec<Contact>`
  (memory only), core `resolve_contact(&[Contact], &str) -> Option<ContactReference>`
  and `contact_match_key(&str) -> Option<String>`.

## Tasks and ownership

- [x] Native lifecycle agent: adapter engine/ffi/bridge and a focused journal
      module/tests. Reliable observed start/end metadata, local intent, busy offers,
      monotonic connected duration, synchronous ordered nonblocking sink.
- [x] History backend agent: desktop history module/SQLite worker, schema and
      recovery, query/delete/callback commands, PhoneSession sink attachment, API docs,
      storage tests. Parent integrates command registration/setup/shutdown.
- [x] History UI agent: HistoryPanel.svelte, history-api.ts, history helpers/tests
      and standalone browser regression. Parent integrates App routing/contact scope.
- [x] Parent: contact matcher + tests, change observer/revision bridge, memory
      cache/enrichment, caller projection/banners, copy/open-app/details/history
      actions and keyboard navigation, App/event integration, generated exports.
- [x] Assemble permissions/docs and run tests, typecheck/build/versions/format,
      type-generation stability, Rust clippy/tests, SDK/native/asterisk checks and
      native bundle verification; independent whole-feature review and targeted fixes.

## Review focus

Fast start/end transitions between UI polls; restart and storage failures;
anonymous or ambiguous callers; denied/revoked contact access; stale callback IDs;
deleted/renamed/unregistered accounts; local reject versus remote cancel; concurrent
deletion and active records; event ordering; query validation/pagination; selected
number copy; keyboard actions and small-window controls. Use synthetic fixtures;
no personal contacts/history in Git or diagnostic logs.
