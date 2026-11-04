# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Product Purpose

A compact SIP softphone for macOS and Windows, implemented with Rust, Tauri 2
and Svelte. The application is named Ruf. Original application code is MIT licensed.

## Operating Context

The current verified telephony milestone is a local Asterisk test environment:
up to eight saved SIP accounts, one SIP/UDP call, G.711, per-account registration/call state,
answer and hangup. Native audio stays in baresip. Bun owns JavaScript tooling.

## Capabilities and Constraints

- Reuse existing Tauri IPC commands and generated Rust/TypeScript DTOs.
- No credentials or private contact/call data in Git; passwords are stored only in the operating-system credential store.
- Browser preview cannot register or place calls and must say so.
- Contacts read the native macOS address book after explicit system permission,
  with name/company/number search, multiple numbers, local favorite IDs and calls
  through the selected SIP account. Windows and other sources remain deferred.
- History records observed native calls locally with outcomes, duration, original account, search/filter/pagination, contact links, safe callback and deletion. Startup marks unfinished records interrupted without invented duration.
- Do not imply that mute, hold, transfer, DTMF or immediate contact push sync
  exist when no corresponding native API exists.
- Native Windows runtime and hardware audio still require platform tests.

## Brand Commitments

The user supplied a compact dark native softphone reference and confirmed a dark,
compact first UI iteration. The updated full-app reference is the user’s local
Zed Settings interface with GitHub Dark Tritanopia and IBM Plex Sans. The phone
default is now 340 × 500 pixels (minimum 320 × 440); preserve that compact density. The supplied
people/numbers are visual reference material, not application data.

## Product Principles

- Prioritize dialing and reachable call controls in a small window.
- The user explicitly prioritizes onboarding with their own SIP account; local
  Asterisk/test audio is a secondary developer path, never the default.
- Account settings open in a separate native window, with Zed-style navigation, search and setting rows; account management follows Telephone.
- Several accounts can remain registered; select the outgoing account in the compact phone header.
- Describe actual runtime state; never fabricate registration or contact/history data.
- Keep common actions accessible by keyboard and named in German.

## Sound settings

Ton provides microphone, call output and independent ringtone output, ringtone
on/off and a native audible preview. Preferences persist with account metadata.
Use platform default devices unless explicitly selected; unavailable or ambiguous
endpoints remain clear and recoverable. Volume/microphone metering is not part of
this iteration. The developer test tone stays secondary and explicit.

## Shared visual direction

The user explicitly chose Zed’s compact design for the entire application. Phone,
settings, onboarding, contacts/history empty states and call views share one dark
palette, thin dividers, 4px controls and locally bundled IBM Plex Sans. The former
mint phone/separate blue-settings split is replaced. Native window controls and
all real application states remain.

## Interface terminology

Use concise German with direct “du” instructions. “SIP-Konto” names the identity;
“Anmeldung” names registration and never proves that audio works. “Anruf” covers
an attempt or incoming offer; “Gespräch” means an established call. Remote hold
must say that the other party is holding the call. “Verlauf” stores call details,
while “Audioaufzeichnung” is reserved for the explicit developer test-tone mode.
“macOS-Kontakte” names the operating-system app; opening it does not select a
specific contact. Destructive actions name “Anruf” or “Verlauf”. Keep protocol
and diagnostic details in supporting text rather than primary action labels.
