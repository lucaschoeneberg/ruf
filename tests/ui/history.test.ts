import { describe, expect, test } from "bun:test";
import type { HistoryEntry } from "../../apps/desktop/src/lib/generated/AppInfo";
import {
  callbackUnavailable,
  groupHistory,
  historyDayLabel,
  historyDuration,
  historyOutcomes,
  historyTitle,
} from "../../apps/desktop/src/lib/history";

const entry: HistoryEntry = {
  id: "synthetic-history",
  accountName: "Testkonto",
  accountRegistrar: "pbx.example.test",
  accountUsername: "1000",
  direction: "incoming",
  number: "+4930123456",
  remoteName: "SIP-Anzeigename",
  startedAtMs: new Date(2026, 10, 4, 10, 0).getTime(),
  answeredAtMs: null,
  endedAtMs: null,
  durationMs: null,
  outcome: "missed",
  sipStatus: null,
  contact: null,
  callbackAccountId: 2,
};

describe("history presentation", () => {
  test("uses authorized contacts then source SIP identity, number and honest anonymous fallback", () => {
    expect(historyTitle(entry)).toBe("SIP-Anzeigename");
    expect(
      historyTitle({
        ...entry,
        contact: { id: "contact", source: "macos", name: "Aktueller Name" },
      }),
    ).toBe("Aktueller Name");
    expect(historyTitle({ ...entry, remoteName: null })).toBe(entry.number!);
    expect(historyTitle({ ...entry, remoteName: null, number: null })).toBe(
      "Unbekannte Nummer",
    );
  });

  test("unknown connected duration remains distinct from zero and durations over one hour", () => {
    expect(historyDuration(null)).toBeNull();
    expect(historyDuration(0)).toBe("0:00");
    expect(historyDuration(59999)).toBe("0:59");
    expect(historyDuration(61000)).toBe("1:01");
    expect(historyDuration(3661000)).toBe("1:01:01");
    expect(historyOutcomes.interrupted).toBe("Unterbrochen");
    expect(historyOutcomes.missed).toBe("Verpasst");
  });

  test("callback requires a dialable number and its original registered account even when another is ready", () => {
    expect(callbackUnavailable(entry, true)).toBeNull();
    expect(
      callbackUnavailable({ ...entry, callbackAccountId: null }, true),
    ).toContain("ursprüngliche Konto");
    expect(callbackUnavailable({ ...entry, number: null }, true)).toContain(
      "Keine Nummer",
    );
    for (const number of ["sip:1000@example.test", "x", "1".repeat(65)])
      expect(callbackUnavailable({ ...entry, number }, true)).toContain(
        "nicht wählbar",
      );
    expect(callbackUnavailable({ ...entry, number: "*21#" }, true)).toBeNull();
    expect(callbackUnavailable(entry, false)).toContain("Telefonie frei");
    expect(
      callbackUnavailable({ ...entry, outcome: "inProgress" }, true),
    ).toContain("Telefonie frei");
  });

  test("groups local calendar days without reordering backend entries or mutating the page", () => {
    const yesterday = new Date(2026, 10, 3, 23, 59).getTime();
    const entries = [
      entry,
      { ...entry, id: "same-day", startedAtMs: entry.startedAtMs - 1000 },
      { ...entry, id: "previous-day", startedAtMs: yesterday },
    ];
    const groups = groupHistory(entries);
    expect(groups.map((group) => group.entries.map((item) => item.id))).toEqual(
      [["synthetic-history", "same-day"], ["previous-day"]],
    );
    expect(entries.map((item) => item.id)).toEqual([
      "synthetic-history",
      "same-day",
      "previous-day",
    ]);
    expect(groupHistory([])).toEqual([]);
    expect(historyDayLabel(entry.startedAtMs, entry.startedAtMs)).toBe("Heute");
    expect(historyDayLabel(yesterday, entry.startedAtMs)).toBe("Gestern");
    expect(
      historyDayLabel(new Date(2025, 0, 2).getTime(), entry.startedAtMs),
    ).toContain("2025");
  });
});
