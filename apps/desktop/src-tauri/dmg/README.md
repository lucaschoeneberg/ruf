# Ruf disk image artwork

`background.svg` is the authored 720×480 installation background. It follows the
desktop's dark palette and IBM Plex Sans, and reuses the canonical
[`ruf.svg`](../../public/ruf.svg) mark. English installation instructions accompany
a phone-cord arrow from the real Ruf icon to the real Applications folder alias.
Finder retains native dragging, copying, labels and keyboard behavior. Its labels
follow the OS; the static artwork is English and independent of the app's runtime
language preference.

Finder draws filenames in black when using an image background, including in
Dark Mode. The light caption underlays preserve their contrast. The layout also
reserves space for the native title/path bars; the configured window dimensions
include that chrome, not just the background canvas.

`background.png` is the checked-in, 1440×960 Retina render at 144 DPI. It is a source
asset, not a screenshot of the app. Regenerate on macOS with the existing Bun,
Playwright browser and bundled font files:

```sh
bun apps/desktop/src-tauri/dmg/generate.ts
```

`SIP_UI_BROWSER_EXECUTABLE` optionally selects an existing Chromium executable,
as in the other UI tooling. No font/network service or new dependency is needed.
Only regenerate when artwork, the logo or fonts change; normal builds consume
the PNG directly. Tauri owns the background embedding, Finder window dimensions
and icon positions through `bundle.macOS.dmg` in `tauri.conf.json`.

The surface deliberately keeps only the two native installation items and short
drag/open/eject instructions. Redundant arrow captions and the second marketing
heading were removed; narrow filename underlays preserve contrast without
resembling full-width buttons. A separate HTML installation guide and its image
postprocessing stage were removed at the user's request. The existing Tauri
packaging/signing path handles the entire image.

Styled images require Tauri's Finder automation step. CI sets
`TAURI_BUNDLER_DMG_IGNORE_CI=true` to retain that step despite `CI=true`; setting
`CI=true` alone deliberately skips backgrounds and icon positioning. Run local
styled builds without `CI=true`, or also set the override. The terminal running
the build needs macOS permission to control Finder.

Verify the **mounted, final DMG** after packaging: background size and DPI, both
icon positions, readable instruction text, Applications link target, signature
and disk-image integrity. Signing and notarization remain separate build concerns.
The background remains a raster asset; it does not supply semantic instructions
to VoiceOver or independent text resizing. Native Finder item names and keyboard
behavior remain available. No full screen-reader conformance is claimed.
