<script lang="ts">
  import { t, text, message, type MessageValue } from "../i18n";
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import type { ContactsPermission } from "../generated/AppInfo";
  import {
    getContactsPermission,
    requestContactsPermission,
    openContactsSettings,
  } from "../contacts-api";

  let { embedded = false }: { embedded?: boolean } = $props();
  let permission = $state<ContactsPermission | null>(null);
  let working = $state(false);
  let checking = $state(false);
  let permissionError = $state<MessageValue | null>(null);
  let actionError = $state<MessageValue | null>(null);
  const error = $derived(permissionError ?? actionError);
  let mounted = false;
  const labels: Record<ContactsPermission, MessageValue> = {
    notDetermined: message("settings.permission_not_requested"),
    authorized: message("settings.permission_granted"),
    denied: message("settings.permission_not_granted"),
    restricted: message("settings.permission_restricted"),
    unsupported: message("settings.macos_only"),
  };

  async function refresh(request = false) {
    if (!mounted || working) return;
    working = true;
    checking = !request;
    if (request) actionError = null;
    else permissionError = null;
    try {
      const result = await (request
        ? requestContactsPermission()
        : getContactsPermission());
      if (mounted) {
        permission = result;
        permissionError = null;
      }
    } catch {
      if (mounted) {
        if (request) actionError = message("settings.contacts_request_failed");
        else {
          permission = null;
          permissionError = message("settings.contacts_check_failed");
        }
      }
    } finally {
      if (mounted) {
        working = false;
        checking = false;
      }
    }
  }

  async function openPrivacy() {
    if (working) return;
    working = true;
    actionError = null;
    try {
      await openContactsSettings();
    } catch {
      if (mounted) actionError = message("settings.contacts_settings_failed");
    } finally {
      if (mounted) working = false;
    }
  }

  onMount(() => {
    mounted = true;
    if (!isTauri()) permission = "unsupported";
    else void refresh();
    const reload = () => {
      if (isTauri() && document.visibilityState !== "hidden") void refresh();
    };
    window.addEventListener("focus", reload);
    document.addEventListener("visibilitychange", reload);
    return () => {
      mounted = false;
      window.removeEventListener("focus", reload);
      document.removeEventListener("visibilitychange", reload);
    };
  });
</script>

{#if !embedded}<div class="settings-title">
    <div>
      <h1>{$t("settings.contacts")}</h1>
      <p>{$t("settings.contacts_description")}</p>
    </div>
  </div>
{/if}
<h2 class="settings-group" id="contacts-permission" tabindex="-1">
  {$t("settings.contacts_access_privacy")}
</h2>
<div class="setting-row">
  <div class="setting-description">
    <h3>{$t("settings.macos_contacts")}</h3>
    <p>
      {$t("settings.contacts_read_help")}
    </p>
    <p class="permission-status" role="status">
      {checking
        ? $t("settings.permission_checking")
        : permissionError
          ? $t("settings.permission_check_failed")
          : permission
            ? $text(labels[permission])
            : $t("settings.permission_unchecked")}
    </p>
    {#if permission === "notDetermined"}<p>
        {$t("settings.contacts_request_help")}
      </p>
    {:else if permission === "denied"}<p>
        {$t("settings.contacts_denied_help")}
      </p>
    {:else if permission === "restricted"}<p>
        {$t("settings.contacts_restricted_help")}
      </p>
    {:else if permission === "authorized"}<p>
        {$t("settings.contacts_location_help")}
      </p>{/if}
  </div>
  <div class="permission-actions">
    {#if permission === "notDetermined"}<button
        class="primary-button"
        type="button"
        disabled={working}
        onclick={() => void refresh(true)}
        >{$t("settings.allow_macos_contacts")}</button
      >{/if}
    {#if permission && permission !== "unsupported"}<button
        class="secondary-button"
        type="button"
        disabled={working}
        onclick={() => void openPrivacy()}
        >{$t("settings.open_system_settings")}</button
      >{/if}
    {#if permission !== "unsupported"}<button
        class="text-button"
        type="button"
        disabled={working}
        onclick={() => void refresh()}
        >{$t("settings.check_permission_again")}</button
      >{/if}
    {#if working}<span class="permission-progress" role="status"
        >{$t("settings.please_wait")}</span
      >{/if}
  </div>
</div>
{#if error}<p
    class="settings-feedback error contacts-permission-error"
    role="alert"
  >
    {$text(error)}
  </p>{/if}
<p class="settings-footnote">
  {$t("settings.contacts_permission_help")}
</p>

<style>
  .setting-description .permission-status {
    color: var(--text);
    margin-top: 10px;
    font-weight: 600;
  }
  .setting-description p + p {
    margin-top: 8px;
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
  .permission-progress {
    color: var(--muted);
    font-size: 12px;
  }
</style>
