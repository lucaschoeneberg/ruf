<script lang="ts">
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
  let permissionError = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  const error = $derived(permissionError ?? actionError);
  let mounted = false;
  const labels: Record<ContactsPermission, string> = {
    notDetermined: "Noch nicht angefragt",
    authorized: "Freigegeben",
    denied: "Nicht freigegeben",
    restricted: "Durch Systemrichtlinie eingeschränkt",
    unsupported: "Nur unter macOS verfügbar",
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
        if (request)
          actionError =
            "Die Kontakte-Freigabe konnte nicht angefordert werden. Starte Ruf neu und versuche es erneut.";
        else {
          permission = null;
          permissionError =
            "Die Kontakte-Freigabe konnte nicht geprüft werden. Versuche es erneut.";
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
      if (mounted)
        actionError =
          "Die Systemeinstellungen konnten nicht geöffnet werden. Öffne dort Datenschutz & Sicherheit → Kontakte.";
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
      <h1>Kontakte</h1>
      <p>Dein macOS-Adressbuch zum Suchen und Anrufen.</p>
    </div>
  </div>
{/if}
<h2 class="settings-group" id="contacts-permission" tabindex="-1">
  Kontakte · Zugriff & Datenschutz
</h2>
<div class="setting-row">
  <div class="setting-description">
    <h3>macOS-Kontakte</h3>
    <p>
      Ruf liest Namen, Firmen und Telefonnummern. Dein Adressbuch bleibt
      unverändert.
    </p>
    <p class="permission-status" role="status">
      {checking
        ? "Freigabe wird geprüft …"
        : permissionError
          ? "Prüfung fehlgeschlagen"
          : permission
            ? labels[permission]
            : "Noch nicht geprüft"}
    </p>
    {#if permission === "notDetermined"}<p>
        Mit „macOS-Kontakte freigeben“ öffnest du den Freigabedialog. Erst
        danach erscheint Ruf in der Kontakte-Datenschutzliste.
      </p>
    {:else if permission === "denied"}<p>
        macOS fragt nach einer Ablehnung nicht erneut. Erlaube Ruf den Zugriff
        unter Systemeinstellungen → Datenschutz & Sicherheit → Kontakte.
      </p>
    {:else if permission === "restricted"}<p>
        Eine Systemrichtlinie verhindert den Zugriff. Wende dich an die Person,
        die diesen Mac verwaltet.
      </p>
    {:else if permission === "authorized"}<p>
        Deine Kontakte findest du im Kontakte-Tab des Telefonfensters. Nach
        Änderungen oder einer neuen Freigabe aktualisiert sich die Liste beim
        Zurückkehren.
      </p>{/if}
  </div>
  <div class="permission-actions">
    {#if permission === "notDetermined"}<button
        class="primary-button"
        type="button"
        disabled={working}
        onclick={() => void refresh(true)}>macOS-Kontakte freigeben</button
      >{/if}
    {#if permission && permission !== "unsupported"}<button
        class="secondary-button"
        type="button"
        disabled={working}
        onclick={() => void openPrivacy()}>Systemeinstellungen öffnen</button
      >{/if}
    {#if permission !== "unsupported"}<button
        class="text-button"
        type="button"
        disabled={working}
        onclick={() => void refresh()}>Freigabe erneut prüfen</button
      >{/if}
    {#if working}<span class="permission-progress" role="status"
        >Bitte warten …</span
      >{/if}
  </div>
</div>
{#if error}<p
    class="settings-feedback error contacts-permission-error"
    role="alert"
  >
    {error}
  </p>{/if}
<p class="settings-footnote">
  Die Freigabe gilt für diese App und wird von macOS verwaltet. Hier werden nur
  die Zugriffsrechte geprüft; Kontaktinhalte werden erst im Kontakte-Tab
  geladen.
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
