import {
  translate,
  type MessageKey,
  type MessageParams,
} from "../../../apps/desktop/src/lib/i18n";
import englishCatalog from "../../../locales/en.json";
import type { Page } from "playwright-core";

export type EnglishSmoke = (
  page: Page,
  flow: string,
  baseUrl: string,
) => Promise<{ passed: true }>;

// Serialized by the runner with prepared labels. Fixture names and SIP data
// remain literal; only product labels come from the shipped English catalog.
/** Resolve expected text with the real engine before entering Playwright's closure-free VM. */
export function englishExpectedLabels(): Record<string, string> {
  const labels: Record<string, string> = {};
  const add = (key: MessageKey, params: MessageParams = {}) => {
    labels[JSON.stringify([key, Object.entries(params).sort()])] = translate(
      "en",
      key,
      params,
    );
  };
  for (const key of Object.keys(
    englishCatalog,
  ) as (keyof typeof englishCatalog)[])
    if (!englishCatalog[key].includes("%{")) add(key);
  for (const name of ["Anna Beispiel", "Ada Beispiel"])
    add("phone.contacts.named_details", { name });
  add("phone.contacts.details", { name: "Anna Beispiel" });
  add("phone.contacts.copy_number", {
    name: "Anna Beispiel",
    label: "Intern",
    number: "1234",
  });
  add("phone.history.show_details", { name: "Anna Beispiel" });
  add("phone.history.details", { name: "Anna Beispiel" });
  add("phone.dtmf.tone", { digit: "#" });
  return labels;
}

export async function englishSmoke(
  page: Page,
  flow: string,
  baseUrl: string,
  labels: Readonly<Record<string, string>>,
): Promise<{ passed: true }> {
  const label = (key: string, params: Record<string, string> = {}) => {
    const value = labels[JSON.stringify([key, Object.entries(params).sort()])];
    if (value === undefined)
      throw new Error(`Unprepared expected label: ${key}`);
    return value;
  };
  const assert = (condition: unknown, reason: string) => {
    if (!condition) throw new Error(reason);
  };
  const button = (key: string) =>
    page.getByRole("button", { name: label(key), exact: true });
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.setDefaultTimeout(10000);
  await page.setViewportSize({ width: 340, height: 500 });

  if (flow === "contacts") {
    await page.setViewportSize({ width: 900, height: 660 });
    await page.goto(`${baseUrl}/?window=settings&section=privacy`);
    await page
      .getByRole("heading", { name: label("settings.privacy"), exact: true })
      .waitFor();
    await button("settings.allow_macos_contacts").click();
    await page
      .getByText(label("settings.permission_not_granted"), { exact: true })
      .waitFor();
    await page.evaluate(() => {
      window.__contactPermissionTest.permission = "authorized";
      window.__contactPermissionTest.emit();
    });
    await page.goto(baseUrl);
    await page.evaluate(() => {
      window.__contactPermissionTest.permission = "authorized";
      window.__contactPermissionTest.emit();
    });
    await button("app.contacts").click();
    await page
      .getByRole("button", {
        name: label("phone.contacts.details", { name: "Anna Beispiel" }),
        exact: true,
      })
      .click();
    const details = page.getByRole("group", {
      name: label("phone.contacts.named_details", { name: "Anna Beispiel" }),
      exact: true,
    });
    await details.getByText("Arbeit", { exact: true }).waitFor();
    await details
      .getByRole("button", {
        name: label("phone.contacts.copy_number", {
          name: "Anna Beispiel",
          label: "Intern",
          number: "1234",
        }),
        exact: true,
      })
      .click();
    await details
      .getByText(label("phone.contacts.copied"), { exact: true })
      .waitFor();
    assert(
      await page.evaluate(
        () => window.__contactPermissionTest.copied === "1234",
      ),
      "English copy action changed the raw number",
    );
  } else if (flow === "history") {
    await page.goto(baseUrl);
    await button("app.history").click();
    await page
      .getByRole("button", {
        name: label("phone.history.show_details", { name: "Anna Beispiel" }),
        exact: true,
      })
      .click();
    await page
      .getByRole("group", {
        name: label("phone.history.details", { name: "Anna Beispiel" }),
        exact: true,
      })
      .getByText("Ursprüngliches Konto", { exact: false })
      .first()
      .waitFor();
    await button("phone.history.copy").click();
    await button("phone.common.copied").waitFor();
    const search = page.getByRole("searchbox", {
      name: label("phone.history.search"),
      exact: true,
    });
    await search.fill("english-no-matching-call");
    await page
      .getByRole("heading", {
        name: label("phone.history.no_matches"),
        exact: true,
      })
      .waitFor();
    await button("phone.history.reset").click();
    await button("phone.history.more").waitFor();
    assert(
      (await page.locator(".history-entry").count()) === 50,
      "English history must retain the real page size",
    );
  } else if (flow === "phone") {
    await page.goto(baseUrl);
    await page.getByText(label("app.registered"), { exact: true }).waitFor();
    await page
      .getByRole("textbox", { name: label("phone.dial.number"), exact: true })
      .fill("1003");
    await button("phone.dial.call").click();
    await page
      .getByText(label("phone.call.calling"), { exact: true })
      .waitFor();
    await page.getByRole("heading", { name: "1003", exact: true }).waitFor();
    await button("phone.call.hang_up").click();
    await button("phone.dial.call").waitFor();
    assert(
      await page.evaluate(() =>
        window.__phoneFixture.commands.some(
          ({ command, args }) =>
            command === "dial" && args.destination === "1003",
        ),
      ),
      "English call did not use the original native contract",
    );
  } else if (flow === "call-controls") {
    await page.goto(baseUrl);
    await page.getByRole("heading", { name: "9002", exact: true }).waitFor();
    await button("phone.call.mute").click();
    await page
      .getByText(label("phone.call.mute_pending"), { exact: true })
      .waitFor();
    await page.evaluate(() => {
      window.__controls.state.calls[0]!.muted = true;
      window.__controls.emit("parent");
    });
    await button("phone.call.unmute").waitFor();
    await button("phone.call.transfer").click();
    await page
      .getByLabel(label("phone.call.target"), { exact: true })
      .fill("sip:bad");
    await button("phone.call.direct_transfer").click();
    await page
      .getByText(label("phone.call.target_invalid"), { exact: true })
      .waitFor();
    assert(
      !(await page.evaluate(() =>
        window.__controls.commands.some(
          ({ command }) => command === "blind_transfer",
        ),
      )),
      "Invalid transfer escaped validation in English",
    );
    await button("phone.call.close_transfer").click();
    await button("phone.call.show_tones").click();
    await page
      .getByRole("button", {
        name: label("phone.dtmf.tone", { digit: "#" }),
        exact: true,
      })
      .click();
    await page.waitForFunction(() =>
      window.__controls.commands.some(
        ({ command, args }) => command === "send_dtmf" && args.digit === "#",
      ),
    );
  } else if (flow === "shortcuts") {
    await page.goto(baseUrl);
    const number = page.getByRole("textbox", {
      name: label("phone.dial.number"),
      exact: true,
    });
    await number.fill("123");
    await page.keyboard.press("Control+k");
    const dialog = page.getByRole("dialog", {
      name: label("app.search"),
      exact: true,
    });
    await dialog.getByRole("combobox").fill("microphone");
    assert(
      (await dialog
        .getByRole("option")
        .first()
        .getAttribute("aria-disabled")) === "true",
      "English preview search must disable call actions",
    );
    await dialog.getByRole("combobox").press("Escape");
    assert(
      (await number.inputValue()) === "123",
      "English shortcuts lost the dial draft",
    );
    await page.keyboard.press("Control+3");
    await page
      .getByRole("heading", { name: label("phone.history.title"), exact: true })
      .waitFor();
    await page.keyboard.press("Control+1");
    assert(
      (await number.inputValue()) === "123",
      "English navigation lost the dial draft",
    );
  } else if (flow === "global-search") {
    await page.goto(baseUrl);
    await button("app.search").click();
    let dialog = page.getByRole("dialog", {
      name: label("app.search"),
      exact: true,
    });
    await dialog.getByRole("combobox").fill("Ada");
    await dialog
      .getByRole("option")
      .filter({ hasText: label("phone.history.missed") })
      .waitFor();
    await dialog
      .getByRole("option")
      .filter({ hasText: "Beispielbüro" })
      .click();
    await page
      .getByRole("group", {
        name: label("phone.contacts.named_details", { name: "Ada Beispiel" }),
        exact: true,
      })
      .waitFor();
    await button("app.search").click();
    dialog = page.getByRole("dialog", {
      name: label("app.search"),
      exact: true,
    });
    await page.evaluate(() => {
      window.__globalSearch.failHistory = true;
    });
    await dialog.getByRole("combobox").fill("Ada");
    await dialog
      .getByText(label("phone.search.history_error"), { exact: true })
      .waitFor();
    await dialog.getByRole("combobox").fill("settings");
    assert(
      (await dialog.getByRole("option").count()) > 0,
      "English action terms were not searchable",
    );
    await dialog.getByRole("combobox").press("Escape");
  } else if (flow === "dialer-identity") {
    await page.goto(`${baseUrl}/?identity-flow=1`);
    await page
      .getByRole("textbox", { name: label("phone.dial.number"), exact: true })
      .fill("030 5550 1234");
    await button("phone.dial.call").click();
    await page
      .getByRole("heading", { name: "Mara Muster", exact: true })
      .waitFor();
    await button("phone.call.show_tones").click();
    await page
      .getByRole("button", {
        name: label("phone.dtmf.tone", { digit: "#" }),
        exact: true,
      })
      .click();
    await page.waitForFunction(() =>
      window.__identity.commands.some(
        ({ command, args }) => command === "send_dtmf" && args.digit === "#",
      ),
    );
    assert(
      await page.evaluate(
        () =>
          window.__identity.commands.find(({ command }) => command === "dial")
            ?.args.destination === "03055501234",
      ),
      "English identity changed the normalized dial number",
    );
  } else if (flow === "settings") {
    await page.setViewportSize({ width: 900, height: 660 });
    await page.goto(`${baseUrl}/?window=settings&section=general`);
    const language = page.getByLabel(label("settings.language"), {
      exact: true,
    });
    await language.selectOption("de");
    assert(
      (await page.locator("html").getAttribute("lang")) === "en",
      "Unsaved language applied immediately",
    );
    await button("settings.discard").click();
    assert(
      (await language.inputValue()) === "system",
      "Discard did not restore the saved language",
    );
    await page
      .getByLabel(label("settings.start_hidden"), { exact: true })
      .uncheck();
    await button("settings.save_general").click();
    await page.getByText(label("settings.saved"), { exact: true }).waitFor();
    assert(
      await page.evaluate(
        () => window.__settingsTest.preferences.startHidden === false,
      ),
      "English general save did not persist the preference",
    );
    await button("settings.privacy").click();
    await page
      .getByRole("heading", { name: label("settings.privacy"), exact: true })
      .waitFor();
    await page
      .getByText(label("settings.permission_granted"), { exact: true })
      .first()
      .waitFor();
  } else if (flow === "layout") {
    for (const [width, height] of [
      [340, 500],
      [320, 440],
    ] as const) {
      await page.setViewportSize({ width, height });
      await page.goto(baseUrl);
      for (const key of [
        "app.keypad",
        "app.contacts",
        "app.history",
        "app.getting_started",
      ]) {
        await button(key).click();
        assert(
          !(await page.evaluate(
            () => document.documentElement.scrollWidth > innerWidth,
          )),
          `English ${key} overflow at ${width}`,
        );
      }
    }
    for (const [width, height] of [
      [900, 660],
      [740, 540],
    ] as const) {
      await page.setViewportSize({ width, height });
      await page.goto(`${baseUrl}/?window=settings&section=general`);
      for (const key of [
        "settings.general",
        "settings.sip_accounts",
        "settings.sound",
        "settings.notifications",
        "settings.contacts_history",
        "settings.privacy",
        "settings.diagnostics_help",
        "settings.about_title",
      ]) {
        const escaped = label(key).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
        await page
          .getByRole("button", {
            name: new RegExp(`^${escaped}(?:\\s+\\d+)?$`),
          })
          .click();
        await page
          .getByRole("heading", { name: label(key), exact: true })
          .waitFor();
        assert(
          !(await page.evaluate(
            () => document.documentElement.scrollWidth > innerWidth,
          )),
          `English ${key} overflow at ${width}`,
        );
      }
    }
  } else throw new Error(`Unknown English flow ${flow}`);
  assert(
    (await page.locator("html").getAttribute("lang")) === "en",
    `${flow} did not render in English`,
  );
  assert(!errors.length, errors.join("\n"));
  return { passed: true };
}
