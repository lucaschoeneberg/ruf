# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Product Purpose

A compact SIP softphone for macOS and Windows, implemented with Rust, Tauri 2
and Svelte. The working name is SIP. Original application code is MIT licensed.

## Operating Context

The current verified telephony milestone is a local Asterisk test environment:
up to eight saved SIP accounts, one SIP/UDP call, G.711, per-account registration/call state,
answer and hangup. Native audio stays in baresip. Bun owns JavaScript tooling.

## Capabilities and Constraints

- Reuse existing Tauri IPC commands and generated Rust/TypeScript DTOs.
- No credentials or private contact/call data in Git; passwords are stored only in the operating-system credential store.
- Browser preview cannot register or place calls and must say so.
- Contacts and history have navigation and honest empty states in this UI phase;
  their data backends/connectors and persistence are not implemented.
- Do not imply that mute, hold, transfer, DTMF or contact sync
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
