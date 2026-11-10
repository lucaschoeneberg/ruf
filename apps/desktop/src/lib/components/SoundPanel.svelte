<script lang="ts">
  import { t, text, message, type MessageValue } from "../i18n";
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
    devicesError: MessageValue | null;
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
      label: message("settings.microphone"),
      description: message("settings.microphone_device_help"),
      direction: "inputs",
    },
    {
      key: "outputDevice",
      label: message("settings.speaker"),
      description: message("settings.speaker_device_help"),
      direction: "outputs",
    },
    {
      key: "ringDevice",
      label: message("settings.ringtone_output"),
      description: message("settings.ringtone_device_help"),
      direction: "outputs",
    },
  ] as const;
</script>

<div class="settings-title">
  <div>
    <h1>{$t("settings.sound")}</h1>
    <p>{$t("settings.sound_description")}</p>
  </div>
</div>
{#if testAudio}<p class="settings-feedback">
    {$t("settings.test_tone_active")}
  </p>{/if}
{#if devicesError}<p class="settings-feedback error" role="alert">
    {$text(devicesError)}
    {$t("settings.devices_retry_help")}
  </p>{/if}
{#if devices && [...devices.inputs, ...devices.outputs].some((device) => !device.selectable)}<p
    class="settings-feedback"
  >
    {$t("settings.duplicate_devices_help")}
  </p>{/if}
<h2 class="settings-group">{$t("settings.audio_devices")}</h2>
<fieldset disabled={disabled || busy}>
  {#each rows as row}
    {@const available = devices?.[row.direction] ?? []}
    {@const missing =
      value[row.key] !== "" &&
      !available.some((device) => device.id === value[row.key])}
    <div class="setting-row">
      <div class="setting-description">
        <label for={row.key}>{$text(row.label)}</label>
        <p id={`${row.key}-help`}>{$text(row.description)}</p>
      </div>
      <div class="setting-control">
        <select
          id={row.key}
          bind:value={value[row.key]}
          aria-describedby={`${row.key}-help${missing && devices ? ` ${row.key}-missing` : ""}`}
          disabled={disabled || busy || devicesLoading}
          ><option value="">{$t("settings.system_default")}</option
          >{#each available as device}<option
              value={device.id}
              disabled={!device.selectable}
              >{device.selectable
                ? device.name
                : $t("settings.duplicate_device", {
                    name: device.name,
                  })}</option
            >{/each}{#if missing}<option value={value[row.key]} disabled
              >{devices
                ? $t("settings.unavailable")
                : $t("settings.saved_device")}: {value[row.key]}</option
            >{/if}</select
        >{#if missing && devices}<p
            id={`${row.key}-missing`}
            class="field-error"
          >
            {$t("settings.missing_device_help")}
          </p>{/if}
      </div>
    </div>
  {/each}
  <div class="sound-device-actions">
    <p>{$t("settings.system_default_help")}</p>
    <button
      class="text-button"
      type="button"
      disabled={!nativeAvailable || busy || devicesLoading}
      onclick={onrefresh}
      >{devicesLoading
        ? $t("settings.devices_loading")
        : $t("settings.refresh_devices")}</button
    >
  </div>
  <h2 class="settings-group sound-group">{$t("settings.ringtone")}</h2>
  <div class="setting-row">
    <div class="setting-description">
      <label for="ringtone-enabled">{$t("settings.ring_on_calls")}</label>
      <p id="ringtone-help">
        {$t("settings.ring_on_calls_help")}
      </p>
    </div>
    <div class="setting-control sound-toggle">
      <input
        id="ringtone-enabled"
        type="checkbox"
        bind:checked={value.ringtoneEnabled}
        aria-describedby="ringtone-help"
      /><span aria-hidden="true"
        >{value.ringtoneEnabled ? $t("settings.on") : $t("settings.off")}</span
      >
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <label for="ringtone-style">{$t("settings.ringtone")}</label>
      <p>{$t("settings.ringtone_help")}</p>
    </div>
    <select id="ringtone-style" bind:value={value.ringtone}
      ><option value="classic">{$t("settings.ringtone_classic")}</option><option
        value="soft">{$t("settings.ringtone_soft")}</option
      ><option value="short">{$t("settings.ringtone_short")}</option></select
    >
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <label for="ringtone-volume">{$t("settings.ringtone_volume")}</label>
      <p>{$t("settings.ringtone_volume_help")}</p>
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
      <h3>{$t("settings.preview_ringtone")}</h3>
      <p>
        {$t("settings.preview_ringtone_help")}
      </p>
    </div>
    <div class="setting-control sound-preview">
      <button
        class="secondary-button"
        type="button"
        disabled={disabled || busy || devicesLoading}
        onclick={playing ? onstop : onpreview}
        >{playing
          ? $t("settings.stop_preview")
          : $t("settings.preview_ringtone")}</button
      ><span class="sound-preview-status" role="status"
        >{playing ? $t("settings.preview_running") : ""}</span
      >
    </div>
  </div>
  <div class="settings-form-actions settings-save-bar">
    <p>
      {dirty
        ? $t("settings.unsaved_notice")
        : $t("settings.all_accounts_scope")}
    </p>
    <div class="settings-actions">
      <button
        type="button"
        class="text-button"
        disabled={disabled || busy || !dirty}
        onclick={ondiscard}>{$t("settings.discard")}</button
      ><button
        class="primary-button"
        type="button"
        disabled={disabled || busy || !dirty}
        onclick={onsave}
        >{busy ? $t("settings.please_wait") : $t("settings.save_sound")}</button
      >
    </div>
  </div>
</fieldset>
