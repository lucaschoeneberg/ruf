// Synthetic native contracts only; never read personal history or place SIP calls.
export default async function historyRegressions(page) {
  const origin = new URL(page.url()).origin;
  page = await page.context().newPage();
  page.setDefaultTimeout(10000);
  await page.addInitScript(() => {
    window.isTauri = !location.search.includes("preview");
    localStorage.setItem("sip.introduction.v1", "seen");

    const callbacks = new Map();
    const listeners = new Map();
    let next = 1;
    const account = {
      id: 1,
      name: "Aktuelles Konto",
      transport: "udp",
      mediaSecurity: "disabled",
      authUsername: null,
      outboundProxy: null,
      registrar: "pbx.example.test",
      username: "1000",
      registration: "registered",
      lastError: null,
      removing: false,
    };
    const state = {
      revision: 1,
      callConnectedAtMs: null,
      callEvents: [],
      callEventsDropped: 0,
      registration: "registered",
      call: "idle",
      callId: null,
      caller: null,
      callerContact: null,
      historyError: null,
      lastError: null,
      accounts: [
        account,
        { ...account, id: 2, name: "Ursprüngliches Konto", username: "2000" },
      ],
      selectedAccountId: 1,
      callAccountId: null,
      audioMode: "system",
      ringtonePlaying: false,
      soundSettings: {
        inputDevice: "",
        outputDevice: "",
        ringDevice: "",
        ringtoneEnabled: true,
      },
    };
    const contact = {
      id: "synthetic-contact",
      source: "macos",
      name: "Anna Beispiel",
      organization: "Testbüro",
      favorite: false,
      phones: [{ label: "Arbeit", number: "1234", destination: "1234" }],
    };
    const entries = Array.from({ length: 55 }, (_, index) => ({
      id: "synthetic-history-" + index,
      accountName: "Ursprüngliches Konto",
      accountRegistrar: "pbx.example.test",
      accountUsername: "2000",
      direction: index % 2 === 0 ? "incoming" : "outgoing",
      number: String(1234 + index),
      remoteName: "Testanruf " + index,
      startedAtMs: Date.now() - index * 60000,
      answeredAtMs: Date.now() - index * 60000 + 1000,
      endedAtMs: Date.now() - index * 60000 + 32000,
      durationMs: 31000,
      outcome: "completed",
      sipStatus: 200,
      contact: null,
      callbackAccountId: 2,
    }));
    Object.assign(entries[0], {
      contact: { id: contact.id, source: "macos", name: contact.name },
      outcome: "missed",
      durationMs: null,
      answeredAtMs: null,
      sipStatus: 487,
    });
    Object.assign(entries[1], {
      number: null,
      remoteName: null,
      callbackAccountId: null,
      outcome: "interrupted",
      endedAtMs: null,
      durationMs: null,
      answeredAtMs: null,
    });
    Object.assign(entries[2], {
      remoteName: "Altes Konto",
      callbackAccountId: null,
    });
    Object.assign(entries[3], {
      remoteName: "Laufender Testanruf",
      outcome: "inProgress",
      durationMs: null,
      endedAtMs: null,
      answeredAtMs: null,
    });
    Object.assign(entries[4], { remoteName: "Archiv [100%]" });
    const fixture = {
      state,
      entries,
      contact,
      contactsAuthorized: true,
      commands: [],
      failLoad: location.search.includes("load-error"),
      failDelete: false,
      failCopy: false,
      copied: null,
      recordingError: null,
      delayed: null,
      holdContacts: false,
      pendingContacts: [],
      emit(event = "history-changed") {
        for (const [id, listener] of listeners)
          if (listener.event === event)
            callbacks.get(listener.handler)?.({
              id,
              event,
              payload:
                event === "phone-state-changed"
                  ? structuredClone(window.__rufTestSyncDialogs(state))
                  : null,
            });
      },
    };
    window.__historyFixture = fixture;
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        async writeText(value) {
          if (fixture.failCopy) throw new Error("Synthetic clipboard error");
          fixture.copied = value;
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
        currentWindow: { label: "main" },
        currentWebview: { label: "main" },
      },
      transformCallback(callback) {
        callbacks.set(++next, callback);
        return next;
      },
      unregisterCallback(id) {
        callbacks.delete(id);
      },
      async invoke(command, args) {
        fixture.commands.push({ command, args });
        if (command === "plugin:event|listen") {
          const id = ++next;
          listeners.set(id, { event: args.event, handler: args.handler });
          return id;
        }
        if (command === "plugin:event|unlisten" || command === "open_settings")
          return null;
        if (command === "get_app_info")
          return { version: "0.1.0", telephony: "linked" };
        if (command === "get_engine_snapshot")
          return structuredClone(window.__rufTestSyncDialogs(state));
        if (command === "get_contacts") {
          const response = {
            permission: fixture.contactsAuthorized ? "authorized" : "denied",
            contacts: fixture.contactsAuthorized
              ? [structuredClone(contact)]
              : [],
          };
          if (fixture.holdContacts)
            return new Promise((resolve) => {
              fixture.pendingContacts.push(() => resolve(response));
            });
          return response;
        }
        if (command === "get_contacts_permission") return "authorized";
        if (command === "refresh_contacts")
          return {
            permission: fixture.contactsAuthorized ? "authorized" : "denied",
            contacts: fixture.contactsAuthorized
              ? [structuredClone(contact)]
              : [],
          };
        if (command === "copy_phone_number") {
          if (fixture.failCopy)
            throw new Error("Synthetic native clipboard error");
          fixture.copied = args.number;
          return null;
        }
        if (command === "get_history") {
          if (fixture.failLoad)
            throw {
              code: "storageFailure",
              message: "Synthetic history error",
            };
          const query = args.query;
          const matching = entries.filter(
            (entry) =>
              (!query.contactNumbers.length ||
                query.contactNumbers.includes(entry.number)) &&
              (query.filter === "all" ||
                (query.filter === "missed"
                  ? entry.outcome === "missed"
                  : entry.direction === query.filter)) &&
              [
                entry.contact?.name,
                entry.remoteName,
                entry.number,
                entry.accountName,
              ].some((value) =>
                value?.toLowerCase().includes(query.search.toLowerCase()),
              ),
          );
          const response = {
            entries: structuredClone(
              matching.slice(query.offset, query.offset + query.limit),
            ),
            total: matching.length,
            hasMore: query.offset + query.limit < matching.length,
            recordingError: fixture.recordingError,
          };
          if (query.search === "delay")
            return new Promise((resolve) => {
              fixture.delayed = () => resolve(response);
            });
          return response;
        }
        if (command === "callback_history") {
          const entry = entries.find((entry) => entry.id === args.id);
          if (!entry?.callbackAccountId || state.call !== "idle")
            throw { code: "invalidState", message: "Unavailable callback" };
          Object.assign(state, {
            call: "calling",
            callId: "synthetic-callback",
            callAccountId: entry.callbackAccountId,
          });
          return structuredClone(window.__rufTestSyncDialogs(state));
        }
        if (command === "delete_history_entry") {
          if (fixture.failDelete)
            throw { code: "storageFailure", message: "Synthetic delete error" };
          const index = entries.findIndex((entry) => entry.id === args.id);
          if (index >= 0 && entries[index].outcome !== "inProgress")
            entries.splice(index, 1);
          fixture.emit();
          return null;
        }
        if (command === "clear_history") {
          if (fixture.failDelete)
            throw { code: "storageFailure", message: "Synthetic clear error" };
          for (let index = entries.length - 1; index >= 0; --index)
            if (entries[index].outcome !== "inProgress")
              entries.splice(index, 1);
          fixture.emit();
          return null;
        }
        throw new Error("Unexpected synthetic history IPC: " + command);
      },
    };
  });
  const results = [];
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  async function check(name, task) {
    try {
      await task();
      results.push({ name, passed: true });
    } catch (error) {
      results.push({ name, passed: false, error: error.message });
    }
  }
  const search = () =>
    page.getByRole("searchbox", {
      name: "Verlauf nach Name, Nummer oder Konto suchen",
    });
  const details = (name) =>
    page.getByRole("button", {
      name: `${name}: Anrufdetails anzeigen`,
      exact: true,
    });
  async function fresh(suffix = "") {
    await page.goto(origin + "/" + suffix);
    await page.getByRole("button", { name: "Verlauf", exact: true }).click();
    if (!suffix) await details("Anna Beispiel").waitFor();
  }
  async function ensure(condition, message) {
    if (!(await condition)) throw new Error(message);
  }
  await page.setViewportSize({ width: 340, height: 500 });
  await check("real page size and incremental pagination", async () => {
    await fresh();
    await ensure(
      page
        .locator(".history-entry")
        .count()
        .then((count) => count === 50),
      "Expected one 50-entry page",
    );
    await page
      .getByRole("button", { name: "Weitere Anrufe anzeigen", exact: true })
      .click();
    await page.waitForFunction(
      () => document.querySelectorAll(".history-entry").length === 55,
    );
    await ensure(
      page.evaluate(() =>
        window.__historyFixture.commands.some(
          ({ command, args }) =>
            command === "get_history" &&
            args.query.offset === 50 &&
            args.query.limit === 50,
        ),
      ),
      "No paginated IPC query",
    );
  });
  await check(
    "shifted pagination never duplicates a rendered call",
    async () => {
      await fresh();
      await page.evaluate(() => {
        const f = window.__historyFixture;
        f.entries.unshift({
          ...f.entries[0],
          id: "newest-call",
          contact: null,
        });
      });
      await page
        .getByRole("button", { name: "Weitere Anrufe anzeigen", exact: true })
        .click();
      await page.locator('.history-panel[aria-busy="false"]').waitFor();
      await ensure(
        page
          .locator(".history-entry")
          .count()
          .then((count) => count === 55),
        "An offset shift duplicated an existing call instead of appending unique calls",
      );
      await details("Testanruf 54").waitFor();
    },
  );
  await check(
    "a fully overlapping page still advances to unread server records",
    async () => {
      await fresh();
      await page.evaluate(() => {
        const f = window.__historyFixture;
        const added = Array.from({ length: 50 }, (_, index) => ({
          ...f.entries[0],
          id: `newest-call-${index}`,
          contact: null,
        }));
        f.entries.unshift(...added);
      });
      const more = page.getByRole("button", {
        name: "Weitere Anrufe anzeigen",
        exact: true,
      });
      await more.click();
      await page.locator('.history-panel[aria-busy="false"]').waitFor();
      await ensure(
        page
          .locator(".history-entry")
          .count()
          .then((count) => count === 50),
        "A fully overlapping page duplicated rendered calls",
      );
      await more.click();
      await page.locator('.history-panel[aria-busy="false"]').waitFor();
      await ensure(
        page
          .locator(".history-entry")
          .count()
          .then((count) => count === 55),
        "Pagination repeated the fully overlapping offset instead of reaching unread calls",
      );
      await details("Testanruf 54").waitFor();
      await more.waitFor({ state: "detached" });
    },
  );
  await check(
    "refresh retains more than two pages, expanded details and keyboard position",
    async () => {
      await fresh();
      await page.evaluate(() => {
        const f = window.__historyFixture;
        const source = f.entries[5];
        for (let index = 55; index < 130; index++)
          f.entries.push({
            ...source,
            id: `synthetic-history-${index}`,
            number: String(1234 + index),
            remoteName: `Testanruf ${index}`,
            startedAtMs: source.startedAtMs - (index - 5) * 60000,
            answeredAtMs: source.answeredAtMs - (index - 5) * 60000,
            endedAtMs: source.endedAtMs - (index - 5) * 60000,
          });
      });
      await page.getByRole("button", { name: "Verlauf aktualisieren" }).click();
      for (const count of [100, 130]) {
        await page
          .getByRole("button", { name: "Weitere Anrufe anzeigen" })
          .click();
        await page.waitForFunction(
          (count) =>
            document.querySelectorAll(".history-entry").length === count,
          count,
        );
      }
      await details("Testanruf 129").click();
      const expanded = page.getByRole("button", {
        name: "Testanruf 129: Anrufdetails ausblenden",
        exact: true,
      });
      await expanded.focus();
      const scrollTop = await page
        .locator(".history-panel")
        .evaluate((e) => e.scrollTop);
      for (const trigger of ["event", "focus"]) {
        await page.evaluate((trigger) => {
          const f = window.__historyFixture;
          f.commands = [];
          if (trigger === "event") f.emit();
          else window.dispatchEvent(new Event("focus"));
        }, trigger);
        await page.waitForFunction(() =>
          window.__historyFixture.commands.some(
            ({ command }) => command === "get_history",
          ),
        );
        await page.locator('.history-panel[aria-busy="false"]').waitFor();
        await ensure(
          page
            .locator(".history-entry")
            .count()
            .then((n) => n === 130),
          "Refresh discarded loaded pages",
        );
        await ensure(expanded.count(), "Refresh collapsed the open call");
        await ensure(
          expanded.evaluate((e) => e === document.activeElement),
          "Refresh lost keyboard focus",
        );
        await ensure(
          page
            .locator(".history-panel")
            .evaluate((e, top) => Math.abs(e.scrollTop - top) < 2, scrollTop),
          "Refresh moved the reading position",
        );
      }
      await page.evaluate(() => {
        window.__historyFixture.failLoad = true;
        window.__historyFixture.emit();
      });
      await page.getByRole("alert").waitFor();
      await page.evaluate(() => (window.__historyFixture.failLoad = false));
      await page
        .getByRole("button", { name: "Erneut versuchen", exact: true })
        .click();
      await page.locator('.history-panel[aria-busy="false"]').waitFor();
      await ensure(
        page
          .locator(".history-entry")
          .count()
          .then((n) => n === 130),
        "Retry discarded the retained window",
      );
    },
  );
  await check(
    "unchanged contact scope retains loaded pages and details on focus",
    async () => {
      await fresh();
      await page.evaluate(() => {
        const f = window.__historyFixture;
        for (const entry of f.entries) entry.number = "1234";
      });
      await details("Anna Beispiel").click();
      await page
        .getByRole("button", { name: "Kontakt Anna Beispiel öffnen" })
        .click();
      await page
        .getByRole("button", { name: "Anrufverlauf von Anna Beispiel" })
        .click();
      await page.getByText("Für Anna Beispiel", { exact: true }).waitFor();
      await page
        .getByRole("button", { name: "Weitere Anrufe anzeigen" })
        .click();
      await page.waitForFunction(
        () => document.querySelectorAll(".history-entry").length === 55,
      );
      await details("Testanruf 54").click();
      await page.evaluate(() => window.dispatchEvent(new Event("focus")));
      await page.locator('.history-panel[aria-busy="false"]').waitFor();
      await page.evaluate(
        () => new Promise((resolve) => setTimeout(resolve, 0)),
      );
      await ensure(
        page
          .locator(".history-entry")
          .count()
          .then((n) => n === 55),
        "Unchanged contact scope discarded loaded pages",
      );
      await ensure(
        page
          .getByRole("button", {
            name: "Testanruf 54: Anrufdetails ausblenden",
          })
          .count(),
        "Unchanged contact scope collapsed details",
      );
    },
  );
  await check(
    "search batches typing and retains results while a response is pending",
    async () => {
      await fresh();
      await page.evaluate(() => (window.__historyFixture.commands = []));
      await search().pressSequentially("Testanruf", { delay: 10 });
      await ensure(
        page
          .locator(".history-entry")
          .count()
          .then((n) => n === 50),
        "Typing removed readable results",
      );
      await page.waitForFunction(() =>
        window.__historyFixture.commands.some(
          ({ command, args }) =>
            command === "get_history" && args.query.search === "Testanruf",
        ),
      );
      const queries = await page.evaluate(() =>
        window.__historyFixture.commands
          .filter(({ command }) => command === "get_history")
          .map(({ args }) => args.query.search),
      );
      if (queries.length !== 1 || queries[0] !== "Testanruf")
        throw new Error(`Typing was not batched: ${JSON.stringify(queries)}`);
      const count = await page.locator(".history-entry").count();
      await search().fill("delay");
      await page.waitForFunction(() => !!window.__historyFixture.delayed);
      await ensure(
        page
          .locator(".history-entry")
          .count()
          .then((n) => n === count),
        "Pending search removed previous results",
      );
      await page.evaluate(() => window.__historyFixture.delayed());
    },
  );
  await check(
    "literal name number account search and no-result recovery",
    async () => {
      await fresh();
      await search().fill("[100%]");
      await page.waitForFunction(
        () =>
          window.__historyFixture.commands
            .filter(({ command }) => command === "get_history")
            .at(-1)?.args.query.search === "[100%]" &&
          document.querySelector('.history-panel[aria-busy="false"]'),
      );
      await details("Archiv [100%]").waitFor();
      await ensure(
        page
          .locator(".history-entry")
          .count()
          .then((count) => count === 1),
        "Literal search matched extra entries",
      );
      await search().fill("1234");
      await details("Anna Beispiel").waitFor();
      await search().fill("ursprüngliches konto");
      await page.waitForFunction(
        () => document.querySelectorAll(".history-entry").length === 50,
      );
      await search().fill("keine treffer");
      await page
        .getByRole("heading", { name: "Keine passenden Anrufe", exact: true })
        .waitFor();
      await page
        .getByRole("button", {
          name: "Suche und Filter zurücksetzen",
          exact: true,
        })
        .click();
      await details("Anna Beispiel").waitFor();
    },
  );
  await check("missed and direction filters use backend queries", async () => {
    await fresh();
    await page.getByRole("button", { name: "Verpasst", exact: true }).click();
    await page.waitForFunction(
      () => document.querySelectorAll(".history-entry").length === 1,
    );
    await page.getByRole("button", { name: "Ausgehend", exact: true }).click();
    await details("Unbekannte Nummer").waitFor();
    await ensure(
      page
        .locator(".history-entry")
        .count()
        .then((count) => count === 27),
      "Wrong outgoing filter",
    );
    await page.getByRole("button", { name: "Eingehend", exact: true }).click();
    await details("Anna Beispiel").waitFor();
    await ensure(
      page
        .locator(".history-entry")
        .count()
        .then((count) => count === 28),
      "Wrong incoming filter",
    );
  });
  await check("latest search supersedes older delayed response", async () => {
    await fresh();
    await search().fill("delay");
    await page.waitForFunction(() => !!window.__historyFixture.delayed);
    await page.locator('.history-panel[aria-busy="true"]').waitFor();
    await search().fill("Anna");
    await page.waitForFunction(
      () =>
        window.__historyFixture.commands
          .filter(({ command }) => command === "get_history")
          .at(-1)?.args.query.search === "Anna" &&
        document.querySelector('.history-panel[aria-busy="false"]'),
    );
    await details("Anna Beispiel").waitFor();
    await page.evaluate(() => window.__historyFixture.delayed());
    await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 0)));
    await details("Anna Beispiel").waitFor();
  });
  await check(
    "initial history storage error offers retry without fabricating empty history",
    async () => {
      await fresh("?load-error");
      await page.getByRole("alert").waitFor();
      await ensure(
        page
          .getByRole("heading", { name: "Noch keine Anrufe", exact: true })
          .count()
          .then((count) => count === 0),
        "Storage error appeared as empty history",
      );
      await page.evaluate(() => (window.__historyFixture.failLoad = false));
      await page
        .getByRole("button", { name: "Erneut versuchen", exact: true })
        .click();
      await details("Anna Beispiel").waitFor();
    },
  );
  await check(
    "load and pagination errors recover without account-storage copy",
    async () => {
      await fresh();
      await page.evaluate(() => (window.__historyFixture.failLoad = true));
      await page
        .getByRole("button", { name: "Weitere Anrufe anzeigen", exact: true })
        .click();
      await page.getByRole("alert").waitFor();
      await ensure(
        page
          .locator(".history-entry")
          .count()
          .then((count) => count === 50),
        "Failed pagination removed previous page",
      );
      await ensure(
        page
          .getByRole("alert")
          .textContent()
          .then(
            (text) =>
              text.includes("App-Datenordner") &&
              !text.includes("Schlüsselbund"),
          ),
        "Misleading history storage error",
      );
      await page.evaluate(() => (window.__historyFixture.failLoad = false));
      await page
        .getByRole("button", { name: "Erneut versuchen", exact: true })
        .click();
      await page.waitForFunction(
        () => document.querySelectorAll(".history-entry").length === 55,
      );
      await page.evaluate(() => (window.__historyFixture.failLoad = true));
      await page
        .getByRole("button", { name: "Verlauf aktualisieren", exact: true })
        .click();
      await page.getByRole("alert").waitFor();
      await page.evaluate(() => (window.__historyFixture.failLoad = false));
      await page
        .getByRole("button", { name: "Erneut versuchen", exact: true })
        .click();
      await page.waitForFunction(
        () => document.querySelectorAll(".history-entry").length === 55,
      );
      await page.locator('.history-panel[aria-busy="false"]').waitFor();
      await ensure(
        page.evaluate(
          () =>
            window.__historyFixture.commands
              .filter(({ command }) => command === "get_history")
              .slice(-2)
              .map(({ args }) => args.query.offset)
              .join(",") === "0,50",
        ),
        "Refresh retry appended instead of reloading its original query",
      );
    },
  );
  await check(
    "callback sends history ID and retains original account",
    async () => {
      await fresh();
      await details("Anna Beispiel").click();
      await page.getByRole("button", { name: "Rückruf", exact: true }).click();
      await page.waitForFunction(() =>
        window.__historyFixture.commands.some(
          ({ command, args }) =>
            command === "callback_history" && args.id === "synthetic-history-0",
        ),
      );
      await ensure(
        page.evaluate(
          () =>
            window.__historyFixture.state.callAccountId === 2 &&
            window.__historyFixture.state.selectedAccountId === 1,
        ),
        "Callback switched to unrelated selected account",
      );
      await page
        .getByRole("region", { name: "Aktuelles Gespräch", exact: true })
        .getByRole("heading", { name: "1234", exact: true })
        .waitFor();
      await page.evaluate(() => {
        const f = window.__historyFixture;
        Object.assign(f.state, {
          call: "idle",
          callId: null,
          callAccountId: null,
        });
        f.emit("phone-state-changed");
      });
    },
  );
  await check(
    "unknown durations and unavailable callback are explicit",
    async () => {
      await fresh();
      await details("Unbekannte Nummer").click();
      await page
        .getByText("Keine Nummer für den Rückruf vorhanden.", { exact: true })
        .waitFor();
      await ensure(
        page.getByRole("button", { name: "Rückruf", exact: true }).isDisabled(),
        "Anonymous callback enabled",
      );
      await page.getByText("Unbekannt", { exact: true }).waitFor();
      await details("Altes Konto").click();
      await page
        .getByText(
          "Das ursprüngliche Konto ist nicht verbunden oder nicht mehr vorhanden.",
          { exact: true },
        )
        .waitFor();
      await ensure(
        page.getByRole("button", { name: "Rückruf", exact: true }).isDisabled(),
        "Deleted account silently eligible for callback",
      );
    },
  );
  await check("copy errors are visible and copy can be retried", async () => {
    await fresh();
    await details("Anna Beispiel").click();
    await page.evaluate(() => (window.__historyFixture.failCopy = true));
    await page
      .getByRole("button", { name: "Nummer kopieren", exact: true })
      .click();
    await page.getByRole("alert").waitFor();
    await page.evaluate(() => (window.__historyFixture.failCopy = false));
    await page
      .getByRole("button", { name: "Nummer kopieren", exact: true })
      .click();
    await page.getByRole("button", { name: "Kopiert", exact: true }).waitFor();
    await ensure(
      page.evaluate(() => window.__historyFixture.copied === "1234"),
      "Wrong copied number",
    );
  });
  await check(
    "single delete is explicit, cancellable and recoverable",
    async () => {
      await fresh();
      await details("Anna Beispiel").click();
      await page
        .getByRole("button", { name: "Anruf löschen …", exact: true })
        .click();
      await ensure(
        page.evaluate(
          () =>
            !window.__historyFixture.commands.some(
              ({ command }) => command === "delete_history_entry",
            ),
        ),
        "Delete ran before confirmation",
      );
      await page
        .getByRole("button", { name: "Abbrechen", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Anruf löschen …", exact: true })
        .click();
      await page.evaluate(() => (window.__historyFixture.failDelete = true));
      await page
        .getByRole("button", { name: "Anruf löschen", exact: true })
        .click();
      await page.getByRole("alert").waitFor();
      await page.evaluate(() => (window.__historyFixture.failDelete = false));
      await page
        .getByRole("button", { name: "Anruf löschen", exact: true })
        .click();
      await page
        .getByText("Anna Beispiel", { exact: true })
        .waitFor({ state: "detached" });
      await ensure(
        page.evaluate(
          () =>
            !window.__historyFixture.entries.some(
              ({ id }) => id === "synthetic-history-0",
            ),
        ),
        "Confirmed record not deleted",
      );
    },
  );
  await check(
    "clear confirms full scope and preserves ongoing entries",
    async () => {
      await fresh();
      await details("Laufender Testanruf").click();
      await ensure(
        page
          .getByRole("button", { name: "Anruf löschen …", exact: true })
          .isDisabled(),
        "Ongoing call can be deleted",
      );
      await page
        .getByRole("button", { name: "Verlauf leeren", exact: true })
        .click();
      await page
        .getByText(
          /Gesamten Verlauf löschen\? Laufende Anrufe bleiben erhalten/,
        )
        .waitFor();
      await ensure(
        page.evaluate(
          () =>
            !window.__historyFixture.commands.some(
              ({ command }) => command === "clear_history",
            ),
        ),
        "Clear ran without confirmation",
      );
      await page
        .getByRole("button", { name: "Verlauf löschen", exact: true })
        .click();
      await page.waitForFunction(
        () => document.querySelectorAll(".history-entry").length === 1,
      );
      await details("Laufender Testanruf").waitFor();
    },
  );
  await check(
    "history and contact events refresh current authorized names",
    async () => {
      await fresh();
      await page.evaluate(() => {
        const f = window.__historyFixture;
        f.entries[0].contact.name = "Neuer Kontaktname";
        f.emit("contacts-changed");
      });
      await details("Neuer Kontaktname").waitFor();
      await page.evaluate(() => {
        const f = window.__historyFixture;
        f.entries[0].contact = null;
        f.emit("history-changed");
      });
      await details("Testanruf 0").waitFor();
      await ensure(
        page
          .getByText("Neuer Kontaktname", { exact: true })
          .count()
          .then((count) => count === 0),
        "Revoked projection remained visible",
      );
    },
  );
  await check(
    "contact links open the matching authorized source contact",
    async () => {
      await fresh();
      await details("Anna Beispiel").click();
      await page
        .getByRole("button", {
          name: "Kontakt Anna Beispiel öffnen",
          exact: true,
        })
        .click();
      await page
        .getByRole("heading", { name: "Kontakte", exact: true })
        .waitFor();
      await page.getByText("Anna Beispiel", { exact: true }).waitFor();
      await page
        .getByRole("button", {
          name: "Anrufverlauf von Anna Beispiel",
          exact: true,
        })
        .click();
      await page.getByText("Für Anna Beispiel", { exact: true }).waitFor();
      await details("Anna Beispiel").waitFor();
      await ensure(
        page
          .locator(".history-entry")
          .count()
          .then((count) => count === 1),
        "Contact scope did not restrict numbers",
      );
      await ensure(
        page.evaluate(() =>
          window.__historyFixture.commands.some(
            ({ command, args }) =>
              command === "get_history" &&
              args.query.contactNumbers.join(",") === "1234",
          ),
        ),
        "Contact scope missing from query",
      );
      await page.getByRole("button", { name: "Tastatur", exact: true }).click();
      await page.evaluate(async () => {
        const f = window.__historyFixture;
        f.contact.name = "Anna Umbenannt";
        f.entries[0].contact.name = "Anna Umbenannt";
        f.emit("contacts-changed");
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
      await page
        .getByRole("textbox", { name: "Nummer oder Nebenstelle", exact: true })
        .waitFor();
      await page.getByRole("button", { name: "Verlauf", exact: true }).click();
      await page.getByText("Für Anna Umbenannt", { exact: true }).waitFor();
      await page
        .getByRole("button", { name: "Alle Anrufe", exact: true })
        .click();
      await page.waitForFunction(
        () => document.querySelectorAll(".history-entry").length === 50,
      );
    },
  );
  await check(
    "contact invalidation removes retained names even when hidden or storage reload fails",
    async () => {
      await fresh();
      await page.evaluate(() => {
        const f = window.__historyFixture;
        f.contactsAuthorized = false;
        f.entries[0].contact = null;
        f.failLoad = true;
        f.emit("contacts-changed");
      });
      await details("Testanruf 0").waitFor();
      await page.getByRole("alert").waitFor();
      await ensure(
        page
          .getByText("Anna Beispiel", { exact: true })
          .count()
          .then((count) => count === 0),
        "Failed reload retained a revoked contact name",
      );
      await fresh();
      await page.evaluate(() => {
        Object.defineProperty(document, "visibilityState", {
          configurable: true,
          get: () => "hidden",
        });
        const f = window.__historyFixture;
        f.contactsAuthorized = false;
        f.entries[0].contact = null;
        f.emit("contacts-changed");
      });
      await details("Testanruf 0").waitFor();
      await ensure(
        page
          .getByText("Anna Beispiel", { exact: true })
          .count()
          .then((count) => count === 0),
        "Hidden page retained revoked contact projection",
      );
    },
  );
  await check(
    "scope revocation clears contact name even when history storage fails",
    async () => {
      await fresh();
      await details("Anna Beispiel").click();
      await page
        .getByRole("button", {
          name: "Kontakt Anna Beispiel öffnen",
          exact: true,
        })
        .click();
      await page
        .getByRole("button", {
          name: "Anrufverlauf von Anna Beispiel",
          exact: true,
        })
        .click();
      await page.getByText("Für Anna Beispiel", { exact: true }).waitFor();
      await page.evaluate(() => {
        const f = window.__historyFixture;
        f.contactsAuthorized = false;
        f.entries[0].contact = null;
        f.failLoad = true;
        f.emit("contacts-changed");
      });
      await page.locator(".contact-scope").waitFor({ state: "detached" });
      await page.getByRole("alert").waitFor();
      await ensure(
        page
          .getByText("Anna Beispiel", { exact: true })
          .count()
          .then((count) => count === 0),
        "Scope or row retained revoked contact name",
      );
    },
  );
  await check(
    "contact history scope ignores older same-contact reads",
    async () => {
      await fresh();
      await details("Anna Beispiel").click();
      await page
        .getByRole("button", {
          name: "Kontakt Anna Beispiel öffnen",
          exact: true,
        })
        .click();
      await page
        .getByRole("button", {
          name: "Anrufverlauf von Anna Beispiel",
          exact: true,
        })
        .click();
      await page.getByText("Für Anna Beispiel", { exact: true }).waitFor();
      await page.evaluate(() => {
        const f = window.__historyFixture;
        f.holdContacts = true;
        window.dispatchEvent(new Event("focus"));
        f.contact.name = "Aktueller Kontaktname";
        window.dispatchEvent(new Event("focus"));
      });
      await page.waitForFunction(
        () => window.__historyFixture.pendingContacts.length === 2,
      );
      await page.evaluate(() => window.__historyFixture.pendingContacts[1]());
      await page
        .getByText("Für Aktueller Kontaktname", { exact: true })
        .waitFor();
      await page.evaluate(async () => {
        window.__historyFixture.pendingContacts[0]();
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
      await ensure(
        page
          .getByText("Für Aktueller Kontaktname", { exact: true })
          .count()
          .then((count) => count === 1),
        "An older read reverted the current contact scope",
      );
    },
  );
  await check(
    "recording errors stay explicit beside existing records",
    async () => {
      await fresh();
      await page.evaluate(() => {
        const f = window.__historyFixture;
        f.recordingError = {
          code: "storageFailure",
          message: "Synthetic recording failure",
        };
        f.emit();
      });
      await page.getByRole("alert").waitFor();
      await details("Anna Beispiel").waitFor();
    },
  );
  await check(
    "compact layouts preserve readable details without horizontal overflow",
    async () => {
      for (const size of [
        { width: 340, height: 500 },
        { width: 320, height: 440 },
      ]) {
        await page.setViewportSize(size);
        await fresh();
        await details("Anna Beispiel").click();
        await page
          .getByRole("button", { name: "Nummer kopieren", exact: true })
          .scrollIntoViewIfNeeded();
        await ensure(
          page.evaluate(
            () =>
              document.documentElement.scrollWidth <= innerWidth &&
              [
                ...document.querySelectorAll(
                  ".history-panel, .history-details",
                ),
              ].every((element) => element.scrollWidth <= element.clientWidth),
          ),
          "Horizontal overflow at minimum size",
        );
        await page.screenshot({
          path: `output/playwright/history-details-${size.width}.png`,
        });
      }
    },
  );
  await check(
    "browser preview and true empty states contain no fabricated records",
    async () => {
      await fresh();
      await page.evaluate(() => {
        const f = window.__historyFixture;
        f.entries.length = 0;
        f.emit();
      });
      await page
        .getByRole("heading", { name: "Noch keine Anrufe", exact: true })
        .waitFor();
      await fresh("?preview");
      await page
        .getByRole("heading", {
          name: "Verlauf in der Desktop-App",
          exact: true,
        })
        .waitFor();
      await ensure(
        page
          .locator(".history-entry")
          .count()
          .then((count) => count === 0),
        "Browser preview invented history",
      );
    },
  );
  if (errors.length)
    results.push({
      name: "no browser exceptions",
      passed: false,
      error: errors.join("; "),
    });
  await page.close();
  return { results, passed: results.every((result) => result.passed) };
}
