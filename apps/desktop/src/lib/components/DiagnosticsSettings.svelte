<script lang="ts">
  import { phoneActions, shortcutLabel } from "../shortcuts";
  import { onMount } from "svelte";
  import type { DiagnosticsReport } from "../generated/AppInfo";
  import { getDiagnostics, copyDiagnostics } from "../api";
  let report = $state<DiagnosticsReport | null>(null),
    loading = $state(false),
    error = $state(""),
    feedback = $state("");
  let mounted = false;
  async function refresh() {
    if (loading) return;
    loading = true;
    error = "";
    try {
      const value = await getDiagnostics();
      if (mounted) report = value;
    } catch {
      if (mounted)
        error =
          "Der Diagnosebericht konnte nicht geladen werden. Versuche es erneut.";
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
      if (mounted)
        feedback = "Der Diagnosebericht ohne persönliche Daten wurde kopiert.";
    } catch {
      if (mounted) error = "Der Diagnosebericht konnte nicht kopiert werden.";
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
    <h1>Diagnose & Hilfe</h1>
    <p>Technische Zustände prüfen und Probleme gezielt eingrenzen.</p>
  </div>
</div>
<h2 class="settings-group" id="diagnostics-status" tabindex="-1">
  Aktueller Zustand
</h2>
{#if loading}<p role="status">Diagnose wird geladen …</p>{/if}
{#if error}<p class="settings-feedback error" role="alert">{error}</p>{/if}
{#if report}
  <div class="setting-row">
    <div class="setting-description">
      <h3>Telefonie</h3>
      <p>
        {report.telephony === "linked"
          ? "Native Telefonie ist in diesem Build enthalten."
          : "Dieser Build enthält keine native Telefonie."}
      </p>
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <h3>Konten</h3>
      <p>
        {report.accountsRegistered === null
          ? "Kontozustand nicht verfügbar."
          : `${report.accountsRegistered} von ${report.accountsTotal} Konten haben eine bestätigte Anmeldung.`}
      </p>
      <p>
        Eine Anmeldung allein bestätigt nicht die Zustellung eines externen
        Anrufs.
      </p>
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <h3>Systemfreigaben</h3>
      <p>
        Mitteilungen: {report.notificationPermission === null
          ? "Prüfung fehlgeschlagen"
          : report.notificationPermission === "authorized"
            ? "Erlaubt"
            : report.notificationPermission === "denied"
              ? "Im System blockiert"
              : report.notificationPermission === "notDetermined"
                ? "Noch nicht angefragt"
                : "Nicht verfügbar"}.
      </p>
      <p>
        Mikrofon: {report.microphonePermission === null
          ? "Prüfung fehlgeschlagen"
          : report.microphonePermission === "authorized"
            ? "Erlaubt"
            : report.microphonePermission === "denied"
              ? "Im System blockiert"
              : report.microphonePermission === "restricted"
                ? "Durch Systemrichtlinie eingeschränkt"
                : report.microphonePermission === "notDetermined"
                  ? "Noch nicht angefragt"
                  : "Nicht verfügbar"}.
      </p>
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <h3>Audiogeräte</h3>
      <p>
        {report.audioDevicesAvailable === null
          ? "Geräteliste konnte nicht gelesen werden."
          : report.audioDevicesAvailable
            ? "Ein- und Ausgänge werden erkannt. Ton und Mikrofon lassen sich unter Ton ausdrücklich testen."
            : "Nicht für beide Richtungen werden Audiogeräte erkannt. Prüfe die Auswahl unter Ton."}
      </p>
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <h3>Schlüsselbund</h3>
      <p>
        {report.credentialError === "credentialAccessRequired"
          ? "Eine Freigabe für gespeicherte Zugangsdaten ist erforderlich."
          : report.credentialError === "credentialMissing"
            ? "Ein gespeichertes Passwort fehlt."
            : report.credentialError === "credentialFailure"
              ? "Beim Zugriff auf gespeicherte Zugangsdaten wurde ein Fehler gemeldet."
              : "Kein Zugriffsfehler gemeldet. Diese Prüfung liest keine Passwörter und bestätigt keinen Schlüsselbundzugriff."}
      </p>
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <h3>Verlauf</h3>
      <p>
        {report.historyAvailable
          ? "Kein Speicherfehler gemeldet."
          : "Die Verlaufsspeicherung meldet einen Fehler."}
      </p>
    </div>
  </div>
  {#if report.lastError}<p class="settings-feedback" role="status">
      Gemeldeter Fehler: {report.lastError}
    </p>{/if}
  <details class="settings-lab">
    <summary>Diagnosebericht ohne persönliche Daten ansehen</summary>
    <pre>{JSON.stringify(report, null, 2)}</pre>
  </details>
{/if}
<div class="settings-form-actions">
  <button
    class="secondary-button"
    type="button"
    disabled={loading}
    onclick={() => void refresh()}>Status erneut prüfen</button
  ><button
    class="secondary-button"
    type="button"
    disabled={!report || loading}
    onclick={() => void copy()}>Diagnosebericht kopieren</button
  >
</div>
{#if feedback}<p class="settings-feedback" role="status">{feedback}</p>{/if}
<h2 class="settings-group sound-group">Wenn etwas nicht funktioniert</h2>
<div class="setting-row">
  <div class="setting-description">
    <h3>Konto nicht angemeldet</h3>
    <p>
      Öffne das betroffene Konto in der linken Seitenleiste. „Erneut verbinden“
      verwendet die gespeicherten Zugangsdaten. Eine ausstehende
      Schlüsselbundfreigabe bestätigst du direkt im Systemdialog.
    </p>
  </div>
</div>
<div class="setting-row">
  <div class="setting-description">
    <h3>Keine Mitteilung oder kein Ton</h3>
    <p>
      Prüfe unter Mitteilungen die Systemfreigabe und unter Ton die gewählten
      Geräte. Ein aktiver Fokus kann Banner unterdrücken.
    </p>
  </div>
</div>
<h2 id="keyboard-shortcuts" class="settings-group sound-group">
  Tastenkombinationen
</h2>
<p class="shortcut-note">
  Die Kürzel gelten, solange Ruf im Vordergrund ist. Gesprächsaktionen sind nur
  im passenden Anrufstatus verfügbar.
</p>
<dl class="shortcut-list">
  {#each phoneActions as action}
    <div>
      <dt>{action.label}</dt>
      <dd><kbd>{shortcutLabel(action)}</kbd></dd>
    </div>
  {/each}
  <div>
    <dt>Nummer im Nummernfeld anrufen</dt>
    <dd><kbd>Enter</kbd></dd>
  </div>
  <div>
    <dt>Suche schließen / Starthilfe verlassen</dt>
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
