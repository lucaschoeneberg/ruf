<script lang="ts">
  import type { CallState } from "../generated/AppInfo";
  import Icon from "./Icon.svelte";
  let {
    state,
    peer,
    elapsed,
    busy,
    compact,
    testAudio = false,
    onanswer,
    onhangup,
  }: {
    state: CallState;
    peer: string;
    elapsed: string;
    busy: boolean;
    compact: boolean;
    testAudio?: boolean;
    onanswer: () => void;
    onhangup: () => void;
  } = $props();
  const labels: Record<CallState, string> = {
    idle: "Kein Gespräch",
    calling: "Verbindung wird aufgebaut …",
    ringing: "Es klingelt …",
    incoming: "Eingehender Anruf",
    active: "Gespräch läuft",
  };
</script>

<section
  class="call-panel"
  class:compact-call={compact}
  aria-label="Aktuelles Gespräch"
>
  <div class="call-details" role="status" aria-live="polite">
    <div class="call-symbol">
      <Icon name="phone" size={compact ? 18 : 28} />
    </div>
    <p class="call-state">{labels[state]}</p>
    <h1 class="call-peer">{peer}</h1>
  </div>
  {#if state === "active"}<p class="call-timer" aria-label="Gesprächsdauer">
      {elapsed}
    </p>{/if}
  {#if testAudio}<p class="call-test-note">
      Testton · empfangenes Audio wird aufgezeichnet
    </p>{/if}
  <div class="call-actions">
    {#if state === "incoming"}<button
        class="primary-button"
        disabled={busy}
        onclick={onanswer}><Icon name="phone" size={18} />Annehmen</button
      >{/if}
    <button class="danger-button" disabled={busy} onclick={onhangup}
      ><Icon name="endCall" size={20} />{state === "incoming"
        ? "Ablehnen"
        : "Auflegen"}</button
    >
  </div>
</section>
