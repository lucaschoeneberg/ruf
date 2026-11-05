# Confirmed native registration after Contact correction — 2026-11-05

The coordinating installed-app diagnostics confirmed a successful initial
REGISTER response with one Contact, followed by a corrective REGISTER response
with no Contact at all. The application nevertheless reported registration
success. The next scheduled refresh, approximately 54 seconds later, returned
the matching active binding. The absence of a confirmed binding during that
interval is observed; the peer's internal handling of the combined old-Contact
deletion/new-Contact addition remains an inference.

## Reproduction and correction

The SDK UDP registrar fixture now simulates the combined correction being
treated as deletion: it accepts the first request, returns 200 with no binding
for the old/new correction, and confirms the new Contact only when sent alone.
Before the fix, `test_sip_nat_registration` stopped after two requests and
reported success. Its new assertion failed with `expected=3, actual=2`.

The minimal change reuses the existing Contact matcher for the client's user,
transport and exact numeric endpoint. An active registration requires a matching
own Contact with positive expiry. A 200 response alone cannot publish success.
When a NAT correction containing the old deletion lacks that binding, the SDK
keeps registration false and makes one immediate request containing only the
new Contact. A second empty, unrelated or zero-expiry response returns `EPROTO`
through the existing failure callback and schedules normal failure backoff.
Ordinary active refreshes lacking an own active binding also fail.

This reflects the complete-binding response semantics in
[RFC 3261 §§10.2.3–10.3](https://www.rfc-editor.org/rfc/rfc3261.html#section-10.2.3).
The one extra compatibility attempt does not reset or increase the existing
two-correction NAT budget. A fresh cycle resets the new retry flag and can
recover. Explicit unregister and terminated/retiring registrations cannot
trigger active recovery, and no wildcard deletion is introduced.

## Observed checks

- Isolated SDK regression: RED with `expected=3, actual=2`, then GREEN after
  the production fix.
- Both isolated SDK NAT test cases passed.
- The UDP registration case verifies successful single-Contact recovery, failure
  and bounded backoff for missing/unrelated/zero-expiry retry bindings, recovery
  on a fresh cycle, rejection of a normal active refresh without its own binding,
  and successful explicit unregister without another registration request.
- `bun run native:build`: passed both SDK NAT regressions and staged the SDK.
- `sdk.json`'s `patches.reSipNat` exactly matches the final tracked patch.
- `bun run test:asterisk`: six interoperability tests passed with the rebuilt
  SDK. Actual RTP echo measured 4.04 seconds, RMS 5811 and 440.1 Hz.
- `bun fmt --check native/patches/README.md native/README.md`: passed.
- `git diff --check`: passed.

Changes are limited to `native/patches/re-sip-nat.patch` and SDK documentation.
The tests reuse the existing fixture and native build targets; no duplicate Rust
SIP fixture or dependency was added. The patch was regenerated from an ignored
isolated source copy without editing the fetched upstream checkout. The shared
SDK build waited for the coordinating agent's clearance. The installed process,
real accounts and OS credentials were not accessed by this regression work.

Actual post-fix installed-app registration and incoming-call acceptance belong
to the separate coordinating test. Windows and older-macOS execution are not
claimed; the existing host OpenSSL deployment-target linker warnings remain.
