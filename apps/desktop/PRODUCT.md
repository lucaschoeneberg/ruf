# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Product Purpose

A compact SIP softphone for macOS and Windows, implemented with Rust, Tauri 2
and Svelte. The working name is SIP. Original application code is MIT licensed.

## Operating Context

The current verified telephony milestone is a local Asterisk test environment:
one ephemeral account, one SIP/UDP call, G.711, explicit registration/call state,
answer and hangup. Native audio stays in baresip. Bun owns JavaScript tooling.

## Capabilities and Constraints

- Reuse existing Tauri IPC commands and generated Rust/TypeScript DTOs.
- No credentials or private contact/call data in Git; passwords remain ephemeral.
- Browser preview cannot register or place calls and must say so.
- Contacts and history have navigation and honest empty states in this UI phase;
  their data backends/connectors and persistence are not implemented.
- Do not imply that mute, hold, transfer, DTMF, account switching or contact sync
  exist when no corresponding native API exists.
- Native Windows runtime and incoming/hardware audio still require platform tests.

## Brand Commitments

The user supplied a compact dark native softphone reference and confirmed a dark,
compact first UI iteration at approximately 360 × 560 pixels. Preserve that
information density; improve typography, controls and navigation. The supplied
people/numbers are visual reference material, not application data.

## Product Principles

- Prioritize dialing and reachable call controls in a small window.
- Hide account configuration behind an explicit settings action.
- Describe actual runtime state; never fabricate registration or contact/history data.
- Keep common actions accessible by keyboard and named in German.
