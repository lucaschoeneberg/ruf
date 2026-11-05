<script lang="ts">
  let {
    autostart = $bindable(),
    startHidden = $bindable(),
    available,
    disabled,
    dirty,
    onsave,
    ondiscard,
  }: {
    autostart: boolean;
    startHidden: boolean;
    available: boolean;
    disabled: boolean;
    dirty: boolean;
    onsave: () => void;
    ondiscard: () => void;
  } = $props();
</script>

<div class="settings-title">
  <div>
    <h1>Allgemein</h1>
    <p>Start und Hintergrundbetrieb auf deinem Computer.</p>
  </div>
</div>
<h2 class="settings-group">Start & Hintergrund</h2>
<fieldset {disabled}>
  <div class="setting-row">
    <div class="setting-description">
      <label for="autostart-enabled">Bei der Anmeldung starten</label>
      <p>Ruf beim Anmelden am Computer öffnen.</p>
      {#if !available}<p>
          Installiere Ruf an einem festen Ort. In Entwicklungs-Builds ist der
          automatische Start nicht verfügbar.
        </p>{/if}
    </div>
    <div class="setting-control sound-toggle">
      <input
        id="autostart-enabled"
        type="checkbox"
        bind:checked={autostart}
        disabled={!available}
      /><span aria-hidden="true">{autostart ? "Ein" : "Aus"}</span>
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <label for="start-hidden">Bei der Anmeldung im Hintergrund starten</label>
      <p>
        Das Telefonfenster bleibt beim Anmelden geschlossen. Manuelles Öffnen
        zeigt es weiterhin an.
      </p>
    </div>
    <div class="setting-control sound-toggle">
      <input
        id="start-hidden"
        type="checkbox"
        bind:checked={startHidden}
      /><span aria-hidden="true">{startHidden ? "Ein" : "Aus"}</span>
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <h3>Beim Schließen erreichbar bleiben</h3>
      <p>
        Schließt du das Telefonfenster, läuft Ruf weiter. Über das Ruf-Symbol
        öffnest du es wieder. „Ruf beenden“ beendet auch die Telefonie.
      </p>
    </div>
  </div>
  <div class="settings-form-actions settings-save-bar">
    <p>
      {dirty
        ? "Deine Änderungen sind noch nicht gespeichert."
        : "Gilt für Ruf auf deinem Computer."}
    </p>
    <div class="settings-actions">
      <button
        class="text-button"
        type="button"
        disabled={!dirty}
        onclick={ondiscard}>Verwerfen</button
      ><button
        class="primary-button"
        type="button"
        disabled={!dirty}
        onclick={onsave}>Allgemein speichern</button
      >
    </div>
  </div>
</fieldset>
