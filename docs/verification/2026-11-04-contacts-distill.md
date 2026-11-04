# Contacts distill

Impeccable distill was applied to the contacts tab. The primary task stays finding
a person and calling the intended number. The shared dark palette, typography,
native permission flow and neighboring views are preserved.

## Simplification

- One compact row per source contact, with company and number/count in a common
  secondary line. A single supported number calls directly; multiple numbers
  expand inline before choosing the exact destination. At most one row expands.
- One favorites toggle beside search replaces the separate Alle/Favoriten row.
- Contact count is beside the heading; refresh is a labeled icon. Permission
  states keep their explicit recovery actions without a duplicate header refresh.
- Repeated source/footer explanations and technical favorite-storage copy are
  removed from the browsing path. Permission and privacy details remain in the
  existing Contacts settings section.
- Contacts without numbers remain visible without a misleading phone icon;
  unsupported numbers show Nicht wählbar. Keyboard labels and focus are retained.

No contacts capability, API, DTO, provider, native permission or call pipeline was
removed or changed. The only interaction change is explicit inline disclosure
for multiple numbers. Existing German labels and generated Contact DTOs are reused.

## Files

- `apps/desktop/src/lib/components/ContactsPanel.svelte`
- `apps/desktop/src/lib/components/ContactRow.svelte`
- `apps/desktop/src/lib/components/Icon.svelte`
- `tests/ui/browser/contacts.mjs`
- The UI description in `docs/api/contacts.md` and this verification record

Other agents are changing caller identity, branding and desktop behavior in the
same checkout. Their edits, including App.svelte and native code, were preserved.

## Observed verification

- Browser regression observed red before disclosure implementation; the complete
  contacts, phone, settings and layout browser flows passed afterward.
- Contacts coverage: explicit access and denied recovery, collapsed multi-number
  rows, exact selected destination, direct single-number calls, favorite add and
  mouse/keyboard filter toggle, Unicode search, unsupported numbers, keyboard tab
  order and no horizontal overflow at minimum width.
- Batched visual checks at 340×500 and 320×440, including expanded numbers and
  compact calling. Synthetic screenshots are in ignored `output/playwright/`;
  before captures are in ignored `artifacts/contacts/`.
- Svelte/TypeScript check: zero errors/warnings. Frontend build and version checks
  passed. Contact search tests: 4 passed. Changed files are formatted.
- Impeccable detector returned `[]` for the changed UI targets. A bounded polish
  pass removed a redundant no-number phone icon, corrected inline spacing and
  kept aria-controls bound only to an existing expanded group.
- Native desktop build completed; signed contacts and microphone bundle checks
  passed. No SIP/audio code changed in this distill task.
- The full UI unit command encountered another agent's new incoming-caller test
  `incoming caller name and number are shown as plain text`, whose expected SSR
  escaping did not match Svelte's output. That test was corrected by the parallel
  work; the later full suite passed all 17 tests. A full formatting check reported
  App.svelte from concurrent work. A subsequent full typecheck encountered the
  concurrent caller-number optional-property change in App.svelte (`undefined`
  passed to a `string | null` prop). Neither unrelated file was changed here; no
  all-workspace-green claim is made for those overlapping snapshots.

The final native build uses the parallel branding task's current product name
Ruf. Both contact and microphone signature checks passed for the actual
`target/release/bundle/macos/Ruf.app`; no stale SIP bundle is used as evidence for
that build. This refinement did not replace or restart the user's running app.

No personal contacts were used for screenshots or exported. Windows runtime,
hosted CI and fresh macOS consent were not exercised by this UI refinement.
