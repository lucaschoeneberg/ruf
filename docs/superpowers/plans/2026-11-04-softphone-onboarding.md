# Softphone onboarding pass

**Goal:** Apply Impeccable onboard to the existing compact softphone, prioritizing
the user's own SIP account (explicitly confirmed). Preserve dark/compact design,
existing native commands/DTOs and the optional developer test environment.

- [x] Add a skippable/replayable first-run introduction, with only a dismissal
      flag persisted. Adapt it to browser/unlinked/loading/registered states.
- [x] Explain and validate required SIP account fields inline. Use normal system
      audio by default; disclose test audio and recording only when chosen. Keep
      local Asterisk instructions optional. Never persist account/password data.
- [x] Treat a queued registration as pending; show success/first-call guidance
      only from real native registration state. Improve failure recovery, focus and
      truthful contacts/history empty copy.
- [x] Test pure helpers, keyboard/first-run/returning/error/skip/storage states,
      compact sizes, native packaging and unchanged generated contracts. Inspect
      screenshots in a bounded batch, run Impeccable detector once, record relevant
      design behavior and verification, and obtain independent final review.

No tour libraries, analytics, fake contacts/history, native SIP changes or credential
storage. API shapes remain unchanged. This is refinement of the confirmed design.

Review fix: a restored native session must not infer system audio from the new
form default. A browser regression reproduced unsafe enabled calling before the
fix; explicit unknown audio now blocks new activity/reconfiguration until app
restart while keeping hangup/reject. The reviewer scored the P2 resolved.
