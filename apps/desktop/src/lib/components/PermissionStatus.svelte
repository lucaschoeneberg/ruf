<script lang="ts">
  import { t, text, message, type MessageValue } from "../i18n";
  import type {
    ContactsPermission,
    NotificationPermission,
  } from "../generated/AppInfo";
  let {
    title,
    status,
    checking = false,
    working = false,
    disabled = false,
    error = null,
    requestLabel = message("settings.request_access"),
    oncheck,
    onrequest,
    onopen,
  }: {
    title: string;
    status: ContactsPermission | NotificationPermission | null;
    checking?: boolean;
    working?: boolean;
    disabled?: boolean;
    error?: MessageValue | null;
    requestLabel?: MessageValue;
    oncheck: () => void;
    onrequest?: (() => void) | undefined;
    onopen?: (() => void) | undefined;
  } = $props();
  const labels = {
    authorized: message("settings.permission_allowed"),
    notDetermined: message("settings.permission_not_requested"),
    denied: message("settings.permission_blocked"),
    restricted: message("settings.permission_restricted"),
    unsupported: message("settings.unavailable"),
  };
  const statusLabel = $derived.by((): MessageValue => {
    if (checking) return message("settings.permission_checking");
    if (error) return message("settings.permission_check_failed");
    return status ? labels[status] : message("settings.permission_unchecked");
  });
</script>

<div class="setting-row permission-row">
  <div class="setting-description">
    <h3>{title}</h3>
    <p
      class:warning={!!error || status === "denied" || status === "restricted"}
      class="permission-state"
      role="status"
      aria-live="polite"
    >
      {$text(statusLabel)}
    </p>
    {#if error}<p class="field-error" role="alert">{$text(error)}</p>{/if}
  </div>
  <div class="permission-actions">
    {#if status === "notDetermined" && !error && onrequest}<button
        type="button"
        class="primary-button"
        disabled={disabled || checking || working}
        onclick={onrequest}
        >{working
          ? $t("settings.confirm_system_prompt")
          : $text(requestLabel)}</button
      >{/if}
    {#if status && status !== "unsupported" && onopen}<button
        type="button"
        class="secondary-button"
        disabled={disabled || working}
        onclick={onopen}>{$t("settings.open_system_settings")}</button
      >{/if}
    <button
      type="button"
      class="text-button"
      disabled={disabled || checking || working}
      onclick={oncheck}
      >{checking
        ? $t("settings.checking")
        : $t("settings.check_permission_again")}</button
    >
  </div>
</div>

<style>
  .permission-state {
    color: var(--text);
    font-weight: 600;
    margin-top: 8px;
  }
  .permission-state.warning {
    color: var(--caution);
  }
  .permission-actions {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    min-width: 0;
  }
  .permission-actions button {
    max-width: 100%;
    text-align: left;
  }
</style>
