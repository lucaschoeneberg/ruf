<script lang="ts">
  import { onMount } from "svelte";
  import {
    answer,
    configureAccount,
    dial,
    errorMessage,
    getAppInfo,
    getEngineSnapshot,
    hangup,
  } from "./lib/api";
  import type {
    AppInfo,
    AudioMode,
    EngineSnapshot,
  } from "./lib/generated/AppInfo";

  let info = $state<AppInfo | null>(null);
  let snapshot = $state<EngineSnapshot | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let registrar = $state("127.0.0.1:5070");
  let username = $state("1000");
  let password = $state("");
  let audioMode = $state<AudioMode>("testTone");
  let destination = $state("600");
  const callIdle = $derived(!snapshot || snapshot.call === "idle");

  onMount(() => {
    let disposed = false;
    let polling = false;
    const timer = setInterval(() => {
      if (disposed || polling || busy || info?.telephony !== "linked") return;
      polling = true;
      void getEngineSnapshot()
        .then(
          (result) => {
            if (!disposed) snapshot = result;
          },
          (reason: unknown) => {
            if (!disposed) error = errorMessage(reason);
          },
        )
        .finally(() => {
          polling = false;
        });
    }, 500);
    void getAppInfo().then(
      (result) => {
        if (!disposed) info = result;
      },
      (reason: unknown) => {
        if (!disposed) error = errorMessage(reason);
      },
    );
    return () => {
      disposed = true;
      clearInterval(timer);
    };
  });

  async function perform(operation: () => Promise<EngineSnapshot>) {
    busy = true;
    error = null;
    try {
      snapshot = await operation();
    } catch (reason: unknown) {
      error = errorMessage(reason);
    } finally {
      busy = false;
    }
  }

  async function register(event: SubmitEvent) {
    event.preventDefault();
    const account = { registrar, username, password, audioMode };
    password = "";
    await perform(() => configureAccount(account));
  }
</script>

<main>
  <header>
    <p class="eyebrow">SIP · Lokaler Telefonietest</p>
    <h1>Der erste Anruf.</h1>
    <p>Ein Konto, ein Gespräch – gegen deine lokale Testanlage.</p>
    <p class="notice">
      Entwicklungsmodus: SIP/UDP ohne Verschlüsselung. Nur in einem isolierten
      Testnetz verwenden.
    </p>
  </header>

  <div role="status" aria-live="polite">
    {#if !info && !error}<p>Desktop-Status wird geladen …</p>{/if}
    {#if info?.telephony === "notLinked"}<p class="notice">
        baresip ist nicht verbunden. Native SDK bauen und mit <code
          >bun run dev:native</code
        > starten.
      </p>{/if}
    {#if error}<p class="notice">{error}</p>{/if}
    {#if snapshot?.lastError}<p class="notice">
        {snapshot.lastError.message}
      </p>{/if}
  </div>

  {#if info?.telephony === "linked"}
    <section aria-labelledby="account-title">
      <h2 id="account-title">Testkonto anmelden</h2>
      <form onsubmit={register}>
        <fieldset disabled={busy || !callIdle}>
          <label
            >Registrar<input
              bind:value={registrar}
              required
              autocomplete="off"
              placeholder="127.0.0.1:5070"
            /></label
          >
          <label
            >Benutzername<input
              bind:value={username}
              required
              autocomplete="off"
            /></label
          >
          <label
            >Passwort<input
              bind:value={password}
              type="password"
              required
              autocomplete="off"
            /></label
          >
          <label
            >Audio<select bind:value={audioMode}
              ><option value="testTone"
                >Testton senden, Rückweg aufzeichnen</option
              ><option value="system">Mikrofon und Lautsprecher</option></select
            ></label
          >
          <button type="submit">Anmelden</button>
        </fieldset>
      </form>
      <p>
        Das Konto bleibt nur im Arbeitsspeicher. Der Testton-Modus benötigt kein
        Mikrofon; empfangenes Audio wird im App-Cache als received.wav
        gespeichert.
      </p>
    </section>

    <section aria-labelledby="call-title">
      <h2 id="call-title">Anruf</h2>
      <dl>
        <div>
          <dt>Registrierung</dt>
          <dd>{snapshot?.registration ?? "unconfigured"}</dd>
        </div>
        <div>
          <dt>Gespräch</dt>
          <dd>{snapshot?.call ?? "idle"}</dd>
        </div>
      </dl>
      <form
        onsubmit={(event) => {
          event.preventDefault();
          void perform(() => dial(destination));
        }}
      >
        <label
          >Nummer oder Nebenstelle<input
            bind:value={destination}
            inputmode="tel"
            required
            disabled={busy || !callIdle}
          /></label
        >
        <div class="actions">
          <button
            type="submit"
            disabled={busy ||
              !callIdle ||
              snapshot?.registration !== "registered"}>Anrufen</button
          >
          <button
            type="button"
            onclick={() => perform(answer)}
            disabled={busy || snapshot?.call !== "incoming"}>Annehmen</button
          >
          <button
            type="button"
            onclick={() => perform(hangup)}
            disabled={busy || callIdle}>Auflegen / ablehnen</button
          >
        </div>
      </form>
    </section>
  {/if}
</main>
