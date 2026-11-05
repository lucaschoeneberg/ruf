<script lang="ts">
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
    error = $state(""),
    feedback = $state("");
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
          error = "Erlaube Ruf zuerst den Mikrofonzugriff unter Datenschutz.";
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
          ? "Mikrofontest läuft, höchstens zehn Sekunden."
          : "Lautsprechertest läuft, zwei Sekunden.";
    } catch {
      if (mounted && current === revision)
        error =
          "Der Audiotest konnte nicht gestartet werden. Prüfe Gerät und Freigabe.";
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
        if (mounted) error = "Der Audiotest konnte nicht gestoppt werden.";
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
            ? "Audiotest für einen Anruf beendet."
            : "Audiotest beendet.";
        if (state.audioTestError)
          error = "Der Audiotest wurde vom Audiogerät beendet.";
      }
    } catch {
      if (mounted) error = "Der Audioteststatus konnte nicht gelesen werden.";
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
  Audiotests
</h2>
<div class="setting-row">
  <div class="setting-description">
    <h3>Mikrofon testen</h3>
    <p>
      Zeigt den Eingangspegel für höchstens zehn Sekunden. Es wird keine
      Aufnahme gespeichert.
    </p>
    {#if kind === "microphone"}<meter
        aria-label="Mikrofonpegel"
        min="0"
        max="1"
        value={level}
      ></meter>{/if}
  </div>
  <button
    type="button"
    class="secondary-button"
    disabled={disabled || pending || !!owner}
    onclick={() => void start("microphone")}>Mikrofon testen</button
  >
</div>
<div class="setting-row">
  <div class="setting-description">
    <h3>Lautsprecher testen</h3>
    <p>Zwei Sekunden auf dem gewählten Gesprächsausgang.</p>
  </div>
  <button
    type="button"
    class="secondary-button"
    disabled={disabled || pending || !!owner}
    onclick={() => void start("speaker")}>Lautsprecher testen</button
  >
</div>
{#if owner || pending}<button
    type="button"
    class="secondary-button"
    onclick={() => void stop()}>Audiotest stoppen</button
  >{/if}
{#if feedback}<p class="settings-footnote" role="status">{feedback}</p>{/if}
{#if error}<p class="settings-feedback error" role="alert">{error}</p>
  <button type="button" class="text-button" onclick={onprivacy}
    >Freigaben prüfen</button
  >{/if}

<style>
  meter {
    width: 100%;
    margin-top: 8px;
    accent-color: var(--accent);
  }
</style>
