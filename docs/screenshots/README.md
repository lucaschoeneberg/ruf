# Ruf repository media

These are captures of the current English Svelte interface using the existing
`installAppStoreFixtures` initializer. All accounts, contact names, registration,
call states, devices and history are synthetic. No native app, OS address book,
keychain, microphone or SIP connection is used. UI text comes from the real
English catalog. They are not proof of successful registration, media, provider
interoperability or OS permissions.

- `dialer.png`, `contacts.png`, `call.png`, `history.png`: 360×560 logical UI at
  2× pixel density; `settings.png`: 900×660 logical UI at 2×.
- `demo.gif`: a 30-second, five-screen UI walkthrough, labelled sample data on
  every frame. It shows available controls rather than pretending a live call.
- `social-preview.png`: 1280×640 repository sharing image using the canonical
  Ruf mark, actual dialer capture and bundled IBM Plex Sans.
- The same generation writes an MP4 to ignored
  `artifacts/release-preparation/media/ruf-0.1.0-ui-demo.mp4` for later sharing.

Regenerate from the repository root:

```sh
bun run media:repository
```

Requires the pinned Bun/dependencies, an existing Chromium installation and
`ffmpeg` on PATH. `SIP_UI_BROWSER_EXECUTABLE` optionally selects a browser.
On macOS the generator can reuse an installed Playwright Chromium cache; on
other hosts install the matching Chromium or specify its executable. No browser
or system package is installed by the command.

The generator starts and stops its own loopback Vite server, blocks off-origin
browser traffic, reuses the existing synthetic IPC contracts and writes scratch
frames/privacy report under ignored `output/playwright/repository`. It checks for
page errors, horizontal overflow and unexpected permission/call/audio commands.
Review the generated images before committing or uploading them. Changing these
assets does not update GitHub's Social preview setting or publish any media.
