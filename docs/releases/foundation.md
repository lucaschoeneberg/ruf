# Native development build

First native baresip integration: one ephemeral SIP/UDP account, G.711 calls,
registration/call status and answer/hangup controls. A local Asterisk lab verifies
registration, call establishment and returned 440 Hz RTP audio.

macOS arm64 and Windows x64 native SDK/installer jobs are configured. Actual
hosted/platform execution must be checked in the corresponding workflow run.
Microphone/headset testing remains manual. No TLS/SRTP, persistence, contacts,
history, calendar, multiple accounts or production signing/update flow yet.

Use only an isolated test network. These are development packages, not a public
production release. Own code is MIT; bundled dependencies retain upstream terms.
