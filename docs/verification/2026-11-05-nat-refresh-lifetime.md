# SIP/UDP mapping lifetime across registration refreshes — 2026-11-05

The deployed diagnostic sequence showed initially working incoming calls, then
an incoming answer advertising the local endpoint instead of the previously
learned translated endpoint. Its successful answer retransmitted without an ACK;
a subsequent registration cycle learned the translated endpoint again. The
existing trace did not expose Via parameter presence, so that sequence alone
does not prove which response fields the real peer omitted.

## Reproduced native failure mode

The tracked SDK patch stores a mapping for twice the negotiated registration
expiry, with a 120-second minimum. Before this fix, only successful REGISTER
responses containing both valid `received` and `rport` renewed that lifetime.
A peer omitting `received` on later successful refreshes could leave registration
green while the mapping expired. Incoming response Contact serialization then
fell back to the local endpoint until another registration discovered translation
again. A 60-second binding refreshed every 54 seconds can therefore outlive its
cached mapping between refreshes.

The SDK's real UDP registrar regression now:

1. Learns a translated port through normal REGISTER and corrective re-registration.
2. Shortens that already learned entry to a one-second lifetime using the existing
   SDK API; no production clock, timeout or cache limit is changed for the test.
3. Successfully refreshes registration with numeric `rport`, echoed translated
   Via sent-by, and no `received` parameter.
4. Waits past the original one-second expiry and requires both exact-peer and
   alternate-port/same-peer-IP lookups to retain the translated endpoint.

With only the regression added, `bun run native:build` failed at the exact-peer
lookup after expiry (`test_sip_nat_registration`, assertion at generated
`test/sip.c:1435`). Registration itself had succeeded. After the minimal validator
change, the complete SDK builder passed both `test_sip_nat` and
`test_sip_nat_registration` and staged a manifest matching the exact tracked patch.

## Validation boundary

[RFC 3581 §4](https://www.rfc-editor.org/rfc/rfc3581.html#section-4) requires a
server to include `received` when processing a requested `rport`. The new path is
therefore a narrow compatibility renewal, not a new discovery rule or a claim
that the missing parameter conforms to that requirement.

The validator now receives the owning SDK SIP instance. It retains the existing
successful REGISTER, UDP, exact peer/source, local destination, Call-ID,
transmitted CSeq and echoed Via sent-by checks. Only an existing live cache entry
for that exact local-socket/peer-endpoint tuple can supply the unchanged mapped
endpoint when `received` is absent. Its mapped address must equal the advertised
sent-by and its port must equal the validated numeric `rport`.

Initial missing-received discovery, expired entries, unrelated local/peer tuples,
missing/invalid/mismatched rport and present-but-empty/malformed received stay
rejected. SDK tests exercise those boundaries. The validator never changes the
cache itself; the existing REGISTER handler installs the confirmed value with
the unchanged negotiated-expiry lifetime calculation. Socket binding, RTP,
remote dialog targets, network recovery and the 32-entry bound are unchanged.

All modifications are in the tracked patch and documentation. An ignored
isolated source copy was used to regenerate the patch; fetched upstream checkouts
were not edited. The SDK manifest continues to enforce exact patch identity.
The changed private C function signature and full argument/return contract are
documented in `native/patches/README.md` and linked from `docs/api/sip-events.md`.
There are no frontend command or DTO changes.

## Checks and limits

- SDK regression: observed RED before the validator change, then GREEN afterward.
- `bun run native:build`: passed; both native SDK NAT test cases passed.
- SDK manifest `patches.reSipNat` equals the tracked patch exactly.
- `cargo test --locked -p baresip-adapter --features native -- --test-threads=1`:
  46 library cases and the real lifecycle integration test passed; the six
  Asterisk-only cases were ignored in that invocation.
- `bun run test:asterisk`: all six interoperability cases passed with the rebuilt
  SDK, including translated incoming signaling, remote ACK/BYE, initial incoming
  before outgoing, multiple accounts, history and stale actions. Actual RTP echo
  measured 4.04 seconds, RMS 5811 and 440.1 Hz.
- `bun run test:network-roam`: six state-machine tests passed with the rebuilt SDK.
- `cargo clippy --locked -p baresip-adapter --features native --all-targets -- -D warnings`:
  passed.
- `cargo fmt -p baresip-adapter -- --check`: passed.
- `bun fmt --check native/patches/README.md native/README.md docs/api/sip-events.md`:
  passed.
- `git diff --check`: passed.

The local SDK linker still warns that host Homebrew OpenSSL objects target a
newer macOS version than the configured 14.0 minimum. That dependency/target was
not changed here, and execution on older macOS or Windows was not observed.
The synthetic reproduction establishes this cache-lifetime failure mode; actual
provider parameter presence and post-fix installed-call acceptance require the
separate coordinating diagnostic run. No real account or credential was accessed
for this regression.
