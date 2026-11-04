<script lang="ts">
  import type { CallState } from "../generated/AppInfo";
  import Icon from "./Icon.svelte";
  let {
    state,
    peer,
    number = null,
    elapsed,
    busy,
    compact,
    testAudio = false,
    audioUnknown = false,
    receivedDtmf = null,
    onanswer,
    onhangup,
  }: {
    state: CallState;
    peer: string;
    number?: string | null;
    elapsed: string;
    busy: boolean;
    compact: boolean;
    testAudio?: boolean;
    audioUnknown?: boolean;
    receivedDtmf?: string | null;
    onanswer: () => void;
    onhangup: () => void;
  } = $props();
  const labels: Record<CallState, string> = {
    idle: "Kein Gespräch",
    calling: "Verbindung wird aufgebaut …",
    ringing: "Es klingelt …",
    earlyMedia: "Frühe Medienverbindung",
    incoming: "Eingehender Anruf",
    answering: "Wird angenommen …",
    active: "Gespräch läuft",
    held: "Gespräch gehalten",
  };
</script>

<section
  class="call-panel"
  class:compact-call={compact}
  aria-label="Aktuelles Gespräch"
>
  <div class="call-details" role="status" aria-live="polite">
    <div class="call-symbol">
      <Icon name="phone" size={compact ? 16 : 22} />
    </div>
    <p class="call-state">{labels[state]}</p>
    <h1 class="call-peer">{peer}</h1>
    {#if number && number !== peer}<p class="call-number">{number}</p>{/if}
  </div>
  {#if state === "active" || state === "held"}<p
      class="call-timer"
      aria-label="Gesprächsdauer"
    >
      {elapsed}
    </p>{/if}
  {#if receivedDtmf}<p class="call-test-note">
      Empfangener Tastenton: {receivedDtmf}
    </p>{/if}
  {#if audioUnknown}<p class="call-test-note">
      Audioquelle unbekannt. Es kann ein Testton mit Aufzeichnung aktiv sein.
      Beende das Gespräch und starte die App neu.
    </p>{/if}
  {#if testAudio}<p class="call-test-note">
      Testton · empfangenes Audio wird aufgezeichnet
    </p>{/if}
  <div class="call-actions">
    {#if state === "incoming"}<button
        class="primary-button"
        disabled={busy || audioUnknown}
        onclick={onanswer}><Icon name="phone" size={18} />Annehmen</button
      >{/if}
    <button class="danger-button" disabled={busy} onclick={onhangup}
      ><Icon name="endCall" size={20} />{state === "incoming"
        ? "Ablehnen"
        : "Auflegen"}</button
    >
  </div>
</section>
