import { describe, expect, test } from "bun:test";
import {
  normalizeAccount,
  validateAccount,
  hasSeenIntroduction,
  rememberIntroduction,
} from "../../apps/desktop/src/lib/onboarding";
import { errorMessage } from "../../apps/desktop/src/lib/api";
import type { AccountConfig } from "../../apps/desktop/src/lib/generated/AppInfo";

const account = (): AccountConfig => ({
  registrar: "pbx.example.org:5060",
  username: "alice",
  password: "  secret;with@punctuation  ",
  audioMode: "system",
});

describe("SIP account onboarding", () => {
  test("normalizes server/user whitespace without altering the password", () => {
    const original = {
      ...account(),
      registrar: " pbx.example.org:5060 ",
      username: " alice ",
    };
    const normalized = normalizeAccount(original);
    expect(normalized.registrar).toBe("pbx.example.org:5060");
    expect(normalized.username).toBe("alice");
    expect(normalized.password).toBe(original.password);
    expect(original.registrar).toBe(" pbx.example.org:5060 ");
    expect(validateAccount(normalized)).toEqual({});
  });
  test("identifies each missing field for inline recovery", () => {
    expect(
      Object.keys(
        validateAccount({
          ...account(),
          registrar: "",
          username: "",
          password: "",
        }),
      ),
    ).toEqual(["registrar", "username", "password"]);
  });
  test("matches native registrar/port restrictions", () => {
    for (const registrar of [
      "sip:pbx.example.org",
      "host:+5070",
      "host:0",
      "host:65536",
      "host:abc",
      "bad;transport=tls",
      "-bad.example",
    ]) {
      expect(
        validateAccount({ ...account(), registrar }).registrar,
      ).toBeDefined();
    }
    for (const registrar of [
      "localhost",
      "127.0.0.1:5070",
      "pbx.example.org:65535",
    ]) {
      expect(validateAccount({ ...account(), registrar })).toEqual({});
    }
  });
  test("checks the authentication username and password byte limit", () => {
    expect(
      validateAccount({ ...account(), username: "alice@example.org" }).username,
    ).toBeDefined();
    expect(
      validateAccount({ ...account(), password: "ü".repeat(513) }).password,
    ).toBeDefined();
    expect(
      validateAccount({ ...account(), password: "a\0b" }).password,
    ).toBeDefined();
  });
  test("remembers only the introduction dismissal and tolerates blocked storage", () => {
    const values = new Map<string, string>();
    const storage = {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => {
        values.set(key, value);
      },
    };
    expect(hasSeenIntroduction(storage)).toBe(false);
    rememberIntroduction(storage);
    expect(hasSeenIntroduction(storage)).toBe(true);
    expect([...values.values()]).toEqual(["seen"]);
    const blocked = {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("blocked");
      },
    };
    expect(hasSeenIntroduction(blocked)).toBe(false);
    expect(() => rememberIntroduction(blocked)).not.toThrow();
  });
  test("returns recovery-oriented German messages for typed native failures", () => {
    expect(
      errorMessage({ code: "notRegistered", message: "Do not display me" }),
    ).toContain("Konto");
    expect(
      errorMessage({ code: "invalidInput", message: "Do not display me" }),
    ).toContain("Eingaben");
    expect(
      errorMessage({ code: "nativeFailure", message: "Do not display me" }),
    ).not.toContain("Do not display me");
    expect(errorMessage(new Error("Bekannter Hinweis"))).toBe(
      "Bekannter Hinweis",
    );
  });
});
