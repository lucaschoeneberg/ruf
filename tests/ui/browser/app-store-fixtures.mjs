// Store artwork uses only synthetic IPC responses. No native process, SIP peer,
// system address book, credentials, microphone or persisted user data is read.
// Keep this initializer self-contained for Playwright context.addInitScript.
export function installAppStoreFixtures() {
  window.isTauri = true;
  localStorage.setItem("sip.introduction.v1", "seen");
  const callbacks = new Map();
  const listeners = new Map();
  let nextId = 1;
  const reference = (contact) => ({
    id: contact.id,
    source: contact.source,
    name: contact.name,
  });
  const contacts = [
    ["Anna Beispiel", "Designstudio", "1201", true],
    ["Ben Beispiel", "Projektteam", "1202", true],
    ["Clara Muster", "Designstudio", "1203", false],
    ["David Muster", "Projektteam", "1204", false],
    ["Empfang", "Beispielbüro", "1000", false],
    ["Werkstatt", "Beispielbüro", "1300", false],
  ].map(([name, organization, number, favorite], index) => ({
    id: `synthetic-store-contact-${index}`,
    source: "macos",
    name,
    organization,
    favorite,
    phones: [{ label: "Büro", number, destination: number }],
  }));
  contacts[0].phones.push({
    label: "Projektbüro",
    number: "2201",
    destination: "2201",
  });
  const account = {
    id: 1,
    name: "Büro",
    registrar: "sip.example.test",
    username: "1100",
    enabled: true,
    enabledChanging: false,
    registeredAtMs: Date.now() - 3600000,
    registration: "registered",
    lastError: null,
    removing: false,
    transport: "udp",
    mediaSecurity: "disabled",
    authUsername: null,
    outboundProxy: null,
  };
  const state = {
    revision: 1,
    registration: "registered",
    call: "idle",
    callId: null,
    callConnectedAtMs: null,
    callEvents: [],
    callEventsDropped: 0,
    caller: null,
    callerContact: null,
    historyError: null,
    lastError: null,
    lastCallFailure: null,
    calls: [],
    accounts: [
      account,
      { ...account, id: 2, name: "Projektbüro", username: "2100" },
    ],
    selectedAccountId: 1,
    callAccountId: null,
    audioMode: "system",
    soundSettings: {
      inputDevice: "headset-input",
      outputDevice: "headset-output",
      ringDevice: "",
      ringtoneEnabled: true,
      ringtone: "classic",
      ringtoneVolume: 70,
    },
    ringtonePlaying: false,
    audioTest: null,
    audioTestError: null,
  };
  const entries = contacts.map((contact, index) => {
    const started = Date.now() - (index * 42 + 12) * 60000;
    const missed = index === 1;
    const duration = (index + 2) * 47000;
    return {
      id: `synthetic-store-history-${index}`,
      accountName: "Büro",
      accountRegistrar: account.registrar,
      accountUsername: account.username,
      direction: index % 2 ? "incoming" : "outgoing",
      number: contact.phones[0].number,
      remoteName: contact.name,
      startedAtMs: started,
      answeredAtMs: missed ? null : started + 4000,
      endedAtMs: started + (missed ? 24000 : duration + 4000),
      durationMs: missed ? null : duration,
      outcome: missed ? "missed" : "completed",
      sipStatus: missed ? 487 : 200,
      contact: reference(contact),
      callbackAccountId: 1,
    };
  });
  const fixture = {
    state,
    contacts,
    commands: [],
    emit() {
      state.revision++;
      for (const [id, listener] of listeners)
        if (listener.event === "phone-state-changed")
          callbacks.get(listener.handler)?.({
            id,
            event: listener.event,
            payload: structuredClone(state),
          });
    },
    setCall(kind) {
      const incoming = kind === "incoming";
      const contact = reference(contacts[0]);
      const caller = { name: contact.name, number: "1201" };
      const connected = incoming ? null : Date.now() - 143000;
      Object.assign(state, {
        call: kind,
        callId: "synthetic-store-call",
        callConnectedAtMs: connected,
        caller,
        callerContact: contact,
        callAccountId: 1,
        calls: [
          {
            id: "synthetic-store-call",
            accountId: 1,
            contact,
            state: kind,
            destination: null,
            caller,
            connectedAtMs: connected,
            muted: false,
            localHeld: false,
            remoteHeld: false,
            holdPending: false,
            transfer: "idle",
            consultation: false,
            events: [],
            eventsDropped: 0,
            lastError: null,
            security: {
              signalingTransport: "udp",
              tlsPeerVerified: false,
              media: "unprotected",
            },
          },
        ],
      });
      this.emit();
    },
  };
  window.__appStoreFixture = fixture;
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener(_event, id) {
      listeners.delete(id);
    },
  };
  window.__TAURI_INTERNALS__ = {
    metadata: {
      currentWindow: {
        label: location.search.includes("window=settings")
          ? "settings"
          : "main",
      },
      currentWebview: {
        label: location.search.includes("window=settings")
          ? "settings"
          : "main",
      },
    },
    transformCallback(callback) {
      const id = nextId++;
      callbacks.set(id, callback);
      return id;
    },
    unregisterCallback(id) {
      callbacks.delete(id);
    },
    async invoke(command, args) {
      fixture.commands.push({ command, args });
      if (command === "plugin:event|listen") {
        const id = nextId++;
        listeners.set(id, { event: args.event, handler: args.handler });
        return id;
      }
      if (
        ["plugin:event|unlisten", "settings_ready", "open_settings"].includes(
          command,
        )
      )
        return;
      if (command === "get_app_info")
        return { version: "0.1.0", telephony: "linked" };
      if (command === "get_engine_snapshot") return structuredClone(state);
      if (command === "get_preferences")
        return {
          startHidden: true,
          showCallerInNotifications: true,
          contacts: {
            favoritesFirst: true,
            showOrganization: true,
            matchGermanNumbers: true,
          },
          history: { recordingEnabled: true, retentionDays: null },
        };
      if (command === "get_desktop_settings")
        return {
          autostartEnabled: false,
          autostartAvailable: true,
          notificationsEnabled: true,
          notificationsGranted: true,
          notificationPermission: "authorized",
        };
      if (
        [
          "get_notification_permission",
          "get_contacts_permission",
          "get_microphone_permission",
        ].includes(command)
      )
        return "authorized";
      if (["get_contacts", "refresh_contacts"].includes(command))
        return {
          permission: "authorized",
          contacts: structuredClone(contacts),
        };
      if (command === "get_audio_devices")
        return {
          inputs: [
            { id: "headset-input", name: "USB-Headset", selectable: true },
          ],
          outputs: [
            { id: "headset-output", name: "USB-Headset", selectable: true },
            { id: "speakers", name: "Interne Lautsprecher", selectable: true },
          ],
        };
      if (command === "get_history") {
        const query = args.query;
        const matching = entries.filter(
          (entry) =>
            (query.filter === "all" ||
              (query.filter === "missed"
                ? entry.outcome === "missed"
                : entry.direction === query.filter)) &&
            (!query.contactNumbers.length ||
              query.contactNumbers.includes(entry.number)) &&
            `${entry.remoteName} ${entry.number} ${entry.accountName}`
              .toLowerCase()
              .includes(query.search.toLowerCase()),
        );
        return {
          entries: structuredClone(
            matching.slice(query.offset, query.offset + query.limit),
          ),
          total: matching.length,
          hasMore: false,
          recordingError: null,
        };
      }
      throw new Error(`Unexpected screenshot IPC: ${command}`);
    },
  };
}
