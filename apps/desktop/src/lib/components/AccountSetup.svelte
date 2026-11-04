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
    busy,
    disabled,
    onconfigure,
    onremove,
  }: {
    account: AccountConfig;
    name: string;
    existing: AccountSnapshot | null;
    busy: boolean;
    disabled: boolean;
    onconfigure: (account: AccountConfig, name: string) => Promise<void>;
    onremove: () => void;
  } = $props();
  let errors = $state<AccountErrors>({});
  let nameError = $state("");
  let form: HTMLFormElement;
  let confirmRemove = $state(false);
  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (disabled || busy) return;
    const normalized = normalizeAccount(account);
    errors = validateAccount(normalized, existing !== null);
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
      (form.elements.namedItem(first) as HTMLInputElement | null)?.focus();
      return;
    }
    await onconfigure(normalized, name.trim());
  }
  const status = $derived(
    existing?.removing
      ? "Wird entfernt …"
      : existing?.registration === "registered"
        ? "Verbunden"
        : existing?.registration === "registering"
          ? "Wird verbunden …"
          : existing?.registration === "failed"
            ? "Anmeldung fehlgeschlagen"
            : "Noch nicht verbunden",
  );
</script>

<div class="settings-title">
  <div>
    <h1>{existing ? existing.name : "Konto hinzufügen"}</h1>
    <p>
      {existing
        ? `${existing.username}@${existing.registrar}`
        : "Verbinde eine Telefonanlage oder deinen SIP-Anbieter."}
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
    {errorMessage(existing.lastError)} Prüfe die Zugangsdaten und verbinde das Konto
    erneut.
  </p>{/if}

<form bind:this={form} onsubmit={submit} novalidate>
  <fieldset disabled={disabled || busy || existing?.removing}>
    <h2 class="settings-group">Kontoinformationen</h2>
    <div class="setting-row">
      <div class="setting-description">
        <label for="name">Name</label>
        <p id="name-help">So erscheint das Konto in deinem Telefon.</p>
      </div>
      <div class="setting-control">
        <input
          id="name"
          name="name"
          bind:value={name}
          placeholder="z. B. Büro"
          maxlength="64"
          autocomplete="off"
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
          Serveradresse deiner Telefonanlage, optional mit Port.
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
          Deine SIP-Anmeldekennung. Häufig eine Nebenstelle.
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
          {existing
            ? "Leer lassen, um das gespeicherte Passwort zu behalten."
            : "Wird sicher im System-Schlüsselbund gespeichert."}
        </p>
      </div>
      <div class="setting-control">
        <input
          id="password"
          name="password"
          bind:value={account.password}
          type="password"
          placeholder={existing ? "Gespeichertes Passwort" : "SIP-Passwort"}
          autocomplete="new-password"
          required={!existing}
          aria-invalid={!!errors.password}
          aria-describedby={errors.password
            ? "password-error"
            : "password-help"}
        />{#if errors.password}<p id="password-error" class="field-error">
            {errors.password}
          </p>{/if}
      </div>
    </div>
    <div class="settings-form-actions">
      <p>Änderungen werden mit dem Speichern übernommen.</p>
      <button class="primary-button" type="submit"
        >{busy
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
