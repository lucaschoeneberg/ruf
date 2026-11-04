<script lang="ts">
  import type { RegistrationState } from "../generated/AppInfo";
  import Icon from "./Icon.svelte";
  let {
    runtime,
    registration,
    onsetup,
    onexplore,
  }: {
    runtime: "desktop" | "browser" | "unlinked" | "unavailable";
    registration: RegistrationState;
    onsetup: () => void;
    onexplore: () => void;
  } = $props();
  const ready = $derived(registration === "registered");
</script>

<section class="getting-started" aria-labelledby="start-title">
  <div class="start-symbol"><Icon name="phone" size={28} /></div>
  <h1 id="start-title">
    {runtime === "browser"
      ? "Lerne dein Telefon kennen"
      : runtime === "unlinked"
        ? "Telefonie ist noch nicht verfügbar"
        : runtime === "unavailable"
          ? "Telefonstatus nicht verfügbar"
          : ready
            ? "Dein Telefon ist verbunden"
            : "Verbinde dein SIP-Konto"}
  </h1>
  <p class="start-description">
    {runtime === "browser"
      ? "Hier kannst du Tastatur und Navigation ausprobieren. Zum Telefonieren brauchst du die Desktop-App."
      : runtime === "unlinked"
        ? "Du kannst die Oberfläche ansehen. Für echte Anrufe benötigst du einen Desktop-Build mit nativer Telefonie."
        : runtime === "unavailable"
          ? "Der Verbindungsstatus konnte nicht geladen werden. Prüfe die Meldung oben und versuche es erneut."
          : ready
            ? "Wähle eine Nummer oder Nebenstelle. Mit Anrufen oder der Eingabetaste startest du dein Gespräch."
            : "Mit den Zugangsdaten deiner Telefonanlage kannst du anschließend Nummern und Nebenstellen anrufen."}
  </p>
  {#if runtime === "desktop" && !ready}
    <p class="start-requirements">
      <strong>Du brauchst:</strong> SIP-Server, Benutzername und Passwort deiner Telefonanlage.
    </p>
  {:else if ready}
    <p class="start-note">
      Für eine internationale Nummer: 0 gedrückt halten, um ein Plus einzugeben.
    </p>
  {/if}
  <div class="start-actions">
    {#if runtime === "desktop" && !ready}<button
        class="primary-button"
        onclick={onsetup}>SIP-Konto einrichten</button
      ><button class="text-button" onclick={onexplore}
        >Erst die Tastatur ansehen</button
      >
    {:else}<button class="primary-button" onclick={onexplore}
        >{ready ? "Nummer wählen" : "Tastatur ausprobieren"}</button
      ><button class="text-button" onclick={onsetup}
        >{ready ? "Kontoeinstellungen öffnen" : "Einrichtung ansehen"}</button
      >{/if}
  </div>
  <p class="start-note">Starthilfe jederzeit oben öffnen.</p>
</section>
