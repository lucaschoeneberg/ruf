<script lang="ts">
  import { tick } from "svelte";
  import type { AccountConfig, AccountSnapshot } from "../generated/AppInfo";
  import {
    normalizeAccount,
    validateAccount,
    type AccountErrors,
    type AccountField,
  } from "../onboarding";
  import { errorMessage } from "../api";
  let {
    account = $bindable(),
    name = $bindable(),
    existing,
    removed = false,
    embedded = false,
    busy,
    disabled,
    onconfigure,
    onremove,
    onreconnect,
  }: {
    account: AccountConfig;
    name: string;
    existing: AccountSnapshot | null;
    removed?: boolean;
    embedded?: boolean;
    busy: boolean;
    disabled: boolean;
    onconfigure: (account: AccountConfig, name: string) => Promise<void>;
    onremove: () => void;
    onreconnect: () => void;
  } = $props();
  let errors = $state<AccountErrors>({});
  let nameError = $state("");
  let form: HTMLFormElement;
  let confirmRemove = $state(false);
  const missingCredential = $derived(
    existing?.lastError?.code === "credentialMissing",
  );
  const canReconnect = $derived(
    existing?.enabled !== false &&
      existing?.registration === "failed" &&
      existing.lastError?.code !== "credentialMissing" &&
      existing.lastError?.code !== "storageFailure",
  );
  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (disabled || busy || removed) return;
    const normalized = normalizeAccount(account);
    errors = validateAccount(
      normalized,
      existing !== null && !missingCredential,
    );
    nameError =
      !name.trim() ||
      [...name.trim()].length > 64 ||
      /[\u0000-\u001f\u007f]/u.test(name)
        ? "Gib einen Namen mit 1–64 Zeichen an."
        : "";
    const first = nameError
      ? "name"
      : (Object.keys(errors) as AccountField[])[0];
    if (first) {
      await tick();
      const control = form.elements.namedItem(first) as
        HTMLInputElement | HTMLSelectElement | null;
      const details = control?.closest("details");
      if (details) details.open = true;
      control?.focus();
      return;
    }
    await onconfigure(normalized, name.trim());
  }
  const status = $derived.by((): string => {
    if (existing?.enabledChanging) return "Wird geändert …";
    if (existing?.enabled === false) return "Deaktiviert";
    if (existing?.removing) return "Wird entfernt …";
    switch (existing?.registration) {
      case "registered":
        return "Angemeldet";
      case "registering":
        return "Wird angemeldet …";
      case "failed":
        return "Anmeldung fehlgeschlagen";
      default:
        return "Noch nicht angemeldet";
    }
  });
</script>

<div class="settings-title">
  <div>
    <svelte:element this={embedded ? "h2" : "h1"}>
      {removed
        ? "Konto entfernt"
        : existing
          ? existing.name
          : "Konto hinzufügen"}
    </svelte:element>
    <p>
      {removed
        ? `${account.username}@${account.registrar}`
        : existing
          ? `${existing.username}@${existing.registrar}`
          : "SIP-Server, Benutzername und Passwort erhältst du von deiner Telefonanlage oder deinem SIP-Anbieter. Wähle außerdem einen Namen für das Konto."}
    </p>
  </div>
  {#if existing}<span
      class="account-status"
      class:connected={existing.registration === "registered"}
      class:failed={existing.registration === "failed"}
      role="status"
      ><span class="status-dot" aria-hidden="true"></span>{status}</span
    >{/if}
</div>
{#if existing?.lastError}<p class="settings-feedback error" role="status">
    {errorMessage(existing.lastError)}
  </p>{/if}
{#if canReconnect}<button
    class="secondary-button"
    type="button"
    disabled={disabled || busy || removed || existing?.removing}
    onclick={onreconnect}>Erneut verbinden</button
  >{/if}

<form bind:this={form} onsubmit={submit} novalidate>
  <fieldset disabled={disabled || busy || removed || existing?.removing}>
    <svelte:element this={embedded ? "h3" : "h2"} class="settings-group"
      >Kontoinformationen</svelte:element
    >
    <div class="setting-row">
      <div class="setting-description">
        <label for="name">Name</label>
        <p id="name-help">Erforderlich: 1–64 Zeichen, z. B. Büro.</p>
      </div>
      <div class="setting-control">
        <input
          id="name"
          name="name"
          bind:value={name}
          placeholder="z. B. Büro"
          maxlength="64"
          autocomplete="off"
          required
          aria-invalid={!!nameError}
          aria-describedby={nameError ? "name-error" : "name-help"}
        />{#if nameError}<p id="name-error" class="field-error">
            {nameError}
          </p>{/if}
      </div>
    </div>
    <div class="setting-row">
      <div class="setting-description">
        <label for="registrar">SIP-Server</label>
        <p id="registrar-help">
          Erforderlich: Serveradresse, optional mit Port. Ohne sip: oder Pfad.
        </p>
      </div>
      <div class="setting-control">
        <input
          id="registrar"
          name="registrar"
          bind:value={account.registrar}
          placeholder="pbx.firma.de:5060"
          autocomplete="off"
          autocapitalize="none"
          spellcheck="false"
          required
          aria-invalid={!!errors.registrar}
          aria-describedby={errors.registrar
            ? "registrar-error"
            : "registrar-help"}
        />{#if errors.registrar}<p id="registrar-error" class="field-error">
            {errors.registrar}
          </p>{/if}
      </div>
    </div>
    <div class="setting-row">
      <div class="setting-description">
        <label for="username">Benutzername</label>
        <p id="username-help">
          Erforderlich: SIP-Benutzername aus deinen Zugangsdaten, häufig eine
          Nebenstelle. Keine E-Mail-Adresse.
        </p>
      </div>
      <div class="setting-control">
        <input
          id="username"
          name="username"
          bind:value={account.username}
          placeholder="z. B. 1000"
          autocomplete="off"
          autocapitalize="none"
          spellcheck="false"
          required
          aria-invalid={!!errors.username}
          aria-describedby={errors.username
            ? "username-error"
            : "username-help"}
        />{#if errors.username}<p id="username-error" class="field-error">
            {errors.username}
          </p>{/if}
      </div>
    </div>
    <div class="setting-row">
      <div class="setting-description">
        <label for="password">Passwort</label>
        <p id="password-help">
          {missingCredential
            ? "Gib das Passwort ein und speichere das Konto."
            : existing || removed
              ? "Leer lassen, um das gespeicherte Passwort zu behalten."
              : "Erforderlich. Wird im System-Schlüsselbund gespeichert."}
        </p>
      </div>
      <div class="setting-control">
        <input
          id="password"
          name="password"
          bind:value={account.password}
          type="password"
          placeholder={(existing || removed) && !missingCredential
            ? "Gespeichertes Passwort"
            : "SIP-Passwort"}
          autocomplete="new-password"
          required={missingCredential || (!existing && !removed)}
          aria-invalid={!!errors.password}
          aria-describedby={errors.password
            ? "password-error"
            : "password-help"}
        />{#if errors.password}<p id="password-error" class="field-error">
            {errors.password}
          </p>{/if}
      </div>
    </div>
    <svelte:element this={embedded ? "h3" : "h2"} class="settings-group"
      >Verbindung & Sicherheit</svelte:element
    >
    <div class="setting-row">
      <div class="setting-description">
        <label for="transport">SIP-Transport</label>
        <p id="transport-help">
          TLS prüft das Zertifikat und den Servernamen. Ruf wechselt nicht
          automatisch auf eine unverschlüsselte Verbindung.
        </p>
      </div>
      <div class="setting-control">
        <select
          id="transport"
          name="transport"
          bind:value={account.transport}
          aria-describedby="transport-help"
          ><option value="udp">UDP</option><option value="tcp">TCP</option
          ><option value="tls">TLS</option></select
        >
      </div>
    </div>
    <div class="setting-row">
      <div class="setting-description">
        <label for="media-security">Audioschutz</label>
        <p id="media-security-help">
          Empfohlen: TLS mit erforderlichem SRTP, sofern deine Telefonanlage es
          unterstützt. Schutz bis zur Anlage, keine Garantie für
          Ende-zu-Ende-Verschlüsselung.
        </p>
      </div>
      <div class="setting-control">
        <select
          id="media-security"
          name="mediaSecurity"
          bind:value={account.mediaSecurity}
          aria-invalid={!!errors.mediaSecurity}
          aria-describedby={errors.mediaSecurity
            ? "media-security-error"
            : "media-security-help"}
          ><option value="disabled">Deaktiviert</option><option value="required"
            >SRTP erforderlich</option
          ></select
        >{#if errors.mediaSecurity}<p
            id="media-security-error"
            class="field-error"
          >
            {errors.mediaSecurity}
          </p>{/if}
      </div>
    </div>
    <p class="settings-trust-note">
      Private Telefonanlagen: Vertraue deren Zertifikaten bewusst über die
      Vertrauenseinstellungen deines Betriebssystems. Ruf verwendet diese
      Systemfreigaben; die Zertifikatsprüfung bleibt aktiv.
    </p>
    <details class="account-advanced">
      <summary>Weitere Verbindungsangaben</summary>
      <div class="setting-row">
        <div class="setting-description">
          <label for="auth-username">Separate Anmeldekennung</label>
          <p id="auth-username-help">
            Optional, falls dein Anbieter eine eigene Kennung für die Anmeldung
            vorgibt. Ohne Angabe wird der SIP-Benutzername verwendet.
          </p>
        </div>
        <div class="setting-control">
          <input
            id="auth-username"
            name="authUsername"
            value={account.authUsername ?? ""}
            oninput={(event) =>
              (account.authUsername = event.currentTarget.value || null)}
            maxlength="80"
            autocomplete="off"
            autocapitalize="none"
            spellcheck="false"
            aria-invalid={!!errors.authUsername}
            aria-describedby={errors.authUsername
              ? "auth-username-error"
              : "auth-username-help"}
          />{#if errors.authUsername}<p
              id="auth-username-error"
              class="field-error"
            >
              {errors.authUsername}
            </p>{/if}
        </div>
      </div>
      <div class="setting-row">
        <div class="setting-description">
          <label for="outbound-proxy">Outbound-Proxy</label>
          <p id="outbound-proxy-help">
            Optional: Serveradresse mit optionalem Port. Ohne sip:, Pfad oder
            Parameter.
          </p>
        </div>
        <div class="setting-control">
          <input
            id="outbound-proxy"
            name="outboundProxy"
            value={account.outboundProxy ?? ""}
            oninput={(event) =>
              (account.outboundProxy = event.currentTarget.value || null)}
            placeholder="proxy.firma.de:5061"
            maxlength="255"
            autocomplete="off"
            autocapitalize="none"
            spellcheck="false"
            aria-invalid={!!errors.outboundProxy}
            aria-describedby={errors.outboundProxy
              ? "outbound-proxy-error"
              : "outbound-proxy-help"}
          />{#if errors.outboundProxy}<p
              id="outbound-proxy-error"
              class="field-error"
            >
              {errors.outboundProxy}
            </p>{/if}
        </div>
      </div>
    </details>
    <div class="settings-form-actions settings-save-bar">
      <p>Änderungen werden mit dem Speichern übernommen.</p>
      <button class="primary-button" type="submit"
        >{removed
          ? "Konto entfernt"
          : busy
            ? "Wird gespeichert …"
            : existing
              ? "Speichern & verbinden"
              : "Konto hinzufügen"}</button
      >
    </div>
  </fieldset>
</form>
{#if existing}<div class="account-removal">
    {#if confirmRemove}<p id="remove-question">
        „{existing.name}“ und das gespeicherte Passwort entfernen?
      </p>
      <div class="removal-actions">
        <button
          class="secondary-button"
          type="button"
          onclick={() => (confirmRemove = false)}>Abbrechen</button
        ><button
          class="danger-button"
          type="button"
          disabled={disabled || busy || existing.removing}
          aria-describedby="remove-question"
          onclick={onremove}>Konto entfernen</button
        >
      </div>
    {:else}<button
        class="text-button remove-link"
        type="button"
        disabled={disabled || busy || existing.removing}
        onclick={() => (confirmRemove = true)}>Konto entfernen …</button
      >{/if}
  </div>{/if}
