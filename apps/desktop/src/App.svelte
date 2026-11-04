<script lang="ts">
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    answer,
    openSettings,
    selectAccount,
    dial,
    errorMessage,
    getAppInfo,
    getEngineSnapshot,
    hangup,
  } from "./lib/api";
  import type {
    AppInfo,
    Contact,
    ContactReference,
    EngineSnapshot,
    RegistrationState,
  } from "./lib/generated/AppInfo";
  import Icon from "./lib/components/Icon.svelte";
  import Dialpad from "./lib/components/Dialpad.svelte";
  import CallPanel from "./lib/components/CallPanel.svelte";
  import GettingStarted from "./lib/components/GettingStarted.svelte";
  import HistoryPanel from "./lib/components/HistoryPanel.svelte";
  import { getContacts } from "./lib/contacts-api";
  import { callbackHistory } from "./lib/history-api";
  import ContactsPanel from "./lib/components/ContactsPanel.svelte";
  import { hasSeenIntroduction, rememberIntroduction } from "./lib/onboarding";

  type View = "dialer" | "contacts" | "history" | "start";
  let view = $state<View>("dialer");
  let contactFocusId = $state<string | null>(null);
  let historyContactId: string | null = null;
  let historyScope = $state<{ name: string; numbers: string[] } | null>(null);
  function showContactHistory(contact: Contact) {
    historyContactId = contact.id;
    historyScope = {
      name: contact.name,
      numbers: contact.phones.filter((p) => p.destination).map((p) => p.number),
    };
    selectView("history");
  }
  function showHistoryContact(contact: ContactReference) {
    contactFocusId = contact.id;
    selectView("contacts");
  }
  async function returnCall(id: string, number: string | null) {
    if (
      await perform(() =>
        callbackHistory(id).catch((reason: unknown) => {
          if (
            reason &&
            typeof reason === "object" &&
            "code" in reason &&
            reason.code === "storageFailure"
          )
            throw new Error(
              "Der Rückruf konnte nicht aus dem Verlauf geladen werden. Prüfe den App-Datenordner und starte Ruf neu.",
            );
          throw reason;
        }),
      )
    ) {
      lastDialed = number ?? "";
      selectView("dialer");
    }
  }
  function clearHistoryScope() {
    historyContactId = null;
    historyScope = null;
  }
  async function refreshHistoryScope() {
    const id = historyContactId;
    if (!id) return;
    try {
      const contacts = await getContacts();
      if (!mounted || historyContactId !== id) return;
      const contact =
        contacts.permission === "authorized"
          ? contacts.contacts.find((contact) => contact.id === id)
          : null;
      if (contact?.phones.some((phone) => phone.destination)) {
        historyScope = {
          name: contact.name,
          numbers: contact.phones
            .filter((p) => p.destination)
            .map((p) => p.number),
        };
      } else clearHistoryScope();
    } catch {
      if (mounted && historyContactId === id) clearHistoryScope();
    }
  }
  let info = $state<AppInfo | null>(null);
  let snapshot = $state<EngineSnapshot | null>(null);
  let browser = $state(false);
  let loading = $state(true);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let lastNativeError: string | null = null;
  let operationEpoch = 0;
  let mounted = false;
  let introductionSeen = true;
  let introductionStorage: Storage | null = null;
  let initialSnapshotReceived = false;
  let navigated = false;
  let firstCallHint = $state(false);
  const sessionAudio = $derived(snapshot?.audioMode ?? null);
  const selectedAccount = $derived(
    snapshot?.accounts.find((a) => a.id === snapshot?.selectedAccountId),
  );
  const callAccount = $derived(
    snapshot?.accounts.find((a) => a.id === snapshot?.callAccountId),
  );
  const accountName = $derived((callAccount ?? selectedAccount)?.name ?? "Ruf");
  const incomingPeer = $derived(
    snapshot?.callerContact?.name ??
      snapshot?.caller?.name ??
      snapshot?.caller?.number ??
      "Unbekannter Anrufer",
  );
  let destination = $state("");
  let lastDialed = $state("");
  const incoming = $derived(
    !!snapshot?.caller ||
      snapshot?.call === "incoming" ||
      snapshot?.call === "answering",
  );
  const connectedAt = $derived(snapshot?.callConnectedAtMs ?? 0);
  const receivedDtmf = $derived(
    snapshot?.callEvents
      .slice()
      .reverse()
      .find(
        (event) =>
          event.callId === snapshot?.callId && event.event.kind === "dtmfStart",
      )?.event,
  );
  let now = $state(Date.now());
  const nativeAvailable = $derived(info?.telephony === "linked");
  const registration = $derived(snapshot?.registration ?? "unconfigured");
  const audioUnknown = $derived(
    registration !== "unconfigured" && sessionAudio === null,
  );

  const callState = $derived(snapshot?.call ?? "idle");
  const callIdle = $derived(callState === "idle");
  const registered = $derived(registration === "registered");
  const canCall = $derived(
    nativeAvailable &&
      !audioUnknown &&
      registered &&
      callIdle &&
      /^[0-9+*#]{1,64}$/.test(destination),
  );
  const elapsed = $derived(
    connectedAt ? Math.max(0, Math.floor((now - connectedAt) / 1000)) : 0,
  );
  const elapsedLabel = $derived(
    `${Math.floor(elapsed / 60)
      .toString()
      .padStart(2, "0")}:${(elapsed % 60).toString().padStart(2, "0")}`,
  );
  const registrationLabels: Record<RegistrationState, string> = {
    unconfigured: "Nicht angemeldet",
    registering: "Wird verbunden …",
    registered: "Angemeldet",
    failed: "Anmeldung fehlgeschlagen",
  };
  const availability = $derived(
    loading
      ? "Telefonie wird geladen …"
      : browser
        ? "Browser-Vorschau"
        : info?.telephony === "notLinked"
          ? "Telefonie nicht eingebunden"
          : !snapshot
            ? "Status nicht verfügbar"
            : registrationLabels[registration],
  );
  const tabs = [
    { id: "dialer", label: "Tastatur", icon: "keypad" },
    { id: "contacts", label: "Kontakte", icon: "contacts" },
    { id: "history", label: "Verlauf", icon: "history" },
  ] as const;

  const introRuntime = $derived(
    browser
      ? "browser"
      : info?.telephony === "notLinked"
        ? "unlinked"
        : nativeAvailable
          ? "desktop"
          : "unavailable",
  );
  const setupLabel = $derived(
    !error && !browser && !snapshot && info?.telephony !== "notLinked"
      ? "Status erneut laden"
      : browser || !nativeAvailable
        ? "Starthilfe öffnen"
        : registration === "failed"
          ? "Verbindung prüfen"
          : registration === "registering"
            ? "Anmeldung läuft · Angaben ansehen"
            : "SIP-Konto verbinden",
  );
  const connectionHint = $derived(
    audioUnknown
      ? "Audioquelle unbekannt · App neu starten"
      : sessionAudio === "testTone"
        ? "Testton statt Mikrofon · Audio wird aufgezeichnet"
        : firstCallHint
          ? "Verbunden. Jetzt eine Nummer wählen."
          : "0 halten für + · Enter zum Anrufen",
  );

  function markIntroduction() {
    if (introductionSeen) return;
    introductionSeen = true;
    if (introductionStorage) rememberIntroduction(introductionStorage);
  }
  function selectView(next: View) {
    navigated = true;
    if (view === "start" && next !== "start") markIntroduction();
    view = next;
  }
  function applySnapshot(next: EngineSnapshot) {
    if (snapshot && next.revision < snapshot.revision) return;
    const becameRegistered =
      next.registration === "registered" &&
      (snapshot?.registration !== "registered" ||
        snapshot?.selectedAccountId !== next.selectedAccountId);
    now = Date.now();
    const nextError = next.lastError
      ? `${next.lastError.code}:${next.lastError.sipStatus}:${next.lastError.message}`
      : null;
    if (
      nextError &&
      nextError !== lastNativeError &&
      next.registration !== "failed"
    )
      error = errorMessage(next.lastError);
    lastNativeError = nextError;
    snapshot = next;
    if (!initialSnapshotReceived) {
      initialSnapshotReceived = true;
      if (
        !introductionSeen &&
        !navigated &&
        next.call === "idle" &&
        next.registration === "unconfigured"
      )
        view = "start";
    }
    if (next.accounts.length) markIntroduction();
    if (next.registration === "registered") {
      if (becameRegistered) firstCallHint = true;
      if (view === "start" && becameRegistered) selectView("dialer");
    } else if (next.registration === "failed") firstCallHint = false;
  }
  async function refreshNativeState() {
    if (browser) return;
    loading = true;
    error = null;
    const epoch = operationEpoch;
    const revision = snapshot?.revision;
    try {
      const result = await getAppInfo();
      if (!mounted || epoch !== operationEpoch) return;
      info = result;
      if (result.telephony === "linked") {
        const state = await getEngineSnapshot();
        if (mounted) applySnapshot(state);
      } else if (!introductionSeen && !navigated) view = "start";
    } catch (reason: unknown) {
      if (
        mounted &&
        epoch === operationEpoch &&
        revision === snapshot?.revision
      ) {
        error = errorMessage(reason);
        if (!introductionSeen && !navigated) view = "start";
      }
    } finally {
      if (mounted) loading = false;
    }
  }
  onMount(() => {
    mounted = true;
    try {
      introductionStorage = window.localStorage;
    } catch {
      /* Restricted webview: session-only introduction. */
    }
    introductionSeen = introductionStorage
      ? hasSeenIntroduction(introductionStorage)
      : false;
    browser = !isTauri();
    let polling = false;
    if (browser) {
      loading = false;
      if (!introductionSeen) view = "start";
    }
    let unlistenState: UnlistenFn | undefined;
    let unlistenContacts: UnlistenFn | undefined;
    if (!browser)
      void listen("contacts-changed", () => void refreshHistoryScope())
        .then((unlisten) => {
          if (!mounted) unlisten();
          else unlistenContacts = unlisten;
        })
        .catch(() => {
          /* Focus restoration revalidates the contact scope. */
        });
    const contactFocus = () => {
      if (!browser) void refreshHistoryScope();
    };
    window.addEventListener("focus", contactFocus);
    if (!browser)
      void listen<EngineSnapshot>("phone-state-changed", ({ payload }) => {
        if (!mounted) return;
        applySnapshot(payload);
      })
        .then((unlisten) => {
          if (!mounted) return unlisten();
          unlistenState = unlisten;
          void refreshNativeState();
        })
        .catch(() => {
          if (mounted) void refreshNativeState();
        });
    const poll = () => {
      if (
        !mounted ||
        document.visibilityState === "hidden" ||
        loading ||
        polling ||
        busy ||
        !nativeAvailable
      )
        return;
      polling = true;
      const epoch = operationEpoch;
      const revision = snapshot?.revision;
      void getEngineSnapshot()
        .then((result) => {
          if (mounted) applySnapshot(result);
        })
        .catch((reason: unknown) => {
          if (
            mounted &&
            epoch === operationEpoch &&
            revision === snapshot?.revision &&
            !busy
          )
            error = errorMessage(reason);
        })
        .finally(() => {
          polling = false;
        });
    };
    const timer = setInterval(poll, 2000);
    const clock = setInterval(() => {
      now = Date.now();
    }, 1000);
    document.addEventListener("visibilitychange", poll);
    return () => {
      mounted = false;
      clearInterval(timer);
      clearInterval(clock);
      unlistenState?.();
      unlistenContacts?.();
      window.removeEventListener("focus", contactFocus);
      document.removeEventListener("visibilitychange", poll);
    };
  });

  async function perform(
    operation: () => Promise<EngineSnapshot>,
  ): Promise<boolean> {
    if (busy) return false;
    operationEpoch++;
    busy = true;
    error = null;
    try {
      const result = await operation();
      if (!mounted) return false;
      applySnapshot(result);
      return true;
    } catch (reason: unknown) {
      if (mounted) error = errorMessage(reason);
      return false;
    } finally {
      if (mounted) busy = false;
    }
  }
  let openingSettings = $state(false);
  async function showSettings() {
    if (openingSettings) return;
    openingSettings = true;
    try {
      if (browser)
        window.open("?window=settings", "sip-settings", "width=900,height=660");
      else await openSettings();
      markIntroduction();
    } catch (reason) {
      error = errorMessage(reason);
    } finally {
      openingSettings = false;
    }
  }
  async function placeCall() {
    if (!canCall || busy) return;
    const target = destination;
    if (await perform(() => dial(target))) {
      lastDialed = target;
      firstCallHint = false;
    }
  }
  async function placeContactCall(target: string) {
    if (!nativeAvailable || audioUnknown || !registered || !callIdle || busy)
      return;
    destination = target;
    await placeCall();
  }
  function onEscape(event: KeyboardEvent) {
    if (browser && (event.metaKey || event.ctrlKey) && event.key === ",") {
      event.preventDefault();
      void showSettings();
    }
    if (event.key === "Escape" && view === "start") {
      event.preventDefault();
      selectView("dialer");
    }
  }
</script>

<svelte:window onkeydown={onEscape} />

<div class="softphone" aria-label="Ruf Softphone">
  <header class="app-header">
    <div class="app-brand">
      <img src="/ruf.svg" alt="" width="32" height="32" />
      <div class="identity">
        {#if (snapshot?.accounts.length ?? 0) > 1}<label
            class="sr-only"
            for="outgoing-account">Konto für ausgehende Anrufe</label
          ><select
            class="account-picker"
            id="outgoing-account"
            value={snapshot?.selectedAccountId ?? ""}
            disabled={busy || !callIdle}
            onchange={(event) => {
              const id = Number(event.currentTarget.value);
              event.currentTarget.value = String(
                snapshot?.selectedAccountId ?? "",
              );
              void perform(() => selectAccount(id));
            }}
            >{#each snapshot?.accounts ?? [] as item}<option
                value={item.id}
                disabled={item.removing}>{item.name}</option
              >{/each}</select
          >{:else}<span class="app-name">{accountName}</span>{/if}<span
          class="availability"
          class:available={registered}
          class:failed={registration === "failed"}
          ><span class="status-dot" aria-hidden="true"
          ></span>{availability}</span
        >
      </div>
    </div>
    <div class="header-actions">
      <button
        class="icon-button"
        type="button"
        aria-label="Starthilfe"
        title="Starthilfe"
        aria-pressed={view === "start"}
        onclick={() => selectView(view === "start" ? "dialer" : "start")}
        ><Icon name="info" size={16} /></button
      >
      <button
        class="icon-button settings-button"
        type="button"
        aria-label="Einstellungen öffnen"
        title="Einstellungen (⌘/Ctrl+,)"
        disabled={openingSettings}
        onclick={() => {
          void showSettings();
        }}><Icon name="settings" size={16} /></button
      >
    </div>
  </header>

  {#if snapshot?.historyError}<p class="error-notice" role="alert">
      Der Anrufverlauf kann gerade nicht gespeichert werden. Prüfe den Zugriff
      auf den App-Datenordner.
    </p>{/if}
  {#if error}<div class="error-notice">
      <div class="error-copy">
        <p role="alert">{error}</p>
        {#if (!info || !snapshot) && !browser}<button
            class="text-button"
            type="button"
            disabled={loading || busy}
            onclick={() => {
              void refreshNativeState();
            }}>Status erneut laden</button
          >{:else if registration === "failed"}<button
            class="text-button"
            type="button"
            onclick={() => {
              void showSettings();
            }}>Verbindung prüfen</button
          >{/if}
      </div>
      <button
        class="icon-button"
        type="button"
        aria-label="Fehlermeldung schließen"
        onclick={() => (error = null)}><Icon name="close" size={15} /></button
      >
    </div>{/if}

  <main class="phone-content" class:in-call={!callIdle}>
    {#if !callIdle}
      <CallPanel
        state={callState}
        peer={incoming ? incomingPeer : lastDialed || "Aktueller Anruf"}
        number={incoming ? (snapshot?.caller?.number ?? null) : null}
        elapsed={elapsedLabel}
        receivedDtmf={receivedDtmf?.kind === "dtmfStart"
          ? receivedDtmf.digit
          : null}
        busy={busy || !snapshot?.callId}
        testAudio={sessionAudio === "testTone"}
        {audioUnknown}
        compact={view !== "dialer"}
        onanswer={() => {
          const callId = snapshot?.callId;
          if (!audioUnknown && callId) void perform(() => answer(callId));
        }}
        onhangup={() => {
          const callId = snapshot?.callId;
          if (callId) void perform(() => hangup(callId));
        }}
      />
    {/if}

    {#if view === "start"}
      <GettingStarted
        runtime={introRuntime}
        {registration}
        hasAccount={(snapshot?.accounts.length ?? 0) > 0}
        {audioUnknown}
        onsetup={() => {
          void showSettings();
        }}
        onexplore={() => selectView("dialer")}
      />
    {:else if view === "dialer"}
      {#if callIdle}<Dialpad
          bind:value={destination}
          {canCall}
          {busy}
          {registered}
          {connectionHint}
          {setupLabel}
          setupEnabled={!loading}
          oncall={() => {
            void placeCall();
          }}
          onsetup={() => {
            if (
              !error &&
              !browser &&
              !snapshot &&
              info?.telephony !== "notLinked"
            )
              void refreshNativeState();
            else if (nativeAvailable) void showSettings();
            else selectView("start");
          }}
        />{/if}
    {:else if view === "contacts"}
      <ContactsPanel
        focusId={contactFocusId}
        onhistory={showContactHistory}
        canDial={nativeAvailable &&
          !audioUnknown &&
          registered &&
          callIdle &&
          !busy}
        callHint={!nativeAvailable
          ? "Telefonieren ist in diesem Build nicht verfügbar."
          : audioUnknown
            ? "Audioquelle unbekannt · App neu starten"
            : !callIdle
              ? "Beende das laufende Gespräch, um einen Kontakt anzurufen."
              : !registered
                ? "Verbinde dein SIP-Konto, um Kontakte anzurufen."
                : "Die Telefonie ist gerade beschäftigt."}
        oncall={(target) => {
          void placeContactCall(target);
        }}
      />
    {:else}
      <HistoryPanel
        canDial={nativeAvailable && !audioUnknown && callIdle && !busy}
        oncallback={(id, number) => void returnCall(id, number)}
        oncontact={showHistoryContact}
        contactScope={historyScope}
        onclearscope={clearHistoryScope}
      />
    {/if}
  </main>

  <nav class="bottom-nav" aria-label="Hauptnavigation">
    {#each tabs as tab}<button
        type="button"
        class:active={view === tab.id}
        aria-current={view === tab.id ? "page" : undefined}
        onclick={() => selectView(tab.id)}
        ><Icon name={tab.icon} size={16} /><span>{tab.label}</span></button
      >{/each}
  </nav>
</div>
