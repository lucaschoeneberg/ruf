<script lang="ts">
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
  let permissionError = $state<string | null>(null),
    actionError = $state<string | null>(null),
    testError = $state<string | null>(null),
    feedback = $state("");
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
        permissionError =
          "Die Systemfreigabe konnte nicht geprüft werden. Versuche es erneut.";
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
          feedback =
            "Mitteilungen sind erlaubt. Änderungen an den App-Einstellungen werden erst beim Speichern übernommen.";
        else if (requestFailed && !permissionError)
          actionError =
            "Die Anfrage konnte nicht abgeschlossen werden. Prüfe die Systemfreigabe erneut.";
      }
    }
  }
  async function open() {
    actionError = null;
    try {
      await openNotificationSettings();
    } catch {
      if (mounted)
        actionError =
          "Die Systemeinstellungen konnten nicht geöffnet werden. Öffne dort den Bereich Mitteilungen für Ruf.";
    }
  }
  async function test() {
    if (permission !== "authorized" || testing) return;
    testing = true;
    testError = null;
    feedback =
      "Die Testmitteilung wird in fünf Sekunden angefordert. Du kannst Ruf jetzt in den Hintergrund legen.";
    try {
      await testNotification();
      if (mounted)
        feedback =
          "Testmitteilung angefordert. Fokus und Systemregeln können ihre Anzeige unterdrücken.";
    } catch {
      if (mounted) {
        feedback = "";
        testError =
          "Die Testmitteilung konnte nicht angefordert werden. Versuche es erneut. Bleibt der Fehler bestehen, prüfe die Systemfreigabe.";
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
    <h1>Mitteilungen</h1>
    <p>App-Einstellung und Systemfreigabe für eingehende Anrufe.</p>
  </div>
</div>
<h2 class="settings-group">In Ruf</h2>
<fieldset disabled={busy || !canSave}>
  <div class="setting-row">
    <div class="setting-description">
      <label for="notifications-enabled">Eingehende Anrufe anzeigen</label>
      <p id="notifications-help">
        Zeigt eine Systemmitteilung mit Annehmen und Ablehnen. Der Klingelton
        wird unter Ton eingestellt.
      </p>
    </div>
    <div class="setting-control sound-toggle">
      <input
        id="notifications-enabled"
        type="checkbox"
        bind:checked={enabled}
        aria-describedby="notifications-help"
      /><span aria-hidden="true">{enabled ? "Ein" : "Aus"}</span>
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <label for="notification-caller">Anruferdetails anzeigen</label>
      <p>
        Name und Rufnummer in Systemmitteilungen. Ausgeschaltet erscheint ein
        neutraler Hinweis.
      </p>
    </div>
    <div class="setting-control sound-toggle">
      <input
        id="notification-caller"
        type="checkbox"
        bind:checked={showCaller}
      /><span aria-hidden="true">{showCaller ? "Ein" : "Aus"}</span>
    </div>
  </div>
</fieldset>
<h2 class="settings-group" id="notification-permission" tabindex="-1">
  Systemfreigabe
</h2>
<PermissionStatus
  title="Mitteilungen für Ruf"
  status={permission}
  {checking}
  working={requesting}
  error={permissionError}
  requestLabel="Mitteilungen erlauben"
  oncheck={() => void check()}
  onrequest={() => void request()}
  onopen={isTauri() ? () => void open() : undefined}
/>
{#if actionError}<p class="settings-feedback error" role="alert">
    {actionError}
  </p>{/if}
{#if permission === "denied"}<p class="settings-feedback">
    Mitteilungen sind in den Systemeinstellungen für Ruf deaktiviert. Erlaube
    sie dort. Anrufe bleiben im Telefonfenster sichtbar.
  </p>{:else if permission === "authorized"}<p class="settings-footnote">
    Mitteilungen sind erlaubt. Fokus, Banner-Einstellungen und andere
    Systemregeln können die Anzeige weiterhin unterdrücken.
  </p>{:else if permission === "unsupported"}<p class="settings-footnote">
    Systemmitteilungen sind in diesem Startmodus nicht verfügbar. Nutze die
    installierte App.
  </p>{/if}
<div class="setting-row">
  <div class="setting-description">
    <h3>Testmitteilung</h3>
    <p>
      Nach fünf Sekunden, ohne Anruf oder Anruferdaten. Fordert keine neue
      Systemfreigabe an.
    </p>
  </div>
  <button
    id="test-notification"
    type="button"
    class="secondary-button"
    disabled={permission !== "authorized" || testing || requesting}
    onclick={() => void test()}
    >{testing
      ? "Testmitteilung wird angefordert …"
      : "Testmitteilung senden"}</button
  >
</div>
{#if testError}<p class="settings-feedback error" role="alert">
    {testError}
  </p>{/if}
{#if feedback}<p role="status" class="settings-feedback">{feedback}</p>{/if}
<div class="settings-form-actions settings-save-bar">
  <p>
    {dirty
      ? "Deine Änderungen sind noch nicht gespeichert."
      : "Gilt für Ruf auf diesem Computer."}
  </p>
  <div class="settings-actions">
    <button
      type="button"
      class="text-button"
      disabled={!dirty || busy}
      onclick={ondiscard}>Verwerfen</button
    ><button
      type="button"
      class="primary-button"
      disabled={!canSave || !dirty || busy}
      onclick={onsave}
      >{busy ? "Wird gespeichert …" : "Mitteilungen speichern"}</button
    >
  </div>
</div>
