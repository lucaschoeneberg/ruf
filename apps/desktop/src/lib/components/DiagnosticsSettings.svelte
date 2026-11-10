<script lang="ts">
  import { t, text, message, locale, type MessageValue } from "../i18n";
  import { phoneActions, shortcutLabel } from "../shortcuts";
  import { onMount } from "svelte";
  import type { DiagnosticsReport } from "../generated/AppInfo";
  import { getDiagnostics, copyDiagnostics } from "../api";
  let report = $state<DiagnosticsReport | null>(null),
    loading = $state(false),
    error = $state<MessageValue>(""),
    feedback = $state<MessageValue>("");
  let mounted = false;
  async function refresh() {
    if (loading) return;
    loading = true;
    error = "";
    try {
      const value = await getDiagnostics();
      if (mounted) report = value;
    } catch {
      if (mounted) error = message("settings.diagnostics_load_failed");
    } finally {
      if (mounted) loading = false;
    }
  }
  async function copy() {
    if (!report) return;
    try {
      try {
        await navigator.clipboard.writeText(
          JSON.stringify($state.snapshot(report)),
        );
      } catch {
        await copyDiagnostics();
      }
      if (mounted) feedback = message("settings.diagnostics_copied");
    } catch {
      if (mounted) error = message("settings.diagnostics_copy_failed");
    }
  }
  onMount(() => {
    mounted = true;
    void refresh();
    return () => {
      mounted = false;
    };
  });
</script>

<div class="settings-title">
  <div>
    <h1>{$t("settings.diagnostics_help")}</h1>
    <p>{$t("settings.diagnostics_description")}</p>
  </div>
</div>
<h2 class="settings-group" id="diagnostics-status" tabindex="-1">
  {$t("settings.current_status")}
</h2>
{#if loading}<p role="status">{$t("settings.diagnostics_loading")}</p>{/if}
{#if error}<p class="settings-feedback error" role="alert">
    {$text(error)}
  </p>{/if}
{#if report}
  <div class="setting-row">
    <div class="setting-description">
      <h3>{$t("settings.telephony")}</h3>
      <p>
        {report.telephony === "linked"
          ? $t("settings.native_build")
          : $t("settings.no_native_build")}
      </p>
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <h3>{$t("settings.accounts")}</h3>
      <p>
        {report.accountsRegistered === null || report.accountsTotal === null
          ? $t("settings.account_status_unavailable")
          : $t("settings.accounts_registered", {
              count: report.accountsRegistered,
              total: report.accountsTotal,
            })}
      </p>
      <p>
        {$t("settings.registration_limit")}
      </p>
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <h3>{$t("settings.system_permissions")}</h3>
      <p>
        {$t("settings.notifications_prefix")}
        {report.notificationPermission === null
          ? $t("settings.permission_check_failed")
          : report.notificationPermission === "authorized"
            ? $t("settings.permission_allowed")
            : report.notificationPermission === "denied"
              ? $t("settings.permission_blocked")
              : report.notificationPermission === "notDetermined"
                ? $t("settings.permission_not_requested")
                : $t("settings.unavailable")}.
      </p>
      <p>
        {$t("settings.microphone_prefix")}
        {report.microphonePermission === null
          ? $t("settings.permission_check_failed")
          : report.microphonePermission === "authorized"
            ? $t("settings.permission_allowed")
            : report.microphonePermission === "denied"
              ? $t("settings.permission_blocked")
              : report.microphonePermission === "restricted"
                ? $t("settings.permission_restricted")
                : report.microphonePermission === "notDetermined"
                  ? $t("settings.permission_not_requested")
                  : $t("settings.unavailable")}.
      </p>
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <h3>{$t("settings.audio_devices")}</h3>
      <p>
        {report.audioDevicesAvailable === null
          ? $t("settings.device_list_failed")
          : report.audioDevicesAvailable
            ? $t("settings.devices_available")
            : $t("settings.devices_missing")}
      </p>
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <h3>{$t("settings.keychain")}</h3>
      <p>
        {report.credentialError === "credentialAccessRequired"
          ? $t("settings.keychain_permission_needed")
          : report.credentialError === "credentialMissing"
            ? $t("settings.keychain_password_missing")
            : report.credentialError === "credentialFailure"
              ? $t("settings.keychain_access_failed")
              : $t("settings.keychain_no_error")}
      </p>
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <h3>{$t("settings.history")}</h3>
      <p>
        {report.historyAvailable
          ? $t("settings.history_no_error")
          : $t("settings.history_storage_error")}
      </p>
    </div>
  </div>
  {#if report.lastError}<p class="settings-feedback" role="status">
      {$t("settings.reported_error")}
      {report.lastError}
    </p>{/if}
  <details class="settings-lab">
    <summary>{$t("settings.view_diagnostics")}</summary>
    <pre>{JSON.stringify(report, null, 2)}</pre>
  </details>
{/if}
<div class="settings-form-actions">
  <button
    class="secondary-button"
    type="button"
    disabled={loading}
    onclick={() => void refresh()}>{$t("settings.check_status_again")}</button
  ><button
    class="secondary-button"
    type="button"
    disabled={!report || loading}
    onclick={() => void copy()}>{$t("settings.copy_diagnostics")}</button
  >
</div>
{#if feedback}<p class="settings-feedback" role="status">
    {$text(feedback)}
  </p>{/if}
<h2 class="settings-group sound-group">{$t("settings.troubleshooting")}</h2>
<div class="setting-row">
  <div class="setting-description">
    <h3>{$t("settings.account_unregistered")}</h3>
    <p>
      {$t("settings.account_unregistered_help")}
    </p>
  </div>
</div>
<div class="setting-row">
  <div class="setting-description">
    <h3>{$t("settings.no_notification_sound")}</h3>
    <p>
      {$t("settings.no_notification_sound_help")}
    </p>
  </div>
</div>
<h2 id="keyboard-shortcuts" class="settings-group sound-group">
  {$t("settings.keyboard_shortcuts")}
</h2>
<p class="shortcut-note">
  {$t("settings.shortcuts_help")}
</p>
<dl class="shortcut-list">
  {#each phoneActions as action}
    <div>
      <dt>{$text(action.label)}</dt>
      <dd><kbd>{shortcutLabel(action, $locale)}</kbd></dd>
    </div>
  {/each}
  <div>
    <dt>{$t("settings.dialpad_enter")}</dt>
    <dd><kbd>Enter</kbd></dd>
  </div>
  <div>
    <dt>{$t("settings.close_search")}</dt>
    <dd><kbd>Esc</kbd></dd>
  </div>
</dl>

<style>
  .shortcut-note {
    color: var(--muted);
    max-width: 60ch;
  }
  .shortcut-list {
    margin: 12px 0 0;
  }
  .shortcut-list > div {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
  }
  dt {
    color: var(--text-secondary);
  }
  dd {
    flex-shrink: 0;
    margin: 0;
  }
  kbd {
    font: inherit;
    font-size: 12px;
    color: var(--text);
  }

  pre {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font-size: 12px;
  }
</style>
