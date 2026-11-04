export default async function phoneRegressions(page) {
  const origin = new URL(page.url()).origin;
  page = await page.context().newPage();
  page.setDefaultTimeout(10000);
  await page.addInitScript(() => {
    const callbacks = new Map();
    const listeners = new Map();
    let nextId = 1;
    const account = {
      id: 1,
      name: "Büro",
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
      caller: null,
      callerContact: null,
      historyError: null,
      registration: "registered",
      call: "idle",
      callId: null,
      lastError: null,
      accounts: [
        account,
        { ...account, id: 2, name: "Privat", username: "2000" },
      ],
      selectedAccountId: 1,
      callAccountId: null,
      audioMode: "system",
      soundSettings: {
        inputDevice: "",
        outputDevice: "",
        ringDevice: "",
        ringtoneEnabled: true,
      },
      ringtonePlaying: false,
    };
    if (location.search.includes("restored-")) {
      Object.assign(state, {
        call: location.search.includes("restored-held") ? "held" : "active",
        callId: "restored-call",
        callAccountId: 2,
        callConnectedAtMs: Date.now() - 125000,
        caller: { name: "Remote <img src=x>", number: "+4930123456" },
        callerContact: { id: "contact", source: "macos", name: "Ada <Büro>" },
      });
    }
    const fixture = {
      state,
      failSnapshot: location.search.includes("snapshot-error"),
      failSelect: false,
      holdSnapshot: false,
      rejectSnapshot: null,
      resolveSnapshot: null,
      holdSelect: false,
      resolveSelect: null,
      emit(payload) {
        if (!payload) state.revision++;

        for (const [id, listener] of listeners)
          if (listener.event === "phone-state-changed")
            callbacks.get(listener.handler)({
              id,
              event: listener.event,
              payload: structuredClone(payload ?? state),
            });
      },
    };
    window.__phoneFixture = fixture;
    window.isTauri = true;
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener(_event, id) {
        listeners.delete(id);
      },
    };
    window.__TAURI_INTERNALS__ = {
      transformCallback(fn) {
        const id = nextId++;
        callbacks.set(id, fn);
        return id;
      },
      async invoke(command, args) {
        if (command === "plugin:event|listen") {
          const id = nextId++;
          listeners.set(id, { event: args.event, handler: args.handler });
          return id;
        }
        if (command === "plugin:event|unlisten" || command === "open_settings")
          return;
        if (command === "get_app_info")
          return { version: "0.1.0", telephony: "linked" };
        if (command === "get_engine_snapshot") {
          state.revision++;
          if (fixture.holdSnapshot) {
            const captured = structuredClone(state);
            return new Promise((resolve, reject) => {
              fixture.rejectSnapshot = reject;
              fixture.resolveSnapshot = () => resolve(captured);
            });
          }
          if (fixture.failSnapshot)
            throw {
              code: "nativeFailure",
              message: "Synthetic snapshot failure",
            };
          return structuredClone(state);
        }
        if (command === "select_account") {
          if (fixture.failSelect)
            throw {
              code: "nativeFailure",
              message: "Synthetic account failure",
            };
          state.selectedAccountId = args.id;
          state.revision++;
          const captured = structuredClone(state);
          if (fixture.holdSelect)
            return new Promise((resolve) => {
              fixture.resolveSelect = () => resolve(captured);
            });
          return captured;
        }
        if (command === "answer" || command === "hangup") {
          if (!args.callId || args.callId !== state.callId)
            throw { code: "invalidState", message: "Stale call" };
          state.revision++;
          state.call = command === "answer" ? "answering" : "idle";
          if (command === "hangup") {
            state.callId = null;
            state.callAccountId = null;
          }
          return structuredClone(state);
        }
        throw new Error("Unexpected synthetic IPC: " + command);
      },
    };
    localStorage.setItem("sip.introduction.v1", "seen");
  });
  const results = [];
  async function check(name, test) {
    try {
      await test();
      results.push({ name, passed: true });
    } catch (error) {
      results.push({ name, passed: false, error: error.message });
    }
  }
  async function fresh(search = "") {
    await page.goto(origin + "/" + search);
    await page.getByRole("button", { name: "Anrufen", exact: true }).waitFor();
    await page
      .getByText(search ? "Status nicht verfügbar" : "Angemeldet", {
        exact: true,
      })
      .waitFor();
  }
  await page.setViewportSize({ width: 340, height: 500 });
  await check(
    "invalid number exposes its correction to assistive technology",
    async () => {
      await fresh();
      const input = page.getByRole("textbox", {
        name: "Nummer oder Nebenstelle",
      });
      await input.fill("abc");
      const described = await input.evaluate((e) =>
        (e.getAttribute("aria-describedby") ?? "")
          .split(/\s+/)
          .map((id) => document.getElementById(id)?.textContent ?? "")
          .join(" "),
      );
      if (!described.includes("Ziffern") || !described.includes("+"))
        throw new Error("Invalid number has no associated correction hint");
    },
  );
  await check("initial snapshot error offers retry and recovers", async () => {
    await fresh("?snapshot-error");
    const retry = page.getByRole("button", {
      name: "Status erneut laden",
      exact: true,
    });
    if (!(await retry.count()))
      throw new Error(
        "Missing retry after get_app_info succeeded and get_engine_snapshot failed",
      );
    await page.evaluate(() => (window.__phoneFixture.failSnapshot = false));
    await retry.click();
    await page.getByText("Angemeldet", { exact: true }).waitFor();
    if (await page.getByRole("alert").count())
      throw new Error("Recovered load still shows error");
  });
  await check(
    "dismissed initial status error remains recoverable",
    async () => {
      await fresh("?snapshot-error");
      await page
        .getByRole("button", { name: "Fehlermeldung schließen", exact: true })
        .click();
      const retry = page.getByRole("button", {
        name: "Status erneut laden",
        exact: true,
      });
      if (!(await retry.count()))
        throw new Error(
          "Dismissing the status error also removed its only retry action",
        );
      await page.evaluate(() => (window.__phoneFixture.failSnapshot = false));
      await retry.click();
      await page.getByText("Angemeldet", { exact: true }).waitFor();
    },
  );
  await check(
    "failed account switch preserves authoritative outgoing identity",
    async () => {
      await fresh();
      await page.evaluate(() => (window.__phoneFixture.failSelect = true));
      const picker = page.getByRole("combobox", {
        name: "Konto für ausgehende Anrufe",
      });
      await picker.selectOption("2");
      await page.getByRole("alert").waitFor();
      if ((await picker.inputValue()) !== "1")
        throw new Error(
          "Picker shows account 2 although native selection remains account 1",
        );
      await page.evaluate(() => (window.__phoneFixture.failSelect = false));
      await picker.selectOption("2");
      await page.waitForFunction(
        () =>
          document.querySelector("#outgoing-account")?.value === "2" &&
          !document.querySelector("#outgoing-account")?.disabled,
      );
    },
  );
  for (const registration of ["failed", "registering", "unconfigured"])
    await check(
      "onboarding explains " + registration + " saved account",
      async () => {
        await fresh();
        await page.evaluate((registration) => {
          const f = window.__phoneFixture;
          f.state.registration = registration;
          f.state.accounts[0].registration = registration;
          f.emit();
        }, registration);
        await page
          .getByRole("button", { name: "Starthilfe", exact: true })
          .click();
        const action = page.getByRole("button", {
          name:
            registration === "failed"
              ? "Verbindung prüfen"
              : "Kontoeinstellungen öffnen",
          exact: true,
        });
        if (!(await action.count()))
          throw new Error(
            "Existing " + registration + " account is presented as new setup",
          );
      },
    );
  await check(
    "manual introduction navigation survives unchanged native snapshots",
    async () => {
      await fresh();
      await page
        .getByRole("button", { name: "Starthilfe", exact: true })
        .click();
      await page.evaluate(() => window.__phoneFixture.emit());
      if (
        !(await page
          .getByRole("heading", {
            name: "Dein Telefon ist verbunden",
            exact: true,
          })
          .count())
      )
        throw new Error(
          "Unchanged native snapshot closed manually opened introduction",
        );
    },
  );
  await check("zero hold is canceled when the window loses focus", async () => {
    await fresh();
    const zero = page.getByRole("button", {
      name: "0, gedrückt halten für Plus",
      exact: true,
    });
    await zero.dispatchEvent("pointerdown", {
      pointerId: 1,
      isPrimary: true,
      button: 0,
    });
    await page.evaluate(() => window.dispatchEvent(new Event("blur")));
    await page.waitForTimeout(650);
    if (
      (await page
        .getByRole("textbox", { name: "Nummer oder Nebenstelle" })
        .inputValue()) !== ""
    )
      throw new Error("Canceled zero hold inserted a plus after blur");
  });
  await check("secondary pointer cannot start a zero hold", async () => {
    await fresh();
    await page
      .getByRole("button", { name: "0, gedrückt halten für Plus", exact: true })
      .dispatchEvent("pointerdown", {
        pointerId: 2,
        isPrimary: false,
        button: 0,
      });
    await page.waitForTimeout(650);
    if (
      (await page
        .getByRole("textbox", { name: "Nummer oder Nebenstelle" })
        .inputValue()) !== ""
    )
      throw new Error("Secondary pointer inserted a plus");
  });
  await check(
    "pointer cancellation leaves the next keyboard zero responsive",
    async () => {
      await fresh();
      const zero = page.getByRole("button", {
        name: "0, gedrückt halten für Plus",
        exact: true,
      });
      await zero.dispatchEvent("pointerdown", {
        pointerId: 1,
        isPrimary: true,
        button: 0,
      });
      await page.waitForFunction(
        () => document.querySelector("input[type=tel]")?.value === "+",
      );
      await zero.dispatchEvent("pointercancel", { pointerId: 1 });
      await zero.focus();
      await page.keyboard.press("Enter");
      if (
        (await page
          .getByRole("textbox", { name: "Nummer oder Nebenstelle" })
          .inputValue()) !== "+0"
      )
        throw new Error("Canceled hold swallowed next keyboard activation");
    },
  );
  await check(
    "newer native state supersedes an older polling failure",
    async () => {
      await fresh();
      await page.evaluate(() => {
        window.__phoneFixture.holdSnapshot = true;
        document.dispatchEvent(new Event("visibilitychange"));
      });
      await page.waitForFunction(
        () => window.__phoneFixture.rejectSnapshot !== null,
      );
      await page.evaluate(() => {
        const f = window.__phoneFixture;
        f.state.call = "incoming";
        f.state.callId = "new-call";
        f.state.callAccountId = 2;
        f.emit();
        f.rejectSnapshot({ code: "nativeFailure", message: "Stale poll" });
      });
      await page
        .getByRole("button", { name: "Annehmen", exact: true })
        .waitFor();
      await page.evaluate(
        () => new Promise((resolve) => setTimeout(resolve, 0)),
      );
      if (await page.getByRole("alert").count())
        throw new Error(
          "Obsolete polling failure overrides newer native state",
        );
      await page.getByRole("button", { name: "Ablehnen", exact: true }).click();
    },
  );
  await check(
    "incoming answer active and hangup remain reachable in every destination",
    async () => {
      await fresh();
      for (const view of ["Kontakte", "Verlauf", "Starthilfe"]) {
        await page.getByRole("button", { name: view, exact: true }).click();
        await page.evaluate(() => {
          const f = window.__phoneFixture;
          Object.assign(f.state, {
            call: "incoming",
            callId: "synthetic-call",
            callAccountId: 2,
            caller: { name: "Ada Weber", number: "+4930123456" },
          });
          f.emit();
        });
        await page.getByText("Ada Weber", { exact: true }).waitFor();
        await page.getByText("+4930123456", { exact: true }).waitFor();
        await page
          .getByRole("button", { name: "Annehmen", exact: true })
          .click();
        await page.getByText("Wird angenommen …", { exact: true }).waitFor();
        if (
          await page
            .getByRole("button", { name: "Annehmen", exact: true })
            .count()
        )
          throw new Error("Answer offered twice");
        await page.evaluate(() => {
          const f = window.__phoneFixture;
          f.state.call = "active";
          f.emit();
        });
        await page.getByText("Gespräch läuft", { exact: true }).waitFor();
        await page
          .getByRole("button", { name: "Auflegen", exact: true })
          .click();
        await page
          .getByRole("region", { name: "Aktuelles Gespräch" })
          .waitFor({ state: "detached" });
        if (view === "Starthilfe")
          await page
            .getByRole("button", { name: "Starthilfe", exact: true })
            .click();
      }
      await page.getByRole("button", { name: "Tastatur", exact: true }).click();
      await page.setViewportSize({ width: 320, height: 440 });
      if (
        await page.evaluate(
          () => document.documentElement.scrollWidth > innerWidth,
        )
      )
        throw new Error("Horizontal overflow at minimum window");
    },
  );
  for (const restoredState of ["active", "held"])
    await check(
      "restores caller and native duration in " + restoredState,
      async () => {
        await page.goto(origin + "/?restored-" + restoredState);
        await page.getByText("Ada <Büro>", { exact: true }).waitFor();
        const timer = page.getByLabel("Gesprächsdauer");
        await timer.waitFor();
        const duration = await timer.textContent();
        if (!/^02:[0-5][0-9]$/.test(duration.trim()))
          throw new Error("Native connection timestamp was lost: " + duration);
        if (await page.locator(".call-peer img").count())
          throw new Error("Caller markup was interpreted as HTML");
        await page.evaluate(() => {
          const f = window.__phoneFixture;
          f.state.call = "held";
          f.emit();
          f.state.call = "active";
          f.emit();
        });
        await page.getByText("Gespräch läuft", { exact: true }).waitFor();
        if (!(await timer.textContent()).trim().startsWith("02:"))
          throw new Error("Hold/resume reset the duration");
      },
    );
  await check("stale event cannot replace a newer snapshot", async () => {
    await fresh();
    await page.evaluate(() => {
      const f = window.__phoneFixture;
      const stale = structuredClone(f.state);
      f.holdSnapshot = true;
      Object.assign(f.state, {
        call: "incoming",
        callId: "new-call",
        callAccountId: 2,
        caller: { name: "Latest caller", number: "1002" },
      });
      f.emit();
      f.emit(stale);
    });
    await page.getByText("Latest caller", { exact: true }).waitFor();
  });
  await check("older poll success cannot replace a newer event", async () => {
    await fresh();
    await page.evaluate(() => {
      window.__phoneFixture.holdSnapshot = true;
      document.dispatchEvent(new Event("visibilitychange"));
    });
    await page.waitForFunction(
      () => window.__phoneFixture.resolveSnapshot !== null,
    );
    await page.evaluate(() => {
      const f = window.__phoneFixture;
      Object.assign(f.state, {
        call: "incoming",
        callId: "new-call",
        callAccountId: 2,
      });
      f.emit();
      f.resolveSnapshot();
    });
    await page.getByRole("button", { name: "Annehmen", exact: true }).waitFor();
  });
  await check(
    "newer poll success wins over an event delivered during its read",
    async () => {
      await fresh();
      await page.evaluate(() => {
        const f = window.__phoneFixture;
        const older = structuredClone(f.state);
        Object.assign(f.state, {
          call: "incoming",
          callId: "new-call",
          callAccountId: 2,
        });
        f.holdSnapshot = true;
        f.older = older;
        document.dispatchEvent(new Event("visibilitychange"));
      });
      await page.waitForFunction(
        () => window.__phoneFixture.resolveSnapshot !== null,
      );
      await page.evaluate(() => {
        const f = window.__phoneFixture;
        f.emit(f.older);
        f.resolveSnapshot();
      });
      await page
        .getByRole("button", { name: "Annehmen", exact: true })
        .waitFor();
    },
  );
  await check(
    "newer command success wins over a delayed older event",
    async () => {
      await fresh();
      await page.evaluate(() => {
        const f = window.__phoneFixture;
        f.older = structuredClone(f.state);
        f.holdSelect = true;
      });
      await page
        .getByRole("combobox", { name: "Konto für ausgehende Anrufe" })
        .selectOption("2");
      await page.waitForFunction(
        () => window.__phoneFixture.resolveSelect !== null,
      );
      await page.evaluate(() => {
        const f = window.__phoneFixture;
        f.emit(f.older);
        f.resolveSelect();
      });
      await page.waitForFunction(
        () => document.querySelector("#outgoing-account")?.value === "2",
      );
    },
  );
  await check(
    "older command success cannot replace a newer event",
    async () => {
      await fresh();
      await page.evaluate(() => {
        window.__phoneFixture.holdSelect = true;
      });
      await page
        .getByRole("combobox", { name: "Konto für ausgehende Anrufe" })
        .selectOption("2");
      await page.waitForFunction(
        () => window.__phoneFixture.resolveSelect !== null,
      );
      await page.evaluate(() => {
        const f = window.__phoneFixture;
        f.holdSnapshot = true;
        Object.assign(f.state, {
          call: "incoming",
          callId: "newer-command-call",
          callAccountId: 1,
          caller: { name: "Newer than command", number: "1001" },
        });
        f.emit();
        f.resolveSelect();
      });
      await page.getByText("Newer than command", { exact: true }).waitFor();
      await page
        .getByRole("button", { name: "Annehmen", exact: true })
        .waitFor();
    },
  );
  await check(
    "read failure keeps a connected call and its native duration",
    async () => {
      await page.goto(origin + "/?restored-active");
      await page.getByText("Gespräch läuft", { exact: true }).waitFor();
      await page.evaluate(() => {
        window.__phoneFixture.failSnapshot = true;
        document.dispatchEvent(new Event("visibilitychange"));
      });
      await page.getByRole("alert").waitFor();
      if (
        !(await page.getByLabel("Gesprächsdauer").textContent())
          .trim()
          .startsWith("02:")
      )
        throw new Error("Read failure cleared the valid connected call");
    },
  );
  await check(
    "early media and received DTMF survive complete event-buffer delivery",
    async () => {
      await fresh();
      await page.evaluate(() => {
        const f = window.__phoneFixture;
        Object.assign(f.state, {
          call: "earlyMedia",
          callId: "media-call",
          callAccountId: 1,
        });
        f.emit();
      });
      await page.getByText("Frühe Medienverbindung", { exact: true }).waitFor();
      await page.evaluate(() => {
        const f = window.__phoneFixture;
        Object.assign(f.state, {
          call: "held",
          callConnectedAtMs: Date.now() - 65000,
          callEvents: [
            {
              sequence: 1,
              callId: "previous-call",
              event: { kind: "dtmfStart", digit: "9" },
            },
            {
              sequence: 1,
              callId: "media-call",
              event: { kind: "dtmfStart", digit: "#" },
            },
            { sequence: 2, callId: "media-call", event: { kind: "dtmfEnd" } },
          ],
        });
        f.emit();
      });
      await page.getByText("Gespräch gehalten", { exact: true }).waitFor();
      await page
        .getByText("Empfangener Tastenton: #", { exact: true })
        .waitFor();
      await page.evaluate(() => {
        const f = window.__phoneFixture;
        f.state.call = "idle";
        f.state.callId = null;
        f.state.lastError = {
          code: "sipBusy",
          sipStatus: 486,
          message: "<script>secret remote reason</script>",
        };
        f.emit();
      });
      const alert = await page.getByRole("alert").textContent();
      if (!alert.includes("besetzt") || alert.includes("secret"))
        throw new Error("Remote SIP reason reached the error UI: " + alert);
    },
  );
  await page.close();
  return { results, passed: results.every((result) => result.passed) };
}
