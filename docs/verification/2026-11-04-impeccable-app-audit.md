# Compact Zed app audit — 2026-11-04

## Direction and scope

Impeccable Operate refinement: preserve GitHub Dark Tritanopia, bundled IBM Plex
Sans, 4px controls, native chrome and compact dimensions. No replacement visual
world, new phone capabilities or fabricated history/contact data were introduced.
The existing enabled design hook remains enabled; no ignores were added. Native
WKWebView has no CDP detector endpoint, so browser rendering and native accessibility
checks are reported separately rather than claiming a native rendered detector run.

The previously uncommitted native/account/audio/background work was preserved in
`8524a9a`; its design and verification records in `f4e412a`. Audit fixes are separate.
Contacts integration began concurrently in the shared checkout. Its uncommitted
files and hunks were excluded from these commits. A self-contained export of the
baseline plus the audit changes was verified to avoid compiling or rewriting that
other task's intermediate state. The new ContactsPanel was read for palette,
permission/empty/error recovery and call-control integration, but its new backend
and real contacts were not exercised as part of this audit.

## Findings fixed

- Failed/pending outgoing-account selection no longer leaves a different visible
  identity from the native account used to dial.
- Initial snapshot failure has an explicit retry, including after dismissing its
  error notice. Stale poll failures cannot overwrite a newer native state event.
- Unchanged registration polling no longer closes manually opened help. Help
  distinguishes a new, registering, failed and connected account.
- Holding zero is canceled on blur/pointer cancellation; secondary/right pointers
  cannot start a stray plus gesture.
- Desktop-preference loading failures no longer block account/audio settings.
  Failed account saves preserve retry drafts, including the entered password.
- Dirty edits survive external account removal with an explicit removed-account
  state. Live metadata updates do not replace dirty drafts.
- Window close, app Quit and section navigation share draft protection. Busy saves
  remain protected; owned ringtone previews stop when leaving. Fields cannot accept
  new edits after explicit close/discard has begun.
- Explicit Quit reveals minimized Settings so the discard decision is visible.
- Foreground credential authorization runs outside the shared phone lock. Native
  observation, call controls and shutdown continue. Commit revalidates account,
  audio and call state; crash cleanup preserves active journals and disk references.
- Native phone windows fill their content area; the rounded browser-preview frame
  is limited to browser previews. Selection/caret/scrollbar styling is shared,
  intermediate font weights removed, and disabled hover styling is consistent.

## Verification

- 12 repeatable phone scenarios and 13 settings scenarios run in Chromium through
  pinned Playwright CLI 0.1.23 via Bun. All pass. Confirmed defects were observed
  failing before their fixes; the credential-lock mutation also fails its regression.
- Layout suite captures 17 views across 340×500, 320×440, 900×660, 740×540 and
  an enlarged 600×650 native fixture. It checks overflow, shared font and selection.
  Screenshots were visually inspected; artifacts stay ignored under output/playwright.
- Native macOS smoke bundle uses a separate identifier and uniquely named executable,
  with no user accounts. Native dirty-close protection and minimized-Settings Quit
  reveal were exercised through accessibility. Keep editing, explicit discard and
  final Quit were verified without accessing real contacts or keychain credentials.
- Exported audit snapshot: Svelte/TypeScript check, 12 unit tests, production frontend
  build, version alignment, formatting, portable/native workspace tests and Clippy
  with warnings denied all pass. Generated DTOs are unchanged by this audit.
- Native build and signed microphone entitlement/purpose check pass. Six C network
  recovery tests pass. Asterisk interop passes four cases with a 440.1Hz RTP echo;
  the real incoming-call-during-blocked-authorization regression also passes.
- SIP diagnostics pass 60 allowlisted markers and default-off privacy behavior.
- GitHub Actions workflow syntax passes actionlint 1.7.12. Browser regression CI is
  prepared, including browser installation and capture artifacts; hosted CI was not run.

Representative text contrast: body 17.39:1, muted text 6.50:1, white primary-action
text 4.63:1, error text 8.57:1. These measurements do not constitute a complete
screen-reader or WCAG certification.

| Dimension                | Score / 4 | Evidence / remaining limit                                                                     |
| ------------------------ | --------- | ---------------------------------------------------------------------------------------------- |
| Accessibility            | 3         | Keyboard, focus, names and text contrast checked; no complete spoken screen-reader run.        |
| Performance              | 3         | Lean frontend and event-driven UI; no hardware performance profile.                            |
| Theming                  | 4         | Shared palette/font, native/preview separation and consistent browser surfaces.                |
| Responsive layout        | 3         | Supported compact/minimum sizes and enlarged native fixture; Windows display scaling untested. |
| Implementation integrity | 4         | Recovery paths, guarded drafts and real state transitions covered by regressions.              |

## Remaining limits

Windows runtime, physical audio devices, real UniFi/Tailscale acceptance and actual
login/wake behavior remain platform acceptance work. Local ad-hoc signing is not
production notarization; the host SDK deployment-target warnings remain as recorded
in the background-calls verification. The concurrently developed Contacts changes
remain outside these audit commits and their independent native verification.
