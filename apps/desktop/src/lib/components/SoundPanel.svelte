<script lang="ts">
  import { onMount } from "svelte";
  import type { AudioDevices, SoundSettings } from "../generated/AppInfo";
  let {
    value = $bindable(),
    devices,
    devicesLoading,
    devicesError,
    disabled,
    nativeAvailable,
    busy,
    dirty,
    playing,
    testAudio,
    onrefresh,
    onsave,
    onpreview,
    onstop,
    ondiscard,
  }: {
    value: SoundSettings;
    devices: AudioDevices | null;
    devicesLoading: boolean;
    devicesError: string | null;
    disabled: boolean;
    nativeAvailable: boolean;
    busy: boolean;
    dirty: boolean;
    playing: boolean;
    testAudio: boolean;
    onrefresh: () => void;
    onsave: () => void;
    onpreview: () => void;
    onstop: () => void;
    ondiscard: () => void;
  } = $props();
  onMount(() => onrefresh());
  const rows = [
    {
      key: "inputDevice",
      label: "Mikrofon",
      description: "Für deine Stimme im Gespräch.",
      direction: "inputs",
    },
    {
      key: "outputDevice",
      label: "Lautsprecher",
      description: "Für die Stimme deines Gegenübers.",
      direction: "outputs",
    },
    {
      key: "ringDevice",
      label: "Klingelton-Ausgabe",
      description:
        "Kann unabhängig vom Gespräch auf einem anderen Gerät klingeln.",
      direction: "outputs",
    },
  ] as const;
</script>

<div class="settings-title">
  <div>
    <h1>Ton</h1>
    <p>Mikrofon, Gesprächston und Klingelton für alle Konten.</p>
  </div>
</div>
{#if testAudio}<p class="settings-feedback">
    Der Entwicklungs-Testton ist aktiv und ersetzt das Mikrofon. Die
    Geräteauswahl gilt für normale Gespräche mit Mikrofon und Lautsprecher.
  </p>{/if}
{#if devicesError}<p class="settings-feedback error" role="alert">
    {devicesError} Du kannst die Liste erneut laden oder „Systemstandard“ wählen.
  </p>{/if}
{#if devices && [...devices.inputs, ...devices.outputs].some((device) => !device.selectable)}<p
    class="settings-feedback"
  >
    Gleichnamige Geräte lassen sich nicht eindeutig auswählen. Benenne sie in
    den Audioeinstellungen deines Betriebssystems um oder verwende
    Systemstandard.
  </p>{/if}
<h2 class="settings-group">Audiogeräte</h2>
<fieldset disabled={disabled || busy}>
  {#each rows as row}
    {@const available = devices?.[row.direction] ?? []}
    {@const missing =
      value[row.key] !== "" &&
      !available.some((device) => device.id === value[row.key])}
    <div class="setting-row">
      <div class="setting-description">
        <label for={row.key}>{row.label}</label>
        <p id={`${row.key}-help`}>{row.description}</p>
      </div>
      <div class="setting-control">
        <select
          id={row.key}
          bind:value={value[row.key]}
          aria-describedby={`${row.key}-help${missing && devices ? ` ${row.key}-missing` : ""}`}
          disabled={disabled || busy || devicesLoading}
          ><option value="">Systemstandard</option
          >{#each available as device}<option
              value={device.id}
              disabled={!device.selectable}
              >{device.selectable
                ? device.name
                : `${device.name} (Name mehrfach vorhanden)`}</option
            >{/each}{#if missing}<option value={value[row.key]} disabled
              >{devices ? "Nicht verfügbar" : "Gespeichertes Gerät"}: {value[
                row.key
              ]}</option
            >{/if}</select
        >{#if missing && devices}<p
            id={`${row.key}-missing`}
            class="field-error"
          >
            Gespeichertes Gerät nicht verfügbar. Verbinde es erneut oder wähle
            ein anderes Gerät.
          </p>{/if}
      </div>
    </div>
  {/each}
  <div class="sound-device-actions">
    <p>Systemstandard folgt der Auswahl deines Betriebssystems.</p>
    <button
      class="text-button"
      type="button"
      disabled={!nativeAvailable || busy || devicesLoading}
      onclick={onrefresh}
      >{devicesLoading
        ? "Geräte werden geladen …"
        : "Geräte aktualisieren"}</button
    >
  </div>
  <h2 class="settings-group sound-group">Klingelton</h2>
  <div class="setting-row">
    <div class="setting-description">
      <label for="ringtone-enabled">Bei Anrufen klingeln</label>
      <p id="ringtone-help">
        Ein akustischer Hinweis für eingehende Anrufe. Die Anrufanzeige bleibt
        auch bei ausgeschaltetem Ton sichtbar.
      </p>
    </div>
    <div class="setting-control sound-toggle">
      <input
        id="ringtone-enabled"
        type="checkbox"
        bind:checked={value.ringtoneEnabled}
        aria-describedby="ringtone-help"
      /><span aria-hidden="true">{value.ringtoneEnabled ? "Ein" : "Aus"}</span>
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <label for="ringtone-style">Klingelton</label>
      <p>Ein integrierter Ton für eingehende Anrufe.</p>
    </div>
    <select id="ringtone-style" bind:value={value.ringtone}
      ><option value="classic">Klassisch</option><option value="soft"
        >Sanft</option
      ><option value="short">Kurz</option></select
    >
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <label for="ringtone-volume">Klingellautstärke</label>
      <p>Ändert nur Ruf, nicht die System- oder Gesprächslautstärke.</p>
    </div>
    <div class="setting-control">
      <input
        id="ringtone-volume"
        type="range"
        min="0"
        max="100"
        step="1"
        bind:value={value.ringtoneVolume}
      /><output for="ringtone-volume">{value.ringtoneVolume} %</output>
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <h3>Klingelton anhören</h3>
      <p>
        Zwei Sekunden auf der gewählten Klingelton-Ausgabe. Ton und Lautstärke
        entsprechen den gespeicherten Einstellungen; speichere Änderungen daran
        vor der Hörprobe.
      </p>
    </div>
    <div class="setting-control sound-preview">
      <button
        class="secondary-button"
        type="button"
        disabled={disabled || busy || devicesLoading}
        onclick={playing ? onstop : onpreview}
        >{playing ? "Hörprobe stoppen" : "Klingelton anhören"}</button
      ><span class="sound-preview-status" role="status"
        >{playing ? "Hörprobe läuft …" : ""}</span
      >
    </div>
  </div>
  <div class="settings-form-actions settings-save-bar">
    <p>
      {dirty
        ? "Deine Änderungen sind noch nicht gespeichert."
        : "Die Auswahl gilt für alle SIP-Konten."}
    </p>
    <div class="settings-actions">
      <button
        type="button"
        class="text-button"
        disabled={disabled || busy || !dirty}
        onclick={ondiscard}>Verwerfen</button
      ><button
        class="primary-button"
        type="button"
        disabled={disabled || busy || !dirty}
        onclick={onsave}>{busy ? "Bitte warten …" : "Ton speichern"}</button
      >
    </div>
  </div>
</fieldset>
