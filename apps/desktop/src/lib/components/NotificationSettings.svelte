<script lang="ts">
  import { t, text, message, type MessageValue } from "../i18n";
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import type { NotificationPermission } from "../generated/AppInfo";
  import {
    getNotificationPermission,
    requestNotificationPermission,
    openNotificationSettings,
    testNotification,
  } from "../api";
  import PermissionStatus from "./PermissionStatus.svelte";
  let {
    enabled = $bindable(),
    showCaller = $bindable(),
    dirty,
    busy,
    canSave,
    onsave,
    ondiscard,
  }: {
    enabled: boolean;
    showCaller: boolean;
    dirty: boolean;
    busy: boolean;
    canSave: boolean;
    onsave: () => void;
    ondiscard: () => void;
  } = $props();
  let permission = $state<NotificationPermission | null>(null);
  let checking = $state(false),
    requesting = $state(false),
    testing = $state(false);
  let permissionError = $state<MessageValue | null>(null),
    actionError = $state<MessageValue | null>(null),
    testError = $state<MessageValue | null>(null),
    feedback = $state<MessageValue>("");
  let mounted = false,
    revision = 0;
  async function check() {
    if (!mounted || checking || requesting) return;
    const current = ++revision;
    checking = true;
    permissionError = null;
    try {
      const value = isTauri()
        ? await getNotificationPermission()
        : "unsupported";
      if (mounted && current === revision) permission = value;
    } catch {
      if (mounted && current === revision) {
        permission = null;
        permissionError = message("settings.notifications_check_failed");
      }
    } finally {
      if (mounted && current === revision) checking = false;
    }
  }
  async function request() {
    if (!mounted || requesting || checking || permission !== "notDetermined")
      return;
    revision++;
    requesting = true;
    actionError = null;
    feedback = "";
    let requestFailed = false;
    try {
      await requestNotificationPermission();
    } catch {
      requestFailed = true;
    } finally {
      if (mounted) {
        requesting = false;
        await check();
        if ((permission as NotificationPermission | null) === "authorized")
          feedback = message("settings.notifications_granted_feedback");
        else if (requestFailed && !permissionError)
          actionError = message("settings.notifications_request_failed");
      }
    }
  }
  async function open() {
    actionError = null;
    try {
      await openNotificationSettings();
    } catch {
      if (mounted)
        actionError = message("settings.notifications_settings_failed");
    }
  }
  async function test() {
    if (permission !== "authorized" || testing) return;
    testing = true;
    testError = null;
    feedback = message("settings.test_notification_wait");
    try {
      await testNotification();
      if (mounted) feedback = message("settings.test_notification_requested");
    } catch {
      if (mounted) {
        feedback = "";
        testError = message("settings.test_notification_failed");
      }
    } finally {
      if (mounted) testing = false;
    }
  }
  onMount(() => {
    mounted = true;
    void check();
    const focus = () => {
      if (document.visibilityState !== "hidden") void check();
    };
    window.addEventListener("focus", focus);
    document.addEventListener("visibilitychange", focus);
    return () => {
      mounted = false;
      revision++;
      window.removeEventListener("focus", focus);
      document.removeEventListener("visibilitychange", focus);
    };
  });
</script>

<div class="settings-title">
  <div>
    <h1>{$t("settings.notifications")}</h1>
    <p>{$t("settings.notifications_description")}</p>
  </div>
</div>
<h2 class="settings-group">{$t("settings.in_ruf")}</h2>
<fieldset disabled={busy || !canSave}>
  <div class="setting-row">
    <div class="setting-description">
      <label for="notifications-enabled"
        >{$t("settings.show_incoming_calls")}</label
      >
      <p id="notifications-help">
        {$t("settings.show_incoming_calls_help")}
      </p>
    </div>
    <div class="setting-control sound-toggle">
      <input
        id="notifications-enabled"
        type="checkbox"
        bind:checked={enabled}
        aria-describedby="notifications-help"
      /><span aria-hidden="true"
        >{enabled ? $t("settings.on") : $t("settings.off")}</span
      >
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <label for="notification-caller">{$t("settings.show_caller")}</label>
      <p>
        {$t("settings.show_caller_help")}
      </p>
    </div>
    <div class="setting-control sound-toggle">
      <input
        id="notification-caller"
        type="checkbox"
        bind:checked={showCaller}
      /><span aria-hidden="true"
        >{showCaller ? $t("settings.on") : $t("settings.off")}</span
      >
    </div>
  </div>
</fieldset>
<h2 class="settings-group" id="notification-permission" tabindex="-1">
  {$t("settings.system_permission")}
</h2>
<PermissionStatus
  title={$t("settings.notifications_for_ruf")}
  status={permission}
  {checking}
  working={requesting}
  error={$text(permissionError)}
  requestLabel={$t("settings.allow_notifications")}
  oncheck={() => void check()}
  onrequest={() => void request()}
  onopen={isTauri() ? () => void open() : undefined}
/>
{#if actionError}<p class="settings-feedback error" role="alert">
    {$text(actionError)}
  </p>{/if}
{#if permission === "denied"}<p class="settings-feedback">
    {$t("settings.notifications_denied")}
  </p>{:else if permission === "authorized"}<p class="settings-footnote">
    {$t("settings.notifications_allowed")}
  </p>{:else if permission === "unsupported"}<p class="settings-footnote">
    {$t("settings.notifications_unavailable")}
  </p>{/if}
<div class="setting-row">
  <div class="setting-description">
    <h3>{$t("settings.test_notification")}</h3>
    <p>
      {$t("settings.test_notification_help")}
    </p>
  </div>
  <button
    id="test-notification"
    type="button"
    class="secondary-button"
    disabled={permission !== "authorized" || testing || requesting}
    onclick={() => void test()}
    >{testing
      ? $t("settings.test_notification_requesting")
      : $t("settings.send_test_notification")}</button
  >
</div>
{#if testError}<p class="settings-feedback error" role="alert">
    {$text(testError)}
  </p>{/if}
{#if feedback}<p role="status" class="settings-feedback">
    {$text(feedback)}
  </p>{/if}
<div class="settings-form-actions settings-save-bar">
  <p>
    {dirty ? $t("settings.unsaved_notice") : $t("settings.this_computer_scope")}
  </p>
  <div class="settings-actions">
    <button
      type="button"
      class="text-button"
      disabled={!dirty || busy}
      onclick={ondiscard}>{$t("settings.discard")}</button
    ><button
      type="button"
      class="primary-button"
      disabled={!canSave || !dirty || busy}
      onclick={onsave}
      >{busy
        ? $t("settings.saving")
        : $t("settings.save_notifications")}</button
    >
  </div>
</div>
