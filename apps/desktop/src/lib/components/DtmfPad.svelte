<script lang="ts">
  import { t } from "../i18n";
  import { onMount } from "svelte";
  let {
    disabled,
    onsend,
  }: { disabled: boolean; onsend: (digit: string) => void } = $props();
  const keys = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "*", "0", "#"];
  onMount(() => {
    const keydown = (event: KeyboardEvent) => {
      const target = event.target;
      if (
        disabled ||
        event.defaultPrevented ||
        document.querySelector("dialog[open]") ||
        event.repeat ||
        event.metaKey ||
        event.ctrlKey ||
        event.altKey ||
        (target instanceof HTMLElement &&
          (target.isContentEditable || target.closest("input,textarea,select")))
      )
        return;
      if (/^[0-9*#]$/.test(event.key)) {
        event.preventDefault();
        onsend(event.key);
      }
    };
    window.addEventListener("keydown", keydown);
    return () => window.removeEventListener("keydown", keydown);
  });
</script>

<div class="dtmf-panel">
  <p class="number-hint">
    {$t("phone.dtmf.hint")}
  </p>
  <div class="keypad dtmf-keypad" aria-label={$t("phone.dtmf.keypad")}>
    {#each keys as digit}<button
        type="button"
        class="dial-key"
        {disabled}
        aria-label={$t("phone.dtmf.tone", { digit })}
        onclick={() => onsend(digit)}
        ><span class="key-digit">{digit}</span></button
      >{/each}
  </div>
</div>
