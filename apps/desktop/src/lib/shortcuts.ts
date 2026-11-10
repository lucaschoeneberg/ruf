import { formatMessage, getLocale, message, type MessageValue } from "./i18n";
import type { AppLocale } from "./generated/AppInfo";
/** Window-local actions. Native menus forward to the same guarded controls. */
export const phoneActions = [
  {
    id: "dialer",
    label: message("actions.show_keypad"),
    key: "1",
    terms: message("actions.phone_dial_number"),
  },
  {
    id: "contacts",
    label: message("actions.show_contacts"),
    key: "2",
    terms: message("actions.address_book_people"),
  },
  {
    id: "history",
    label: message("actions.show_history"),
    key: "3",
    terms: message("actions.calls_callback"),
  },
  {
    id: "number",
    label: message("actions.enter_number"),
    key: "l",
    terms: message("actions.dial_phone"),
  },
  {
    id: "find",
    label: message("actions.search_this_view"),
    key: "f",
    terms: message("actions.search_find_contacts_history"),
  },
  {
    id: "search",
    label: message("actions.search_ruf"),
    key: "k",
    terms: message("actions.commands_keyboard_shortcuts"),
  },
  {
    id: "settings",
    label: message("actions.open_settings"),
    key: ",",
    terms: message("actions.options"),
  },
  {
    id: "answer",
    label: message("actions.answer_call"),
    key: "Enter",
    terms: message("actions.answer_call"),
  },
  {
    id: "end",
    label: message("actions.end_reject_call"),
    key: ".",
    terms: message("actions.hang_up_stop"),
  },
  {
    id: "mute",
    label: message("actions.toggle_microphone"),
    key: "m",
    shift: true,
    terms: message("actions.mute_sound"),
  },
  {
    id: "hold",
    label: message("actions.hold_resume_call"),
    key: "h",
    shift: true,
    terms: message("actions.pause"),
  },
  {
    id: "transfer",
    label: message("actions.transfer_consultation"),
    key: "t",
    shift: true,
    terms: message("actions.forward_transfer"),
  },
  {
    id: "tones",
    label: message("actions.toggle_in_call_keypad"),
    key: "d",
    shift: true,
    terms: message("actions.dtmf_tones_digits"),
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
export function shortcutLabel(
  action: (typeof phoneActions)[number],
  language: AppLocale = getLocale(),
): string {
  const shift = "shift" in action && action.shift;
  const key = action.key === "Enter" ? "↵" : action.key.toUpperCase();
  return isMac()
    ? `⌘${shift ? "⇧" : ""}${key}`
    : `${language === "de" ? "Strg" : "Ctrl"}+${shift ? (language === "de" ? "Umschalt+" : "Shift+") : ""}${key}`;
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
export function searchActions<
  T extends { label: MessageValue; terms: MessageValue },
>(
  actions: readonly T[],
  query: string,
  language: AppLocale = getLocale(),
): T[] {
  const terms = query
    .trim()
    .toLocaleLowerCase(language)
    .split(/\s+/)
    .filter(Boolean);
  return actions.filter((action) => {
    const text =
      `${formatMessage(language, action.label)} ${formatMessage(language, action.terms)}`.toLocaleLowerCase(
        language,
      );
    return terms.every((term) => text.includes(term));
  });
}
