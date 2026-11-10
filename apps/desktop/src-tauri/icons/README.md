# Ruf desktop icons

The canonical vector is [`public/ruf.svg`](../../public/ruf.svg). It defines the
existing blue handset/lettermark and dark rounded tile on a 1024×1024 canvas.
Geometry, colors and transparent outer margins remain unchanged.

From the repository root, regenerate with the pinned Bun and existing Tauri CLI:

```sh
bun apps/desktop/src-tauri/icons/generate.ts
```

The generator renders directly from SVG and writes desktop PNGs at 16, 32, 48,
64, 128, 256 and 512 pixels, `128x128@2x.png`, a 1024×1024 `icon.png`, and the
existing `icon.icns`/`icon.ico` names used by Tauri. `icon.icns` includes the full
macOS 1×/2× set through `icon_512x512@2x.png` (1024 pixels). The already-present
ICNS had that largest representation; the standalone `icon.png` was only 512px.
No low-resolution raster image is used as the generation source.

Only desktop outputs are retained. Tauri's temporary mobile outputs are removed;
the separate template/status-bar files `tray.svg` and `tray.png` are untouched.
An SVG copy and `ruf-1024.png` are also exported under ignored
`artifacts/app-store/icons` for review. A visual review sheet may be saved there
as `ruf-preview.png`; it is artwork for inspection, not a screenshot of the app.

The classic macOS ICNS keeps the brand's transparent outside margins. This is
separate from the opaque background-layer guidance for modern layered Icon
Composer assets; no Icon Composer or iOS asset migration is implied here.
Do not apply an iOS-specific alpha rule to this desktop asset pipeline.

The app icon is delivered to App Store Connect with the uploaded build; it is not
an independently uploaded screenshot. Rebuild the app after changing these files.
Local asset checks do not replace Apple's validation of the final signed build.

References:

- [Apple: complete macOS iconset and Retina sizes](https://developer.apple.com/library/archive/documentation/Xcode/Reference/xcode_ref-Asset_Catalog_Format/IconSetType.html)
- [Apple: high-resolution icon resources and iconutil](https://developer.apple.com/library/archive/documentation/GraphicsAnimation/Conceptual/HighResolutionOSX/Optimizing/Optimizing.html)
- [Apple: app icon formats, canvas and layer guidance](https://developer.apple.com/design/human-interface-guidelines/app-icons)
- [Apple: delivering the app icon through a build](https://developer.apple.com/help/app-store-connect/manage-app-information/add-an-app-icon)
