import type { CallOutcome, HistoryEntry } from "./generated/AppInfo";
import { MAX_NUMBER_LENGTH } from "./dialing";

export const historyOutcomes: Record<CallOutcome, string> = {
  inProgress: "Laufend",
  completed: "Beendet",
  missed: "Verpasst",
  rejected: "Abgelehnt",
  busy: "Besetzt",
  cancelled: "Abgebrochen",
  failed: "Fehlgeschlagen",
  interrupted: "Unterbrochen",
};

/** Current authorized contact names take precedence over historical SIP names. */
export function historyTitle(entry: HistoryEntry): string {
  return (
    entry.contact?.name ||
    entry.remoteName ||
    entry.number ||
    "Unbekannte Nummer"
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
): string | null {
  if (!entry.number) return "Keine Nummer für den Rückruf vorhanden.";
  if (
    entry.number.length > MAX_NUMBER_LENGTH ||
    !/^[0-9+*#]+$/.test(entry.number)
  )
    return "Diese Nummer ist nicht wählbar.";
  if (entry.callbackAccountId === null)
    return "Das ursprüngliche Konto ist nicht verbunden oder nicht mehr vorhanden.";
  if (!canDial || entry.outcome === "inProgress")
    return "Rückruf erst möglich, wenn die Telefonie frei ist.";
  return null;
}

/** Calendar labels follow local days, including daylight-saving transitions. */
export function historyDayLabel(
  atMs: number,
  now = Date.now(),
  locale = "de-DE",
): string {
  const date = new Date(atMs);
  const today = new Date(now);
  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  if (date.toDateString() === today.toDateString()) return "Heute";
  if (date.toDateString() === yesterday.toDateString()) return "Gestern";
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
