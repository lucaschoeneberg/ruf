import type { Page } from "playwright-core";

// Preview-only keyboard checks: no SIP, contacts, history or keychain access.
export default async function shortcuts(
  page: Page,
  baseUrl: string,
  capture = false,
) {
  const target = await page.context().newPage();
  target.setDefaultTimeout(10000);
  const errors: string[] = [];
  target.on("pageerror", (error) => errors.push(error.message));
  await target.addInitScript(() =>
    localStorage.setItem("sip.introduction.v1", "seen"),
  );
  const assert = (ok: unknown, message: string) => {
    if (!ok) throw new Error(message);
  };
  await target.setViewportSize({ width: 320, height: 440 });
  await target.goto(baseUrl);
  const button = target.getByRole("button", {
    name: "Ruf durchsuchen",
    exact: true,
  });
  await button.waitFor();
  const number = target.getByRole("textbox", {
    name: "Nummer oder Nebenstelle",
  });
  await number.fill("123");
  await target.keyboard.press("Control+k");
  const dialog = target.getByRole("dialog", {
    name: "Ruf durchsuchen",
  });
  const input = dialog.getByRole("combobox");
  await input.fill("mikrofon");
  assert(
    (await dialog.getByRole("option").first().getAttribute("aria-disabled")) ===
      "true",
    "Preview call action must be unavailable",
  );
  await input.press("Enter");
  assert(await dialog.isVisible(), "Disabled call action must not execute");
  await input.fill("xyz-keine-aktion");
  await dialog.getByRole("status").waitFor();
  await input.press("ArrowDown");
  await input.press("Escape");
  assert(
    await number.evaluate((el) => el === document.activeElement),
    "Escape restores number entry focus",
  );
  await target.keyboard.press("Control+k");
  await input.fill("verlauf anzeigen");
  await input.press("Enter");
  await target
    .locator('.bottom-nav button[aria-current="page"]')
    .filter({ hasText: "Verlauf" })
    .waitFor();
  await target.keyboard.press("Control+1");
  assert(
    (await number.inputValue()) === "123",
    "Navigation preserves dial draft",
  );
  await target.keyboard.press("Control+l");
  assert(
    await number.evaluate((el) => el === document.activeElement),
    "Number shortcut focuses entry",
  );
  await target.keyboard.press("Control+2");
  await target
    .locator('.bottom-nav button[aria-current="page"]')
    .filter({ hasText: "Kontakte" })
    .waitFor();
  await target.keyboard.press("Control+3");
  await target
    .locator('.bottom-nav button[aria-current="page"]')
    .filter({ hasText: "Verlauf" })
    .waitFor();
  await button.click();
  await input.fill("ton");
  const first = await input.getAttribute("aria-activedescendant");
  await input.press("ArrowDown");
  assert(
    (await input.getAttribute("aria-activedescendant")) !== first,
    "Arrow key changes active option",
  );
  await input.press("Escape");
  assert(
    await button.evaluate((el) => el === document.activeElement),
    "Closing restores toolbar focus",
  );
  if (capture) {
    await target.keyboard.press("Control+1");
    await target.screenshot({
      path: "output/playwright/shortcuts-phone-320.png",
    });
  }
  assert(!errors.length, errors.join("\n"));
  await target.close();
  return { passed: true };
}
