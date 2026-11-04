# Compact Softphone UI Implementation Plan

**Goal:** Replace the development form with the user-approved dark compact
softphone, keypad and contacts/history navigation. Target window 360 × 560 px.

**Architecture:** Keep Svelte 5, generated DTOs and existing Tauri IPC. Separate
reusable icon, dialpad and call-panel presentation from application state. No new
runtime dependencies, fake contacts/history, native protocol or credential storage.

**Approved scope:** The user confirmed dark/compact and honest contacts/history
empty states. Product record: apps/desktop/PRODUCT.md. The attached reference
sets compactness/native familiarity, not fabricated people or data.

- [x] Write failing dialing-helper tests for caret insertion, deletion and limits;
      implement with Bun's existing test runner. Add the check to CI.
- [x] Build the dark shell, accessible keypad, persistent real call controls,
      settings and real navigation/empty states. Preserve polling/cleanup and
      async operation guards; explicitly represent browser and portable builds.
- [x] Set compact Tauri dimensions and document the visible behavior without
      changing IPC/DTO contracts.
- [x] Check types, formatting, build, native packaging and real SIP/RTP regression.
      Inspect the UI at native/compact/narrow sizes, test keyboard and call states,
      capture real native screenshots, request independent code/visual review and
      record only implemented design rules.
