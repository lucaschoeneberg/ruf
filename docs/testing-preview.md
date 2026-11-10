# Try the Ruf Developer Preview

Ruf 0.1.0 is a free Developer Preview for people with a test SIP account or PBX.
There are no in-app purchases. Use an isolated test environment and keep another
way to make important calls. Provider and platform qualification is still open.

## Get the preview

Check [GitHub Releases](https://github.com/mardu-systems/ruf/releases) for a
published **Ruf 0.1.0 Developer Preview**. Until that release is published there
is no public installer; use the [source setup](../README.md#run-locally).
GitHub's automatic source archives are not desktop installers.

| Target                     | Installer        | Current qualification boundary                                                                                                       |
| -------------------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| macOS 14+ on Apple Silicon | DMG with Ruf.app | Local build/lab evidence; check the exact release's signing and notarization status. Intel Macs are not a supported target.          |
| Windows 11 x64             | NSIS setup EXE   | Experimental until installed Windows 11 and physical-audio acceptance is recorded. A Windows Server CI build is not that acceptance. |

Compare the downloaded installer with the release's `SHA256SUMS` before opening
it. On macOS run `shasum -a 256 -c SHA256SUMS` with both listed installers present;
on Windows use `Get-FileHash -Algorithm SHA256 .\<installer>.exe` and compare its
hash with the matching line. Checksums detect changes, not publisher identity.
Read the exact package's signing information. Do not disable Gatekeeper or other
system protection to test an untrusted package. No automatic updater is included.

## Connect your test account

1. Install and open Ruf. It starts without a configured account.
2. Open **Settings → SIP accounts**, add an account and enter the server,
   username and password supplied for your test PBX. A separate authentication
   username and outbound proxy are available when your PBX requires them.
3. Match the PBX's transport and media policy. TLS verifies the certificate and
   hostname. Required SRTP needs TLS and fails instead of falling back to
   plaintext. UDP/TCP with ordinary RTP is unencrypted. See
   [account security](api/account-security.md) for exact behavior.
4. Save, then wait for the account to show **Registered**. Registration alone
   does not prove that audio works.
5. In **Sound**, choose the microphone, call output and ringtone output. Grant
   microphone access when explicitly requested. Contacts access is optional;
   you can dial a test extension manually.
6. Call a known echo extension or a consenting test peer. Confirm outgoing and
   incoming audio independently before trying any other actions.

The German equivalents are **Einstellungen → SIP-Konten**, **Angemeldet** and
**Ton**. The app follows the system language or your saved German/English choice.

## Help test compatibility

Run only scenarios available in your controlled test environment. Record each
as passed, failed or not tested; never turn an untested row into a passing claim.

| Scenario                 | What to observe                                                                                        |
| ------------------------ | ------------------------------------------------------------------------------------------------------ |
| Registration and restart | Correct identity restores, failures remain visible and secrets are not printed.                        |
| Incoming/outgoing audio  | Both peers can hear one another; answer, decline and hangup complete on both sides.                    |
| Mute and hold            | The other peer confirms mute, hold and restored audio after resume.                                    |
| Keypad                   | A known IVR/echo service receives digits, star and hash.                                               |
| Direct/attended transfer | The destination and original peer remain connected after Ruf hands the call over.                      |
| Two accounts             | Incoming identity and selected outgoing identity remain correct.                                       |
| Background/recovery      | Hide/reopen, permission denial, sleep/wake and network recovery are observed separately.               |
| Audio hardware           | Record headset/output model, removal/reconnection and a longer test call.                              |
| Security                 | Record the negotiated transport/media status; test certificate rejection only on your own test server. |

Submit a [compatibility report](https://github.com/mardu-systems/ruf/issues/new?template=compatibility_report.yml)
with app version, OS/architecture, PBX version, transport, codec, media protection
and network topology. See the [qualification matrix](release-checklist.md#supported-beta-qualification)
for the larger supported-beta acceptance scope. One successful call does not
qualify every PBX, network or platform.

Reports are public. Use fictional identities and sanitized results; exclude
passwords, real phone numbers, contact lists, addresses and raw SIP/SDP captures.
Use [SECURITY.md](../SECURITY.md) for security-sensitive findings.

## First contributions

- Review the setup instructions on a fresh Mac and report the missing step.
- Exercise the Windows test matrix with a real microphone/headset.
- Exercise the FreeSWITCH matrix with the exact server version and settings.
- Improve reproducible examples and German/English wording after discussing scope.

Follow [CONTRIBUTING.md](../CONTRIBUTING.md). Planned community issues are prepared
in the [launch package](releases/0.1.0-launch.md); they have not been posted yet.
