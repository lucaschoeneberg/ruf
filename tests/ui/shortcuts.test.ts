import { expect, test } from "bun:test";
import {
  keyboardAction,
  searchActions,
  phoneActions,
} from "../../apps/desktop/src/lib/shortcuts";
const key = (key: string, overrides: Partial<KeyboardEvent> = {}) =>
  ({
    key,
    repeat: false,
    isComposing: false,
    altKey: false,
    metaKey: false,
    ctrlKey: true,
    shiftKey: false,
    ...overrides,
  }) as KeyboardEvent;

test("typing and platform editing combinations never trigger call controls", () => {
  expect(keyboardAction(key("m", { ctrlKey: false }))).toBeUndefined();
  expect(keyboardAction(key("m"))).toBeUndefined();
  expect(keyboardAction(key("M", { shiftKey: true }))).toBe("mute");
  expect(
    keyboardAction(key("H", { ctrlKey: false, metaKey: true, shiftKey: true })),
  ).toBe("hold");
  expect(
    keyboardAction(key("M", { shiftKey: true, repeat: true })),
  ).toBeUndefined();
  expect(
    keyboardAction(key("M", { shiftKey: true, isComposing: true })),
  ).toBeUndefined();
  expect(
    keyboardAction(key("M", { shiftKey: true, altKey: true })),
  ).toBeUndefined();
  expect(keyboardAction(key("Enter"))).toBe("answer");
  expect(keyboardAction(key(".", { ctrlKey: false }))).toBeUndefined();
});

test("search finds German labels and synonyms with all query words", () => {
  expect(
    searchActions(phoneActions, "  MIKROFON ton ").map((a) => a.id),
  ).toEqual(["mute"]);
  expect(searchActions(phoneActions, "weiterleiten").map((a) => a.id)).toEqual([
    "transfer",
  ]);
  expect(searchActions(phoneActions, "unbekannte aktion")).toEqual([]);
  expect(searchActions(phoneActions, " ")).toHaveLength(phoneActions.length);
});
