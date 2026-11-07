# Follow-up SIP corrections

Fix all eight accepted follow-up findings, keeping the existing checkout and previous corrections. Reuse native SDP recovery, libre transactions, generation guards and the SIP Lab. No public DTO or dependency change.

- [x] Install permanent scenario regressions from the 52 executed audit cases; strengthen recovery, wire ACKs and surviving-call assertions; observe red baseline.
- [x] Preserve confirmed manual hold intent and defer return while the survivor has an outstanding offer.
- [x] Validate SDP answers, retain ACK obligations after end, handle malformed answers in ACK, and validate/cache REFER NOTIFY replies.
- [x] Retain a bounded native transfer owner after remote original BYE; match surviving peer by generation, retire on outcome/deadline/stop and preserve mute/hangup.
- [x] Rebuild the pinned SDK, verify new/old native regressions and security, then run workspace, frontend, Asterisk, network and desktop checks.
- [x] Obtain one independent review, resolve actionable findings with regressions, update API contracts and verification report.

The user accepted the eight detailed findings and requested all fixes plus permanent tests. Implementation remains within the existing native architecture. Temporary SDP/native source copies and test traces stay outside tracked source. SDK patch changes extend the existing exact-identity reliability patches.

Progress: all eight findings and the three independent-review edge cases are corrected. The original red baseline reproduced 17 failures; blind-result, late-terminal-response and complete-status-line regressions were also observed red before correction. A real GuardMalloc crash verified the late-REFER owner lifetime error before correction. All 66 permanent scenarios and three GuardMalloc cases pass. The final serial native workspace passed 182 adapter tests, core tests and 87 desktop tests. Portable workspace, frontend/browser checks, both Clippy modes, final Asterisk/security, network and both desktop app/DMG builds passed. Generated DTOs are unchanged. Explicitly ignored desktop OS/keychain/lab cases and remaining platform/deployment limits are documented in the verification report.
