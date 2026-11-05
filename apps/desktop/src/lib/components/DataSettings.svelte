<script lang="ts">
  import type { AppPreferences } from "../generated/AppInfo";
  import { clearHistory } from "../history-api";
  import { errorMessage } from "../api";
  let {
    value = $bindable(),
    dirty,
    disabled,
    confirmRetention = $bindable(false),
    onsave,
    ondiscard,
  }: {
    value: AppPreferences;
    dirty: boolean;
    disabled: boolean;
    confirmRetention: boolean;
    onsave: () => void;
    ondiscard: () => void;
  } = $props();
  let confirmClear = $state(false),
    clearing = $state(false),
    feedback = $state("");
  async function clear() {
    if (!confirmClear || clearing) return;
    clearing = true;
    feedback = "";
    try {
      await clearHistory();
      feedback = "Abgeschlossene Anrufe wurden aus dem Verlauf gelöscht.";
      confirmClear = false;
    } catch (e) {
      feedback = errorMessage(e);
    } finally {
      clearing = false;
    }
  }
</script>

<div class="settings-title">
  <div>
    <h1>Kontakte & Verlauf</h1>
    <p>Anzeige und lokale Speicherung deiner Anrufe.</p>
  </div>
</div>
<h2 class="settings-group" id="contacts-display" tabindex="-1">
  Kontaktanzeige
</h2>
<fieldset {disabled}>
  <div class="setting-row">
    <div class="setting-description">
      <label for="favorites-first">Favoriten zuerst</label>
      <p>Favoriten vor den alphabetisch sortierten Kontakten anzeigen.</p>
    </div>
    <input
      id="favorites-first"
      type="checkbox"
      bind:checked={value.contacts.favoritesFirst}
    />
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <label for="show-organization">Firma anzeigen</label>
      <p>Die Firma zusätzlich zum Kontaktnamen anzeigen, sofern vorhanden.</p>
    </div>
    <input
      id="show-organization"
      type="checkbox"
      bind:checked={value.contacts.showOrganization}
    />
  </div>
  <h2 class="settings-group sound-group">Anrufverlauf</h2>
  <div class="setting-row">
    <div class="setting-description">
      <label for="history-recording">Verlauf speichern</label>
      <p>
        Änderungen gelten für künftige Anrufe. Bereits gespeicherte Einträge
        bleiben erhalten.
      </p>
    </div>
    <input
      id="history-recording"
      type="checkbox"
      bind:checked={value.history.recordingEnabled}
    />
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <label for="history-retention">Aufbewahrung</label>
      <p>
        Nur abgeschlossene Einträge werden automatisch gelöscht. Laufende Anrufe
        bleiben geschützt.
      </p>
    </div>
    <select
      id="history-retention"
      bind:value={value.history.retentionDays}
      onchange={() => (confirmRetention = false)}
      ><option value={null}>Unbegrenzt</option><option value={30}
        >30 Tage</option
      ><option value={90}>90 Tage</option><option value={365}>365 Tage</option
      ></select
    >
  </div>
  {#if value.history.retentionDays !== null}<div class="setting-row">
      <div class="setting-description">
        <label for="confirm-retention"
          >Löschung älterer Einträge bestätigen</label
        >
        <p>
          Beim Speichern können abgeschlossene Einträge außerhalb der gewählten
          Frist dauerhaft gelöscht werden.
        </p>
      </div>
      <input
        id="confirm-retention"
        type="checkbox"
        bind:checked={confirmRetention}
      />
    </div>{/if}
  <div class="settings-form-actions settings-save-bar">
    <p>
      {dirty
        ? "Deine Änderungen sind noch nicht gespeichert."
        : "Die Daten bleiben auf diesem Computer."}
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
        onclick={onsave}>Anzeige & Verlauf speichern</button
      >
    </div>
  </div>
</fieldset>
<h2 class="settings-group sound-group">Verlauf löschen</h2>
<p class="settings-footnote">
  Diese Aktion löscht abgeschlossene Anrufe dauerhaft. Sie verändert weder
  Kontakte noch Konten.
</p>
{#if confirmClear}<div class="settings-feedback">
    <p>Alle abgeschlossenen Einträge unwiderruflich löschen?</p>
    <div class="settings-actions">
      <button
        type="button"
        class="secondary-button"
        disabled={clearing}
        onclick={() => (confirmClear = false)}>Abbrechen</button
      ><button
        type="button"
        class="danger-button"
        disabled={disabled || clearing}
        onclick={() => void clear()}>Verlauf endgültig löschen</button
      >
    </div>
  </div>{:else}<button
    id="clear-history"
    type="button"
    class="secondary-button"
    disabled={disabled || clearing}
    onclick={() => (confirmClear = true)}>Verlauf löschen …</button
  >{/if}
{#if feedback}<p class="settings-feedback" role="status">{feedback}</p>{/if}
