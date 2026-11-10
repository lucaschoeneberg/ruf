# Contacts and complete local call history

Historical macOS verification from 4 November 2026. No personal contacts, call
data, credentials, databases, generated bundles or recordings are committed.

## Behavior and implementation

- Incoming caller recognition uses unique authorized contact-number matches.
  Formatting and explicit international prefixes normalize conservatively;
  withheld, unsupported and multiply owned numbers remain unresolved.
- Contact details expose original-number copying, Apple Contacts, favorites,
  scoped history and keyboard navigation. Existing single-number direct dialing
  and multiple-number chooser behavior remain intact.
- A memory-only cache follows native contact-store revisions, independent
  permission checks and manual/focus refresh. Revocation and failures remove
  contact projections, including retained history pages and contact filters.
  Changed notification captions replace the same bound notice/action token.
- One ordered SQLite worker records actual native start/connection/end events,
  including fast calls and busy second offers. Monotonic connected duration,
  local rejection/cancellation, remote failure and interruption are distinct.
  Startup recovers unfinished rows without inventing end times or duration.
- History supports search, direction/missed filters, paging, details, copy,
  confirmed deletion, clear-all preserving ongoing calls, exact contact links
  and callback through the original currently registered account identity.
- Review fixes cover stale callback number display, scope privacy/navigation,
  notification captions, rapid outgoing cancellation generations and effective
  account identity during a pending native account replacement.

Current contracts are maintained in the [contacts](../api/contacts.md) and
[history](../api/history.md) API references.

## Observed verification

- Matcher, native journal/callback, SQLite and UI regressions were developed with
  observed failures before their fixes; relevant suites passed afterward.
- `bun run format:check`: all files formatted; Rust formatting clean.
- `bun run check`: no Svelte/TypeScript errors or warnings.
- `bun run build`, `bun run check:versions`, `bun run test:ui`: passed (21 UI
  unit tests, 91 assertions). Type generation repeated with identical output hash.
- `bun run test:ui:flows`: all five browser modules passed: contacts, history,
  phone, settings and layout. Includes callback number, source crosslink/scope,
  revoked projections during hidden windows/storage failures, retry offset and
  no surprise navigation after a contact rename. Synthetic 320/340-pixel history
  and contact screenshots inspected; controls remain reachable without overflow.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` and the same
  with `--features sip-desktop/native`: passed.
- `cargo test --locked --workspace`: passed. Native workspace suite passed too,
  including loopback caller/NAT and singleton lifecycle tests. OS-keychain and
  explicit Asterisk cases remain intentionally ignored in the ordinary suite.
- `bun run native:build`: SDK staged successfully before final native checks.
- `bun run test:asterisk`: real registration, call/audio echo and five native
  interop tests passed, including history start/connect/end/404 and immediate
  cancellation. Audio evidence: 4.04 seconds, RMS 5811, approximately 440.1 Hz.
- Synthetic Objective-C check: contact revision increments on a source-change
  notification; invalid clipboard values reject without touching the pasteboard.
- `bun run desktop:build --features native --bundles app`: built signed Ruf.app;
  `bun run test:macos-contacts target/release/bundle/macos/Ruf.app` verified usage
  description, signed Address Book entitlement and strict code signature.
- `git diff --check`: clean.

## Limits

Only calls observed from this version onward are recorded; no PBX backlog is
imported. Contact names are current authorized projections, not stored snapshots.
SQLite/contact-name search scans ordered candidate facts for text/scoped queries;
ordinary paging is indexed. A recording warning remains sticky until restart;
startup storage failure needs repaired access and an app restart.

This run used macOS; Windows runtime and hosted CI were not executed. Native
clipboard mutation, a new macOS consent decision and actual OS banner delivery
were not exercised against personal user state. The first default app+DMG build
failed while hdiutil tried to unmount a busy disk image; the subsequent app-only
build passed. DMG packaging and distribution notarization remain separate.
