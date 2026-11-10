# Ruf installation disk image, 10 November 2026

Local macOS arm64 verification of the English, Ruf-themed DMG in the uncommitted
working tree. Existing i18n work and the staged release-checklist edit were
preserved. No commit, push or installed-app replacement was performed.

## Change

The authored installation background reuses the canonical Ruf mark, bundled IBM
Plex Sans and existing dark/blue palette. A phone-cord arrow connects the real
draggable app and Applications alias. English copy explains copying, opening and
ejecting. Tauri's existing DMG configuration owns background, window dimensions
and icon positions; no custom installer or new dependency was added.

Finder renders filenames in black on image backgrounds, so the artwork provides
light caption underlays. The footer fits the content area after title/path bars.
CI configuration retains Tauri's Finder styling step with
`TAURI_BUNDLER_DMG_IGNORE_CI=true`; that hosted execution was not observed.

Source/regeneration details: [DMG artwork guide](../../apps/desktop/src-tauri/dmg/README.md).
The generator is included in the existing strict tooling check and documented
in [tooling commands](../api/tooling.md). No IPC or DTO contract changed.

## Observed checks

- `bun run check`: catalog checks, tooling/frontend TypeScript and Svelte passed,
  with zero Svelte errors/warnings. Final tooling check also passed.
- `bun run format:check` and `git diff --check` passed.
- The existing desktop wrapper built the native App and DMG with pinned Rust
  1.98.1 and the Developer ID Application identity for Mardu GmbH. SBOM/native
  context and installer hash checks passed.
- Strict app/DMG signatures, secure signing timestamps and DMG integrity passed.
- The final read-only mounted image contains the exact 1440×960, 144-DPI artwork,
  Finder `.DS_Store`, an Applications alias targeting `/Applications` and the same
  signed executable as the output App.
- Finder reported icon view, 128px icons, a 720×480 window and positions
  `(180,224)` / `(540,224)`. A real window capture confirmed readable filename
  underlays, arrow alignment and footer instructions without clipping.
- Signed microphone and Contacts declarations/signatures passed the existing
  `test:macos-audio` and `test:macos-contacts` bundle checks.
- An independent scoped code/visual review checked the artwork and packaged
  Finder capture. The initial black-label/clipped-footer findings were corrected
  and recaptured.

One signing attempt failed because Apple's timestamp service was unavailable;
the same build succeeded on retry with a verified secure timestamp. The existing
local `rust-objcopy`/missing-LLVM debug-stripping warning remains. Notarization,
hosted CI, Windows runtime, permission prompts and hardware audio were not tested.
No SIP/audio code changed, so those unrelated suites were not repeated.

## Distilled follow-up

The user requested a simpler installation surface and explicitly removed the
separate HTML guide. The guide source and its entire image postprocessing hook
were deleted; `scripts/build-desktop.ts` again has no diff. The final image uses
only the standard Tauri packaging/signing path, two native installation items,
the Ruf phone-cord arrow and concise English drag/open/eject instructions.
Filename underlays are now 96/132×22 rather than 168×28, the redundant arrow
caption is gone, and the footer contains two action-oriented lines instead of
a second marketing heading. Artwork remains 1440×960 at 144 DPI in a 720×480
Finder window.

Fresh format/type/catalog/Svelte checks and the signed native App/DMG build
passed. The final mounted DMG was verified to contain exactly `Ruf.app` and
`Applications` as its visible entries, with no HTML guide. Background bytes,
Finder positions, Applications target, signed executable, secure Developer ID
timestamps, disk-image integrity and final installer hashes passed. Signed audio
and Contacts declarations also passed. A real Finder recapture showed both
footer lines, readable labels and no scrollbars or clipping. The original staged
release-checklist patch remains unchanged.

The background is still raster text, without semantic instructions or independent
text scaling. Native Finder item names and keyboard behavior remain available;
a full VoiceOver audit and notarization are not claimed. No new dependency,
runtime IPC/DTO change, custom installer, commit or push was introduced.
