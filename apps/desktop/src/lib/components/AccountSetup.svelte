<script lang="ts">
  import { tick } from "svelte";
  import type { AccountConfig, RegistrationState } from "../generated/AppInfo";
  import {
    normalizeAccount,
    validateAccount,
    type AccountErrors,
    type AccountField,
  } from "../onboarding";
  import Icon from "./Icon.svelte";
  let {
    account = $bindable(),
    nativeAvailable,
    browser,
    busy,
    callIdle,
    registration,
    audioLocked,
    audioUnknown,
    initializing,
    onconfigure,
    onback,
  }: {
    account: AccountConfig;
    nativeAvailable: boolean;
    browser: boolean;
    busy: boolean;
    callIdle: boolean;
    registration: RegistrationState;
    audioLocked: boolean;
    audioUnknown: boolean;
    initializing: boolean;
    onconfigure: (account: AccountConfig) => Promise<void>;
    onback: () => void;
  } = $props();
  let errors = $state<AccountErrors>({});
  let form: HTMLFormElement;
  let panel: HTMLElement;
  let feedback = $state<HTMLElement>();
  let submitted = $state(false);
  let observedRegistration: RegistrationState | undefined;
  $effect(() => {
    const current = registration;
    if (
      submitted &&
      current !== observedRegistration &&
      (current === "registering" || current === "failed")
    ) {
      void tick().then(() => {
        const focused = document.activeElement;
        const editing =
          focused instanceof HTMLInputElement ||
          focused instanceof HTMLSelectElement;
        if (!editing && panel?.isConnected && feedback?.isConnected) {
          panel.scrollTop = 0;
          feedback.focus({ preventScroll: true });
        }
      });
    }
    observedRegistration = current;
  });
  function check(field: AccountField) {
    const message = validateAccount(normalizeAccount(account))[field];
    errors = { ...errors, [field]: message };
  }
  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!nativeAvailable || initializing || audioUnknown || busy || !callIdle)
      return;
    const normalized = normalizeAccount(account);
    errors = validateAccount(normalized);
    const first = (Object.keys(errors) as AccountField[])[0];
    if (first) {
      await tick();
      (form.elements.namedItem(first) as HTMLInputElement | null)?.focus();
      return;
    }
    submitted = true;
    await onconfigure(normalized);
  }
</script>

<section
  bind:this={panel}
  class="settings-view"
  aria-labelledby="account-heading"
>
  <div class="view-heading">
    <button
      class="icon-button back-button"
      type="button"
      aria-label="Zurück"
      onclick={onback}><Icon name="back" size={18} /></button
    >
    <h1 id="account-heading">SIP-Konto verbinden</h1>
  </div>
  <p class="view-description">
    Die Zugangsdaten bekommst du von deiner Telefonanlage oder deinem
    SIP-Anbieter.
  </p>
  {#if !nativeAvailable}<p class="inline-note">
      <Icon name="info" size={16} /><span
        >{browser
          ? "Du bist in der Browser-Vorschau. Die Einrichtung abschließen kannst du nur in der Desktop-App."
          : "Telefonie ist derzeit nicht verfügbar. Prüfe den Status oben."}</span
      >
    </p>{/if}
  {#if audioUnknown}<div class="connection-note failed">
      <p role="status"><strong>Audioquelle unbekannt</strong></p>
      <p>
        Die Telefonie läuft bereits, aber diese Ansicht kennt ihre Audioquelle
        nicht. Es kann ein Testton mit Aufzeichnung aktiv sein.
      </p>
      <p>
        Beende ein laufendes Gespräch und starte die App neu, bevor du ein Konto
        neu verbindest oder einen neuen Anruf beginnst. Die Ansicht neu zu laden
        reicht nicht aus.
      </p>
    </div>{/if}
  {#if initializing}<p class="connection-note" role="status">
      Verbindungsstatus wird geladen …
    </p>{/if}
  {#if registration === "registering"}<p
      bind:this={feedback}
      tabindex="-1"
      class="connection-note"
      role="status"
    >
      Anmeldung läuft. Dein Telefon wartet auf die Antwort der Telefonanlage.
    </p>{/if}
  {#if registration === "failed"}<div
      bind:this={feedback}
      tabindex="-1"
      class="connection-note failed"
    >
      <p role="status">
        <strong>Die Anmeldung hat nicht geklappt.</strong> Prüfe Server, Benutzername
        und Passwort. Der Benutzername ist häufig eine eigene SIP-Kennung.
      </p>
      <p>
        Die App unterstützt derzeit SIP/UDP. Benötigt deine Anlage TLS, ist
        diese Version noch nicht geeignet.
      </p>
    </div>{/if}
  {#if registration === "registered"}<p
      class="connection-note connected"
      role="status"
    >
      Dein Konto ist verbunden. Neue Angaben werden erst mit „Erneut verbinden“
      übernommen.
    </p>{/if}
  <p class="setup-scope">
    Diese Entwicklungsversion nutzt SIP/UDP ohne Verschlüsselung. Verbinde dich
    nur im isolierten Testnetz.
  </p>
  <form bind:this={form} onsubmit={submit} novalidate>
    <fieldset disabled={busy || initializing || audioUnknown || !callIdle}>
      <label for="registrar">SIP-Server</label><input
        id="registrar"
        name="registrar"
        bind:value={account.registrar}
        onblur={() => check("registrar")}
        aria-invalid={!!errors.registrar}
        aria-describedby="registrar-help"
        required
        autocomplete="off"
        autocapitalize="none"
        spellcheck="false"
        placeholder="z. B. pbx.firma.de"
      />
      <p
        id="registrar-help"
        class="field-help"
        class:field-error={!!errors.registrar}
      >
        {errors.registrar ||
          "Serveradresse, bei Bedarf mit Port: pbx.firma.de:5060"}
      </p>
      <label for="username">SIP-Benutzername</label><input
        id="username"
        name="username"
        bind:value={account.username}
        onblur={() => check("username")}
        aria-invalid={!!errors.username}
        aria-describedby="username-help"
        required
        autocomplete="off"
        autocapitalize="none"
        spellcheck="false"
        placeholder="Deine SIP-Kennung"
      />
      <p
        id="username-help"
        class="field-help"
        class:field-error={!!errors.username}
      >
        {errors.username ||
          "Die Anmeldekennung deiner Telefonanlage, nicht der Anzeigename."}
      </p>
      <label for="password">SIP-Passwort</label><input
        id="password"
        name="password"
        bind:value={account.password}
        onblur={() => check("password")}
        aria-invalid={!!errors.password}
        aria-describedby="password-help"
        type="password"
        required
        autocomplete="off"
      />
      <p
        id="password-help"
        class="field-help"
        class:field-error={!!errors.password}
      >
        {errors.password ||
          "Wird nur für diese App-Sitzung verwendet und nicht gespeichert."}
      </p>
      <button
        class="primary-button register-button"
        type="submit"
        disabled={!nativeAvailable ||
          initializing ||
          audioUnknown ||
          busy ||
          !callIdle}
        >{busy
          ? "Wird gesendet …"
          : registration === "registering"
            ? "Mit neuen Angaben verbinden"
            : registration === "registered"
              ? "Erneut verbinden"
              : "Konto verbinden"}</button
      >
    </fieldset>
  </form>
  {#if !callIdle}<p class="field-help">
      Beende das aktuelle Gespräch, bevor du das Konto änderst.
    </p>{/if}
  <details class="setup-details">
    <summary>Audio und Hinweise</summary>
    {#if audioLocked}<p>
        Die Audioquelle ist für diese App-Sitzung festgelegt. Starte die App
        neu, um sie zu ändern.
      </p>{:else}<label for="audio">Audioquelle</label><select
        id="audio"
        bind:value={account.audioMode}
        disabled={busy || !callIdle}
        ><option value="system">Mikrofon und Lautsprecher</option><option
          value="testTone">Testton für Entwicklung</option
        ></select
      >{/if}
    {#if audioUnknown}<p>
        Die tatsächliche Audioquelle kann erst nach einem App-Neustart wieder
        festgelegt werden.
      </p>{:else if account.audioMode === "testTone"}<p class="test-audio-note">
        Testton ist gewählt: Ein Ton ersetzt dein Mikrofon. Empfangenes Audio
        wird als WAV im App-Cache gespeichert.
      </p>{:else}<p>
        Standardmäßig nutzt dein Telefon Mikrofon und Lautsprecher. Erlaube den
        Mikrofonzugriff, wenn das Betriebssystem danach fragt.
      </p>{/if}
    <p>
      SIP/UDP ist unverschlüsselt. Nutze diese Entwicklungsversion vorerst in
      einem isolierten Testnetz.
    </p>
  </details>
  <details class="setup-details">
    <summary>Lokale Asterisk-Testanlage</summary>
    <p>
      Nur für Entwicklung: Starte das Labor im Projekt mit <code
        >bun run lab:up</code
      >.
    </p>
    <dl class="lab-reference">
      <dt>SIP-Server</dt>
      <dd>127.0.0.1:5070</dd>
      <dt>Benutzername</dt>
      <dd>1000</dd>
      <dt>Testpasswort</dt>
      <dd>sip-lab-only-not-a-secret</dd>
    </dl>
    <p>
      Die Zugangsdaten sind ein öffentliches Testkonto. Wähle vor dem Verbinden
      unter Audio den Testton. Nach erfolgreicher Anmeldung erreichst du den
      Echotest über Nebenstelle 600.
    </p>
  </details>
</section>
