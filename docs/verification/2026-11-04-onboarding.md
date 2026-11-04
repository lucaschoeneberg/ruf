# Impeccable onboarding verification — 2026-11-04

The user requested an Impeccable `onboard` pass across the compact softphone and
explicitly prioritized their own SIP account. The existing dark 360 × 560 design,
Tauri commands, Rust DTOs and native SIP implementation remain in place.

## Findings and changes

The original first run offered a disabled dialer with little context, prefilled
local-lab credentials, default test audio, minimal field guidance, and immediate
navigation after a merely queued registration. Empty-state copy and native
English failures did not consistently explain a useful next step.

The pass adds a skippable/replayable introduction, blank own-account fields,
system microphone/speaker as the normal default, inline field validation and
normalization of server/user whitespace only. Passwords remain verbatim and
clear on submit/navigation. Local test audio/recording and Asterisk instructions
are advanced, explicitly disclosed developer options. The actual unencrypted
SIP/UDP limitation remains visible before connection.

Only a confirmed native `registered` state completes the setup and offers the
first-call hint. Pending/failed feedback receives focus/scroll visibility without
interrupting a user editing a field. Browser, missing-engine and bootstrap-error
states explain their limits and recovery. Contacts/history describe their missing
data connection truthfully rather than promising populated lists.

Only `sip.introduction.v1 = seen` is stored. No account, password or audio setting
is persisted. Blocked local storage falls back to session-only dismissal; a new
process with storage unavailable cannot remember that dismissal.

## Checks observed locally

- Bun helper suite: **10 passing tests, 42 assertions**. New tests first failed
  because the onboarding helper did not exist, then passed. Coverage includes
  normalization without password changes, per-field recovery, native-compatible
  port/username/password bounds, blocked storage and German error guidance.
- Svelte/TypeScript: zero errors/warnings. Frontend build, formatting, frozen
  installation, application/toolchain versions and generated-contract drift pass.
- Isolated Chromium flows pass with zero page errors: first run, skip/replay,
  returning user, blocked storage, validation/focus, pending vs confirmed native
  registration, failure/retry, password clearing/no credential storage, advanced
  audio/lab disclosure, preview/unlinked/bootstrap retry and persistent call
  controls. Compact 360 × 532 and narrow 320 × 472 client areas plus a wide
  preview were inspected. These are mock-IPC UI fixtures, not SIP evidence.
- Native macOS app packaging and launch succeed; an actual 360 × 560 window was
  captured. Existing dismissal state can show the normal dialer on native launch;
  the introduction screenshots are explicitly isolated UI fixtures.
- Native workspace Clippy/tests and GitHub Actions actionlint pass.
- One mechanical Impeccable scan returned 34 advisory design-token mismatches,
  no primary findings. Most are existing component states/annotations. The new
  introduction size and semantic hint/error colors were documented; no ignores
  or broad design-system rewrite were added.

## Independent review and regression

The independent code/visual review inspected ten onboarding screenshots and
reran the Bun suite. It found one material P2: a restored native session could
assume the new form's system-audio default even when the running engine used
test tone. This hid recording uncertainty and could submit an incompatible mode.

A browser regression reproduced the enabled new-call button before the fix.
The fix represents session audio as unknown unless selected during the current
setup. Restored unknown sessions disclose possible test recording, block new
calls/answering/reconfiguration, and keep hangup/reject available. A full app
restart is the recovery; merely reloading the webview cannot recover missing
native metadata. No API change was needed. The regression now passes, the full
onboarding matrix was rerun, and the reviewer scored the P2 **resolved / ship
for the fix scope**, with no material regression found.

## Limits

This was an onboarding refinement, not a new SIP-engine qualification. No live
external SIP-provider account or microphone/headset call was exercised. Earlier
Asterisk media evidence remains separate; Windows, hosted CI and incoming/hardware
SIP qualification remain outstanding. No real-user completion/drop-off metrics
were collected or invented. The existing design-sidecar freshness advisory was
not regenerated as an unrelated design-system migration.

Screenshots, browser harnesses and logs stay in ignored `.impeccable/review/` and
`native/build/`. See [development](../development.md),
[desktop design](../../apps/desktop/DESIGN.md) and [API notes](../api/README.md).
