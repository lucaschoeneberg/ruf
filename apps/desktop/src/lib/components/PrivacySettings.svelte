<script lang="ts">
  import { t, text, message, type MessageValue } from "../i18n";
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import type { MicrophonePermission } from "../generated/AppInfo";
  import {
    getMicrophonePermission,
    requestMicrophonePermission,
    openMicrophoneSettings,
  } from "../api";
  import ContactsSettings from "./ContactsSettings.svelte";
  import PermissionStatus from "./PermissionStatus.svelte";
  let status = $state<MicrophonePermission | null>(null),
    permissionError = $state<MessageValue | null>(null),
    actionError = $state<MessageValue | null>(null);
  let checking = $state(false),
    requesting = $state(false),
    mounted = false,
    revision = 0;
  async function check() {
    if (!mounted || checking || requesting) return;
    const current = ++revision;
    checking = true;
    permissionError = null;
    try {
      const result = isTauri()
        ? await getMicrophonePermission()
        : "unsupported";
      if (mounted && current === revision) status = result;
    } catch {
      if (mounted && current === revision) {
        status = null;
        permissionError = message("settings.microphone_check_failed");
      }
    } finally {
      if (mounted && current === revision) {
        checking = false;
      }
    }
  }
  async function request() {
    if (!mounted || checking || requesting || status !== "notDetermined")
      return;
    requesting = true;
    actionError = null;
    let requestFailed = false;
    try {
      await requestMicrophonePermission();
    } catch {
      requestFailed = true;
    } finally {
      if (mounted) {
        requesting = false;
        await check();
        if (requestFailed && !permissionError)
          actionError = message("settings.microphone_request_failed");
      }
    }
  }
  async function open() {
    actionError = null;
    try {
      await openMicrophoneSettings();
    } catch {
      if (mounted) actionError = message("settings.microphone_settings_failed");
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
    <h1>{$t("settings.privacy")}</h1>
    <p>{$t("settings.privacy_description")}</p>
  </div>
</div>
<h2 class="settings-group" id="privacy-permissions" tabindex="-1">
  {$t("settings.microphone")}
</h2>
<PermissionStatus
  title={$t("settings.microphone_calls_tests")}
  {status}
  {checking}
  working={requesting}
  error={$text(permissionError)}
  requestLabel={$t("settings.allow_microphone")}
  oncheck={() => void check()}
  onrequest={() => void request()}
  onopen={isTauri() ? () => void open() : undefined}
/>
{#if actionError}<p class="settings-feedback error" role="alert">
    {$text(actionError)}
  </p>{/if}
<p class="settings-footnote">
  {$t("settings.microphone_access_help")}
</p>
<ContactsSettings embedded />
<h2 class="settings-group sound-group">{$t("settings.local_data")}</h2>
<div class="setting-row">
  <div class="setting-description">
    <h3>{$t("settings.accounts_passwords")}</h3>
    <p>
      {$t("settings.accounts_storage_help")}
    </p>
  </div>
</div>
<div class="setting-row">
  <div class="setting-description">
    <h3>{$t("settings.contacts_history")}</h3>
    <p>
      {$t("settings.contacts_storage_help")}
    </p>
  </div>
</div>
<div class="setting-row">
  <div class="setting-description">
    <h3>{$t("settings.diagnostics")}</h3>
    <p>
      {$t("settings.diagnostics_privacy_help")}
    </p>
  </div>
</div>
