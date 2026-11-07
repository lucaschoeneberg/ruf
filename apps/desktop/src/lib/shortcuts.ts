/** Window-local actions. Native menus forward to the same guarded controls. */
export const phoneActions = [
  {
    id: "dialer",
    label: "Tastatur anzeigen",
    key: "1",
    terms: "telefon wählen nummer",
  },
  {
    id: "contacts",
    label: "Kontakte anzeigen",
    key: "2",
    terms: "adressbuch personen",
  },
  {
    id: "history",
    label: "Verlauf anzeigen",
    key: "3",
    terms: "anrufe rückruf",
  },
  { id: "number", label: "Nummer eingeben", key: "l", terms: "wählen telefon" },
  {
    id: "find",
    label: "In dieser Ansicht suchen",
    key: "f",
    terms: "suche finden kontakte verlauf",
  },
  {
    id: "search",
    label: "Ruf durchsuchen",
    key: "k",
    terms: "befehle tastenkürzel shortcuts",
  },
  {
    id: "settings",
    label: "Einstellungen öffnen",
    key: ",",
    terms: "optionen",
  },
  {
    id: "answer",
    label: "Anruf annehmen",
    key: "Enter",
    terms: "antworten gespräch",
  },
  {
    id: "end",
    label: "Anruf beenden / ablehnen",
    key: ".",
    terms: "auflegen stoppen",
  },
  {
    id: "mute",
    label: "Mikrofon ein-/ausschalten",
    key: "m",
    shift: true,
    terms: "stummschalten ton",
  },
  {
    id: "hold",
    label: "Gespräch halten / fortsetzen",
    key: "h",
    shift: true,
    terms: "pause",
  },
  {
    id: "transfer",
    label: "Vermitteln / Rückfrage",
    key: "t",
    shift: true,
    terms: "weiterleiten transfer",
  },
  {
    id: "tones",
    label: "Gesprächstastatur ein-/ausblenden",
    key: "d",
    shift: true,
    terms: "dtmf töne ziffern",
  },
] as const;
export type PhoneAction = (typeof phoneActions)[number]["id"];
const callActions = [
  "answer",
  "end",
  "mute",
  "hold",
  "transfer",
  "tones",
] as const;
export type CallAction = (typeof callActions)[number];
export function isCallAction(id: string): id is CallAction {
  return callActions.some((action) => action === id);
}
export type SearchAction = {
  id: string;
  label: string;
  terms: string;
  shortcut?: string;
  disabled?: boolean;
  reason?: string;
};
export const isMac = () =>
  typeof navigator !== "undefined" &&
  /Mac|iPhone|iPad/.test(navigator.platform);
export function shortcutLabel(action: (typeof phoneActions)[number]): string {
  const shift = "shift" in action && action.shift;
  const key = action.key === "Enter" ? "↵" : action.key.toUpperCase();
  return isMac()
    ? `⌘${shift ? "⇧" : ""}${key}`
    : `Strg+${shift ? "Umschalt+" : ""}${key}`;
}
/** Ignore held keys, IME composition and additional modifiers. Never bind plain letters. */
export function keyboardAction(event: KeyboardEvent): PhoneAction | undefined {
  if (
    event.repeat ||
    event.isComposing ||
    event.altKey ||
    !(event.metaKey || event.ctrlKey)
  )
    return;
  return phoneActions.find(
    (action) =>
      action.key.toLowerCase() === event.key.toLowerCase() &&
      ("shift" in action && action.shift) === event.shiftKey,
  )?.id;
}
export function searchActions(
  actions: readonly SearchAction[],
  query: string,
): SearchAction[] {
  const terms = query
    .trim()
    .toLocaleLowerCase("de")
    .split(/\s+/)
    .filter(Boolean);
  return actions.filter((action) => {
    const text = `${action.label} ${action.terms}`.toLocaleLowerCase("de");
    return terms.every((term) => text.includes(term));
  });
}
