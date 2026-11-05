<script lang="ts">
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import type { MicrophonePermission } from "../generated/AppInfo";
  import {
    getMicrophonePermission,
    requestMicrophonePermission,
    openMicrophoneSettings,
  } from "../api";
  import ContactsSettings from "./ContactsSettings.svelte";
  import PermissionStatus from "./PermissionStatus.svelte";
  let status = $state<MicrophonePermission | null>(null),
    error = $state<string | null>(null);
  let checking = $state(false),
    requesting = $state(false),
    mounted = false,
    revision = 0;
  async function check(request = false) {
    if (!mounted || checking) return;
    const current = ++revision;
    checking = true;
    requesting = request;
    error = null;
    try {
      const result = isTauri()
        ? await (request
            ? requestMicrophonePermission()
            : getMicrophonePermission())
        : "unsupported";
      if (mounted && current === revision) status = result;
    } catch {
      if (mounted && current === revision) {
        status = null;
        error =
          "Die Mikrofonfreigabe konnte nicht geprüft werden. Versuche es erneut.";
      }
    } finally {
      if (mounted && current === revision) {
        checking = false;
        requesting = false;
      }
    }
  }
  async function open() {
    try {
      await openMicrophoneSettings();
    } catch {
      if (mounted)
        error =
          "Öffne Systemeinstellungen → Datenschutz → Mikrofon und erlaube Ruf den Zugriff.";
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
    <h1>Datenschutz</h1>
    <p>Systemfreigaben und Daten auf deinem Computer.</p>
  </div>
</div>
<h2 class="settings-group" id="privacy-permissions" tabindex="-1">Mikrofon</h2>
<PermissionStatus
  title="Mikrofon für Gespräche und Tests"
  {status}
  {checking}
  working={requesting}
  {error}
  requestLabel="Mikrofon erlauben"
  oncheck={() => void check()}
  onrequest={() => void check(true)}
  onopen={isTauri() ? () => void open() : undefined}
/>
<p class="settings-footnote">
  Das Öffnen dieser Seite startet keine Mikrofonaufnahme. Zugriff wird nur bei
  einem Gespräch oder einem ausdrücklich gestarteten Mikrofontest verwendet.
</p>
<ContactsSettings embedded />
<h2 class="settings-group sound-group">Lokale Daten</h2>
<div class="setting-row">
  <div class="setting-description">
    <h3>Konten & Passwörter</h3>
    <p>
      Kontodaten und Einstellungen liegen im App-Datenordner. Passwörter bleiben
      im System-Schlüsselbund und werden nicht an die Oberfläche zurückgegeben.
    </p>
  </div>
</div>
<div class="setting-row">
  <div class="setting-description">
    <h3>Kontakte & Verlauf</h3>
    <p>
      Ruf verändert das Systemadressbuch nicht. Favoriten und Anrufverlauf
      werden lokal gespeichert. Aufbewahrung und Löschung stellst du unter
      Kontakte & Verlauf ein.
    </p>
  </div>
</div>
<div class="setting-row">
  <div class="setting-description">
    <h3>Diagnose</h3>
    <p>
      Der Diagnosebericht enthält technische Zustände ohne Anruferdaten,
      Kontoadressen oder Passwörter. Er wird nur auf deine ausdrückliche Aktion
      kopiert.
    </p>
  </div>
</div>
