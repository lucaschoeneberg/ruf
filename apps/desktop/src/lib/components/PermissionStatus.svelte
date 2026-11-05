<script lang="ts">
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
    requestLabel = "Zugriff anfordern",
    oncheck,
    onrequest,
    onopen,
  }: {
    title: string;
    status: ContactsPermission | NotificationPermission | null;
    checking?: boolean;
    working?: boolean;
    disabled?: boolean;
    error?: string | null;
    requestLabel?: string;
    oncheck: () => void;
    onrequest?: (() => void) | undefined;
    onopen?: (() => void) | undefined;
  } = $props();
  const labels = {
    authorized: "Erlaubt",
    notDetermined: "Noch nicht angefragt",
    denied: "Im System blockiert",
    restricted: "Durch Systemrichtlinie eingeschränkt",
    unsupported: "Nicht verfügbar",
  };
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
      {checking
        ? "Freigabe wird geprüft …"
        : error
          ? "Prüfung fehlgeschlagen"
          : status
            ? labels[status]
            : "Noch nicht geprüft"}
    </p>
    {#if error}<p class="field-error" role="alert">{error}</p>{/if}
  </div>
  <div class="permission-actions">
    {#if status === "notDetermined" && !error && onrequest}<button
        type="button"
        class="primary-button"
        disabled={disabled || checking || working}
        onclick={onrequest}
        >{working ? "Bitte Systemabfrage bestätigen …" : requestLabel}</button
      >{/if}
    {#if status && status !== "unsupported" && onopen}<button
        type="button"
        class="secondary-button"
        disabled={disabled || working}
        onclick={onopen}>Systemeinstellungen öffnen</button
      >{/if}
    <button
      type="button"
      class="text-button"
      disabled={disabled || checking || working}
      onclick={oncheck}
      >{checking ? "Wird geprüft …" : "Freigabe erneut prüfen"}</button
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
