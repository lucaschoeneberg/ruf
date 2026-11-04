# SIP events and call state implementation plan

> Execute in the current checkout, preserving the existing contacts, history and notification changes. Use failing native protocol tests before changing behavior, then review the complete patch.

**Goal:** Carry SIP progress, remote hold/resume, DTMF and redirect events through the native boundary, distinguish protocol failures, preserve caller identity and reject unsupported REFER requests.

**Design:** Reuse baresip/libre, the existing synchronized snapshot, CallerIdentity sanitization, call generations, call journal and ts-rs export. Add `earlyMedia` and `held` states plus a bounded, sequenced in-memory event buffer. A full buffer explicitly reports eviction; it is not a durable event log. Existing native errors remain separate from numeric SIP responses. Caller data stays attached to its call through hold/resume and is cleared on termination.

REFER is unsupported: disable advertising it, reject both dialog and out-of-dialog requests, preserve the existing call and never dial a Refer-To target. The pinned baresip out-of-dialog handler ignores its call-transfer setting, so enforce that setting with a minimal tracked SDK patch using the existing pristine-source/staging/manifest pattern. Redirects are surfaced with a safe error and event; targets are not dialed automatically.

## Tasks

- [ ] Add a real UDP REFER regression and observe it accepting the unsupported request before the fix.
- [ ] Extend canonical Rust contracts for early media/hold, sequenced events and typed SIP errors. Verify classification, normal cancellation, native-error separation and bounded retention.
- [ ] Extend the private C event payload and native projection; preserve lifecycle journaling, caller sanitization and generation guards. Cover progress, hold/resume, SIP INFO DTMF, redirect and failure responses with loopback SIP tests.
- [ ] Disable REFER capabilities, apply the narrowly scoped SDK patch and prove dialog/out-of-dialog rejection without transfer or call loss.
- [ ] Update the existing call panel, restored-snapshot caller selection, timer handling, tray labels and German error messages. Add frontend regressions.
- [ ] Document required/nullable DTO fields, event retention, errors and unsupported-transfer policy in docs/api. Generate TypeScript from Rust and verify regeneration is byte-identical.
- [ ] Run format, frontend checks/build/tests, version checks, portable/native Rust checks, Asterisk/diagnostic regressions and desktop build. Perform an independent final review and address substantive findings.

## Review focus

- Stale events must not change another call's state, identity or event ownership.
- Native errno values must not be interpreted as SIP status codes.
- Local rejection and normal CANCEL must not become remote SIP failures.
- Rapid DTMF start/end events must survive snapshot polling; retention limits must be explicit.
- Holding/resuming must not reset call duration, release the call slot or discard caller identity.
- Existing call-history observation and notification behavior must keep working.
