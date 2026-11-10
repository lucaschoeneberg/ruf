import { getLocale, message, translate, type Message } from "./i18n";
import type { AppLocale } from "./generated/AppInfo";
import type { CallOutcome, HistoryEntry } from "./generated/AppInfo";
import { normalizeDialNumber } from "./dialing";

export const historyOutcomes: Record<CallOutcome, Message> = {
  inProgress: message("history.outcome.inProgress"),
  completed: message("history.outcome.completed"),
  missed: message("history.outcome.missed"),
  rejected: message("history.outcome.rejected"),
  busy: message("history.outcome.busy"),
  cancelled: message("history.outcome.cancelled"),
  failed: message("history.outcome.failed"),
  interrupted: message("history.outcome.interrupted"),
};

/** Current authorized contact names take precedence over historical SIP names. */
export function historyTitle(
  entry: HistoryEntry,
  language: AppLocale = getLocale(),
): string {
  return (
    entry.contact?.name ||
    entry.remoteName ||
    entry.number ||
    translate(language, "history.unknown_number")
  );
}

/** Display only observed connected duration; unknown is distinct from zero. */
export function historyDuration(durationMs: number | null): string | null {
  if (durationMs === null) return null;
  const seconds = Math.floor(durationMs / 1000);
  const minutes = Math.floor(seconds / 60);
  return minutes >= 60
    ? `${Math.floor(minutes / 60)}:${String(minutes % 60).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`
    : `${minutes}:${String(seconds % 60).padStart(2, "0")}`;
}

/** Explain why callback is unavailable; the native command revalidates account identity. */
export function callbackUnavailable(
  entry: HistoryEntry,
  canDial: boolean,
): Message | null {
  if (!entry.number) return message("history.no_callback_number");
  if (normalizeDialNumber(entry.number) === null)
    return message("history.invalid_callback_number");
  if (entry.callbackAccountId === null)
    return message("history.callback_account_unavailable");
  if (!canDial || entry.outcome === "inProgress")
    return message("history.callback_busy");
  return null;
}

/** Calendar labels follow local days, including daylight-saving transitions. */
export function historyDayLabel(
  atMs: number,
  now = Date.now(),
  locale = getLocale() as string,
): string {
  const date = new Date(atMs);
  const today = new Date(now);
  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  if (date.toDateString() === today.toDateString())
    return translate(
      locale.toLowerCase().startsWith("de") ? "de" : "en",
      "history.today",
    );
  if (date.toDateString() === yesterday.toDateString())
    return translate(
      locale.toLowerCase().startsWith("de") ? "de" : "en",
      "history.yesterday",
    );
  return new Intl.DateTimeFormat(locale, {
    day: "numeric",
    month: "long",
    ...(date.getFullYear() === today.getFullYear() ? {} : { year: "numeric" }),
  }).format(date);
}

/** Preserve the backend's newest-first order while grouping by local calendar day. */
export function groupHistory(
  entries: readonly HistoryEntry[],
): { key: string; atMs: number; entries: HistoryEntry[] }[] {
  const groups: { key: string; atMs: number; entries: HistoryEntry[] }[] = [];
  for (const entry of entries) {
    const key = new Date(entry.startedAtMs).toDateString();
    const previous = groups.at(-1);
    if (previous?.key === key) previous.entries.push(entry);
    else groups.push({ key, atMs: entry.startedAtMs, entries: [entry] });
  }
  return groups;
}
