<script lang="ts">
  import {
    t,
    text,
    locale,
    message,
    LocalizedError,
    type MessageValue,
  } from "./lib/i18n";
  import { onMount, tick } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    answer,
    openSettings,
    selectAccount,
    dial,
    errorMessage,
    getAppInfo,
    getDesktopSettings,
    getNotificationPermission,
    getPreferences,
    type SettingsSection,
    getEngineSnapshot,
    hangup,
    reject,
    setCallMuted,
    setCallHeld,
    transferCall,
    startConsultation,
    switchCall,
    cancelConsultation,
    completeTransfer,
    sendDtmf,
  } from "./lib/api";
  import type {
    AppInfo,
    Contact,
    ContactReference,
    ContactsSnapshot,
    ContactsPreferences,
    HistoryEntry,
    EngineSnapshot,
    RegistrationState,
    NotificationPermission,
  } from "./lib/generated/AppInfo";
  import Icon from "./lib/components/Icon.svelte";
  import Dialpad from "./lib/components/Dialpad.svelte";
  import CallPanel from "./lib/components/CallPanel.svelte";
  import GettingStarted from "./lib/components/GettingStarted.svelte";
  import HistoryPanel from "./lib/components/HistoryPanel.svelte";
  import { getContacts, refreshContacts } from "./lib/contacts-api";
  import { callbackHistory } from "./lib/history-api";
  import ContactsPanel from "./lib/components/ContactsPanel.svelte";
  import { hasSeenIntroduction, rememberIntroduction } from "./lib/onboarding";
  import ActionSearch from "./lib/components/ActionSearch.svelte";
  import {
    phoneActions,
    isCallAction,
    keyboardAction,
    shortcutLabel,
    type CallAction,
    type SearchAction,
  } from "./lib/shortcuts";
  import { settingsSections } from "./lib/settings-navigation";
  import { normalizeDialNumber } from "./lib/dialing";
  let searchOpen = $state(false);
  let searchFocusRequest = $state(0);
  let searchQuery = $state<string | null>(null);
  let historyEntryFocusId = $state<string | null>(null);
  function openSearchView(next: "contacts" | "history", query: string) {
    clearHistoryScope();
    contactFocusId = null;
    selectView(next);
    searchQuery = query;
    ++searchFocusRequest;
  }
  function openHistoryResult(entry: HistoryEntry, query: string) {
    openSearchView("history", query);
    historyEntryFocusId = entry.id;
  }
  let callPanel = $state<{
    availableActions: () => Record<CallAction, boolean>;
    runAction: (action: CallAction) => void;
  }>();
  const actions = $derived.by((): SearchAction[] => {
    const available = !callIdle ? callPanel?.availableActions() : undefined;
    return [
      ...phoneActions
        .filter((action) => action.id !== "search")
        .map((action) => {
          const disabled =
            (isCallAction(action.id) && !available?.[action.id]) ||
            (action.id === "number" && !callIdle);
          let reason = "";
          if (disabled)
            reason = callIdle
              ? $t("app.no_call")
              : $t("app.call_action_unavailable");
          return {
            ...action,
            label: $text(action.label),
            terms: $text(action.terms),
            shortcut: shortcutLabel(action, $locale),
            disabled,
            reason,
          };
        }),
      ...settingsSections
        .filter((section) => section.id !== "general")
        .map((section) => ({
          id: `settings:${section.id}`,
          label: $text(section.label),
          terms:
            section.id === "diagnostics"
              ? $t("app.diagnostics_terms")
              : $t("app.settings_terms"),
        })),
      {
        id: "start",
        label: $t("app.start"),
        terms: $t("app.start_terms"),
      },
    ];
  });
  async function runAction(id: string) {
    if (id.startsWith("settings:")) {
      const section = settingsSections.find(
        (section) => `settings:${section.id}` === id,
      );
      if (section) await showSettings(section.id);
      return;
    }
    switch (id) {
      case "search":
        searchOpen = true;
        break;
      case "settings":
        await showSettings();
        break;
      case "dialer":
      case "contacts":
      case "history":
      case "start":
        selectView(id);
        break;
      case "number":
        if (!callIdle) return;
        selectView("dialer");
        await tick();
        document
          .querySelector<HTMLInputElement>(".number-entry input")
          ?.focus();
        break;
      case "find": {
        if (view === "start" || view === "dialer") selectView("contacts");
        ++searchFocusRequest;
        break;
      }
      case "answer":
      case "end":
      case "mute":
      case "hold":
      case "transfer":
      case "tones":
        if (!callIdle) callPanel?.runAction(id);
        break;
    }
  }

  type View = "dialer" | "contacts" | "history" | "start";
  let view = $state<View>("dialer");
  let contactFocusId = $state<string | null>(null);
  let historyContactId: string | null = null;
  let historyScopeRequest = 0;
  let historyScope = $state<{ name: string; numbers: string[] } | null>(null);
  function showContactHistory(contact: Contact) {
    ++historyScopeRequest;
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
  async function returnCall(id: string) {
    if (
      await perform(() =>
        callbackHistory(id).catch((reason: unknown) => {
          if (
            reason &&
            typeof reason === "object" &&
            "code" in reason &&
            reason.code === "storageFailure"
          )
            throw new LocalizedError(message("app.callback_load_error"));
          throw reason;
        }),
      )
    ) {
      selectView("dialer");
    }
  }
  function clearHistoryScope() {
    ++historyScopeRequest;
    historyContactId = null;
    historyScope = null;
  }
  async function refreshHistoryScope() {
    const id = historyContactId;
    if (!id) return;
    const request = ++historyScopeRequest;
    try {
      const contacts = await getContacts();
      if (
        !mounted ||
        historyContactId !== id ||
        request !== historyScopeRequest
      )
        return;
      const contact =
        contacts.permission === "authorized"
          ? contacts.contacts.find((contact) => contact.id === id)
          : null;
      if (contact?.phones.some((phone) => phone.destination)) {
        const numbers = contact.phones
          .filter((p) => p.destination)
          .map((p) => p.number);
        if (
          historyScope?.name !== contact.name ||
          historyScope.numbers.length !== numbers.length ||
          historyScope.numbers.some(
            (number, index) => number !== numbers[index],
          )
        )
          historyScope = { name: contact.name, numbers };
      } else clearHistoryScope();
    } catch {
      if (mounted && historyContactId === id && request === historyScopeRequest)
        clearHistoryScope();
    }
  }
  let info = $state<AppInfo | null>(null);
  let snapshot = $state<EngineSnapshot | null>(null);
  let browser = $state(false);
  let loading = $state(true);
  let busy = $state(false);
  let error = $state<MessageValue | null>(null);
  let notificationPermission = $state<NotificationPermission | null>(null);
  let notificationsEnabled = $state<boolean | null>(null);
  let notificationRequest = 0;
  const notificationHint = $derived(
    notificationsEnabled === true &&
      (notificationPermission === "denied" ||
        notificationPermission === "notDetermined"),
  );
  let lastNativeError: string | null = null;
  let dismissedFailure = $state<string | null>(null);
  let dismissedTransfer = $state<string | null>(null);
  const currentCall = $derived(
    snapshot?.calls.find((call) => call.id === snapshot?.callId) ?? null,
  );
  const conversationKey = $derived(
    snapshot?.calls.find((call) => !call.consultation)?.id ??
      currentCall?.id ??
      "idle",
  );
  const callFailure = $derived(snapshot?.lastCallFailure ?? null);
  const failureKey = $derived(callFailure ? JSON.stringify(callFailure) : null);
  const failureOwner = $derived(
    snapshot?.accounts.find((account) => account.id === callFailure?.accountId)
      ?.name,
  );
  const transferSuccess = $derived(
    snapshot?.call === "idle"
      ? snapshot.callEvents
          .slice()
          .reverse()
          .find((event) => event.event.kind === "transferSucceeded")
      : null,
  );
  const transferSuccessKey = $derived(
    transferSuccess
      ? `${transferSuccess.callId}:${transferSuccess.sequence}`
      : null,
  );
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
  let destination = $state("");
  let dialContacts = $state<ContactsSnapshot | null>(null);
  let contactsPreferences = $state<ContactsPreferences>({
    favoritesFirst: true,
    showOrganization: true,
    matchGermanNumbers: true,
  });
  const matchGermanNumbers = $derived(
    contactsPreferences.matchGermanNumbers ?? true,
  );
  let dialContactsRequest = 0;
  async function refreshDialContacts(force = false) {
    if (browser) return;
    const request = ++dialContactsRequest;
    try {
      const next = await (force ? refreshContacts() : getContacts());
      if (mounted && request === dialContactsRequest) {
        dialContacts =
          next.permission === "authorized" ? next : { ...next, contacts: [] };
        if (next.permission !== "authorized") clearDisplayedContacts();
        else if (snapshot?.calls.length)
          void getEngineSnapshot()
            .then((state) => {
              if (mounted && request === dialContactsRequest)
                applySnapshot(state);
            })
            .catch(() => {
              /* SIP read errors do not invalidate contact authorization. */
            });
      }
    } catch {
      if (mounted && request === dialContactsRequest) {
        dialContacts = null;
        clearDisplayedContacts();
      }
    }
    if (!mounted || request !== dialContactsRequest) return;
    try {
      const next = await getPreferences();
      if (mounted && request === dialContactsRequest)
        contactsPreferences = next.contacts;
    } catch {
      /* Existing preference/read errors never prevent number dialing. */
    }
  }
  function clearDisplayedContacts() {
    if (snapshot)
      snapshot = {
        ...snapshot,
        callerContact: null,
        calls: snapshot.calls.map((call) => ({ ...call, contact: null })),
      };
  }
  function contactsChanged() {
    dialContacts = null;
    clearDisplayedContacts();
    void refreshDialContacts();
    void refreshHistoryScope();
  }
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
      selectedAccount?.enabled !== false &&
      callIdle &&
      normalizeDialNumber(destination) !== null,
  );
  const elapsed = $derived(
    connectedAt ? Math.max(0, Math.floor((now - connectedAt) / 1000)) : 0,
  );
  const elapsedLabel = $derived(
    `${Math.floor(elapsed / 60)
      .toString()
      .padStart(2, "0")}:${(elapsed % 60).toString().padStart(2, "0")}`,
  );
  const registrationLabels = $derived<Record<RegistrationState, string>>({
    unconfigured: $t("app.not_registered"),
    registering: $t("app.registering"),
    registered: $t("app.registered"),
    failed: $t("app.registration_failed"),
  });
  const availability = $derived.by((): string => {
    if (loading) return $t("app.loading");
    if (browser) return $t("app.browser_preview");
    if (info?.telephony === "notLinked") return $t("app.not_linked");
    if (!snapshot) return $t("app.status_unavailable");
    if (selectedAccount?.enabled === false) return $t("app.disabled_account");
    return registrationLabels[registration];
  });
  const tabs = $derived([
    { id: "dialer", label: $t("app.keypad"), icon: "keypad" },
    { id: "contacts", label: $t("app.contacts"), icon: "contacts" },
    { id: "history", label: $t("app.history"), icon: "history" },
  ] as const);

  const introRuntime = $derived.by(() => {
    if (browser) return "browser";
    if (info?.telephony === "notLinked") return "unlinked";
    return nativeAvailable ? "desktop" : "unavailable";
  });
  const setupLabel = $derived.by((): string => {
    if (!error && !browser && !snapshot && info?.telephony !== "notLinked")
      return $t("app.reload");
    if (browser || !nativeAvailable) return $t("app.start");
    if (selectedAccount?.enabled === false) return $t("app.enable_account");
    if (registration === "failed") return $t("app.check_connection");
    if (registration === "registering") return $t("app.registration_details");
    return $t("app.connect_account");
  });
  const connectionHint = $derived.by((): string => {
    if (audioUnknown) return $t("app.audio_unknown");
    if (sessionAudio === "testTone") return $t("app.test_audio");
    if (firstCallHint) return $t("app.ready_to_dial");
    return $t("app.dial_hint");
  });

  function markIntroduction() {
    if (introductionSeen) return;
    introductionSeen = true;
    if (introductionStorage) rememberIntroduction(introductionStorage);
  }
  function selectView(next: View) {
    searchQuery = null;
    historyEntryFocusId = null;
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
    const selected = next.calls.find((call) => call.id === next.callId);
    const sameError = (other: unknown) =>
      other && JSON.stringify(other) === JSON.stringify(next.lastError);
    const nextError =
      next.lastError &&
      !sameError(selected?.lastError) &&
      !sameError(next.lastCallFailure?.error)
        ? `${next.lastError.code}:${next.lastError.sipStatus}:${next.lastError.message}`
        : null;
    if (
      nextError &&
      nextError !== lastNativeError &&
      next.registration !== "failed"
    )
      error = errorMessage(next.lastError);
    lastNativeError = nextError;
    snapshot =
      dialContacts?.permission === "authorized"
        ? next
        : {
            ...next,
            callerContact: null,
            calls: next.calls.map((call) => ({ ...call, contact: null })),
          };
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
  function refreshNotificationStatus() {
    if (!mounted || browser || document.visibilityState === "hidden") return;
    const request = ++notificationRequest;
    // OS status remains readable even if unrelated account metadata is broken.
    void getNotificationPermission()
      .then((permission) => {
        if (mounted && request === notificationRequest)
          notificationPermission = permission;
      })
      .catch(() => {
        if (mounted && request === notificationRequest)
          notificationPermission = null;
      });
    void getDesktopSettings()
      .then((settings) => {
        if (mounted && request === notificationRequest)
          notificationsEnabled = settings.notificationsEnabled;
      })
      .catch(() => {
        if (mounted && request === notificationRequest)
          notificationsEnabled = null;
      });
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
    refreshNotificationStatus();
    let polling = false;
    if (browser) {
      loading = false;
      if (!introductionSeen) view = "start";
    }
    const actionUnlisteners: UnlistenFn[] = [];
    if (!browser) {
      for (const action of phoneActions.filter(
        (action) =>
          action.id !== "settings" &&
          action.id !== "answer" &&
          action.id !== "end",
      )) {
        void listen<string | null>(
          `phone-${action.id}-requested`,
          ({ payload }) => {
            if (!mounted || searchOpen) return;
            if (isCallAction(action.id) && payload !== currentCall?.id) return;
            void runAction(action.id);
          },
        )
          .then((unlisten) => {
            if (mounted) actionUnlisteners.push(unlisten);
            else unlisten();
          })
          .catch(() => {
            /* Browser keyboard fallbacks remain separate from native accelerators. */
          });
      }
      void listen("phone-start-requested", () => {
        if (mounted) void runAction("start");
      })
        .then((unlisten) => {
          if (mounted) actionUnlisteners.push(unlisten);
          else unlisten();
        })
        .catch(() => {});
    }
    let unlistenState: UnlistenFn | undefined;
    let unlistenContacts: UnlistenFn | undefined;
    let contactsReady = false;
    if (!browser)
      void listen("contacts-changed", contactsChanged)
        .then((unlisten) => {
          if (!mounted) unlisten();
          else {
            unlistenContacts = unlisten;
            contactsReady = true;
            void refreshDialContacts();
          }
        })
        .catch(() => {
          contactsReady = true;
          void refreshDialContacts();
          /* Focus restoration revalidates if event subscription failed. */
        });
    const contactFocus = () => {
      if (!browser) {
        void refreshHistoryScope();
        if (contactsReady) void refreshDialContacts(true);
      }
      refreshNotificationStatus();
    };
    window.addEventListener("focus", contactFocus);
    document.addEventListener("visibilitychange", refreshNotificationStatus);
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
    const contactsTimer = setInterval(() => {
      if (!browser && document.visibilityState !== "hidden")
        void refreshDialContacts();
    }, 30000);
    const clock = setInterval(() => {
      now = Date.now();
    }, 1000);
    document.addEventListener("visibilitychange", poll);
    return () => {
      mounted = false;
      ++notificationRequest;
      ++dialContactsRequest;
      clearInterval(timer);
      clearInterval(contactsTimer);
      clearInterval(clock);
      unlistenState?.();
      actionUnlisteners.forEach((unlisten) => unlisten());
      unlistenContacts?.();
      window.removeEventListener("focus", contactFocus);
      document.removeEventListener(
        "visibilitychange",
        refreshNotificationStatus,
      );
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
  async function showSettings(section?: SettingsSection) {
    if (openingSettings) return;
    openingSettings = true;
    try {
      if (browser)
        window.open(
          `?window=settings${section ? `&section=${section}` : ""}`,
          "sip-settings",
          "width=900,height=660",
        );
      else await openSettings(section);
      markIntroduction();
    } catch (reason) {
      error = errorMessage(reason);
    } finally {
      openingSettings = false;
    }
  }
  let queuedTones = 0;
  let toneRequests = Promise.resolve();
  function sendTone(callId: string, digit: string) {
    if (queuedTones >= 32) {
      error = message("app.tone_queue_busy");
      return;
    }
    queuedTones++;
    toneRequests = toneRequests
      .then(async () => {
        if (!mounted || snapshot?.callId !== callId) return;
        try {
          const next = await sendDtmf(callId, digit);
          if (mounted) applySnapshot(next);
        } catch (reason) {
          if (mounted && snapshot?.callId === callId)
            error = errorMessage(reason);
        }
      })
      .finally(() => {
        queuedTones--;
      });
  }
  async function placeCall() {
    if (!canCall || busy) return;
    const target = normalizeDialNumber(destination);
    if (!target) return;
    if (await perform(() => dial(target))) {
      firstCallHint = false;
    }
  }
  async function placeContactCall(target: string) {
    if (!nativeAvailable || audioUnknown || !registered || !callIdle || busy)
      return;
    destination = target;
    await placeCall();
  }
  function keyboard(event: KeyboardEvent) {
    if (event.defaultPrevented || event.isComposing || searchOpen) return;
    const action = keyboardAction(event);
    // Native accelerators are dispatched once by the menu, including from Settings.
    if (browser && action) {
      event.preventDefault();
      void runAction(action);
    }
    if (event.key === "Escape" && view === "start") {
      event.preventDefault();
      selectView("dialer");
    }
  }
</script>

<svelte:window onkeydown={keyboard} />

{#if searchOpen}<ActionSearch
    {actions}
    contacts={dialContacts}
    preferences={contactsPreferences}
    oncontact={(contact) => showHistoryContact(contact)}
    onhistory={openHistoryResult}
    onsearchview={openSearchView}
    onaction={(id) => void runAction(id)}
    onclose={() => (searchOpen = false)}
  />{/if}

<div class="softphone" aria-label={$t("app.softphone")}>
  <header class="app-header">
    <div class="app-brand">
      <img src="/ruf.svg" alt="" width="32" height="32" />
      <div class="identity">
        {#if (snapshot?.accounts.length ?? 0) > 1}<label
            class="sr-only"
            for="outgoing-account">{$t("app.outgoing_account")}</label
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
        aria-label={$t("app.search")}
        title={`${$t("app.search")} (${shortcutLabel(
          phoneActions.find((action) => action.id === "search")!,
          $locale,
        )})`}
        onclick={() => (searchOpen = true)}
        ><Icon name="search" size={16} /></button
      >
      <button
        class="icon-button"
        type="button"
        aria-label={$t("app.getting_started")}
        title={$t("app.getting_started")}
        aria-pressed={view === "start"}
        onclick={() => selectView(view === "start" ? "dialer" : "start")}
        ><Icon name="info" size={16} /></button
      >
      <button
        class="icon-button settings-button"
        type="button"
        aria-label={$t("app.settings")}
        title={$t("app.settings_tooltip", {
          shortcut: shortcutLabel(
            phoneActions.find((action) => action.id === "settings")!,
            $locale,
          ),
        })}
        disabled={openingSettings}
        onclick={() => {
          void showSettings();
        }}><Icon name="settings" size={16} /></button
      >
    </div>
  </header>

  {#if notificationHint}
    <div class="notification-hint" role="status">
      <p>{$t("app.notifications_unavailable")}</p>
      <button
        class="text-button"
        type="button"
        disabled={openingSettings}
        onclick={() => void showSettings("notifications")}
        >{$t("app.check_notifications")}</button
      >
    </div>
  {/if}

  {#if snapshot?.historyError}<p class="error-notice" role="alert">
      {$t("app.history_save_error")}
    </p>{/if}
  {#if error}<div class="error-notice">
      <div class="error-copy">
        <p role="alert">{$text(error)}</p>
        {#if (!info || !snapshot) && !browser}<button
            class="text-button"
            type="button"
            disabled={loading || busy}
            onclick={() => {
              void refreshNativeState();
            }}>{$t("app.reload")}</button
          >{:else if registration === "failed"}<button
            class="text-button"
            type="button"
            onclick={() => {
              void showSettings();
            }}>{$t("app.check_connection")}</button
          >{/if}
      </div>
      <button
        class="icon-button"
        type="button"
        aria-label={$t("app.dismiss_error")}
        onclick={() => (error = null)}><Icon name="close" size={15} /></button
      >
    </div>{/if}

  {#if callFailure && failureKey !== dismissedFailure}
    <div class="error-notice owned-call-failure">
      <p role="alert">
        {$t("app.call_error")}{failureOwner ? ` · ${failureOwner}` : ""}: {$text(
          errorMessage(callFailure.error),
        )}
      </p>
      <button
        class="icon-button"
        type="button"
        aria-label={$t("app.dismiss_call_error")}
        onclick={() => (dismissedFailure = failureKey)}
        ><Icon name="close" size={15} /></button
      >
    </div>
  {/if}
  {#if transferSuccessKey && transferSuccessKey !== dismissedTransfer}
    <div class="transfer-success" role="status">
      <p>{$t("app.transfer_confirmed")}</p>
      <button
        class="icon-button"
        type="button"
        aria-label={$t("app.dismiss_transfer")}
        onclick={() => (dismissedTransfer = transferSuccessKey)}
        ><Icon name="close" size={15} /></button
      >
    </div>
  {/if}

  <main class="phone-content" class:in-call={!callIdle}>
    {#if !callIdle}
      {#if currentCall}{#key conversationKey}
          <CallPanel
            bind:this={callPanel}
            call={currentCall}
            calls={snapshot?.calls ?? []}
            callerContact={snapshot?.callerContact ?? null}
            {matchGermanNumbers}
            accounts={snapshot?.accounts ?? []}
            elapsed={elapsedLabel}
            receivedDtmf={receivedDtmf?.kind === "dtmfStart"
              ? receivedDtmf.digit
              : null}
            {busy}
            testAudio={sessionAudio === "testTone"}
            {audioUnknown}
            compact={view !== "dialer"}
            onanswer={(id) => {
              if (!audioUnknown) void perform(() => answer(id));
            }}
            onhangup={(id) => {
              const end = currentCall?.state === "incoming" ? reject : hangup;
              void perform(() => end(id));
            }}
            onsenddtmf={sendTone}
            onmute={(id, muted) => perform(() => setCallMuted(id, muted))}
            onhold={(id, held) => void perform(() => setCallHeld(id, held))}
            ontransfer={(id, target) =>
              void perform(() => transferCall(id, target))}
            onconsult={(id, target) =>
              void perform(() => startConsultation(id, target))}
            onswitch={(id) => void perform(() => switchCall(id))}
            oncancelconsult={(id) => void perform(() => cancelConsultation(id))}
            oncompletetransfer={(id) =>
              void perform(() => completeTransfer(id))}
          />
        {/key}{/if}
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
          contacts={dialContacts?.permission === "authorized"
            ? dialContacts.contacts
            : []}
          {matchGermanNumbers}
          contactsHint={dialContacts?.permission === "authorized"
            ? null
            : dialContacts?.permission === "unsupported"
              ? $t("app.contacts_unsupported")
              : dialContacts?.permission === "restricted"
                ? $t("app.contacts_restricted")
                : dialContacts
                  ? $t("app.contacts_after_permission")
                  : $t("app.contacts_unavailable")}
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
        focusSearchRequest={view === "contacts" ? searchFocusRequest : 0}
        {searchQuery}
        focusId={contactFocusId}
        onhistory={showContactHistory}
        canDial={nativeAvailable &&
          !audioUnknown &&
          registered &&
          callIdle &&
          !busy}
        callHint={!nativeAvailable
          ? $t("app.calling_unavailable")
          : audioUnknown
            ? $t("app.audio_unknown")
            : !callIdle
              ? $t("app.end_before_contact")
              : !registered
                ? $t("app.connect_before_contact")
                : $t("app.phone_busy")}
        oncall={(target) => {
          void placeContactCall(target);
        }}
      />
    {:else}
      <HistoryPanel
        focusSearchRequest={view === "history" ? searchFocusRequest : 0}
        {searchQuery}
        focusEntryId={historyEntryFocusId}
        canDial={nativeAvailable && !audioUnknown && callIdle && !busy}
        oncallback={(id) => void returnCall(id)}
        oncontact={showHistoryContact}
        contactScope={historyScope}
        onclearscope={clearHistoryScope}
      />
    {/if}
  </main>

  <nav class="bottom-nav" aria-label={$t("app.main_navigation")}>
    {#each tabs as tab}<button
        type="button"
        class:active={view === tab.id}
        title={`${tab.label} (${shortcutLabel(
          phoneActions.find((action) => action.id === tab.id)!,
          $locale,
        )})`}
        aria-current={view === tab.id ? "page" : undefined}
        onclick={() => selectView(tab.id)}
        ><Icon name={tab.icon} size={16} /><span>{tab.label}</span></button
      >{/each}
  </nav>
</div>

<style>
  .notification-hint {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 2px 10px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.45;
  }
  .notification-hint p {
    margin: 0;
  }
  .notification-hint button {
    font-size: 11px;
  }
</style>
