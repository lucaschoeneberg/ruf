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
    AppInfo,
    AudioMode,
    EngineSnapshot,
    RegistrationState,
  } from "./lib/generated/AppInfo";
  import Icon from "./lib/components/Icon.svelte";
  import Dialpad from "./lib/components/Dialpad.svelte";
  import CallPanel from "./lib/components/CallPanel.svelte";

  type View = "dialer" | "contacts" | "history" | "settings";
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
  let registrar = $state("127.0.0.1:5070");
  let username = $state("1000");
  let password = $state("");
  let audioMode = $state<AudioMode>("testTone");
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
        : !nativeAvailable
          ? "Telefonie nicht eingebunden"
          : registrationLabels[registration],
  );
  const tabs = [
    { id: "dialer", label: "Tastatur", icon: "keypad" },
    { id: "contacts", label: "Kontakte", icon: "contacts" },
    { id: "history", label: "Verlauf", icon: "history" },
  ] as const;

  function selectView(next: View) {
    password = "";
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
    if (nextError && nextError !== lastNativeError) error = nextError;
    lastNativeError = nextError;
    snapshot = next;
  }
  onMount(() => {
    mounted = true;
    browser = !isTauri();
    let polling = false;
    if (browser) loading = false;
    else
      void getAppInfo()
        .then((result) => {
          if (mounted) info = result;
        })
        .catch((reason: unknown) => {
          if (mounted) error = errorMessage(reason);
        })
        .finally(() => {
          if (mounted) loading = false;
        });
    const timer = setInterval(() => {
      if (!mounted || polling || busy || !nativeAvailable) return;
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
      password = "";
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
  async function register(event: SubmitEvent) {
    event.preventDefault();
    if (!nativeAvailable || !callIdle || busy) return;
    const account = { registrar, username, password, audioMode };
    password = "";
    if (await perform(() => configureAccount(account))) {
      accountName = account.username;
      selectView("dialer");
    }
  }
  async function placeCall() {
    if (!canCall || busy) return;
    const target = destination;
    if (await perform(() => dial(target))) lastDialed = target;
  }
  function onEscape(event: KeyboardEvent) {
    if (event.key === "Escape" && view === "settings") {
      event.preventDefault();
      selectView(previousView);
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
  </header>

  {#if error}<div class="error-notice">
      <p role="alert">{error}</p>
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
        compact={view !== "dialer"}
        onanswer={() => {
          void perform(answer);
        }}
        onhangup={() => {
          void perform(hangup);
        }}
      />
    {/if}

    {#if view === "dialer"}
      {#if callIdle}<Dialpad
          bind:value={destination}
          {canCall}
          {busy}
          {registered}
          oncall={() => {
            void placeCall();
          }}
          onsetup={() => selectView("settings")}
        />{/if}
    {:else if view === "settings"}
      <section class="settings-view" aria-labelledby="account-heading">
        <div class="view-heading">
          <button
            class="icon-button back-button"
            type="button"
            aria-label="Zurück"
            onclick={() => selectView(previousView)}
            ><Icon name="back" size={18} /></button
          >
          <h1 id="account-heading">SIP-Konto</h1>
        </div>
        <p class="view-description">
          Verbinde dein Telefon mit der Testanlage.
        </p>
        {#if !nativeAvailable}<p class="inline-note">
            <Icon name="info" size={16} /><span
              >{browser
                ? "Die Vorschau zeigt das Layout. Anmelden und telefonieren kannst du in der Desktop-App."
                : "Dieser Build enthält keine native Telefonie-Engine."}</span
            >
          </p>{/if}
        <form onsubmit={register}>
          <fieldset disabled={busy || !callIdle}>
            <label for="registrar">Registrar</label><input
              id="registrar"
              bind:value={registrar}
              required
              autocomplete="off"
              spellcheck="false"
              placeholder="sip.example.org"
            />
            <label for="username">Benutzername</label><input
              id="username"
              bind:value={username}
              required
              autocomplete="off"
              spellcheck="false"
            />
            <label for="password">Passwort</label><input
              id="password"
              bind:value={password}
              type="password"
              required
              autocomplete="off"
            />
            <label for="audio">Audio</label><select
              id="audio"
              bind:value={audioMode}
              ><option value="testTone">Testton · lokale Testanlage</option
              ><option value="system">Mikrofon und Lautsprecher</option></select
            >
            <button
              class="primary-button register-button"
              type="submit"
              disabled={!nativeAvailable || busy || !callIdle}
              >{busy
                ? "Wird angemeldet …"
                : registered
                  ? "Konto neu anmelden"
                  : "Anmelden"}</button
            >
          </fieldset>
        </form>
        <div class="settings-notes">
          <p>Dein Passwort bleibt nur im Arbeitsspeicher.</p>
          <p>
            SIP/UDP ist unverschlüsselt. Verwende vorerst ein isoliertes
            Testnetz.
          </p>
          <p>
            Im Testton-Modus wird empfangenes Audio im App-Cache gespeichert.
            Ein Wechsel des Audiomodus erfordert einen App-Neustart.
          </p>
        </div>
      </section>
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
              ? "Deine Kontakte. Bald hier."
              : "Deine Anrufe im Überblick."}
          </h2>
          <p>
            {view === "contacts"
              ? "Die Adressbuch-Anbindung ist noch nicht verfügbar. Wähle eine Nummer direkt über die Tastatur."
              : "Der Anrufverlauf wird noch nicht gespeichert. Hier findest du ihn, sobald die Anbindung verfügbar ist."}
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
