<script lang="ts">
  import { onMount } from "svelte";
  import { getAppInfo } from "./lib/api";
  import type { AppInfo } from "./lib/generated/AppInfo";

  let info = $state<AppInfo | null>(null);
  let error = $state<string | null>(null);

  onMount(() => {
    void getAppInfo().then(
      (result) => {
        info = result;
      },
      (reason: unknown) => {
        error =
          reason instanceof Error
            ? reason.message
            : "Der native Status konnte nicht geladen werden.";
      },
    );
  });
</script>

<main>
  <header>
    <p class="eyebrow">SIP · Projektstart</p>
    <h1>Die Grundlage steht.</h1>
    <p>Hier entsteht ein schlankes Telefon für macOS und Windows.</p>
  </header>

  <section aria-labelledby="status-title">
    <h2 id="status-title">Entwicklungsstand</h2>
    <p>
      Als Nächstes verbinden wir die Telefonie-Engine. Anrufe sind noch nicht
      möglich.
    </p>
    <div role="status" aria-live="polite">
      {#if error}
        <p class="notice">{error}</p>
      {:else if info}
        <dl>
          <div>
            <dt>App-Version</dt>
            <dd>{info.version}</dd>
          </div>
          <div>
            <dt>Telefonie</dt>
            <dd>
              {info.telephony === "notLinked"
                ? "baresip noch nicht verbunden"
                : "Unbekannter Status"}
            </dd>
          </div>
        </dl>
      {:else}
        <p>Desktop-Status wird geladen …</p>
      {/if}
    </div>
  </section>
</main>
