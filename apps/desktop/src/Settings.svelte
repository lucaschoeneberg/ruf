<script lang="ts">
  import { onMount, tick } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    errorMessage,
    getAppInfo,
    getEngineSnapshot,
    saveAccount,
    removeAccount,
    getAudioDevices,
    saveSoundSettings,
    previewRingtone,
    stopRingtonePreview,
    getDesktopSettings,
    saveDesktopSettings,
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
  let pendingNavigation = $state<{
    section: Section;
    id: number | null;
  } | null>(null);
  let mounted = false;
  let epoch = 0;
  let snapshotRevision = 0;
  let firstLoad = true;
  let discardNotice = $state<HTMLDivElement>();
  let keepEditing = $state<HTMLButtonElement>();
  const existing = $derived(
    snapshot?.accounts.find((a) => a.id === editedId) ?? null,
  );
  const accounts = $derived(snapshot?.accounts ?? []);
  const callIdle = $derived(!snapshot || snapshot.call === "idle");
  const disabled = $derived(
    loading || !info || info.telephony !== "linked" || !callIdle,
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
      id: "about",
      label: "Über SIP",
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
      pendingNavigation = { section: next, id };
      void tick().then(() => {
        discardNotice?.scrollIntoView({ block: "nearest" });
        keepEditing?.focus({ preventScroll: true });
      });
      return;
    }
    applyNavigation(next, id);
  }
  function applyNavigation(next: Section, id: number | null) {
    if (section === "audio" && soundDirty)
      resetSound(snapshot?.soundSettings ?? defaultSound());
    if (section === "general" && desktopSettings) resetDesktop(desktopSettings);
    account.password = "";
    section = next;
    edit(id);
    pendingNavigation = null;
  }
  function applySnapshot(result: EngineSnapshot) {
    if (!soundDirty) resetSound(result.soundSettings);
    snapshot = result;
    if (firstLoad) {
      firstLoad = false;
      edit(result.selectedAccountId ?? result.accounts[0]?.id ?? null);
    } else if (
      !accountDirty &&
      editedId !== null &&
      !result.accounts.some((a) => a.id === editedId)
    )
      edit(result.accounts[0]?.id ?? null);
    if (result.audioMode) account.audioMode = result.audioMode;
  }
  async function refresh() {
    if (browser) return;
    loading = true;
    error = null;
    const current = epoch;
    const revision = snapshotRevision;
    try {
      const result = await getAppInfo();
      if (!mounted || current !== epoch) return;
      info = result;
      const [state, desktop] = await Promise.all([
        result.telephony === "linked" ? getEngineSnapshot() : null,
        getDesktopSettings(),
      ]);
      if (!mounted || current !== epoch) return;
      if (state && revision === snapshotRevision) applySnapshot(state);
      else if (!state && !accountDirty) edit(null);
      if (!desktopDirty) resetDesktop(desktop);
      else desktopSettings = desktop;
      if (section === "audio") void loadDevices();
    } catch (reason) {
      if (mounted && current === epoch) error = errorMessage(reason);
    } finally {
      if (mounted) loading = false;
    }
  }
  onMount(() => {
    mounted = true;
    document.title = "SIP · Einstellungen";
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
    if (browser) {
      loading = false;
      edit(null);
    } else
      void Promise.allSettled([
        listen<EngineSnapshot>("phone-state-changed", ({ payload }) => {
          if (!mounted) return;
          snapshotRevision++;
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
      const revision = snapshotRevision;
      void getEngineSnapshot()
        .then((result) => {
          if (
            mounted &&
            current === epoch &&
            revision === snapshotRevision &&
            !busy
          )
            applySnapshot(result);
        })
        .catch((reason) => {
          if (mounted && current === epoch) error = errorMessage(reason);
        })
        .finally(() => {
          polling = false;
        });
    };
    const timer = setInterval(poll, 2000);
    document.addEventListener("visibilitychange", poll);
    return () => {
      mounted = false;
      clearInterval(timer);
      for (const unlisten of unlisteners) unlisten();
      document.removeEventListener("visibilitychange", poll);
      account.password = "";
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
    busy = true;
    error = null;
    feedback = "";
    try {
      const result = await saveDesktopSettings(
        autostartEnabled,
        notificationsEnabled,
      );
      if (!mounted) return;
      resetDesktop(result);
      feedback = "Allgemeine Einstellungen gespeichert.";
    } catch (reason) {
      if (mounted) error = errorMessage(reason);
    } finally {
      if (mounted) busy = false;
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
    const revision = snapshotRevision;
    try {
      const result = await operation();
      if (!mounted) return;
      if (revision === snapshotRevision) applySnapshot(result);
      if (saved) {
        resetSound(result.soundSettings);
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
    if (busy || disabled) return;
    busy = true;
    epoch++;
    error = null;
    feedback = "";
    const before = new Set(accounts.map((a) => a.id));
    const revision = snapshotRevision;
    try {
      const result = await saveAccount(editedId, label, config);
      if (!mounted) return;
      if (revision === snapshotRevision) applySnapshot(result);
      edit(
        editedId ?? result.accounts.find((a) => !before.has(a.id))?.id ?? null,
      );
      feedback = "Gespeichert. Die Anmeldung an der Telefonanlage läuft.";
      if (result.lastError) error = errorMessage(result.lastError);
    } catch (reason) {
      if (mounted) error = errorMessage(reason);
    } finally {
      account.password = "";
      if (mounted) busy = false;
    }
  }
  async function remove() {
    if (busy || disabled || editedId === null) return;
    busy = true;
    epoch++;
    error = null;
    const revision = snapshotRevision;
    try {
      const result = await removeAccount(editedId);
      if (mounted) {
        if (revision === snapshotRevision) applySnapshot(result);
        edit(accounts.find((a) => !a.removing)?.id ?? null);
        feedback = "Konto entfernt.";
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
      if (query) query = "";
      else pendingNavigation = null;
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
        Keine Treffer für „{query}“.
      </p>{/if}
    <div class="settings-sidebar-footer">
      <span>SIP</span><span>{info ? `v${info.version}` : "Einstellungen"}</span>
    </div>
  </aside>
  <main class="settings-main">
    <div class="settings-breadcrumb">
      Einstellungen<span aria-hidden="true">/</span><span
        >{sections.find((s) => s.id === section)?.label}</span
      >
    </div>
    <div class="settings-scroll">
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
      {#if error}<div class="settings-feedback error" role="alert">
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
              onclick={() => (pendingNavigation = null)}
              >Weiter bearbeiten</button
            ><button
              class="secondary-button"
              type="button"
              onclick={() => {
                if (pendingNavigation)
                  applyNavigation(
                    pendingNavigation.section,
                    pendingNavigation.id,
                  );
              }}>Änderungen verwerfen</button
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
                SIP startet im Hintergrund und bleibt für Anrufe erreichbar.
              </p>
              {#if !desktopSettings?.autostartAvailable}<p
                  id="autostart-unavailable"
                >
                  Installiere SIP an einem festen Ort. In Entwicklungs-Builds
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
                Schließt du das Telefonfenster, läuft SIP weiter. Über das
                SIP-Symbol öffnest du es wieder. „SIP beenden“ beendet auch die
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
                Eine Systemmitteilung weist auf Anrufe hin, während das
                Telefonfenster im Hintergrund ist. Beim Aktivieren kann das
                Betriebssystem nach deiner Erlaubnis fragen.
              </p>
              {#if desktopSettings?.notificationsEnabled && desktopSettings.notificationsGranted === false}<p
                  id="notifications-denied"
                >
                  Mitteilungen sind im Betriebssystem nicht erlaubt. Erlaube sie
                  in den Systemeinstellungen für SIP. Anrufe bleiben im
                  Telefonfenster sichtbar.
                </p>{:else if desktopSettings?.notificationsGranted === null}<p
                  id="notifications-permission"
                >
                  Die Systemeinstellungen entscheiden, ob Mitteilungen für SIP
                  erscheinen.
                </p>{/if}
            </div>
            <div class="setting-control sound-toggle">
              <input
                id="notifications-enabled"
                type="checkbox"
                bind:checked={notificationsEnabled}
                aria-describedby={`notifications-help${desktopSettings?.notificationsEnabled && desktopSettings.notificationsGranted === false ? " notifications-denied" : desktopSettings?.notificationsGranted === null ? " notifications-permission" : ""}`}
              /><span aria-hidden="true"
                >{notificationsEnabled ? "Ein" : "Aus"}</span
              >
            </div>
          </div>
          <div class="settings-form-actions">
            <p>
              {desktopDirty
                ? "Deine Änderungen sind noch nicht gespeichert."
                : "Gilt für diese SIP-App auf deinem Computer."}
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
      {:else if section === "accounts"}
        {#key editedId}<AccountSetup
            bind:account
            bind:name
            {existing}
            {busy}
            {disabled}
            onconfigure={save}
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
            void soundOperation(() => previewRingtone(device), false);
          }}
          onstop={() => {
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
            <h1>Über SIP</h1>
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
              gleichzeitig; Kontakte und Verlauf sind noch nicht angebunden.
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
