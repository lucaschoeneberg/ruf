<script lang="ts">
  import { onMount, tick } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    errorMessage,
    getAppInfo,
    getEngineSnapshot,
    saveAccount,
    removeAccount,
    reconnectAccount,
    getAudioDevices,
    saveSoundSettings,
    previewRingtone,
    stopRingtonePreview,
    getDesktopSettings,
    requestNotificationPermission,
    saveDesktopSettings,
    quitApp,
    settingsReady,
    type SettingsSection,
  } from "./lib/api";
  import type {
    AccountConfig,
    AccountSnapshot,
    AppInfo,
    EngineSnapshot,
    SoundSettings,
    AudioDevices,
    DesktopSettings,
  } from "./lib/generated/AppInfo";
  import AccountSetup from "./lib/components/AccountSetup.svelte";
  import SoundPanel from "./lib/components/SoundPanel.svelte";
  import ContactsSettings from "./lib/components/ContactsSettings.svelte";
  import Icon from "./lib/components/Icon.svelte";

  type Section = SettingsSection;
  let section = $state<Section>("accounts");
  let query = $state("");
  let snapshot = $state<EngineSnapshot | null>(null);
  let info = $state<AppInfo | null>(null);
  let loading = $state(true);
  let busy = $state(false);
  let browser = $state(false);
  let error = $state<string | null>(null);
  let feedback = $state("");
  let permissionFeedback = $state("");
  let editedId = $state<number | null>(null);
  let name = $state("");
  let account = $state<AccountConfig>({
    registrar: "",
    username: "",
    password: "",
    audioMode: "system",
  });
  const defaultSound = (): SoundSettings => ({
    inputDevice: "",
    outputDevice: "",
    ringDevice: "",
    ringtoneEnabled: true,
  });
  let sound = $state<SoundSettings>(defaultSound());
  let soundBaseline = $state(JSON.stringify(defaultSound()));
  const soundDirty = $derived(JSON.stringify(sound) !== soundBaseline);
  let desktopSettings = $state<DesktopSettings | null>(null);
  let autostartEnabled = $state(false);
  let notificationsEnabled = $state(false);
  let desktopBaseline = $state("");
  const desktopDirty = $derived(
    desktopBaseline !== "" &&
      JSON.stringify({ autostartEnabled, notificationsEnabled }) !==
        desktopBaseline,
  );
  let devices = $state<AudioDevices | null>(null);
  let devicesLoading = $state(false);
  let devicesError = $state<string | null>(null);
  let baseline = $state("");
  type PendingAction =
    | { kind: "navigate"; section: Section; id: number | null }
    | { kind: "close" | "quit" };
  let pendingNavigation = $state<PendingAction | null>(null);
  let ownsPreview = false;
  let closing = false;
  let mounted = false;
  let registerWindowGuard: (() => Promise<void>) | null = null;
  let epoch = 0;
  let firstLoad = true;
  let discardNotice = $state<HTMLDivElement>();
  let keepEditing = $state<HTMLButtonElement>();
  let editedControl: HTMLInputElement | HTMLSelectElement | null = null;
  let errorNotice = $state<HTMLDivElement>();
  $effect(() => {
    if (error)
      void tick().then(() => errorNotice?.scrollIntoView({ block: "nearest" }));
  });
  const existing = $derived(
    snapshot?.accounts.find((a) => a.id === editedId) ?? null,
  );
  const accounts = $derived(snapshot?.accounts ?? []);
  const removedDraft = $derived(
    editedId !== null && snapshot !== null && !existing,
  );
  const callIdle = $derived(!snapshot || snapshot.call === "idle");
  const disabled = $derived(
    loading || !info || info.telephony !== "linked" || !snapshot || !callIdle,
  );
  const accountDirty = $derived(
    baseline !== "" &&
      baseline !==
        JSON.stringify({
          name,
          registrar: account.registrar,
          username: account.username,
          password: account.password,
        }),
  );
  const dirty = $derived(
    section === "audio"
      ? soundDirty
      : section === "general"
        ? desktopDirty
        : section === "accounts" && accountDirty,
  );
  const needle = $derived(query.trim().toLocaleLowerCase("de"));
  const sections = [
    {
      id: "general",
      label: "Allgemein",
      icon: "settings",
      terms:
        "autostart anmeldung starten mitteilungen benachrichtigungen hintergrund schließen",
    },
    {
      id: "accounts",
      label: "SIP-Konten",
      icon: "contacts",
      terms:
        "konto konten server benutzername passwort telefonanlage verbindung",
    },
    {
      id: "audio",
      label: "Ton",
      icon: "audio",
      terms:
        "audio ton mikrofon lautsprecher klingelton hörprobe testton aufzeichnung",
    },
    {
      id: "contacts",
      label: "Kontakte",
      icon: "contacts",
      terms:
        "adressbuch kontakte freigabe datenschutz berechtigung zugriff macos",
    },
    {
      id: "about",
      label: "Über Ruf",
      icon: "info",
      terms: "version speicherung schlüsselbund protokoll asterisk testanlage",
    },
  ] as const;
  const visibleSections = $derived(
    sections.filter(
      (s) =>
        !needle ||
        `${s.label} ${s.terms}`.toLocaleLowerCase("de").includes(needle),
    ),
  );
  const visibleAccounts = $derived(
    accounts.filter(
      (a) =>
        !needle ||
        `${a.name} ${a.username} ${a.registrar}`
          .toLocaleLowerCase("de")
          .includes(needle) ||
        visibleSections.some((s) => s.id === "accounts"),
    ),
  );

  function edit(id: number | null) {
    const saved = accounts.find((a) => a.id === id);
    editedId = saved?.id ?? null;
    name = saved?.name ?? "";
    account = {
      registrar: saved?.registrar ?? "",
      username: saved?.username ?? "",
      password: "",
      audioMode: snapshot?.audioMode ?? account.audioMode,
    };
    baseline = JSON.stringify({
      name,
      registrar: account.registrar,
      username: account.username,
      password: account.password,
    });
    feedback = "";
  }
  function navigate(next: Section, id: number | null = editedId) {
    if (busy || (next === section && id === editedId)) return;
    if (dirty) {
      showDiscard({ kind: "navigate", section: next, id });
      return;
    }
    applyNavigation(next, id);
  }
  function showDiscard(action: PendingAction) {
    pendingNavigation = action;
    void tick().then(() => {
      discardNotice?.scrollIntoView({ block: "nearest" });
      keepEditing?.focus({ preventScroll: true });
    });
  }
  function continueEditing() {
    pendingNavigation = null;
    void tick().then(() => {
      if (editedControl?.isConnected && !editedControl.disabled)
        editedControl.focus();
      else document.getElementById("settings-search")?.focus();
    });
  }
  async function stopOwnedPreview() {
    if (!ownsPreview) return;
    ownsPreview = false;
    if (!callIdle) return;
    const result = await stopRingtonePreview();
    if (mounted) applySnapshot(result);
  }
  async function finishWindowAction(kind: "close" | "quit") {
    if (busy) return;
    busy = true;
    try {
      await stopOwnedPreview();
      if (kind === "quit") await quitApp();
      else {
        closing = true;
        await getCurrentWindow().destroy();
      }
    } catch (reason) {
      closing = false;
      if (mounted) error = errorMessage(reason);
    } finally {
      if (mounted) busy = false;
    }
  }
  function requestWindowAction(kind: "close" | "quit") {
    if (busy) {
      error =
        "Eine Änderung wird noch gespeichert. Warte kurz und versuche es erneut.";
    } else if (dirty) showDiscard({ kind });
    else void finishWindowAction(kind);
  }
  function discardChanges() {
    if (busy) return;
    const action = pendingNavigation;
    pendingNavigation = null;
    if (!action) return;
    if (action.kind === "navigate") applyNavigation(action.section, action.id);
    else void finishWindowAction(action.kind);
  }
  function applyNavigation(next: Section, id: number | null) {
    if (section === "audio")
      void stopOwnedPreview().catch((reason) => {
        if (mounted) error = errorMessage(reason);
      });
    if (section === "audio" && soundDirty)
      resetSound(snapshot?.soundSettings ?? defaultSound());
    if (section === "general" && desktopSettings) resetDesktop(desktopSettings);
    account.password = "";
    section = next;
    edit(id);
    pendingNavigation = null;
  }
  function applySnapshot(result: EngineSnapshot) {
    if (snapshot && result.revision < snapshot.revision) return;
    if (!soundDirty) resetSound(result.soundSettings);
    snapshot = result;
    if (firstLoad) {
      firstLoad = false;
      edit(result.selectedAccountId ?? result.accounts[0]?.id ?? null);
    } else if (!accountDirty && editedId !== null) {
      const updated = result.accounts.find((a) => a.id === editedId);
      if (!updated) edit(result.accounts[0]?.id ?? null);
      else if (
        name !== updated.name ||
        account.registrar !== updated.registrar ||
        account.username !== updated.username
      )
        edit(updated.id);
    }
    if (result.audioMode) account.audioMode = result.audioMode;
  }
  async function refresh() {
    if (browser) return;
    loading = true;
    error = null;
    const current = epoch;
    const revision = snapshot?.revision;
    try {
      await registerWindowGuard?.();
      if (!mounted || current !== epoch) return;
      const result = await getAppInfo();
      if (!mounted || current !== epoch) return;
      info = result;
      const [state, desktop] = await Promise.allSettled([
        result.telephony === "linked" ? getEngineSnapshot() : null,
        getDesktopSettings(),
      ]);
      if (!mounted || current !== epoch) return;
      if (state.status === "fulfilled") {
        if (state.value) applySnapshot(state.value);
        else if (!state.value && !accountDirty) edit(null);
      } else if (revision === snapshot?.revision)
        error = errorMessage(state.reason);
      if (desktop.status === "fulfilled") {
        if (!desktopDirty) resetDesktop(desktop.value);
        else desktopSettings = desktop.value;
      } else error = errorMessage(desktop.reason);
      if (section === "audio") void loadDevices();
    } catch (reason) {
      if (mounted && current === epoch) error = errorMessage(reason);
    } finally {
      if (mounted && current === epoch) loading = false;
    }
  }
  onMount(() => {
    mounted = true;
    document.title = "Ruf · Einstellungen";
    browser = !isTauri();
    const requested = new URLSearchParams(window.location.search).get(
      "section",
    );
    const initialSection = sections.find((item) => item.id === requested)?.id;
    if (initialSection) section = initialSection;
    const unlisteners: UnlistenFn[] = [];
    function retain(unlisten: UnlistenFn) {
      if (mounted) unlisteners.push(unlisten);
      else unlisten();
    }
    let closeGuardInstalled = false;
    let quitGuardInstalled = false;
    registerWindowGuard = async () => {
      await Promise.all([
        closeGuardInstalled
          ? Promise.resolve()
          : getCurrentWindow()
              .onCloseRequested((event) => {
                if (closing) return;
                event.preventDefault();
                requestWindowAction("close");
              })
              .then((unlisten) => {
                closeGuardInstalled = true;
                retain(unlisten);
              }),
        quitGuardInstalled
          ? Promise.resolve()
          : listen("settings-quit-requested", () => {
              if (mounted) requestWindowAction("quit");
            }).then((unlisten) => {
              quitGuardInstalled = true;
              retain(unlisten);
            }),
      ]);
      if (mounted) await settingsReady();
    };
    if (browser) {
      loading = false;
      edit(null);
    } else
      void Promise.allSettled([
        listen<EngineSnapshot>("phone-state-changed", ({ payload }) => {
          if (!mounted) return;
          applySnapshot(payload);
        }).then(retain),
        listen<string>("settings-section", ({ payload }) => {
          if (!mounted) return;
          const target = sections.find((item) => item.id === payload)?.id;
          if (target) navigate(target);
        }).then(retain),
      ]).then(() => {
        if (mounted) void refresh();
      });
    let polling = false;
    const poll = () => {
      if (
        !mounted ||
        document.visibilityState === "hidden" ||
        loading ||
        busy ||
        polling ||
        info?.telephony !== "linked"
      )
        return;
      polling = true;
      const current = epoch;
      const revision = snapshot?.revision;
      void getEngineSnapshot()
        .then((result) => {
          if (mounted) applySnapshot(result);
        })
        .catch((reason) => {
          if (
            mounted &&
            current === epoch &&
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
    const refreshAuthorization = () => void refreshDesktopState();
    window.addEventListener("focus", refreshAuthorization);
    document.addEventListener("visibilitychange", poll);
    return () => {
      mounted = false;
      registerWindowGuard = null;
      clearInterval(timer);
      for (const unlisten of unlisteners) unlisten();
      document.removeEventListener("visibilitychange", poll);
      window.removeEventListener("focus", refreshAuthorization);
      account.password = "";
      void stopOwnedPreview().catch(() => {});
    };
  });
  function resetDesktop(value: DesktopSettings) {
    desktopSettings = value;
    autostartEnabled = value.autostartEnabled;
    notificationsEnabled = value.notificationsEnabled;
    desktopBaseline = JSON.stringify({
      autostartEnabled,
      notificationsEnabled,
    });
  }
  async function saveDesktop() {
    if (busy || loading || browser || !desktopSettings || !desktopDirty) return;
    const current = ++epoch;
    busy = true;
    error = null;
    feedback = "";
    try {
      const result = await saveDesktopSettings(
        autostartEnabled,
        notificationsEnabled,
      );
      if (!mounted || current !== epoch) return;
      resetDesktop(result);
      feedback = "Allgemeine Einstellungen gespeichert.";
    } catch (reason) {
      if (mounted && current === epoch) error = errorMessage(reason);
    } finally {
      if (mounted && current === epoch) busy = false;
    }
  }
  async function refreshDesktopState() {
    if (browser || !mounted || loading || busy || !desktopSettings) return;
    const current = ++epoch;
    try {
      const result = await getDesktopSettings();
      if (!mounted || current !== epoch) return;
      if (!desktopDirty) resetDesktop(result);
      else desktopSettings = result;
      permissionFeedback = "";
    } catch (reason) {
      if (mounted && current === epoch) error = errorMessage(reason);
    }
  }
  async function allowNotifications() {
    if (
      busy ||
      loading ||
      browser ||
      !notificationsEnabled ||
      desktopSettings?.notificationPermission !== "notDetermined"
    )
      return;
    const current = ++epoch;
    busy = true;
    error = null;
    permissionFeedback = "";
    try {
      const result = await requestNotificationPermission();
      if (!mounted || current !== epoch) return;
      // Permission is OS state; preserve unsaved preference edits and their baseline.
      desktopSettings = result;
      const messages = {
        authorized:
          "Die Freigabe wurde aktualisiert. Speichere gegebenenfalls deine Änderungen.",
        denied:
          "Die Systemfreigabe ist weiterhin deaktiviert. Der Klingelton funktioniert unabhängig davon.",
        notDetermined:
          "Die Anfrage wurde noch nicht bestätigt. Du kannst sie erneut starten.",
        unsupported:
          "Die Freigabe konnte in diesem Startmodus nicht angefragt werden.",
      };
      permissionFeedback = messages[result.notificationPermission];
    } catch (reason) {
      if (mounted && current === epoch) error = errorMessage(reason);
    } finally {
      if (mounted && current === epoch) busy = false;
    }
  }
  function resetSound(value: SoundSettings) {
    sound = { ...value };
    soundBaseline = JSON.stringify(value);
  }
  async function loadDevices() {
    if (devicesLoading || info?.telephony !== "linked") return;
    devicesLoading = true;
    devicesError = null;
    try {
      const result = await getAudioDevices();
      if (mounted) devices = result;
    } catch {
      if (mounted)
        devicesError = "Die Audiogeräte konnten nicht geladen werden.";
    } finally {
      if (mounted) devicesLoading = false;
    }
  }
  async function soundOperation(
    operation: () => Promise<EngineSnapshot>,
    saved: boolean,
  ) {
    if (busy || disabled) return;
    epoch++;
    busy = true;
    error = null;
    feedback = "";
    try {
      const result = await operation();
      if (!mounted) return;
      applySnapshot(result);
      if (saved) {
        resetSound(snapshot?.soundSettings ?? result.soundSettings);
        feedback =
          "Ton-Einstellungen gespeichert. Sie gelten ab dem nächsten Gespräch.";
      }
    } catch (reason) {
      if (mounted)
        error = `${errorMessage(reason)} Prüfe bei Tonproblemen das ausgewählte Gerät.`;
    } finally {
      if (mounted) busy = false;
    }
  }
  async function save(config: AccountConfig, label: string) {
    if (busy || disabled || removedDraft || existing?.removing) return;
    busy = true;
    epoch++;
    error = null;
    feedback = "";
    const before = new Set(accounts.map((a) => a.id));
    try {
      const result = await saveAccount(editedId, label, config);
      if (!mounted) return;
      applySnapshot(result);
      edit(
        editedId ?? result.accounts.find((a) => !before.has(a.id))?.id ?? null,
      );
      feedback = "Gespeichert. Die Anmeldung an der Telefonanlage läuft.";
      if (result.lastError) error = errorMessage(result.lastError);
    } catch (reason) {
      if (mounted) error = errorMessage(reason);
    } finally {
      if (mounted) busy = false;
    }
  }
  async function reconnect() {
    const target = existing;
    if (
      busy ||
      disabled ||
      removedDraft ||
      !target ||
      target.removing ||
      target.registration !== "failed" ||
      (target.lastError?.code !== "credentialAccessRequired" &&
        target.lastError?.code !== "credentialFailure")
    )
      return;
    const current = ++epoch;
    busy = true;
    error = null;
    feedback = "";
    try {
      const result = await reconnectAccount(target.id);
      if (!mounted || current !== epoch) return;
      applySnapshot(result);
      const updated = snapshot?.accounts.find((item) => item.id === target.id);
      if (updated?.lastError) error = errorMessage(updated.lastError);
      else feedback = "Die Anmeldung mit dem gespeicherten Passwort läuft.";
    } catch (reason) {
      if (mounted && current === epoch) error = errorMessage(reason);
    } finally {
      if (mounted && current === epoch) busy = false;
    }
  }
  async function remove() {
    if (busy || disabled || editedId === null) return;
    busy = true;
    epoch++;
    error = null;
    try {
      const result = await removeAccount(editedId);
      if (mounted) {
        applySnapshot(result);
        edit(accounts.find((a) => !a.removing)?.id ?? null);
        feedback = result.accounts.some((item) => item.removing)
          ? "Das Konto wird entfernt …"
          : "Konto entfernt.";
        if (result.lastError) error = errorMessage(result.lastError);
      }
    } catch (reason) {
      if (mounted) error = errorMessage(reason);
    } finally {
      if (mounted) busy = false;
    }
  }
  function keyboard(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "f") {
      event.preventDefault();
      document.getElementById("settings-search")?.focus();
    }
    if (event.key === "Escape") {
      if (pendingNavigation) continueEditing();
      else if (query) query = "";
    }
  }
</script>

<svelte:window onkeydown={keyboard} />
<div class="settings-shell">
  <aside class="settings-sidebar" aria-label="Einstellungsnavigation">
    <div class="settings-search">
      <Icon name="search" size={15} /><input
        id="settings-search"
        aria-label="Konten und Bereiche suchen"
        bind:value={query}
        placeholder="Einstellungen suchen …"
        autocomplete="off"
        spellcheck="false"
      />{#if query}<button
          class="icon-button"
          type="button"
          aria-label="Suche leeren"
          onclick={() => (query = "")}><Icon name="close" size={13} /></button
        >{/if}
    </div>
    <nav class="settings-navigation" aria-label="Bereiche">
      {#each visibleSections as item}<button
          type="button"
          class:current={section === item.id}
          aria-current={section === item.id ? "page" : undefined}
          disabled={busy}
          onclick={() => navigate(item.id)}
          ><Icon name={item.icon} size={16} /><span>{item.label}</span
          >{#if item.id === "accounts"}<span class="nav-count"
              >{accounts.length}</span
            >{/if}</button
        >{/each}
    </nav>
    {#if visibleAccounts.length || visibleSections.some((s) => s.id === "accounts")}<div
        class="accounts-heading"
      >
        <h2>Deine Konten</h2>
        <button
          class="icon-button"
          type="button"
          aria-label="SIP-Konto hinzufügen"
          title="Konto hinzufügen"
          disabled={busy || loading || accounts.length >= 8}
          onclick={() => navigate("accounts", null)}
          ><Icon name="plus" size={16} /></button
        >
      </div>
      <nav class="accounts-list" aria-label="SIP-Konten">
        {#each visibleAccounts as item}<button
            type="button"
            class:current={section === "accounts" && editedId === item.id}
            disabled={busy || item.removing}
            aria-current={section === "accounts" && editedId === item.id
              ? "page"
              : undefined}
            onclick={() => navigate("accounts", item.id)}
            ><span class="account-list-name"
              ><span
                class="status-dot"
                class:connected={item.registration === "registered"}
                class:failed={item.registration === "failed"}
                aria-hidden="true"
              ></span><span>{item.name}</span></span
            ><small>{item.username}@{item.registrar}</small><span
              class="sr-only"
              >{item.registration === "registered"
                ? "Verbunden"
                : item.registration === "failed"
                  ? "Anmeldung fehlgeschlagen"
                  : "Nicht verbunden"}</span
            ></button
          >{/each}
        {#if !accounts.length}<p class="accounts-empty">
            Noch keine Konten.<br />Füge dein erstes SIP-Konto hinzu.
          </p>{/if}
      </nav>
      {#if accounts.length >= 8}<p class="accounts-empty">
          Maximal acht Konten.
        </p>{/if}
    {:else if !visibleSections.length}<p class="accounts-empty" role="status">
        Keine Treffer für „{query}“. Ändere die Suche oder leere das Suchfeld.
      </p>{/if}
    <div class="settings-sidebar-footer">
      <span>Ruf</span><span>{info ? `v${info.version}` : "Einstellungen"}</span>
    </div>
  </aside>
  <main class="settings-main">
    <div class="settings-breadcrumb">
      Einstellungen<span aria-hidden="true">/</span><span
        >{sections.find((s) => s.id === section)?.label}</span
      >
    </div>
    <div
      class="settings-scroll"
      onfocusin={(event) => {
        const target = event.target;
        if (
          target instanceof HTMLInputElement ||
          target instanceof HTMLSelectElement
        )
          editedControl = target;
      }}
    >
      {#if browser || info?.telephony === "notLinked"}<p
          class="settings-feedback"
        >
          {browser
            ? "Browser-Vorschau · Konten lassen sich in der nativen Desktop-App verbinden."
            : "Dieser Build enthält keine native Telefonie. Nutze den nativen Desktop-Build zum Einrichten."}
        </p>{/if}
      {#if loading}<p class="settings-feedback" role="status">
          Einstellungen werden geladen …
        </p>{/if}
      {#if error}<div
          bind:this={errorNotice}
          class="settings-feedback error"
          role="alert"
        >
          <p>{error}</p>
          <button
            class="text-button"
            type="button"
            disabled={busy || loading}
            onclick={() => {
              void refresh();
            }}>Status erneut laden</button
          >
        </div>{/if}
      {#if !callIdle && (section === "accounts" || section === "audio")}<p
          class="settings-feedback"
          role="status"
        >
          Ein Gespräch läuft. Beende es im Telefonfenster, bevor du
          Einstellungen änderst.
        </p>{/if}
      {#if pendingNavigation}<div
          bind:this={discardNotice}
          class="settings-feedback discard-note"
          role="group"
          aria-label="Ungespeicherte Änderungen"
        >
          <p>Deine Änderungen sind noch nicht gespeichert.</p>
          <div>
            <button
              class="secondary-button"
              type="button"
              bind:this={keepEditing}
              onclick={continueEditing}>Weiter bearbeiten</button
            ><button
              class="secondary-button"
              type="button"
              disabled={busy}
              onclick={discardChanges}>Änderungen verwerfen</button
            >
          </div>
        </div>{/if}
      {#if feedback && !dirty}<p class="settings-feedback" role="status">
          {feedback}
        </p>{/if}
      {#if section === "general"}
        <div class="settings-title">
          <div>
            <h1>Allgemein</h1>
            <p>Start und Hinweise für eingehende Anrufe.</p>
          </div>
        </div>
        <h2 class="settings-group">Start & Hintergrund</h2>
        <fieldset disabled={loading || busy || browser || !desktopSettings}>
          <div class="setting-row">
            <div class="setting-description">
              <label for="autostart-enabled">Bei der Anmeldung starten</label>
              <p id="autostart-help">
                Ruf startet im Hintergrund und bleibt für Anrufe erreichbar.
              </p>
              {#if !desktopSettings?.autostartAvailable}<p
                  id="autostart-unavailable"
                >
                  Installiere Ruf an einem festen Ort. In Entwicklungs-Builds
                  ist der automatische Start nicht verfügbar.
                </p>{/if}
            </div>
            <div class="setting-control sound-toggle">
              <input
                id="autostart-enabled"
                type="checkbox"
                bind:checked={autostartEnabled}
                disabled={!desktopSettings?.autostartAvailable}
                aria-describedby={`autostart-help${!desktopSettings?.autostartAvailable ? " autostart-unavailable" : ""}`}
              /><span aria-hidden="true"
                >{autostartEnabled ? "Ein" : "Aus"}</span
              >
            </div>
          </div>
          <div class="setting-row">
            <div class="setting-description">
              <h3>Beim Schließen erreichbar bleiben</h3>
              <p>
                Schließt du das Telefonfenster, läuft Ruf weiter. Über das
                Ruf-Symbol öffnest du es wieder. „Ruf beenden“ beendet auch die
                Telefonie.
              </p>
            </div>
          </div>
          <h2 class="settings-group sound-group">Mitteilungen</h2>
          <div class="setting-row">
            <div class="setting-description">
              <label for="notifications-enabled"
                >Eingehende Anrufe anzeigen</label
              >
              <p id="notifications-help">
                Zeigt Name oder Rufnummer mit „Annehmen“ und „Ablehnen“, auch
                wenn das Telefonfenster verborgen ist. Beim Aktivieren kann das
                Betriebssystem nach deiner Erlaubnis fragen.
              </p>
              {#if notificationsEnabled && desktopSettings}<p
                  id="notifications-status"
                >
                  {#if desktopSettings.notificationPermission === "notDetermined"}
                    Noch keine Freigabe erteilt. Klicke auf „Mitteilungen
                    erlauben“, um die Systemabfrage zu öffnen.
                  {:else if desktopSettings.notificationPermission === "denied"}
                    Mitteilungen sind in den Systemeinstellungen für Ruf
                    deaktiviert. Erlaube sie dort. Anrufe bleiben im
                    Telefonfenster sichtbar.
                  {:else if desktopSettings.notificationPermission === "authorized"}
                    Mitteilungen sind erlaubt.
                  {:else}
                    Systemmitteilungen sind in diesem Startmodus nicht
                    verfügbar. Nutze dafür die installierte App. Der Klingelton
                    bleibt verfügbar.
                  {/if}
                </p>{/if}
            </div>
            <div class="setting-control sound-toggle">
              <input
                id="notifications-enabled"
                type="checkbox"
                bind:checked={notificationsEnabled}
                aria-describedby={`notifications-help${notificationsEnabled && desktopSettings ? " notifications-status" : ""}`}
              /><span aria-hidden="true"
                >{notificationsEnabled ? "Ein" : "Aus"}</span
              >
            </div>
          </div>
          {#if notificationsEnabled && desktopSettings?.notificationPermission === "notDetermined"}
            <div class="settings-form-actions">
              <p>
                Die Freigabe gilt für Systemmitteilungen auf diesem Computer.
              </p>
              <button
                type="button"
                class="secondary-button"
                onclick={() => void allowNotifications()}
              >
                Mitteilungen erlauben
              </button>
            </div>
          {/if}
          {#if permissionFeedback}<p class="settings-feedback" role="status">
              {permissionFeedback}
            </p>{/if}
          <div class="settings-form-actions">
            <p>
              {desktopDirty
                ? "Deine Änderungen sind noch nicht gespeichert."
                : "Gilt für Ruf auf deinem Computer."}
            </p>
            <button
              class="primary-button"
              type="button"
              disabled={!desktopDirty ||
                busy ||
                loading ||
                browser ||
                !desktopSettings}
              onclick={() => {
                void saveDesktop();
              }}>{busy ? "Bitte warten …" : "Allgemein speichern"}</button
            >
          </div>
        </fieldset>
      {:else if section === "contacts"}
        <ContactsSettings />
      {:else if section === "accounts"}
        {#if removedDraft}<div class="settings-feedback error" role="alert">
            <p>
              Dieses Konto wurde entfernt. Dein Entwurf bleibt sichtbar. Verwirf
              ihn, um ein anderes Konto zu bearbeiten.
            </p>
            <button
              class="text-button"
              type="button"
              onclick={() => navigate("accounts", accounts[0]?.id ?? null)}
              >Anderes Konto wählen</button
            >
          </div>{/if}
        {#key editedId}<AccountSetup
            bind:account
            bind:name
            {existing}
            removed={removedDraft}
            {busy}
            disabled={disabled || removedDraft}
            onconfigure={save}
            onreconnect={() => {
              void reconnect();
            }}
            onremove={() => {
              void remove();
            }}
          />{/key}
        <p class="settings-footnote">
          Bis zu acht Konten bleiben gleichzeitig verbunden. Das Konto für
          ausgehende Anrufe wählst du im Telefonfenster.
        </p>
      {:else if section === "audio"}
        <SoundPanel
          bind:value={sound}
          {devices}
          {devicesLoading}
          {devicesError}
          {disabled}
          nativeAvailable={info?.telephony === "linked"}
          {busy}
          dirty={soundDirty}
          playing={callIdle && (snapshot?.ringtonePlaying ?? false)}
          testAudio={snapshot?.audioMode === "testTone"}
          onrefresh={() => {
            void loadDevices();
          }}
          onsave={() => {
            const selected = { ...sound };
            void soundOperation(() => saveSoundSettings(selected), true);
          }}
          onpreview={() => {
            const device = sound.ringDevice;
            ownsPreview = true;
            void soundOperation(() => previewRingtone(device), false);
          }}
          onstop={() => {
            ownsPreview = false;
            void soundOperation(stopRingtonePreview, false);
          }}
        />
        <details class="settings-lab sound-developer">
          <summary>Audio für Entwicklung</summary>
          <div class="setting-row">
            <div class="setting-description">
              <label for="audio-mode">Audioquelle</label>
              <p>
                {snapshot?.audioMode
                  ? "Für diese App-Sitzung festgelegt. Zum Ändern die App neu starten."
                  : "Wird beim Start der Telefonie übernommen."}
              </p>
            </div>
            <div class="setting-control">
              <select
                id="audio-mode"
                bind:value={account.audioMode}
                disabled={!!snapshot?.audioMode || busy || loading}
                ><option value="system">Mikrofon & Lautsprecher</option><option
                  value="testTone">Testton für Entwicklung</option
                ></select
              >
            </div>
          </div>
          <p>
            Der Testton ersetzt dein Mikrofon durch 440 Hz und zeichnet
            empfangenes Audio als received.wav im App-Cache auf. Im Testmodus
            klingeln eingehende Anrufe nicht automatisch. Die Hörprobe bleibt
            eine bewusste Aktion.
          </p>
        </details>
      {:else}
        <div class="settings-title">
          <div>
            <img src="/ruf.svg" alt="" width="48" height="48" />
            <h1>Über Ruf</h1>
            <p>Ein kompaktes Telefon für deinen Desktop.</p>
          </div>
        </div>
        <h2 class="settings-group">Diese Version</h2>
        <div class="setting-row">
          <div class="setting-description">
            <h3>Version</h3>
            <p>Open Source · MIT-Lizenz</p>
          </div>
          <span>{info?.version ?? "Browser-Vorschau"}</span>
        </div>
        <div class="setting-row">
          <div class="setting-description">
            <h3>Konten & Passwörter</h3>
            <p>
              Konten bleiben nach einem Neustart erhalten. Passwörter liegen im
              macOS-Schlüsselbund bzw. Windows Credential Manager und werden nie
              an diese Ansicht zurückgegeben.
            </p>
          </div>
        </div>
        <div class="setting-row">
          <div class="setting-description">
            <h3>Telefonie</h3>
            <p>
              Derzeit SIP/UDP ohne Verschlüsselung und G.711. Nutze diese
              Entwicklungsversion nur im isolierten Testnetz. Ein Gespräch
              gleichzeitig. macOS-Kontakte lassen sich nach deiner Freigabe
              durchsuchen und anrufen. Der lokale Verlauf zeigt deine Anrufe mit
              Ergebnis, Dauer und dem verwendeten Konto.
            </p>
          </div>
        </div>
        <details class="settings-lab">
          <summary>Lokale Asterisk-Testanlage</summary>
          <p>
            Starte im Projekt <code>bun run lab:up</code>. Server:
            <code>127.0.0.1:5070</code>, Benutzer: <code>1000</code>,
            öffentliches Testpasswort: <code>sip-lab-only-not-a-secret</code>.
            Der Echotest ist unter <code>600</code> erreichbar. Für den automatischen
            Audiotest wähle vor dem ersten Konto den Testton.
          </p>
        </details>
      {/if}
    </div>
  </main>
</div>
