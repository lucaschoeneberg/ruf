<script lang="ts">
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import {
    answer,
    configureAccount,
    dial,
    errorMessage,
    getAppInfo,
    getEngineSnapshot,
    hangup,
  } from "./lib/api";
  import type {
    AccountConfig,
    AppInfo,
    EngineSnapshot,
    RegistrationState,
  } from "./lib/generated/AppInfo";
  import Icon from "./lib/components/Icon.svelte";
  import Dialpad from "./lib/components/Dialpad.svelte";
  import CallPanel from "./lib/components/CallPanel.svelte";
  import AccountSetup from "./lib/components/AccountSetup.svelte";
  import GettingStarted from "./lib/components/GettingStarted.svelte";
  import { hasSeenIntroduction, rememberIntroduction } from "./lib/onboarding";

  type View = "dialer" | "contacts" | "history" | "settings" | "start";
  let view = $state<View>("dialer");
  let previousView: Exclude<View, "settings"> = "dialer";
  let info = $state<AppInfo | null>(null);
  let snapshot = $state<EngineSnapshot | null>(null);
  let browser = $state(false);
  let loading = $state(true);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let lastNativeError: string | null = null;
  let operationEpoch = 0;
  let mounted = false;
  let account = $state<AccountConfig>({
    registrar: "",
    username: "",
    password: "",
    audioMode: "system",
  });
  let introductionSeen = true;
  let introductionStorage: Storage | null = null;
  let initialSnapshotReceived = false;
  let navigated = false;
  let pendingRegistration = false;
  let pendingName = "";
  let audioLocked = $state(false);
  let firstCallHint = $state(false);
  let accountName = $state("");
  let destination = $state("");
  let lastDialed = $state("");
  let incoming = $state(false);
  let connectedAt = $state(0);
  let now = $state(Date.now());
  const nativeAvailable = $derived(info?.telephony === "linked");
  const registration = $derived(snapshot?.registration ?? "unconfigured");
  const callState = $derived(snapshot?.call ?? "idle");
  const callIdle = $derived(callState === "idle");
  const registered = $derived(registration === "registered");
  const canCall = $derived(
    nativeAvailable &&
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
    browser || !nativeAvailable
      ? "Starthilfe öffnen"
      : registration === "failed"
        ? "Verbindung prüfen"
        : registration === "registering"
          ? "Anmeldung läuft · Angaben ansehen"
          : "SIP-Konto verbinden",
  );
  const connectionHint = $derived(
    account.audioMode === "testTone"
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
    account.password = "";
    navigated = true;
    if (view === "start" && next !== "start") markIntroduction();
    if (next === "settings" && view !== "settings") previousView = view;
    view = next;
  }
  function applySnapshot(next: EngineSnapshot) {
    if (next.call === "incoming") incoming = true;
    if (next.call === "calling") incoming = false;
    if (next.call === "active" && snapshot?.call !== "active")
      connectedAt = Date.now();
    if (next.call === "idle") {
      connectedAt = 0;
      incoming = false;
    }
    now = Date.now();
    const nextError = next.lastError?.message ?? null;
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
    if (next.registration !== "unconfigured") audioLocked = true;
    if (next.registration === "registered") {
      markIntroduction();
      if (pendingRegistration) {
        pendingRegistration = false;
        accountName = pendingName;
        firstCallHint = true;
        if (view === "settings" || view === "start") selectView("dialer");
      }
    } else if (next.registration === "failed") {
      pendingRegistration = false;
      firstCallHint = false;
    }
  }
  async function refreshNativeState() {
    if (browser) return;
    loading = true;
    error = null;
    const epoch = operationEpoch;
    try {
      const result = await getAppInfo();
      if (!mounted || epoch !== operationEpoch) return;
      info = result;
      if (result.telephony === "linked") {
        const state = await getEngineSnapshot();
        if (mounted && epoch === operationEpoch && !busy) applySnapshot(state);
      } else if (!introductionSeen && !navigated) view = "start";
    } catch (reason: unknown) {
      if (mounted) {
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
    } else void refreshNativeState();
    const timer = setInterval(() => {
      if (!mounted || loading || polling || busy || !nativeAvailable) return;
      polling = true;
      const epoch = operationEpoch;
      void getEngineSnapshot()
        .then((result) => {
          if (mounted && epoch === operationEpoch && !busy)
            applySnapshot(result);
        })
        .catch((reason: unknown) => {
          if (mounted && epoch === operationEpoch) error = errorMessage(reason);
        })
        .finally(() => {
          polling = false;
        });
    }, 500);
    return () => {
      mounted = false;
      clearInterval(timer);
      account.password = "";
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
  async function register(submitted: AccountConfig) {
    if (!nativeAvailable || !callIdle || busy) return;
    account = { ...submitted, password: "" };
    pendingRegistration = true;
    pendingName = submitted.username;
    markIntroduction();
    const queued = await perform(() => configureAccount(submitted));
    if (queued) audioLocked = true;
    else pendingRegistration = false;
  }
  async function placeCall() {
    if (!canCall || busy) return;
    const target = destination;
    if (await perform(() => dial(target))) {
      lastDialed = target;
      firstCallHint = false;
    }
  }
  function onEscape(event: KeyboardEvent) {
    if (event.key === "Escape" && (view === "settings" || view === "start")) {
      event.preventDefault();
      selectView(view === "start" ? "dialer" : previousView);
    }
  }
</script>

<svelte:window onkeydown={onEscape} />

<div class="softphone" aria-label="SIP Softphone">
  <header class="app-header">
    <div class="identity">
      <span class="app-name">{accountName || "SIP"}</span><span
        class="availability"
        class:available={registered}
        class:failed={registration === "failed"}
        ><span class="status-dot" aria-hidden="true"></span>{availability}</span
      >
    </div>
    <div class="header-actions">
      <button
        class="icon-button"
        type="button"
        aria-label="Starthilfe"
        title="Starthilfe"
        aria-pressed={view === "start"}
        onclick={() => selectView(view === "start" ? "dialer" : "start")}
        ><Icon name="info" size={19} /></button
      >
      <button
        class="icon-button settings-button"
        class:selected={view === "settings"}
        type="button"
        aria-label={view === "settings"
          ? "Einstellungen schließen"
          : "Kontoeinstellungen"}
        title={view === "settings"
          ? "Einstellungen schließen"
          : "Kontoeinstellungen"}
        aria-pressed={view === "settings"}
        onclick={() =>
          selectView(view === "settings" ? previousView : "settings")}
        ><Icon
          name={view === "settings" ? "close" : "settings"}
          size={20}
        /></button
      >
    </div>
  </header>

  {#if error}<div class="error-notice">
      <div class="error-copy">
        <p role="alert">{error}</p>
        {#if !info && !browser}<button
            class="text-button"
            type="button"
            disabled={loading}
            onclick={() => {
              void refreshNativeState();
            }}>Status erneut laden</button
          >{:else if registration === "failed" && view !== "settings"}<button
            class="text-button"
            type="button"
            onclick={() => selectView("settings")}>Verbindung prüfen</button
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
        peer={incoming ? "Unbekannte Nummer" : lastDialed || "Aktueller Anruf"}
        elapsed={elapsedLabel}
        {busy}
        testAudio={account.audioMode === "testTone"}
        compact={view !== "dialer"}
        onanswer={() => {
          void perform(answer);
        }}
        onhangup={() => {
          void perform(hangup);
        }}
      />
    {/if}

    {#if view === "start"}
      <GettingStarted
        runtime={introRuntime}
        {registration}
        onsetup={() => selectView("settings")}
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
          onsetup={() => selectView(nativeAvailable ? "settings" : "start")}
        />{/if}
    {:else if view === "settings"}
      <AccountSetup
        bind:account
        {nativeAvailable}
        {browser}
        {busy}
        {callIdle}
        {registration}
        {audioLocked}
        onconfigure={register}
        onback={() => selectView(previousView)}
      />
    {:else}
      <section class="collection-view" aria-labelledby="collection-heading">
        <div class="view-heading">
          <h1 id="collection-heading">
            {view === "contacts" ? "Kontakte" : "Verlauf"}
          </h1>
        </div>
        <div class="empty-state">
          <Icon name={view === "contacts" ? "contacts" : "history"} size={32} />
          <h2>
            {view === "contacts"
              ? "Kontakte noch nicht verbunden"
              : "Verlauf noch nicht verfügbar"}
          </h2>
          <p>
            {view === "contacts"
              ? "Hier sollen deine Kontakte zum schnellen Anrufen erscheinen. Die Adressbuch-Anbindung fehlt noch; du kannst Nummern bereits direkt wählen."
              : "Hier sollen vergangene und verpasste Anrufe erscheinen. Diese Version speichert noch keinen Verlauf. Wähle direkt über die Tastatur."}
          </p>
          <button
            class="secondary-button"
            type="button"
            onclick={() => selectView("dialer")}>Zur Tastatur</button
          >
        </div>
      </section>
    {/if}
  </main>

  <nav class="bottom-nav" aria-label="Hauptnavigation">
    {#each tabs as tab}<button
        type="button"
        class:active={view === tab.id}
        aria-current={view === tab.id ? "page" : undefined}
        onclick={() => selectView(tab.id)}
        ><Icon name={tab.icon} size={20} /><span>{tab.label}</span></button
      >{/each}
  </nav>
</div>
