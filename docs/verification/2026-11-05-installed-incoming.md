# Installed Ruf: external incoming and background notification acceptance

Tested the installed `/Applications/Ruf.app`, bundle identity
`org.rufapp.desktop`, with the user's external telephone and current saved
account. No account/password was supplied by the diagnostic tooling. No outgoing
call was placed by Ruf before these incoming tests. Private runtime logs and
window screenshots remain outside Git; this report contains no account address,
caller number, credential reference or contact data.

Historical evidence from 5 November 2026. No executable hash or source revision
was recorded for these installed builds. The results below apply only to the
observed installation, not to every package with version 0.1.0.

## Initial permission checks

The installed code signature verified. Ruf's saved notification preference was
enabled. Its General settings reported native OS authorization as allowed.
macOS's Ruf notification panel also had desktop, Notification Center and lock
screen display enabled, with temporary banner style.

## Fresh startup, without an outgoing call

Gracefully stopped the old process and launched installed Ruf using the actual
LaunchServices path, `--background` and the existing opt-in sanitized SIP trace.
The runtime log was created with private permissions in a private temporary
directory. The fresh app had no visible window.

Observed relative native times:

- Successful registration responses at 171 and 231 ms.
- First external INVITE at 72,508 ms, before any transmitted outgoing INVITE.
- Ruf returned 180 Ringing at 72,513 ms; the user confirmed audible ringing.
- External CANCEL at 81,641 ms; Ruf returned 200 to CANCEL and 487 to the INVITE,
  received ACK and closed the call normally.

This directly verifies incoming recognition after a real fresh startup on this
installed account/network path, without an outgoing call priming it. It does
not certify every idle interval, network change or media path.

## Background banner comparison

The first call rang but the user saw no banner. No native notification
initialization/delivery failure was logged. macOS Focus settings showed
Do Not Disturb active. With Do Not Disturb subsequently off, a repeated external
call produced the Ruf notification; the user explicitly confirmed its appearance.
Ruf remained in the background for the test.
No application-side Focus bypass or notification authorization reset was added.

## Separate signaling anomaly and bounded lifetime correction

Two later accepted incoming calls received ACK about 21–24 ms after their 200
responses and ended with a remote BYE. A subsequent call's 200 response instead
advertised a different address scope, repeated nine times without receiving ACK,
and ended locally. This is separate from notification presentation.

The failure interval aligns with expiration of the existing learned SIP NAT
address between successful registration refreshes. A subsequent refresh triggered
a second registration response, consistent with relearning the mapping. The
user also confirmed successful conversation and two-way audio on accepted calls;
the unacknowledged later attempt is not evidence that every accepted call failed.

A real UDP SDK regression reproduced the defect: shorten the original mapping's
lifetime, accept a matching refresh omitting `received`, then observe that lookup
falls back to the local endpoint after the original expiry despite successful
registration. The test failed before the correction and passed afterward.
Renewal now requires the exact live socket/peer mapping, unchanged advertised
endpoint and matching validated numeric port, with the original transaction
checks. Initial discovery, malformed values and expired/unrelated mappings remain
rejected. See the [consolidated NAT evidence](README.md#native-sip-and-audio).

The corrected installed build adds opt-in presence-only Via diagnostics. Its
real registration showed `received=present rport=valued` during discovery and
`received=missing rport=valued` on the following successful response. This
confirms the actual peer uses the compatibility case, without logging addresses
or parameter values. The diagnostic test first failed on the absent new marker,
then passed with 62 allowlisted markers and tracing disabled by default.

The new local ad-hoc build initially required renewed OS credential access.
The existing foreground reconnect action restored registration; settings-file
bytes remained unchanged by installation. No password was requested in chat or
read by diagnostic tooling.

## Acceptance after the old expiry boundary

The corrected process stayed registered without transmitting an outgoing INVITE.
Successive refreshes continued to report missing `received` with valued `rport`;
there was no new discovery response after the original mapping lifetime.
An external test offer arrived about 247 seconds after initial registration,
was answered about 251 seconds after registration, retained the learned address
scope and received ACK approximately 21 ms after the 200 response. Native Active
was observed. The user confirmed the background notification, conversation and
two-way audio. The call then closed normally with an acknowledged BYE.

## Persisted credential artifact and final normal restart

This section records the repair reported for that historical installation; it is
not current recovery guidance. A retained-password save in the current code keeps
the existing key and does not rewrite the secret. Current behavior is documented
in [credential recovery](../api/credential-recovery.md).

A subsequent normal restart still required credential authorization despite the
user confirming Always Allow. Read-only Security.framework metadata checks used
`kSecReturnRef` and suppressed interaction, never `kSecReturnData` or password
lookup. They found the trusted-application reference already matched current Ruf,
while the item's additional code-hash partition still referenced an older build.

Used the existing Settings account Save & Connect flow with its unchanged saved
fields and blank retained-password input. Existing window draft guards were
respected. Ruf itself read and re-stored the existing secret under a fresh opaque
key through its normal journaled credential path; the diagnostic tools never
received the password and did not modify ACLs. Verified the business account,
selected account and all other preferences were unchanged; cleanup had zero
pending entries. Read-only metadata then confirmed the new item's partition
matched the installed executable.

Fully quit and relaunched the same installed build normally, without tracing or
foreground reconnect. The native menu reported the account registered within
the startup polling interval. This verifies automatic restoration for this exact
build after the credential repair. Future ad-hoc rebuilds change their code hash;
stable signed distribution identity and other OS/network configurations were not
certified by this test.

The user's subsequent normal-start acceptance call did **not** ring. Automatic
registration therefore does not establish incoming reachability, and the final
normal-start acceptance remains failed. At investigation time a different
installed process was running from the one observed immediately after the
credential repair; the installed executable itself had not been replaced.
Gracefully restarted that same executable with the existing opt-in sanitized
trace, without rebuilding or changing account settings. Initial registration
again completed successfully in approximately 230 ms. Investigation of the
failed incoming path continues; the earlier successful calls must not be treated
as proof that this later failure is resolved.

## Confirmed registration gap after the failed acceptance

The repeated cold-start call also did not ring. The opt-in native trace showed
successful registration refreshes but no incoming INVITE during that attempt.
The installed process later exited; no new Ruf crash report was present. The
macOS application firewall was disabled and Ruf's application entry allowed
incoming traffic. The registrar route used a VPN interface. No firewall or VPN
settings were changed.

Added an opt-in bounded REGISTER response comparison: Contact count and presence
of the numeric mapped endpoint, excluding explicitly zero-expiry contacts. The
existing diagnostic regression failed when the marker was absent, then passed
with 64 strictly allowlisted markers and default-off tracing. No endpoint or
account values are emitted. Installed this native diagnostic build, then used
the existing unchanged-fields Save & Connect flow to renew credential access
for its changed local code signature.

The actual registrar then returned:

- Initial discovery success: one Contact, mapped endpoint not yet present.
- NAT correction success 44 ms later: **no Contact at all**.
- First ordinary refresh 54 seconds later: one Contact, mapped endpoint present.
- The following refresh again confirmed that endpoint.

This confirms a period where Ruf reports registered despite the registrar not
confirming a current binding. RFC 3261 sections 10.2.3–10.2.4 and 10.3 require a
successful REGISTER response to enumerate current bindings. The hypothesis that
the peer treats the combined old-Contact deletion/new-Contact registration as
deletion is an inference from this sequence; its internal implementation was
not inspected.

The private SDK patch now requires a matching own active Contact before reporting
registration, and retries once immediately with only the new Contact when the
combined correction leaves no binding. Repeated empty, foreign or zero-expiry
confirmation fails with `EPROTO` through the existing failure/backoff path.
The real UDP SDK regression failed before the fix (`expected=3, actual=2`
requests) and passed afterward, including negative cases, normal refresh,
manual recovery and unregister. Both SDK regressions and all six Asterisk
interop tests passed with the final patch. An independent read-only review
found no required corrections. See the [consolidated NAT evidence](README.md#native-sip-and-audio).

Rebuilt and installed the final native bundle; strict signature verification and
byte comparison with the built executable passed. Credential authorization delayed the final installed account save; it was not
bypassed by diagnostic tooling.

A later read-only check observed successful real-account registration in that
final diagnostic process: initial discovery at 744,267 ms returned one Contact
without the mapped endpoint; correction at 744,334 ms returned no Contact;
the immediate single-Contact retry at 744,383 ms confirmed the mapped endpoint.
The previously missing binding was therefore restored approximately 49 ms after
the empty response, without waiting for the ordinary 54-second refresh and
without placing an outgoing call. The long initial interval was credential
authorization, not a network registration delay.

The save journal then had zero pending entries. Read-only normalized comparison
confirmed business account fields, selected logical account and all other
preferences remained unchanged. This is direct post-fix registrar confirmation.
It does not itself prove a subsequent external incoming call or notification.
No subsequent external acceptance call was observed for this final build.

## Checks and artifacts

- Both SDK NAT regressions: passed, including observed RED then GREEN for renewal.
- The complete native workspace suite against the final SDK passed 132 tests
  (adapter 56, lifecycle 1, core 14, desktop 61); ten opt-in cases were excluded.
- The final SDK passed all six Asterisk cases separately with 4.04-second
  approximately 440.1-Hz synthetic RTP echo.
- Native workspace Clippy with warnings denied and diff checks: passed.
- Formatting of eight changed documentation/script files passed. The concurrent
  whole-workspace format check failed on four SBOM files; this historical run must
  not be counted as a passing workspace format check.
- Sanitized diagnostic checker against the final SDK: 64 allowlisted markers,
  default off, passed.
- Native desktop bundle built and installed; signature plus actual audio/contact
  bundle entitlements/usage descriptions verified.
- Private runtime logs and window screenshots were not committed. No caller,
  account, contact or credential material is included in this report.
