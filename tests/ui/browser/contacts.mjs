// Exercise permission controls through IPC fixtures, never the personal address book.
export default async function contactsPermissions(page, baseUrl) {
  page.setDefaultTimeout(10000);
  await page.addInitScript(() => {
    window.isTauri = true;
    localStorage.setItem("sip.introduction.v1", "seen");

    const callbacks = new Map();
    const listeners = new Map();
    let next = 0;
    window.__contactPermissionTest = {
      preferences: {
        startHidden: true,
        showCallerInNotifications: true,
        contacts: {
          favoritesFirst: true,
          showOrganization: true,
          matchGermanNumbers: true,
        },
        history: { recordingEnabled: true, retentionDays: null },
      },
      permission: "notDetermined",
      decision: "denied",
      failRequest: false,
      failPreferences: false,
      failCopy: false,
      failOpen: false,
      holdFavorite: false,
      holdRead: false,
      resolveFavorite: null,
      resolveRead: null,
      copied: null,
      commands: [],
      emit() {
        for (const [id, listener] of listeners)
          if (listener.event === "contacts-changed")
            callbacks.get(listener.handler)?.({
              id,
              event: listener.event,
              payload: null,
            });
      },
    };
    const snapshot = {
      revision: 1,
      callConnectedAtMs: null,
      callEvents: [],
      callEventsDropped: 0,
      registration: "unconfigured",
      call: "idle",
      callId: null,
      caller: null,
      callerContact: null,
      historyError: null,
      lastError: null,
      accounts: [],
      selectedAccountId: null,
      callAccountId: null,
      audioMode: null,
      ringtonePlaying: false,
      soundSettings: {
        inputDevice: "",
        outputDevice: "",
        ringDevice: "",
        ringtoneEnabled: true,
      },
    };
    window.__contactPermissionTest.snapshot = snapshot;
    window.__contactPermissionTest.contacts = [
      {
        id: "synthetic-multi",
        source: "macos",
        name: "Anna Beispiel",
        organization: "Beispiel GmbH",
        favorite: false,
        phones: [
          {
            label: "Arbeit",
            number: "+49 30 123-456",
            destination: "+4930123456",
          },
          { label: "Intern", number: "1234", destination: "1234" },
        ],
      },
      {
        id: "synthetic-single",
        source: "macos",
        name: "Ben Beispiel",
        organization: "",
        favorite: false,
        phones: [{ label: "Mobil", number: "456", destination: "456" }],
      },
      {
        id: "synthetic-empty",
        source: "macos",
        name: "Ohne Telefonnummer",
        organization: "",
        favorite: false,
        phones: [],
      },
      {
        id: "synthetic-unsupported",
        source: "macos",
        name: "Zoë Weiß",
        organization: "Testbüro",
        favorite: false,
        phones: [
          { label: "Mobil", number: "+49 (0) 170 1234", destination: null },
        ],
      },
    ];
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        async writeText(number) {
          const f = window.__contactPermissionTest;
          if (f.failCopy) throw new Error("Synthetic copy error");
          f.copied = number;
        },
      },
    });
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener(_event, id) {
        listeners.delete(id);
      },
    };
    window.__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: "settings" },
        currentWebview: { label: "settings" },
      },
      transformCallback(callback) {
        callbacks.set(++next, callback);
        return next;
      },
      unregisterCallback(id) {
        callbacks.delete(id);
      },
      async invoke(command, args) {
        const f = window.__contactPermissionTest;
        f.commands.push({ command, args });
        if (command === "plugin:event|listen") {
          const id = ++next;
          listeners.set(id, { event: args.event, handler: args.handler });
          return id;
        }
        if (command === "plugin:event|unlisten" || command === "settings_ready")
          return null;
        if (command === "get_app_info")
          return { version: "0.1.0", telephony: "linked" };
        if (command === "get_engine_snapshot")
          return structuredClone(window.__rufTestSyncDialogs(snapshot));
        if (command === "get_desktop_settings")
          return {
            autostartEnabled: false,
            autostartAvailable: true,
            notificationsEnabled: false,
            notificationsGranted: null,
            notificationPermission: "unsupported",
          };
        if (command === "get_preferences") {
          if (f.failPreferences)
            throw new Error("Synthetic preferences read failure");
          return structuredClone(f.preferences);
        }
        if (command === "get_notification_permission") return "unsupported";
        if (command === "get_microphone_permission") return "unsupported";
        if (command === "get_contacts_permission") return f.permission;
        if (command === "request_contacts_permission") {
          if (f.failRequest)
            throw {
              code: "nativeFailure",
              message: "synthetic request failure",
            };
          f.permission = f.decision;
          return f.permission;
        }
        if (command === "open_contacts_settings") return null;
        if (command === "open_contacts_app") {
          if (f.failOpen) throw new Error("Synthetic open error");
          return null;
        }
        if (command === "copy_phone_number") {
          if (f.failCopy) throw new Error("Synthetic copy error");
          f.copied = args.number;
          return null;
        }
        if (command === "get_contacts" || command === "refresh_contacts") {
          const response = {
            permission: f.permission,
            contacts:
              f.permission === "authorized" ? structuredClone(f.contacts) : [],
          };
          if (f.holdRead) {
            f.holdRead = false;
            return new Promise((resolve) => {
              f.resolveRead = () => resolve(response);
            });
          }
          return response;
        }
        if (command === "set_contact_favorite") {
          if (f.holdFavorite)
            await new Promise((resolve) => {
              f.resolveFavorite = resolve;
            });
          f.contacts.find((item) => item.id === args.id).favorite =
            args.favorite;
          return null;
        }
        if (command === "get_history")
          return {
            entries: [],
            total: 0,
            hasMore: false,
            recordingError: null,
          };
        if (command === "dial") {
          snapshot.call = "calling";
          snapshot.callId = "synthetic-call";
          snapshot.callAccountId = 1;
          return structuredClone(window.__rufTestSyncDialogs(snapshot));
        }
        if (command === "hangup") {
          snapshot.call = "idle";
          snapshot.callId = null;
          snapshot.callAccountId = null;
          return structuredClone(window.__rufTestSyncDialogs(snapshot));
        }
        throw new Error("Unexpected contacts fixture command: " + command);
      },
    };
  });
  const ensure = (condition, message) => {
    if (!condition) throw new Error(message);
  };
  await page.setViewportSize({ width: 900, height: 660 });
  await page.goto(baseUrl + "/?window=settings&section=privacy");
  await page
    .getByRole("heading", { name: "Datenschutz", exact: true })
    .waitFor({ timeout: 5000 });
  await page
    .getByRole("button", { name: "Zugriff anfordern", exact: true })
    .waitFor();
  ensure(
    await page.evaluate(
      () =>
        !window.__contactPermissionTest.commands.some(
          (c) =>
            c.command.startsWith("request_contacts") ||
            c.command === "get_contacts",
        ),
    ),
    "Opening Settings must neither prompt nor enumerate contacts",
  );
  await page.evaluate(
    () => (window.__contactPermissionTest.failRequest = true),
  );
  await page
    .getByRole("button", { name: "Zugriff anfordern", exact: true })
    .click();
  await page.locator(".contacts-permission-error").waitFor();
  ensure(
    await page
      .getByRole("button", { name: "Zugriff anfordern", exact: true })
      .isEnabled(),
    "A failed request must remain recoverable",
  );
  await page.evaluate(
    () => (window.__contactPermissionTest.failRequest = false),
  );
  await page
    .getByRole("button", { name: "Zugriff anfordern", exact: true })
    .click();
  await page.getByText("Nicht freigegeben", { exact: true }).waitFor();
  await page
    .getByRole("button", { name: "Systemeinstellungen öffnen", exact: true })
    .click();
  ensure(
    await page.evaluate(() =>
      window.__contactPermissionTest.commands.some(
        (c) => c.command === "open_contacts_settings",
      ),
    ),
    "Denied access opens native privacy settings",
  );
  await page.evaluate(() => {
    window.__contactPermissionTest.permission = "authorized";
    window.dispatchEvent(new Event("focus"));
  });
  await page.getByText("Freigegeben", { exact: true }).waitFor();
  ensure(
    (await page
      .getByRole("button", { name: "Zugriff anfordern", exact: true })
      .count()) === 0,
    "Authorized access must not prompt again",
  );
  ensure(
    await page.evaluate(
      () =>
        !window.__contactPermissionTest.commands.some(
          (c) => c.command === "get_contacts",
        ),
    ),
    "Settings permission checks never enumerate contact data",
  );
  await page.screenshot({
    path: "output/playwright/contacts-settings-granted.png",
  });
  await page.evaluate(() => {
    window.__contactPermissionTest.permission = "denied";
    window.dispatchEvent(new Event("focus"));
  });
  await page.getByText("Nicht freigegeben", { exact: true }).waitFor();
  await page.screenshot({
    path: "output/playwright/contacts-settings-denied.png",
  });
  await page.goto(baseUrl + "/");
  await page.getByRole("button", { name: "Kontakte", exact: true }).click();
  await page
    .getByRole("button", { name: "macOS-Kontakte freigeben", exact: true })
    .waitFor();
  // New document starts undecided; simulate previously denied access then reload it.
  await page.evaluate(() => {
    window.__contactPermissionTest.permission = "denied";
    window.dispatchEvent(new Event("focus"));
  });
  await page
    .getByRole("heading", { name: "Kontakte nicht freigegeben", exact: true })
    .waitFor();
  await page
    .getByRole("button", { name: "Systemeinstellungen öffnen", exact: true })
    .click();
  ensure(
    await page.evaluate(() =>
      window.__contactPermissionTest.commands.some(
        (c) => c.command === "open_contacts_settings",
      ),
    ),
    "Phonebook denied state offers the same native recovery",
  );
  ensure(
    await page.evaluate(
      () =>
        !window.__contactPermissionTest.commands.some((c) =>
          c.command.startsWith("request_contacts"),
        ),
    ),
    "Denied refresh does not re-request consent",
  );
  await page.evaluate(() => {
    const f = window.__contactPermissionTest;
    f.permission = "authorized";
    Object.assign(f.snapshot, {
      registration: "registered",
      audioMode: "system",
      selectedAccountId: 1,
      accounts: [
        {
          id: 1,
          name: "Testkonto",
          transport: "udp",
          mediaSecurity: "disabled",
          authUsername: null,
          outboundProxy: null,
          registrar: "pbx.example.test",
          username: "1000",
          registration: "registered",
          lastError: null,
          removing: false,
        },
      ],
    });
    window.dispatchEvent(new Event("focus"));
  });
  await page.getByRole("searchbox").waitFor();
  await page.waitForFunction(() =>
    document.querySelector(".availability")?.textContent.includes("Angemeldet"),
  );
  await page.evaluate(() => {
    const f = window.__contactPermissionTest;
    f.contacts[3].favorite = true;
    f.emit();
  });
  await page.waitForFunction(
    () =>
      document
        .querySelector(".contact-entry")
        ?.getAttribute("data-contact-id") === "synthetic-unsupported",
  );
  await page.evaluate(() => {
    const f = window.__contactPermissionTest;
    f.preferences.contacts = { favoritesFirst: false, showOrganization: false };
    window.dispatchEvent(new Event("focus"));
  });
  await page.waitForFunction(
    () =>
      document
        .querySelector(".contact-entry")
        ?.getAttribute("data-contact-id") === "synthetic-multi",
  );
  ensure(
    !(await page.locator(".contact-summary").allTextContents()).some(
      (text) => text.includes("Beispiel GmbH") || text.includes("Testbüro"),
    ),
    "Company-hidden preference removes company labels",
  );
  await page.getByRole("searchbox").fill("Testbüro");
  await page.getByText("Zoë Weiß", { exact: true }).waitFor();
  await page.getByRole("searchbox").fill("");
  await page.evaluate(() => {
    const f = window.__contactPermissionTest;
    f.contacts[3].favorite = false;
    f.preferences.contacts = { favoritesFirst: true, showOrganization: true };
    window.dispatchEvent(new Event("focus"));
  });
  await page.waitForFunction(() =>
    document
      .querySelector(".contact-summary")
      ?.textContent.includes("Beispiel GmbH"),
  );
  await page.evaluate(() => {
    window.__contactPermissionTest.failPreferences = true;
    window.dispatchEvent(new Event("focus"));
  });
  await page
    .getByText(
      "Die Kontaktanzeige konnte nicht geladen werden. Die zuletzt bekannte Ansicht bleibt erhalten.",
      { exact: true },
    )
    .waitFor();
  ensure(
    (await page.locator(".contact-entry").count()) === 4,
    "Metadata failure must not hide independently authorized contacts",
  );
  await page.evaluate(() => {
    window.__contactPermissionTest.failPreferences = false;
    window.dispatchEvent(new Event("focus"));
  });
  await page
    .getByText(
      "Die Kontaktanzeige konnte nicht geladen werden. Die zuletzt bekannte Ansicht bleibt erhalten.",
      { exact: true },
    )
    .waitFor({ state: "hidden" });
  await page.setViewportSize({ width: 340, height: 500 });
  await page.screenshot({ path: "output/playwright/contacts-list-340.png" });
  await page.setViewportSize({ width: 320, height: 440 });
  await page.screenshot({ path: "output/playwright/contacts-list-320.png" });
  ensure(
    (await page
      .getByRole("button", { name: "Anna Beispiel, Intern: 1234 anrufen" })
      .count()) === 0,
    "Multiple numbers stay collapsed until a contact is selected",
  );
  const multi = page.getByRole("button", {
    name: "Anna Beispiel: 2 Nummern anzeigen",
    exact: true,
  });
  await multi.click();
  await page.screenshot({
    path: "output/playwright/contacts-expanded-320.png",
  });
  await page
    .getByRole("button", {
      name: "Anna Beispiel, Intern: 1234 anrufen",
      exact: true,
    })
    .click();
  await page.getByRole("button", { name: "Auflegen", exact: true }).waitFor();
  await page.screenshot({ path: "output/playwright/contacts-calling-320.png" });
  const targets = await page.evaluate(() =>
    window.__contactPermissionTest.commands
      .filter((c) => c.command === "dial")
      .map((c) => c.args.destination),
  );
  ensure(
    targets.join(",") === "1234",
    "Expanded number calls the exact chosen destination",
  );
  await page.getByRole("button", { name: "Auflegen", exact: true }).click();
  await page
    .getByRole("button", { name: "Auflegen", exact: true })
    .waitFor({ state: "hidden" });
  await page
    .getByRole("button", {
      name: "Ben Beispiel, Mobil: 456 anrufen",
      exact: true,
    })
    .click();
  await page.getByRole("button", { name: "Auflegen", exact: true }).waitFor();
  ensure(
    await page.evaluate(
      () =>
        window.__contactPermissionTest.commands
          .filter((c) => c.command === "dial")
          .at(-1).args.destination === "456",
    ),
    "Single number is a direct call",
  );
  await page.getByRole("button", { name: "Auflegen", exact: true }).click();
  await page
    .getByRole("button", { name: "Auflegen", exact: true })
    .waitFor({ state: "hidden" });
  await page
    .getByRole("button", {
      name: "Anna Beispiel: Als Favorit merken",
      exact: true,
    })
    .click();
  const filter = page.getByRole("button", {
    name: "Nur Favoriten",
    exact: true,
  });
  await filter.click();
  ensure(
    (await page.locator(".contact-entry").count()) === 1,
    "One filter toggles favorites on",
  );
  await filter.press("Enter");
  ensure(
    (await page.locator(".contact-entry").count()) === 4,
    "Same filter restores all contacts",
  );
  await page.getByRole("searchbox").fill("zoe weiss");
  ensure(
    (await page.locator(".contact-entry").count()) === 1,
    "Unicode search stays available",
  );
  ensure(
    await page
      .getByRole("button", {
        name: "Zoë Weiß, Mobil: +49 (0) 170 1234 anrufen",
      })
      .isDisabled(),
    "Unsupported number remains visibly unavailable",
  );
  await page.getByRole("searchbox").fill("");
  await page.evaluate(() => {
    const f = window.__contactPermissionTest;
    f.holdFavorite = true;
  });
  await page
    .getByRole("button", {
      name: "Ben Beispiel: Als Favorit merken",
      exact: true,
    })
    .click();
  await page.waitForFunction(
    () => !!window.__contactPermissionTest.resolveFavorite,
  );
  await page.evaluate(() => {
    window.__contactPermissionTest.holdRead = true;
    window.dispatchEvent(new Event("focus"));
  });
  await page.waitForFunction(
    () => !!window.__contactPermissionTest.resolveRead,
  );
  await page.evaluate(() => window.__contactPermissionTest.resolveFavorite());
  await page
    .getByRole("button", {
      name: "Ben Beispiel: Favorit entfernen",
      exact: true,
    })
    .waitFor();
  await page.evaluate(async () => {
    window.__contactPermissionTest.resolveRead();
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
  ensure(
    (await page
      .getByRole("button", {
        name: "Ben Beispiel: Favorit entfernen",
        exact: true,
      })
      .count()) === 1,
    "An older focus refresh reverted the saved favorite",
  );
  await page.getByRole("searchbox").focus();
  await page.keyboard.press("Tab");
  ensure(
    await filter.evaluate((el) => el === document.activeElement),
    "Keyboard reaches favorites immediately after search",
  );
  ensure(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
    "No horizontal overflow at minimum width",
  );
  await page.keyboard.press("ControlOrMeta+f");
  ensure(
    await page
      .getByRole("searchbox")
      .evaluate((element) => element === document.activeElement),
    "Find shortcut focuses contact search",
  );
  await page.getByRole("searchbox").fill("ben");
  await page.keyboard.press("ArrowDown");
  ensure(
    await page
      .getByRole("button", {
        name: "Ben Beispiel, Mobil: 456 anrufen",
        exact: true,
      })
      .evaluate((element) => element === document.activeElement),
    "ArrowDown enters the first result",
  );
  await page.keyboard.press("Escape");
  ensure(
    (await page.getByRole("searchbox").inputValue()) === "",
    "Escape clears search",
  );
  await page
    .getByRole("button", { name: "Details zu Ben Beispiel", exact: true })
    .click();
  await page
    .getByRole("button", { name: "macOS-Kontakte öffnen", exact: true })
    .waitFor();
  await page.evaluate(() => (window.__contactPermissionTest.failCopy = true));
  await page
    .getByRole("button", {
      name: "Ben Beispiel, Mobil: 456 kopieren",
      exact: true,
    })
    .click();
  await page.getByRole("alert").waitFor();
  await page.evaluate(() => (window.__contactPermissionTest.failCopy = false));
  await page
    .getByRole("button", {
      name: "Ben Beispiel, Mobil: 456 kopieren",
      exact: true,
    })
    .click();
  ensure(
    await page.evaluate(() => window.__contactPermissionTest.copied === "456"),
    "Copy uses selected original number",
  );
  await page.evaluate(() => (window.__contactPermissionTest.failOpen = true));
  await page
    .getByRole("button", { name: "macOS-Kontakte öffnen", exact: true })
    .click();
  await page.getByRole("alert").waitFor();
  await page.evaluate(() => (window.__contactPermissionTest.failOpen = false));
  await page
    .getByRole("button", { name: "macOS-Kontakte öffnen", exact: true })
    .click();
  ensure(
    await page.evaluate(() =>
      window.__contactPermissionTest.commands.some(
        ({ command }) => command === "open_contacts_app",
      ),
    ),
    "Source app opens through native command",
  );
  await page
    .getByRole("button", { name: "Details zu Anna Beispiel", exact: true })
    .click();
  await page
    .getByRole("button", {
      name: "Anna Beispiel, Arbeit: +49 30 123-456 kopieren",
      exact: true,
    })
    .click();
  ensure(
    await page.evaluate(
      () => window.__contactPermissionTest.copied === "+49 30 123-456",
    ),
    "Copy preserves raw display number and punctuation",
  );
  await page
    .getByRole("button", { name: "Details zu Ohne Telefonnummer", exact: true })
    .click();
  await page
    .getByText("Keine Telefonnummer vorhanden.", { exact: true })
    .waitFor();
  ensure(
    await page
      .getByRole("button", {
        name: "Anrufverlauf von Ohne Telefonnummer",
        exact: true,
      })
      .isDisabled(),
    "Numberless contact cannot open unscoped history",
  );
  await page.evaluate(() => {
    const f = window.__contactPermissionTest;
    f.contacts[1].name = "Ben Umbenannt";
    f.emit();
  });
  await page.getByText("Ben Umbenannt", { exact: true }).waitFor();
  await page.evaluate(() => {
    const f = window.__contactPermissionTest;
    f.permission = "denied";
    f.emit();
  });
  await page
    .getByRole("heading", { name: "Kontakte nicht freigegeben", exact: true })
    .waitFor();
  ensure(
    (await page.locator(".contact-entry").count()) === 0,
    "Revoked authorization clears contact rows immediately",
  );
  ensure(
    await page.evaluate(() => {
      const commands = window.__contactPermissionTest.commands;
      const firstRead = commands.findIndex(
        ({ command }) => command === "get_contacts",
      );
      return (
        commands
          .slice(0, firstRead)
          .some(
            ({ command, args }) =>
              command === "plugin:event|listen" &&
              args.event === "contacts-changed",
          ) && commands.some(({ command }) => command === "refresh_contacts")
      );
    }),
    "Change listener precedes first read; focus uses native refresh",
  );
  return { passed: true };
}
