<script lang="ts">
  import { onMount, tick } from "svelte";
  import { insertDigit, removeDigit, MAX_NUMBER_LENGTH } from "../dialing";
  import { dialSuggestions } from "../contacts";
  import { normalizeDialNumber } from "../dialing";
  import type { Contact } from "../generated/AppInfo";
  import Icon from "./Icon.svelte";

  let {
    value = $bindable(""),
    canCall,
    busy,
    registered,
    connectionHint,
    setupLabel,
    setupEnabled,
    contacts = [],
    matchGermanNumbers = true,
    contactsHint = null,
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
    contacts?: readonly Contact[];
    matchGermanNumbers?: boolean;
    contactsHint?: string | null;
    oncall: () => void;
    onsetup: () => void;
  } = $props();
  let input: HTMLInputElement;
  let holdTimer: ReturnType<typeof setTimeout> | undefined;
  let holdPointer: number | null = null;
  let plusInserted = false;
  const invalidNumber = $derived(
    value.trim().length > 0 &&
      value.trim() !== "+" &&
      normalizeDialNumber(value) === null,
  );
  const suggestions = $derived(
    dialSuggestions(contacts, value, matchGermanNumbers),
  );
  const keys = [
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "*",
    "0",
    "#",
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
        maxlength={MAX_NUMBER_LENGTH * 2}
        onbeforeinput={(event) => {
          if (
            event.inputType === "insertText" &&
            event.data &&
            /\p{L}/u.test(event.data)
          )
            event.preventDefault();
        }}
        aria-invalid={invalidNumber}
        aria-describedby="number-hint"
        disabled={busy}
      />
      <button
        class="icon-button"
        type="button"
        aria-label="Plus einfügen"
        title="Plus einfügen"
        disabled={busy}
        onclick={() => edit("+", true)}>+</button
      >
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
        ? "Ziffern und Leerzeichen; + nur am Anfang. Keine Buchstaben."
        : value
          ? "Nummer oder Nebenstelle"
          : "Direkt tippen oder Ziffern wählen"}
    </p>
  </div>
  {#if suggestions.length}<ul
      class="dial-suggestions"
      aria-label="Kontaktvorschläge"
    >
      {#each suggestions as suggestion (`${suggestion.contact.id}:${suggestion.phone.destination}`)}<li
        >
          <button
            class="secondary-button"
            type="button"
            disabled={busy}
            onclick={() => {
              if (suggestion.phone.destination)
                value = suggestion.phone.destination;
            }}
            ><strong>{suggestion.contact.name}</strong><span
              >{suggestion.phone.label} · {suggestion.phone.number}</span
            ></button
          >
        </li>{/each}
    </ul>{:else if contactsHint && value}<p class="number-hint">
      {contactsHint}
    </p>{/if}
  <div class="keypad" aria-label="Wähltastatur">
    {#each keys as digit}
      <button
        class="dial-key"
        class:symbol-key={digit === "*" || digit === "#"}
        type="button"
        aria-label={digit === "0" ? "0, gedrückt halten für Plus" : digit}
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
          aria-hidden="true">{digit === "0" ? "+" : "\u00a0"}</span
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
