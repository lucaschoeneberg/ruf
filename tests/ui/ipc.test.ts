import { afterEach, expect, test } from "bun:test";
import { getAppInfo } from "../../apps/desktop/src/lib/api";
import {
  copyPhoneNumber,
  getContacts,
} from "../../apps/desktop/src/lib/contacts-api";
import { getHistory } from "../../apps/desktop/src/lib/history-api";
import type { HistoryQuery } from "../../apps/desktop/src/lib/generated/AppInfo";

const originalGlobals = Object.getOwnPropertyDescriptors(globalThis);
const globals = ["isTauri", "window", "navigator"] as const;
const query: HistoryQuery = {
  search: "",
  filter: "all",
  offset: 0,
  limit: null,
  contactNumbers: [],
};

function setGlobal(name: (typeof globals)[number], value: unknown): void {
  Object.defineProperty(globalThis, name, { configurable: true, value });
}

afterEach(() => {
  for (const name of globals) {
    const descriptor = originalGlobals[name];
    if (descriptor) Object.defineProperty(globalThis, name, descriptor);
    else Reflect.deleteProperty(globalThis, name);
  }
});

test("browser API calls reject asynchronously with context-specific recovery messages", async () => {
  setGlobal("isTauri", false);
  const cases = [
    [
      () => getAppInfo(),
      "Der native Status ist nur in der Desktop-App verfügbar.",
    ],
    [() => getContacts(), "Kontakte sind nur in der macOS-App verfügbar."],
    [
      () => getHistory(query),
      "Der Verlauf ist nur in der Desktop-App verfügbar.",
    ],
  ] as const;
  for (const [call, message] of cases) {
    const result = call();
    expect(result).toBeInstanceOf(Promise);
    await expect(result).rejects.toThrow(message);
  }
});

test("clipboard success works in a browser without invoking native contacts", async () => {
  setGlobal("isTauri", false);
  const copied: string[] = [];
  setGlobal("navigator", {
    clipboard: {
      async writeText(text: string) {
        copied.push(text);
      },
    },
  });
  await copyPhoneNumber("+49 30 123456");
  expect(copied).toEqual(["+49 30 123456"]);
});

test("clipboard denial falls back to the native command without changing the number", async () => {
  setGlobal("isTauri", true);
  setGlobal("navigator", {
    clipboard: {
      async writeText() {
        throw new Error("Denied");
      },
    },
  });
  const requests: unknown[] = [];
  setGlobal("window", {
    __TAURI_INTERNALS__: {
      async invoke(command: string, args: unknown) {
        requests.push({ command, args });
      },
    },
  });
  await copyPhoneNumber("+49 (30) 123456");
  expect(requests).toEqual([
    { command: "copy_phone_number", args: { number: "+49 (30) 123456" } },
  ]);
});

test("native failures preserve the structured error for caller recovery", async () => {
  setGlobal("isTauri", true);
  const error = {
    code: "storageFailure",
    message: "Synthetic storage failure",
    sipStatus: null,
  };
  setGlobal("window", {
    __TAURI_INTERNALS__: {
      async invoke() {
        throw error;
      },
    },
  });
  for (const call of [
    () => getAppInfo(),
    () => getContacts(),
    () => getHistory(query),
  ]) {
    await expect(call()).rejects.toBe(error);
  }
});
