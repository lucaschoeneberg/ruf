import { formatMessage } from "../../apps/desktop/src/lib/i18n";
import { describe, expect, test } from "bun:test";
import {
  normalizeAccount,
  validateAccount as validateAccountMessages,
  hasSeenIntroduction,
  rememberIntroduction,
} from "../../apps/desktop/src/lib/onboarding";
import { errorMessage as errorReference } from "../../apps/desktop/src/lib/api";
import type { AccountConfig } from "../../apps/desktop/src/lib/generated/AppInfo";

const account = (): AccountConfig => ({
  registrar: "pbx.example.org:5060",
  username: "alice",
  password: "  secret;with@punctuation  ",
  audioMode: "system",
  transport: "udp",
  mediaSecurity: "disabled",
  authUsername: null,
  outboundProxy: null,
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
      "Die Telefonie-Aktion konnte nicht abgeschlossen werden.",
    );
  });
});

test("editing a saved account may keep an empty password, creation may not", () => {
  const input: AccountConfig = {
    ...account(),
    registrar: "pbx.test",
    username: "1000",
    password: "",
    audioMode: "system" as const,
  };
  expect(validateAccount(input).password).toBeDefined();
  expect(validateAccount(input, true).password).toBeUndefined();
  expect(
    validateAccount({ ...input, password: "bad\0value" }, true).password,
  ).toBeDefined();
});

for (const [code, guidance] of [
  ["credentialAccessRequired", /Schlüsselbund/],
  ["credentialMissing", /Passwort.*eingeben|Gib.*Passwort/],
  ["credentialFailure", /Schlüsselbund/],
] as const)
  test(`${code} gives local credential recovery without exposing native details`, () => {
    const message = errorMessage({ code, message: "secret native details" });
    expect(message).toMatch(guidance);
    expect(message).not.toContain("secret native details");
    expect(message).not.toMatch(
      /SIP-Anmeldung wurde abgelehnt|falsche.*Passwort/,
    );
  });

test("unusable saved credentials explain both retry and password replacement safely", () => {
  const message = errorMessage({
    code: "credentialFailure",
    message: "private malformed secret contents",
  });
  expect(message).toMatch(/Erneut verbinden/);
  expect(message).toMatch(/Passwort.*ein.*speichere/);
  expect(message).not.toContain("private malformed secret contents");
});

function errorMessage(error: unknown): string {
  return formatMessage("de", errorReference(error));
}

function validateAccount(...args: Parameters<typeof validateAccountMessages>) {
  return Object.fromEntries(
    Object.entries(validateAccountMessages(...args)).map(([key, value]) => [
      key,
      formatMessage("de", value),
    ]),
  );
}
