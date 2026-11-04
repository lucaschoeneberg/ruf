<script lang="ts">
  import { onMount, tick } from "svelte";
  import { insertDigit, removeDigit, MAX_NUMBER_LENGTH } from "../dialing";
  import Icon from "./Icon.svelte";

  let {
    value = $bindable(""),
    canCall,
    busy,
    registered,
    connectionHint,
    setupLabel,
    setupEnabled,
    oncall,
    onsetup,
  }: {
    value?: string;
    canCall: boolean;
    busy: boolean;
    registered: boolean;
    connectionHint: string;
    setupLabel: string;
    setupEnabled: boolean;
    oncall: () => void;
    onsetup: () => void;
  } = $props();
  let input: HTMLInputElement;
  let holdTimer: ReturnType<typeof setTimeout> | undefined;
  let holdPointer: number | null = null;
  let plusInserted = false;
  const invalidNumber = $derived(
    value.length > 0 && !/^[0-9+*#]{1,64}$/.test(value),
  );
  const keys = [
    ["1", ""],
    ["2", "ABC"],
    ["3", "DEF"],
    ["4", "GHI"],
    ["5", "JKL"],
    ["6", "MNO"],
    ["7", "PQRS"],
    ["8", "TUV"],
    ["9", "WXYZ"],
    ["*", ""],
    ["0", "+"],
    ["#", ""],
  ] as const;

  onMount(() => {
    input.focus();
    return cancelHold;
  });
  function cancelHold(event?: PointerEvent) {
    if (event && event.pointerId !== holdPointer) return;
    clearTimeout(holdTimer);
    holdPointer = null;
  }
  function resetHold(event?: PointerEvent) {
    if (event && event.pointerId !== holdPointer) return;
    cancelHold();
    plusInserted = false;
  }
  async function edit(digit: string | null, focus: boolean) {
    const start = input.selectionStart ?? value.length;
    const end = input.selectionEnd ?? value.length;
    const change =
      digit === null
        ? removeDigit(value, start, end)
        : insertDigit(value, digit, start, end);
    value = change.value;
    await tick();
    if (focus) input.focus();
    input.setSelectionRange(change.caret, change.caret);
  }
  function beginHold(event: PointerEvent, digit: string) {
    if (!event.isPrimary || event.button !== 0 || holdPointer !== null) return;
    cancelHold();
    plusInserted = false;
    if (digit === "0") {
      holdPointer = event.pointerId;
      holdTimer = setTimeout(() => {
        plusInserted = true;
        void edit("+", true);
      }, 550);
    }
  }
</script>

<svelte:window onblur={() => resetHold()} />

<form
  class="dial-view"
  onsubmit={(event) => {
    event.preventDefault();
    if (canCall && !busy) oncall();
  }}
>
  <h1 class="sr-only">Telefonnummer wählen</h1>
  <div class="number-block">
    <div class="number-entry">
      <input
        bind:this={input}
        bind:value
        aria-label="Nummer oder Nebenstelle"
        type="tel"
        inputmode="tel"
        autocomplete="off"
        spellcheck="false"
        placeholder="Nummer eingeben"
        maxlength={MAX_NUMBER_LENGTH}
        aria-invalid={invalidNumber}
        aria-describedby="number-hint"
        disabled={busy}
      />
      <button
        class="icon-button erase-number"
        type="button"
        aria-label="Ziffer löschen"
        title="Ziffer löschen"
        disabled={!value || busy}
        onclick={(event) => edit(null, event.detail > 0)}
        ><Icon name="backspace" size={19} /></button
      >
    </div>
    <p id="number-hint" class="number-hint" aria-live="polite">
      {invalidNumber
        ? "Erlaubt sind Ziffern, +, * und #"
        : value
          ? "Nummer oder Nebenstelle"
          : "Direkt tippen oder Ziffern wählen"}
    </p>
  </div>
  <div class="keypad" aria-label="Wähltastatur">
    {#each keys as [digit, letters]}
      <button
        class="dial-key"
        class:symbol-key={digit === "*" || digit === "#"}
        type="button"
        aria-label={digit === "0"
          ? "0, gedrückt halten für Plus"
          : letters
            ? `${digit}, ${letters}`
            : digit}
        disabled={busy}
        onpointerdown={(event) => beginHold(event, digit)}
        onpointerup={cancelHold}
        onpointerleave={resetHold}
        onpointercancel={resetHold}
        onclick={(event) => {
          if (!plusInserted) void edit(digit, event.detail > 0);
          plusInserted = false;
        }}
      >
        <span class="key-digit">{digit}</span><span
          class="key-letters"
          aria-hidden="true">{letters || "\u00a0"}</span
        >
      </button>
    {/each}
  </div>
  <button
    class="primary-button call-button"
    type="submit"
    disabled={!canCall || busy}><Icon name="phone" size={18} />Anrufen</button
  >
  <div class="dial-help">
    {#if !registered}<button
        type="button"
        class="text-button"
        disabled={!setupEnabled}
        onclick={onsetup}>{setupLabel}</button
      >{:else}<span role="status">{connectionHint}</span>{/if}
  </div>
</form>
