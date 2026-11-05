# Application reliability audit and repair plan

**Goal:** Examine the complete application for failures similar to lost events,
stale state, blocked workers, ineffective permission flows and unreliable startup;
repair reproducible defects and verify the resulting application.

**Architecture:** Keep the existing Rust/Tauri/Svelte and baresip boundaries.
Review frontend, native telephony and account persistence independently; review
desktop lifecycle, contacts, history and OS integrations together at their lock
and event boundaries. Preserve the existing uncommitted fixes and generated DTOs.

**Tech stack:** Rust 2024, pinned toolchain, Tauri 2, Svelte, strict TypeScript,
ts-rs, SQLite, native baresip/re and existing macOS/Windows OS bridges.

## Review and implementation

- [x] Read full product implementation in each domain; record coverage and limits.
- [x] Trace external/async boundaries: lock scope, pending work, revision/order,
      cancellation, shutdown, errors, rollback and visibility of actual OS state.
- [x] For each confirmed defect, reproduce a failed regression, make the smallest
      patch using existing APIs/patterns, then verify its recovery and error paths.
- [x] Review cross-domain changes, document changed behavior/API contracts and
      independently review final fixes.
- [x] Run formatting, versions, generated contract stability, frontend/browser
      checks, portable/native Clippy/tests, native OS tests and Asterisk coverage
      where relevant. Build and verify a fresh native desktop bundle.

## Constraints and acceptance limits

No new dependencies without evidence, no speculative SIP/NAT policy changes,
no alternative audio/SIP pipeline, no credentials or personal call/contact data
in artifacts. Use synthetic fixtures and temporary directories. Do not restart
or replace the running installed app during analysis. Windows runtime, physical
audio and the user's provider-specific network require separately observed
acceptance; passing mocks or compilation do not establish them.

Completed implementation, independent review and observed verification are
recorded in [the audit report](../../verification/2026-11-05-app-reliability-audit.md).
The remaining provider/platform acceptance limits are explicit in that report.
