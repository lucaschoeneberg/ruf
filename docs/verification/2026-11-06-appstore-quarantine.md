# App Store package quarantine correction — 2026-11-06

Apple rejected Ruf 0.1.0 build 1 with error 91109. The downloaded provisioning
profile carried `com.apple.quarantine`; copying it preserved the attribute in the
signed app and installer payload. Successful Transporter delivery alone did not
prove successful Apple processing.

## Change

`scripts/prepare-app-store.ts` cleans the staged profile and app before signing,
without modifying the downloaded source profile or following symlinks outside
the staged tree. It checks the final app, installer and fully expanded installer
for quarantine attributes. The existing readiness report records these checks.
The signing certificate extraction argument also uses codesign's required
`--extract-certificates=...` form.

`tests/ui/app-store.test.ts` reproduces quarantine on an actual macOS filesystem.
The regression verifies rejection before cleanup, successful cleanup, unchanged
profile bytes and preservation of attributes on a symlink target outside the app.
`docs/release-checklist.md` documents the packaging checks. No IPC command, DTO,
native SIP behavior or public API changed.

## Observed verification

- The new regression failed before the helper was implemented and passed afterward.
- `bun run format:check`, `bun run check`, `bun run check:versions` and
  `git diff --check` passed.
- `bun run test:ui`: 97 passed, zero failed.
- `RUF_APPSTORE_BUILD_NUMBER=2 bun run appstore:prepare --unsigned` passed with
  native linkage and the expanded installer quarantine checks.
- The signed retry retained the original app code and profile bytes, incremented
  the bundle build to 2, and passed strict application and installer signature
  verification. The expanded signed installer contained no quarantine attributes.
- Transporter delivered build 2 on 2026-11-06 at 13:46 Europe/Berlin. Its displayed
  result subsequently confirmed readiness for internal testing. The Ruf icon
  was visible. This confirms completed processing, not approval for external
  testing or the public App Store.

The delivered package is kept outside Git under
`artifacts/app-store/upload/Ruf_0.1.0_2/`. Its SHA-256 is
`7957bc8b34d778dd4ade4686194fd761096f691b9b3470913922a6604fec4265`.
The signed retry report and permanent pipeline log are stored alongside it.

No fresh SIP/audio, Windows runtime or hosted CI execution was performed for this
packaging-only correction. External TestFlight review and public SIP demo access
remain separate work. The installed application was not replaced by this fix.
