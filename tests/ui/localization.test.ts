import { validateCatalogs } from "../../scripts/i18n-catalogs";
import { expect, test } from "bun:test";
import {
  applyLocalization,
  getLocale,
  resolveLanguage,
  translate,
  formatMessage,
  message,
  LocalizedError,
} from "../../apps/desktop/src/lib/i18n";
import { errorMessage } from "../../apps/desktop/src/lib/api";

test("language selection respects overrides and ordered regional system preferences", () => {
  expect(resolveLanguage("system", ["fr-FR", "de-CH", "en-GB"])).toBe("de");
  expect(resolveLanguage("system", ["en-GB", "de-DE"])).toBe("en");
  expect(resolveLanguage("de", ["en-US"])).toBe("de");
  expect(resolveLanguage("en", ["de-DE"])).toBe("en");
  expect(resolveLanguage("system", ["fr-FR", "invalid"])).toBe("en");
  expect(resolveLanguage("system", [])).toBe("en");
});

test("late initial localization responses cannot undo a newer saved language", () => {
  applyLocalization({ preference: "de", locale: "de", revision: 20 });
  applyLocalization({ preference: "en", locale: "en", revision: 19 });
  expect(getLocale()).toBe("de");
  applyLocalization({ preference: "en", locale: "en", revision: 21 });
  expect(getLocale()).toBe("en");
});

test("catalog validation catches missing keys, placeholders and incomplete plural forms", () => {
  const source = {
    greeting: "Hello %{name}",
    count_one: "%{count, number} call",
    count_other: "%{count, number} calls",
  };
  expect(() =>
    validateCatalogs(source, {
      greeting: "Hallo %{name}",
      count_one: "%{count, number} Anruf",
      count_other: "%{count, number} Anrufe",
    }),
  ).not.toThrow();
  expect(() =>
    validateCatalogs(source, { greeting: "Hallo %{name}" }),
  ).toThrow();
  expect(() =>
    validateCatalogs(
      { greeting: "Hello %{name}" },
      { greeting: "Hallo %{person}" },
    ),
  ).toThrow();
  expect(() =>
    validateCatalogs(
      { count_one: "one", count_other: "many" },
      { count_one: "eins" },
    ),
  ).toThrow();
  expect(() =>
    validateCatalogs(
      { greeting: "Hello %{name}" },
      { greeting: "Hallo %{name} {account" },
    ),
  ).toThrow();
  expect(() =>
    validateCatalogs(
      { greeting: "Hello {%{name}}" },
      { greeting: "Hallo {%{name}}" },
    ),
  ).toThrow();
});

test("plural forms use the actual count and named parameters stay plain text", () => {
  expect(translate("en", "phone.contacts.count_label", { count: 0 })).toBe(
    "contacts",
  );
  expect(translate("en", "phone.contacts.count_label", { count: 1 })).toBe(
    "contact",
  );
  expect(translate("en", "phone.contacts.count_label", { count: 2 })).toBe(
    "contacts",
  );
  expect(translate("de", "phone.contacts.count_label", { count: 1 })).toBe(
    "Kontakt",
  );
  expect(() => translate("en", "phone.contacts.count_label")).toThrow(
    "Missing translation",
  );
  expect(translate("en", "native.callerTitle", { name: "<Ada> {count}" })).toBe(
    "<Ada> {count} is calling",
  );
  expect(() => translate("de", "native.callerTitle")).toThrow("parameter");
});

test("stored errors can be translated again without translating raw user data", () => {
  const error = new LocalizedError(message("validation.password"));
  const reference = errorMessage(error);
  expect(formatMessage("de", reference)).toContain("SIP-Passwort");
  expect(formatMessage("en", reference)).toContain("SIP password");
  expect(formatMessage("en", "Kontakte")).toBe("Kontakte");
  for (const language of ["de", "en"] as const) {
    const output = formatMessage(
      language,
      errorMessage(new Error("private raw SIP details")),
    );
    expect(output).not.toContain("private raw SIP details");
    expect(output).toBe(translate(language, "errors.unknown"));
  }
});

// The grouping convention is supplied by i18next built-in formatting, not a project resolver.
test("library formatting groups numbers and does not interpret interpolated text", () => {
  expect(
    translate("de", "phone.contacts.numbers", { count: 1000, name: "Ada" }),
  ).toContain("1.000");
  expect(
    translate("en", "phone.contacts.numbers", { count: 1000, name: "Ada" }),
  ).toContain("1,000");
  expect(
    translate("en", "native.callerTitle", { name: "$t(validation.password)" }),
  ).toBe("$t(validation.password) is calling");
});
