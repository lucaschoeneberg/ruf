<script lang="ts">
  import {
    t,
    text,
    message,
    locale,
    refreshLocalization,
    type MessageValue,
  } from "./lib/i18n";
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
    saveDesktopSettings,
    getPreferences,
    savePreferences,
    selectAccount,
    setAccountEnabled,
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
    AppPreferences,
  } from "./lib/generated/AppInfo";
  import AccountSetup from "./lib/components/AccountSetup.svelte";
  import SoundPanel from "./lib/components/SoundPanel.svelte";
  import AboutPanel from "./lib/components/AboutPanel.svelte";
  import GeneralSettings from "./lib/components/GeneralSettings.svelte";
  import NotificationSettings from "./lib/components/NotificationSettings.svelte";
  import PrivacySettings from "./lib/components/PrivacySettings.svelte";
  import DiagnosticsSettings from "./lib/components/DiagnosticsSettings.svelte";
  import DataSettings from "./lib/components/DataSettings.svelte";
  import AudioDiagnostics from "./lib/components/AudioDiagnostics.svelte";
  import { settingsSections, searchSettings } from "./lib/settings-navigation";
  import Icon from "./lib/components/Icon.svelte";

  type Section = SettingsSection;
  let section = $state<Section>("accounts");
  let query = $state("");
  let snapshot = $state<EngineSnapshot | null>(null);
  let info = $state<AppInfo | null>(null);
  let loading = $state(true);
  let busy = $state(false);
  let browser = $state(false);
  let error = $state<MessageValue | null>(null);
  let audioError = $state<MessageValue | null>(null);
  let feedback = $state<MessageValue>("");
  let editedId = $state<number | null>(null);
  let name = $state("");
  let account = $state<AccountConfig>({
    transport: "udp",
    mediaSecurity: "disabled",
    authUsername: null,
    outboundProxy: null,
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
    ringtone: "classic",
    ringtoneVolume: 100,
  });
  let sound = $state<SoundSettings>(defaultSound());
  let soundBaseline = $state(JSON.stringify(defaultSound()));
  const soundDirty = $derived(JSON.stringify(sound) !== soundBaseline);
  const defaultPreferences = (): AppPreferences => ({
    language: "system",
    startHidden: true,
    showCallerInNotifications: true,
    contacts: {
      favoritesFirst: true,
      showOrganization: true,
      matchGermanNumbers: true,
    },
    history: { recordingEnabled: true, retentionDays: null },
  });
  let preferences = $state<AppPreferences | null>(null);
  let preferenceDraft = $state<AppPreferences>(defaultPreferences());
  let preferencesLoading = $state(false);
  let preferencesError = $state<MessageValue | null>(null);
  let confirmRetention = $state(false);
  const preferencesDirty = $derived(
    preferences !== null &&
      JSON.stringify(preferenceDraft) !== JSON.stringify(preferences),
  );
  let helpOverlay = $state<Section | null>(null);
  const activeSection = $derived(helpOverlay ?? section);
  const readOnlySections: Section[] = ["privacy", "diagnostics", "about"];
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
  let devicesError = $state<MessageValue | null>(null);
  let baseline = $state("");
  type PendingAction =
    | {
        kind: "navigate";
        section: Section;
        id: number | null;
        target?: string | undefined;
      }
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
    baseline !== "" && baseline !== accountDraftKey(),
  );
  const dirty = $derived.by((): boolean => {
    switch (section) {
      case "audio":
        return soundDirty;
      case "general":
      case "notifications":
      case "contacts":
        return desktopDirty || preferencesDirty;
      case "accounts":
        return accountDirty;
      default:
        return false;
    }
  });
  const needle = $derived(query.trim().toLocaleLowerCase($locale));
  const sections = settingsSections;
  const searchResults = $derived(searchSettings(query, $locale));
  const visibleSections = $derived(
    sections.filter(
      (s) =>
        !needle ||
        $text(s.label).toLocaleLowerCase($locale).includes(needle) ||
        searchResults.some((result) => result.section === s.id),
    ),
  );
  const visibleAccounts = $derived(
    accounts.filter(
      (a) =>
        !needle ||
        `${a.name} ${a.username} ${a.registrar}`
          .toLocaleLowerCase($locale)
          .includes(needle) ||
        visibleSections.some((s) => s.id === "accounts"),
    ),
  );

  function accountDraftKey(): string {
    return JSON.stringify({
      name,
      registrar: account.registrar,
      username: account.username,
      password: account.password,
      transport: account.transport,
      mediaSecurity: account.mediaSecurity,
      authUsername: account.authUsername,
      outboundProxy: account.outboundProxy,
    });
  }
  function edit(id: number | null) {
    const saved = accounts.find((a) => a.id === id);
    editedId = saved?.id ?? null;
    name = saved?.name ?? "";
    account = {
      transport: saved?.transport ?? "udp",
      mediaSecurity: saved?.mediaSecurity ?? "disabled",
      authUsername: saved?.authUsername ?? null,
      outboundProxy: saved?.outboundProxy ?? null,
      registrar: saved?.registrar ?? "",
      username: saved?.username ?? "",
      password: "",
      audioMode: snapshot?.audioMode ?? account.audioMode,
    };
    baseline = accountDraftKey();
    feedback = "";
  }
  function navigate(
    next: Section,
    id: number | null = editedId,
    target?: string,
  ) {
    if (busy) {
      if (readOnlySections.includes(next)) helpOverlay = next;
      return;
    }
    if (helpOverlay && next === section && id === editedId && !target) {
      helpOverlay = null;
      return;
    }
    if (next === section && id === editedId && !target && !helpOverlay) return;
    if (dirty && next !== section) {
      showDiscard({ kind: "navigate", section: next, id, target });
      return;
    }
    if (dirty && next === "accounts" && id !== editedId) {
      showDiscard({ kind: "navigate", section: next, id, target });
      return;
    }
    applyNavigation(next, id, target);
  }
  function showDiscard(action: PendingAction) {
    pendingNavigation = action;
    void tick().then(() => {
      discardNotice?.scrollIntoView({ block: "nearest" });
      keepEditing?.focus({ preventScroll: true });
    });
  }
  function continueEditing() {
    helpOverlay = null;
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
      error = message("settings.save_pending");
    } else if (dirty) showDiscard({ kind });
    else void finishWindowAction(kind);
  }
  function discardChanges() {
    if (busy) return;
    const action = pendingNavigation;
    pendingNavigation = null;
    if (!action) return;
    if (action.kind === "navigate")
      applyNavigation(action.section, action.id, action.target);
    else void finishWindowAction(action.kind);
  }
  function applyNavigation(next: Section, id: number | null, target?: string) {
    helpOverlay = null;
    const leaving =
      next !== section || (next === "accounts" && id !== editedId);
    if (leaving && section === "audio")
      void stopOwnedPreview().catch((reason) => {
        if (mounted) error = errorMessage(reason);
      });
    if (leaving && section === "audio" && soundDirty)
      resetSound(snapshot?.soundSettings ?? defaultSound());
    if (leaving && ["general", "notifications", "contacts"].includes(section))
      discardPreferences();
    if (leaving) account.password = "";
    section = next;
    if (next === "accounts" && (id !== editedId || !accountDirty)) edit(id);
    pendingNavigation = null;
    if (target) {
      query = "";
      void tick().then(() => {
        const el = document.getElementById(target);
        for (
          let ancestor = el?.parentElement;
          ancestor;
          ancestor = ancestor.parentElement
        ) {
          if (ancestor instanceof HTMLDetailsElement) ancestor.open = true;
        }
        el?.scrollIntoView({ block: "center" });
        el?.focus({ preventScroll: true });
        el?.classList.add("settings-search-target");
        setTimeout(() => el?.classList.remove("settings-search-target"), 1800);
      });
    }
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
        account.username !== updated.username ||
        account.transport !== updated.transport ||
        account.mediaSecurity !== updated.mediaSecurity ||
        account.authUsername !== updated.authUsername ||
        account.outboundProxy !== updated.outboundProxy
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
  $effect(() => {
    document.title = $t("settings.window_title");
  });
  onMount(() => {
    mounted = true;
    void loadPreferences();

    browser = !isTauri();
    const requested = new URLSearchParams(window.location.search).get(
      "section",
    );
    const initialSection = sections.find((item) => item.id === requested)?.id;
    if (initialSection) section = initialSection;
    const unlisteners: UnlistenFn[] = [];
    if (isTauri())
      void listen("settings-find-requested", () => {
        if (mounted) document.getElementById("settings-search")?.focus();
      })
        .then(retain)
        .catch(() => {});

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
    if (
      busy ||
      browser ||
      (!desktopDirty && !preferencesDirty) ||
      (desktopDirty && !desktopSettings)
    )
      return;
    const current = ++epoch;
    busy = true;
    error = null;
    feedback = "";
    try {
      if (desktopDirty) {
        const result = await saveDesktopSettings(
          autostartEnabled,
          notificationsEnabled,
        );
        if (!mounted || current !== epoch) return;
        resetDesktop(result);
      }
      if (preferencesDirty) {
        const result = await savePreferences(
          $state.snapshot(preferenceDraft),
          confirmRetention,
        );
        if (!mounted || current !== epoch) return;
        preferences = result;
        preferenceDraft = structuredClone(result);
        confirmRetention = false;
        await refreshLocalization();
        if (!mounted || current !== epoch) return;
      }
      feedback = message("settings.saved");
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
    } catch (reason) {
      if (mounted && current === epoch) error = errorMessage(reason);
    }
  }
  async function loadPreferences() {
    if (preferencesLoading || !isTauri()) return;
    preferencesLoading = true;
    preferencesError = null;
    try {
      const result = await getPreferences();
      if (mounted) {
        if (!preferencesDirty) preferenceDraft = structuredClone(result);
        preferences = result;
      }
    } catch {
      if (mounted)
        preferencesError = message("settings.preferences_load_failed");
    } finally {
      if (mounted) preferencesLoading = false;
    }
  }
  function discardPreferences() {
    if (desktopSettings) resetDesktop(desktopSettings);
    if (preferences)
      preferenceDraft = structuredClone($state.snapshot(preferences));
    confirmRetention = false;
  }
  async function chooseAccount(id: number) {
    if (busy || !callIdle) return;
    busy = true;
    try {
      applySnapshot(await selectAccount(id));
    } catch (e) {
      error = errorMessage(e);
    } finally {
      if (mounted) busy = false;
    }
  }
  async function toggleAccount(id: number, enabled: boolean) {
    if (busy || !callIdle) return;
    busy = true;
    try {
      applySnapshot(await setAccountEnabled(id, enabled));
    } catch (e) {
      error = errorMessage(e);
    } finally {
      if (mounted) busy = false;
    }
  }
  function resetSound(value: SoundSettings) {
    sound = { ...defaultSound(), ...value };
    soundBaseline = JSON.stringify(sound);
  }
  async function loadDevices() {
    if (devicesLoading || info?.telephony !== "linked") return;
    devicesLoading = true;
    devicesError = null;
    try {
      const result = await getAudioDevices();
      if (mounted) devices = result;
    } catch {
      if (mounted) devicesError = message("settings.devices_load_failed");
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
        feedback = message("settings.sound_saved");
      }
    } catch (reason) {
      if (mounted) {
        error = errorMessage(reason);
        audioError = error;
      }
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
      feedback = message("settings.account_saved");
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
      target.enabled === false ||
      target.lastError?.code === "credentialMissing" ||
      target.lastError?.code === "storageFailure"
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
      else feedback = message("settings.reconnecting");
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
          ? message("settings.account_removing")
          : message("settings.account_removed_notice");
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
  <aside class="settings-sidebar" aria-label={$t("settings.navigation")}>
    <div class="settings-search">
      <Icon name="search" size={15} /><input
        id="settings-search"
        aria-label={$t("settings.search_accounts_sections")}
        bind:value={query}
        placeholder={$t("settings.search_placeholder")}
        autocomplete="off"
        spellcheck="false"
      />{#if query}<button
          class="icon-button"
          type="button"
          aria-label={$t("settings.clear_search")}
          onclick={() => (query = "")}><Icon name="close" size={13} /></button
        >{/if}
    </div>
    {#if query.trim() && searchResults.length}<nav
        class="settings-results"
        aria-label={$t("settings.search_results")}
      >
        {#each searchResults as result}<button
            type="button"
            disabled={busy && !readOnlySections.includes(result.section)}
            onclick={() => navigate(result.section, editedId, result.target)}
            ><span>{result.label}</span><small
              >{$text(
                sections.find((s) => s.id === result.section)?.label,
              )}</small
            ></button
          >{/each}
      </nav>{/if}
    <nav class="settings-navigation" aria-label={$t("settings.sections")}>
      {#each visibleSections as item}<button
          type="button"
          class:current={activeSection === item.id}
          aria-current={activeSection === item.id ? "page" : undefined}
          disabled={busy && !readOnlySections.includes(item.id)}
          onclick={() => navigate(item.id)}
          ><Icon name={item.icon} size={16} /><span>{$text(item.label)}</span
          >{#if item.id === "accounts"}<span class="nav-count"
              >{accounts.length}</span
            >{/if}</button
        >{/each}
    </nav>
    {#if visibleAccounts.length || visibleSections.some((s) => s.id === "accounts")}<div
        class="accounts-heading"
      >
        <h2>{$t("settings.your_accounts")}</h2>
        <button
          class="icon-button"
          type="button"
          aria-label={$t("settings.add_sip_account")}
          title={accounts.length >= 8
            ? $t("settings.account_limit")
            : $t("settings.add_account")}
          disabled={busy || loading || accounts.length >= 8}
          onclick={() => navigate("accounts", null, "name")}
          ><Icon name="plus" size={16} /></button
        >
      </div>
      <nav class="accounts-list" aria-label={$t("settings.sip_accounts")}>
        {#each visibleAccounts as item}<button
            type="button"
            class:current={section === "accounts" && editedId === item.id}
            disabled={busy || item.removing}
            aria-current={section === "accounts" && editedId === item.id
              ? "page"
              : undefined}
            onclick={() => navigate("accounts", item.id, "name")}
            ><span class="account-list-name"
              ><span
                class="status-dot"
                class:connected={item.registration === "registered"}
                class:failed={item.registration === "failed"}
                aria-hidden="true"
              ></span><span>{item.name}</span></span
            ><small>{item.username}@{item.registrar}</small><span
              class="sr-only"
              >{item.enabled === false
                ? $t("settings.disabled")
                : item.registration === "registered"
                  ? $t("settings.registered")
                  : item.registration === "failed"
                    ? $t("settings.registration_failed")
                    : $t("settings.not_registered")}</span
            ></button
          >{/each}
        {#if !accounts.length}<p class="accounts-empty">
            {$t("settings.no_accounts")}<br />{$t("settings.first_account")}
          </p>{/if}
      </nav>
      {#if accounts.length >= 8}<p class="accounts-empty accounts-limit">
          {$t("settings.account_limit_notice")}
        </p>{/if}
    {:else if !visibleSections.length}<p class="accounts-empty" role="status">
        {$t("settings.no_search_results", { query })}
      </p>{/if}
    <div class="settings-sidebar-footer">
      <span>Ruf</span><span
        >{info ? `v${info.version}` : $t("settings.title")}</span
      >
    </div>
  </aside>
  <main class="settings-main">
    <div class="settings-breadcrumb">
      {$t("settings.title")}<span aria-hidden="true">/</span><span
        >{$text(sections.find((s) => s.id === activeSection)?.label)}</span
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
            ? $t("settings.browser_preview")
            : $t("settings.native_required")}
        </p>{/if}
      {#if loading}<p class="settings-feedback" role="status">
          {$t("settings.loading")}
        </p>{/if}
      {#if error}<div
          bind:this={errorNotice}
          class="settings-feedback error"
          role="alert"
        >
          <p>
            {$text(error)}{#if error === audioError}
              {$t("settings.audio_error_help")}{/if}
          </p>
          <button
            class="text-button"
            type="button"
            disabled={busy || loading}
            onclick={() => {
              void refresh();
              void loadPreferences();
            }}>{$t("settings.reload_status")}</button
          >
        </div>{/if}
      {#if preferencesError && ["general", "notifications", "contacts"].includes(activeSection)}
        <div class="settings-feedback error" role="alert">
          <p>{$text(preferencesError)}</p>
          <button
            type="button"
            class="text-button"
            disabled={preferencesLoading}
            onclick={() => void loadPreferences()}
            >{$t("settings.reload_settings")}</button
          >
        </div>
      {/if}
      {#if !callIdle && (section === "accounts" || section === "audio")}<p
          class="settings-feedback"
          role="status"
        >
          {$t("settings.call_active")}
        </p>{/if}
      {#if pendingNavigation}<div
          bind:this={discardNotice}
          class="settings-feedback discard-note"
          role="group"
          aria-label={$t("settings.unsaved_changes")}
        >
          <p>{$t("settings.unsaved_notice")}</p>
          <div>
            <button
              class="secondary-button"
              type="button"
              bind:this={keepEditing}
              onclick={continueEditing}>{$t("settings.keep_editing")}</button
            ><button
              class="secondary-button"
              type="button"
              disabled={busy}
              onclick={discardChanges}>{$t("settings.discard_changes")}</button
            >
          </div>
        </div>{/if}
      {#if feedback && !dirty}<p class="settings-feedback" role="status">
          {$text(feedback)}
        </p>{/if}
      {#if activeSection === "general"}
        <GeneralSettings
          bind:autostart={autostartEnabled}
          bind:startHidden={preferenceDraft.startHidden}
          bind:language={preferenceDraft.language}
          available={desktopSettings?.autostartAvailable ?? false}
          disabled={busy ||
            loading ||
            browser ||
            !desktopSettings ||
            !preferences}
          dirty={desktopDirty || preferencesDirty}
          onsave={() => void saveDesktop()}
          ondiscard={discardPreferences}
        />
      {:else if activeSection === "notifications"}
        <NotificationSettings
          bind:enabled={notificationsEnabled}
          bind:showCaller={preferenceDraft.showCallerInNotifications}
          dirty={desktopDirty || preferencesDirty}
          {busy}
          canSave={!loading && !browser && !!desktopSettings && !!preferences}
          onsave={() => void saveDesktop()}
          ondiscard={discardPreferences}
        />
      {:else if activeSection === "contacts"}
        <DataSettings
          bind:value={preferenceDraft}
          bind:confirmRetention
          dirty={preferencesDirty}
          disabled={busy || browser || !preferences || !callIdle}
          onsave={() => void saveDesktop()}
          ondiscard={discardPreferences}
        />
      {:else if activeSection === "privacy"}
        <PrivacySettings />
      {:else if activeSection === "diagnostics"}
        <DiagnosticsSettings />
        <details class="settings-lab sound-developer">
          <summary id="developer-audio"
            >{$t("settings.developer_audio")}</summary
          >
          <div class="setting-row">
            <div class="setting-description">
              <label for="audio-mode">{$t("settings.audio_source")}</label>
              <p>
                {snapshot?.audioMode
                  ? $t("settings.audio_session_fixed")
                  : $t("settings.audio_start_applied")}
              </p>
            </div>
            <div class="setting-control">
              <select
                id="audio-mode"
                bind:value={account.audioMode}
                disabled={!!snapshot?.audioMode || busy || loading}
                ><option value="system"
                  >{$t("settings.microphone_speaker")}</option
                ><option value="testTone"
                  >{$t("settings.developer_tone")}</option
                ></select
              >
            </div>
          </div>
          <p>
            {$t("settings.developer_tone_description")}
          </p>
        </details>
      {:else if activeSection === "accounts"}
        <div class="settings-title">
          <div>
            <h1>{$t("settings.sip_accounts")}</h1>
            <p>{$t("settings.accounts_description")}</p>
          </div>
        </div>
        {#if accounts.length}<section
            class="account-overview"
            aria-labelledby="accounts-overview-heading"
          >
            <h2 class="settings-group" id="accounts-overview-heading">
              {$t("settings.saved_accounts")}
            </h2>
            {#each accounts as item}<div
                class="setting-row"
                role="group"
                aria-labelledby={`overview-account-${item.id}`}
              >
                <div class="setting-description">
                  <h3 id={`overview-account-${item.id}`}>{item.name}</h3>
                  <p>
                    {item.enabled === false
                      ? $t("settings.disabled")
                      : item.registration === "registered"
                        ? $t("settings.registration_confirmed")
                        : item.registration === "registering"
                          ? $t("settings.registering")
                          : $t(
                              "settings.not_registered",
                            )}{snapshot?.selectedAccountId === item.id
                      ? $t("settings.outgoing_default_suffix")
                      : ""}
                  </p>
                  {#if item.registeredAtMs}<p>
                      {$t("settings.last_confirmed")}
                      {new Date(item.registeredAtMs).toLocaleString($locale)}
                    </p>{/if}
                </div>
                <div class="settings-actions">
                  <button
                    type="button"
                    class="text-button"
                    disabled={busy ||
                      accountDirty ||
                      !callIdle ||
                      item.removing ||
                      snapshot?.selectedAccountId === item.id}
                    aria-label={$t("settings.make_default_named", {
                      name: item.name,
                    })}
                    onclick={() => void chooseAccount(item.id)}
                    >{$t("settings.make_default")}</button
                  ><button
                    type="button"
                    class="secondary-button"
                    disabled={busy ||
                      accountDirty ||
                      !callIdle ||
                      item.removing ||
                      item.enabledChanging}
                    aria-label={$t(
                      item.enabled === false
                        ? "settings.enable_named"
                        : "settings.disable_named",
                      { name: item.name },
                    )}
                    onclick={() =>
                      void toggleAccount(item.id, item.enabled === false)}
                    >{item.enabled === false
                      ? $t("settings.enable")
                      : $t("settings.disable")}</button
                  >
                </div>
              </div>{/each}
          </section>{/if}
        {#if removedDraft}<div class="settings-feedback error" role="alert">
            <p>
              {$t("settings.removed_draft")}
            </p>
            <button
              class="text-button"
              type="button"
              onclick={() => navigate("accounts", accounts[0]?.id ?? null)}
              >{$t("settings.choose_other_account")}</button
            >
          </div>{/if}
        {#key editedId}<AccountSetup
            embedded
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
          {$t("settings.accounts_help")}
        </p>
      {:else if activeSection === "audio"}
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
          ondiscard={() =>
            resetSound(snapshot?.soundSettings ?? defaultSound())}
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
        <AudioDiagnostics
          {sound}
          disabled={disabled || busy}
          onprivacy={() => navigate("privacy")}
        />
      {:else}
        <AboutPanel {info} />
      {/if}
    </div>
  </main>
</div>
