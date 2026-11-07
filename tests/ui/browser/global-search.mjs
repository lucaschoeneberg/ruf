// Entirely synthetic native IPC: never query a real phonebook or place calls.
export default async function globalSearch(page, baseUrl, capture = false) {
  const target = await page.context().newPage();
  target.setDefaultTimeout(10000);
  const errors = [];
  target.on("pageerror", (error) => errors.push(error.message));
  await target.addInitScript(() => {
    window.isTauri = true;
    localStorage.setItem("sip.introduction.v1", "seen");
    const callbacks = new Map(),
      listeners = new Map();
    let next = 1;
    const contact = {
      id: "contact-ada",
      source: "macos",
      name: "Ada Beispiel",
      organization: "Beispielbüro",
      favorite: true,
      phones: [
        { label: "Büro", number: "+49 30 123-456", destination: "+4930123456" },
        { label: "Intern", number: "1234", destination: "1234" },
      ],
    };
    const entry = {
      id: "history-ada",
      accountName: "Büro",
      accountRegistrar: "pbx.example.test",
      accountUsername: "1000",
      direction: "incoming",
      number: "+4930123456",
      remoteName: null,
      startedAtMs: Date.now() - 60000,
      answeredAtMs: null,
      endedAtMs: Date.now() - 30000,
      durationMs: null,
      outcome: "missed",
      sipStatus: 487,
      contact: { id: contact.id, name: contact.name, source: "macos" },
      callbackAccountId: null,
    };
    const fixture = {
      permission: "authorized",
      failHistory: false,
      manyContacts: false,
      manyHistory: false,
      failContacts: false,
      holdHistory: false,
      pending: [],
      commands: [],
      emit(event) {
        for (const [id, listener] of listeners)
          if (listener.event === event)
            callbacks.get(listener.handler)?.({ id, event, payload: null });
      },
      release() {
        this.pending.splice(0).forEach((resolve) => resolve());
      },
    };
    window.__globalSearch = fixture;
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener(_event, id) {
        listeners.delete(id);
      },
    };
    window.__TAURI_INTERNALS__ = {
      transformCallback(fn) {
        const id = next++;
        callbacks.set(id, fn);
        return id;
      },
      async invoke(command, args = {}) {
        fixture.commands.push({ command, args });
        if (command === "plugin:event|listen") {
          const id = next++;
          listeners.set(id, args);
          return id;
        }
        if (command === "plugin:event|unlisten") return;
        if (command === "get_app_info")
          return { version: "0.1.0", telephony: "notLinked" };
        if (command === "get_desktop_settings")
          return {
            notificationsEnabled: false,
            autostartEnabled: false,
            autostartAvailable: false,
            notificationPermission: "authorized",
            notificationsGranted: true,
          };
        if (command === "get_notification_permission") return "authorized";
        if (command === "get_preferences")
          return {
            startHidden: false,
            showCallerInNotifications: false,
            contacts: {
              favoritesFirst: true,
              showOrganization: true,
              matchGermanNumbers: true,
            },
            history: { recordingEnabled: true, retentionDays: null },
          };
        if (command === "get_contacts" || command === "refresh_contacts") {
          if (fixture.failContacts)
            throw new Error("Synthetic contact read failure");
          return {
            permission: fixture.permission,
            contacts:
              fixture.permission === "authorized"
                ? fixture.manyContacts
                  ? Array.from({ length: 12 }, (_, i) => ({
                      ...contact,
                      id: `contact-${i}`,
                      name: `Ada Beispiel ${i}`,
                    }))
                  : [contact]
                : [],
          };
        }
        if (command === "get_history") {
          const query = args.query.search;
          const result = {
            entries: /ada|123|49|beispiel/i.test(query)
              ? [
                  {
                    ...entry,
                    contact:
                      fixture.permission === "authorized"
                        ? entry.contact
                        : null,
                  },
                ]
              : [],
            total: 0,
            hasMore: false,
            recordingError: null,
          };
          if (fixture.manyHistory && result.entries.length) {
            result.entries = Array.from({ length: 12 }, (_, i) => ({
              ...result.entries[0],
              id: `history-${i}`,
            }));
          }
          result.total = result.entries.length;
          result.entries = result.entries.slice(
            args.query.offset,
            args.query.offset + args.query.limit,
          );
          result.hasMore =
            result.total > args.query.offset + result.entries.length;
          if (fixture.failHistory)
            throw new Error("Synthetic history read failure");
          if (fixture.holdHistory)
            return new Promise((resolve) =>
              fixture.pending.push(() => resolve(structuredClone(result))),
            );
          return structuredClone(result);
        }
        throw new Error("Unexpected global-search IPC: " + command);
      },
    };
  });
  const assert = (ok, message) => {
    if (!ok) throw new Error(message);
  };
  const open = async () => {
    await target
      .getByRole("button", { name: "Ruf durchsuchen", exact: true })
      .click();
    return target.getByRole("dialog");
  };
  await target.setViewportSize({ width: 320, height: 440 });
  await target.goto(baseUrl);
  let dialog = await open();
  await dialog.getByRole("combobox").fill("Ada");
  await dialog.getByRole("option").filter({ hasText: "Verpasst" }).waitFor();
  assert(
    (await dialog
      .getByRole("option")
      .filter({ hasText: "Ada Beispiel" })
      .count()) === 2,
    "A single search returns contact and call history",
  );
  if (capture)
    await target.screenshot({
      path: "output/playwright/global-search-320.png",
    });
  await dialog.getByRole("option").filter({ hasText: "Beispielbüro" }).click();
  await target
    .getByRole("group", { name: "Ada Beispiel: Kontaktdetails", exact: true })
    .waitFor();
  assert(
    !(await target.evaluate(() =>
      window.__globalSearch.commands.some((c) =>
        ["dial", "callback_history", "request_contacts_access"].includes(
          c.command,
        ),
      ),
    )),
    "Opening contact result never calls or prompts",
  );
  dialog = await open();
  await dialog.getByRole("combobox").fill("Ada");
  await dialog.getByRole("combobox").fill("+49 30 123-456");
  await dialog.getByRole("option").filter({ hasText: "Verpasst" }).waitFor();
  assert(
    (await target.evaluate(
      () =>
        window.__globalSearch.commands
          .filter((c) => c.command === "get_history")
          .at(-1).args.query.search,
    )) === "+4930123456",
    "Formatted telephone lookup reuses number normalization",
  );
  await dialog.getByRole("combobox").fill("Ada");
  await dialog.getByRole("option").filter({ hasText: "Verpasst" }).click();
  await target
    .getByRole("group", { name: "Ada Beispiel: Anrufdetails", exact: true })
    .waitFor();
  assert(
    (await target.locator(".history-search input").inputValue()) === "Ada",
    "History result preserves global query",
  );
  dialog = await open();
  await dialog.getByRole("combobox").fill("Ada");
  await dialog.getByRole("option").filter({ hasText: "Verpasst" }).waitFor();
  await target.evaluate(() => {
    window.__globalSearch.permission = "denied";
    window.__globalSearch.emit("contacts-changed");
  });
  await target.waitForFunction(
    () =>
      !document.querySelector("dialog")?.textContent.includes("Ada Beispiel"),
  );
  await dialog.getByText(/Kontakte.*nicht freigegeben/).waitFor();
  await target.evaluate(() => {
    window.__globalSearch.permission = "authorized";
    window.__globalSearch.failContacts = true;
    window.__globalSearch.emit("contacts-changed");
  });
  await dialog
    .getByText("Kontakte sind momentan nicht verfügbar.", { exact: true })
    .waitFor();
  await dialog.getByRole("option").filter({ hasText: "Verpasst" }).waitFor();
  assert(
    !(await dialog.textContent()).includes("Ada Beispiel"),
    "Failed contact read cannot expose cached history contact names",
  );
  await target.evaluate(() => {
    window.__globalSearch.failHistory = true;
    window.__globalSearch.emit("history-changed");
  });
  await dialog.getByText(/Verlauf.*nicht geladen/).waitFor();
  await dialog.getByRole("combobox").fill("Einstellungen");
  await dialog
    .getByRole("option")
    .filter({ hasText: "Einstellungen öffnen" })
    .waitFor();
  await target.evaluate(() => {
    window.__globalSearch.failHistory = false;
    window.__globalSearch.holdHistory = true;
  });
  await dialog.getByRole("combobox").fill("Ada");
  await target.waitForFunction(() => window.__globalSearch.pending.length > 0);
  await dialog.getByRole("combobox").fill("xyz-ohne-treffer");
  await target.evaluate(() => {
    window.__globalSearch.holdHistory = false;
    window.__globalSearch.release();
  });
  await dialog.getByText(/Keine.*Treffer/).waitFor();
  assert(
    !(await dialog.textContent()).includes("Ada Beispiel"),
    "Stale history response cannot replace current query",
  );
  await target.evaluate(() => {
    Object.assign(window.__globalSearch, {
      permission: "authorized",
      failContacts: false,
      manyContacts: true,
      manyHistory: true,
    });
    window.__globalSearch.emit("contacts-changed");
  });
  await dialog.getByRole("combobox").fill("Ada");
  await dialog
    .getByRole("option")
    .filter({ hasText: "Alle 12 Kontakte anzeigen" })
    .click();
  assert(
    (await target.locator(".contact-search input").inputValue()) === "Ada",
    "All contacts preserves query",
  );
  dialog = await open();
  await dialog.getByRole("combobox").fill("Ada");
  await dialog
    .getByRole("option")
    .filter({ hasText: "Alle 12 Anrufe anzeigen" })
    .click();
  await target.waitForFunction(
    () => document.querySelectorAll(".history-entry").length === 12,
  );
  assert(
    (await target.locator(".history-search input").inputValue()) === "Ada",
    "All history preserves query",
  );
  dialog = await open();
  await dialog.getByRole("combobox").fill("Ada");
  await dialog.getByRole("combobox").dispatchEvent("keydown", {
    key: "Enter",
    isComposing: true,
    bubbles: true,
  });
  assert(
    await dialog.isVisible(),
    "IME confirmation cannot open a search result",
  );
  await dialog.getByRole("combobox").press("Escape");
  assert(!errors.length, errors.join("\n"));
  await target.close();
  return { passed: true };
}
