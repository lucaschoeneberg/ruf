# Incoming calls before the first outgoing call

The reported behavior is that an external incoming call is missed after opening
Ruf, but works after placing an outgoing call. Native notification presentation
is a separate reported failure. The provider/network-specific incoming failure
has not been reproduced against the isolated lab.

## Observed native evidence

Added `initial_registration_accepts_incoming_before_any_outgoing_call` in the
adapter's existing Asterisk integration tests. A fresh engine registers synthetic
account 1003, whose server endpoint has `rewrite_contact=no`, and the lab
immediately originates the first call. The test never dials. It requires
Incoming, a bound call/account identity, answer, Active after ACK, and Idle after
remote hangup without a native error.

The isolated diagnostic run observed REGISTER 401, REGISTER 200 and mapped-Contact
re-registration 200 by 17 ms; first incoming INVITE at 178 ms; ringing response at
180 ms; answer at 196 ms; ACK/Active at 198 ms; remote BYE and successful response
at approximately 1200 ms. There was no preceding outgoing call.

Executed:

- `bun run native:build`: passed, including SDK NAT regression cases.
- The new native test alone with the synthetic fixture password and
  `SIP_DIAGNOSTICS=1`: passed.
- `bun run test:asterisk`: passed all six native interop tests, including the
  new startup test. Tone echo measured 4.04 s, RMS 5811 and 440.1 Hz.

The prior NAT/remote-hangup test starts with an outgoing call, so it could not
establish the absence of an outgoing-first dependency by itself. The existing
synthetic loopback incoming tests and the new translated Docker path both accept
an incoming call immediately after registration.

## Source findings and boundaries

Saved-account restoration starts the engine and queues `ua_register` without a
dial or webview listener. Unavailable saved credentials instead create a failed
account for explicit foreground recovery. Snapshot/account mutation and native
callbacks are serialized; a callback cannot race initial account insertion.
Hidden mode starts the same Rust observer. Call state and native notification
handoff do not wait for the webview to subscribe; a short call may finish between
snapshot polls, while the separate native journal retains its lifecycle.

The normal UDP account requests `regint=60`. Libre schedules refresh at 90% of
the returned matching Contact expiry, otherwise the response Expires/default
expiry. A 60-second negotiated binding therefore refreshes after 54 seconds;
the requested interval is not a guarantee of the server's accepted lifetime or
of a router's UDP mapping lifetime. The account does not enable SIP Outbound;
libre's separate registration keepalive is conditional on an outbound registration
and the server requiring that extension.

The existing NAT extension corrects the advertised Contact before reporting
registration success when a validated REGISTER response contains usable numeric
`received` and `rport` values. Missing or invalid values leave the advertised
address unchanged. Mapping reuse remains scoped to the local socket and registrar
peer IP, with an unambiguous same-IP port fallback. No arbitrary external peer
inherits another peer's mapping.

These mechanisms do not establish that a particular provider sends an INVITE to
the correct reachable address after idle time, from a particular ingress edge,
or across a particular NAT/VPN topology. No keepalive, STUN, proxy or Contact
rewriting policy was changed without evidence identifying that failure.

## Remaining acceptance

The installed application still needs a user-driven external incoming call after
fresh launch, before any outgoing call, and another after remaining idle. Distinguish
whether an INVITE reaches the native trace from whether Incoming state, ringtone,
and OS notification appear. This verification did not access personal account
data, place real-account calls, install/restart Ruf, or prove Windows behavior,
OS notification delivery, machine-sleep availability or arbitrary-provider NAT
reachability.

## Desktop restoration recheck — 2026-11-06

The existing native test configures an account directly. Added
`phone::credential_recovery_tests::saved_account_accepts_first_incoming_call_after_each_restart`
to cover the desktop's actual `PhoneSession::load` path too. The test writes
synthetic account metadata once, restores the saved active account, waits for
native registration and accepts the first incoming call. After remote hangup it
shuts down and recreates both desktop state and the native engine, then repeats
against the same unchanged metadata. Neither cycle dials, saves an account or
requests foreground reconnection. The test requires the restored account's
selection and registration timestamp, the correct incoming account identity,
Active after answer and Idle after remote hangup without errors.

Only the credential-store implementation is substituted with an in-memory
synthetic password. Account 1003 authenticates against the actual isolated
Asterisk server, with server Contact rewriting disabled. A negative control
temporarily supplied a wrong synthetic password: Asterisk returned 401
Unauthorized, the account never registered and the test failed at its bounded
registration wait. The test file was restored byte-for-byte and the positive
case rerun successfully. No production behavior change was needed.

`scripts/asterisk-smoke.mjs` now runs this desktop case on macOS and Windows.
Headless Linux explicitly skips it, preserving the existing adapter-only system
dependencies. The native adapter's incoming-before-outgoing case still runs on
Linux. No command, DTO or API contract changed; no API documentation update is
required. The existing account store, credential-store seam, native engine,
call-token APIs and Asterisk fixture are reused without new dependencies.

Observed locally on macOS arm64:

- `bun run native:build`: passed.
- New desktop case: passed both before and through the complete smoke runner.
- `bun run test:asterisk`: passed 16 adapter lab cases, 13 controlled TLS/security
  cases and the new desktop case. Fresh echo PCM: 4.04 s, RMS 5811, 440.1 Hz.
- Native desktop library suite: 87 passed, five explicitly ignored OS/keychain
  or lab cases; the new ignored lab case was executed separately as above.
- `cargo test --locked --workspace`: passed in portable mode.
- Workspace Clippy with `-D warnings`: passed in portable and native modes.
- `bun run check`, `bun run build`, `bun run check:versions`: passed.
- `bun run types:generate`: passed; generated TypeScript remained unchanged.
- `bun run format:check` and `git diff --check`: passed after an initial format
  failure in an unrelated UI review script.
- `bun run desktop:build --features native --bundles app`: passed and exported
  native build/SBOM hashes. Rust's debug-strip helper emitted the existing local
  missing-libLLVM warning; packaging completed successfully.

These results recreate application state and native engines inside a test
process. They do not restart the installed GUI, read a real account password,
exercise the actual OS keychain, or establish provider/VPN reachability, incoming
availability after long idle/sleep, audible ringing, notification presentation
or Windows runtime behavior. External acceptance after a fresh installed-app
launch remains necessary for that provider-specific assurance.

## Installed-app acceptance — 2026-11-06

At the user's explicit request, checked `/Applications/Ruf.app` with the actual
saved account. No Ruf process was running before launch. Started that installed
application with its unchanged persistent configuration and the existing opt-in
sanitized diagnostics. No outgoing call, manual reconnect, password extraction
or account modification was performed. The installed app reports version 0.1.0;
its executable differs from the latest locally built executable, so this result
applies specifically to the installed package. No executable hash or source
revision was recorded for that package; this is not qualification of the latest
build or of every package sharing version 0.1.0.

The allowlisted native trace observed the real registrar's authentication
challenge, successful REGISTER responses and confirmation of the mapped Contact.
After registration, an incoming INVITE reached the app, which sent 180 Ringing.
The caller then cancelled; Ruf acknowledged CANCEL, returned 487 for the INVITE
and closed the call. No outgoing INVITE occurred. The user independently reported
that the test worked. This confirms incoming ringing immediately after this
actual cold application launch, without first placing an outgoing call.

No Active event was observed: the call was cancelled before answer, so this run
does not establish connected speech or microphone/audio quality. Long idle,
sleep/wake and Windows remain untested. Private local logs remain ignored under
`native/build`; no account, caller, credential or network-address data is copied
into this report. The existing adapter diagnostics and installed startup path
were reused; no production code, API or DTO changed.
After the check, quit Ruf gracefully and verified a new process from a normal
LaunchServices launch, ending the temporary diagnostic-enabled process. That
final normal launch was not followed by another external test call.
