<script lang="ts">
  import { t, text, message, type MessageValue } from "../i18n";
  import { onMount } from "svelte";
  import type { AudioTestKind, SoundSettings } from "../generated/AppInfo";
  import {
    startAudioTest,
    stopAudioTest,
    getEngineSnapshot,
    getMicrophonePermission,
  } from "../api";
  let {
    sound,
    disabled,
    onprivacy,
  }: { sound: SoundSettings; disabled: boolean; onprivacy: () => void } =
    $props();
  let owner = $state<string | null>(null),
    kind = $state<AudioTestKind | null>(null),
    level = $state(0),
    pending = $state(false),
    error = $state<MessageValue>(""),
    feedback = $state<MessageValue>("");
  let mounted = false,
    revision = 0,
    polling = false;
  async function start(value: AudioTestKind) {
    if (disabled || pending || owner) return;
    const current = ++revision;
    pending = true;
    error = "";
    feedback = "";
    try {
      const device =
        value === "microphone" ? sound.inputDevice : sound.outputDevice;
      if (value === "microphone") {
        const permission = await getMicrophonePermission();
        if (!mounted || current !== revision) return;
        if (permission !== "authorized" && permission !== "unsupported") {
          error = message("settings.microphone_permission_required");
          return;
        }
      }
      if (!mounted || current !== revision) return;
      const id = await startAudioTest(value, device);
      if (!mounted || current !== revision) {
        await stopAudioTest(id);
        return;
      }
      owner = id;
      kind = value;
      feedback =
        value === "microphone"
          ? message("settings.microphone_test_running")
          : message("settings.speaker_test_running");
    } catch {
      if (mounted && current === revision)
        error = message("settings.audio_test_start_failed");
    } finally {
      if (mounted && current === revision) pending = false;
    }
  }
  async function stop() {
    revision++;
    const id = owner;
    owner = null;
    kind = null;
    level = 0;
    pending = false;
    if (id)
      try {
        await stopAudioTest(id);
      } catch {
        if (mounted) error = message("settings.audio_test_stop_failed");
      }
  }
  async function poll() {
    if (!mounted || !owner || polling) return;
    const id = owner;
    polling = true;
    try {
      const state = await getEngineSnapshot();
      if (!mounted || owner !== id) return;
      if (state.audioTest?.id === id) level = state.audioTest.level;
      else {
        owner = null;
        kind = null;
        level = 0;
        feedback =
          state.call !== "idle"
            ? message("settings.audio_test_ended_for_call")
            : message("settings.audio_test_ended");
        if (state.audioTestError)
          error = message("settings.audio_test_device_ended");
      }
    } catch {
      if (mounted) error = message("settings.audio_test_status_failed");
    } finally {
      polling = false;
    }
  }
  onMount(() => {
    mounted = true;
    const timer = setInterval(() => void poll(), 200);
    return () => {
      mounted = false;
      clearInterval(timer);
      void stop();
    };
  });
</script>

<h2 class="settings-group sound-group" id="audio-tests" tabindex="-1">
  {$t("settings.audio_tests")}
</h2>
<div class="setting-row">
  <div class="setting-description">
    <h3>{$t("settings.test_microphone")}</h3>
    <p>
      {$t("settings.microphone_test_help")}
    </p>
    {#if kind === "microphone"}<meter
        aria-label={$t("settings.microphone_level")}
        min="0"
        max="1"
        value={level}
      ></meter>{/if}
  </div>
  <button
    type="button"
    class="secondary-button"
    disabled={disabled || pending || !!owner}
    onclick={() => void start("microphone")}
    >{$t("settings.test_microphone")}</button
  >
</div>
<div class="setting-row">
  <div class="setting-description">
    <h3>{$t("settings.test_speaker")}</h3>
    <p>{$t("settings.speaker_test_help")}</p>
  </div>
  <button
    type="button"
    class="secondary-button"
    disabled={disabled || pending || !!owner}
    onclick={() => void start("speaker")}>{$t("settings.test_speaker")}</button
  >
</div>
{#if owner || pending}<button
    type="button"
    class="secondary-button"
    onclick={() => void stop()}>{$t("settings.stop_audio_test")}</button
  >{/if}
{#if feedback}<p class="settings-footnote" role="status">
    {$text(feedback)}
  </p>{/if}
{#if error}<p class="settings-feedback error" role="alert">{$text(error)}</p>
  <button type="button" class="text-button" onclick={onprivacy}
    >{$t("settings.check_permissions")}</button
  >{/if}

<style>
  meter {
    width: 100%;
    margin-top: 8px;
    accent-color: var(--accent);
  }
</style>
