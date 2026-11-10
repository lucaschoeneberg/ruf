<script lang="ts">
  import { t, text, message, type MessageValue } from "../i18n";
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
  let nameError = $state<MessageValue>("");
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
        ? message("settings.name_invalid")
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
  const status = $derived.by((): MessageValue => {
    if (existing?.enabledChanging) return message("settings.changing");
    if (existing?.enabled === false) return message("settings.disabled");
    if (existing?.removing) return message("settings.removing");
    switch (existing?.registration) {
      case "registered":
        return message("settings.registered");
      case "registering":
        return message("settings.registering");
      case "failed":
        return message("settings.registration_failed");
      default:
        return message("settings.not_registered_yet");
    }
  });
</script>

<div class="settings-title">
  <div>
    <svelte:element this={embedded ? "h2" : "h1"}>
      {removed
        ? $t("settings.account_removed")
        : existing
          ? existing.name
          : $t("settings.add_account")}
    </svelte:element>
    <p>
      {removed
        ? `${account.username}@${account.registrar}`
        : existing
          ? `${existing.username}@${existing.registrar}`
          : $t("settings.account_intro")}
    </p>
  </div>
  {#if existing}<span
      class="account-status"
      class:connected={existing.registration === "registered"}
      class:failed={existing.registration === "failed"}
      role="status"
      ><span class="status-dot" aria-hidden="true"></span>{$text(status)}</span
    >{/if}
</div>
{#if existing?.lastError}<p class="settings-feedback error" role="status">
    {$text(errorMessage(existing.lastError))}
  </p>{/if}
{#if canReconnect}<button
    class="secondary-button"
    type="button"
    disabled={disabled || busy || removed || existing?.removing}
    onclick={onreconnect}>{$t("settings.reconnect")}</button
  >{/if}

<form bind:this={form} onsubmit={submit} novalidate>
  <fieldset disabled={disabled || busy || removed || existing?.removing}>
    <svelte:element this={embedded ? "h3" : "h2"} class="settings-group"
      >{$t("settings.account_information")}</svelte:element
    >
    <div class="setting-row">
      <div class="setting-description">
        <label for="name">{$t("settings.account_name")}</label>
        <p id="name-help">{$t("settings.account_name_help")}</p>
      </div>
      <div class="setting-control">
        <input
          id="name"
          name="name"
          bind:value={name}
          placeholder={$t("settings.account_name_placeholder")}
          maxlength="64"
          autocomplete="off"
          required
          aria-invalid={!!nameError}
          aria-describedby={nameError ? "name-error" : "name-help"}
        />{#if nameError}<p id="name-error" class="field-error">
            {$text(nameError)}
          </p>{/if}
      </div>
    </div>
    <div class="setting-row">
      <div class="setting-description">
        <label for="registrar">{$t("settings.sip_server")}</label>
        <p id="registrar-help">
          {$t("settings.server_help")}
        </p>
      </div>
      <div class="setting-control">
        <input
          id="registrar"
          name="registrar"
          bind:value={account.registrar}
          placeholder={$t("settings.server_placeholder")}
          autocomplete="off"
          autocapitalize="none"
          spellcheck="false"
          required
          aria-invalid={!!errors.registrar}
          aria-describedby={errors.registrar
            ? "registrar-error"
            : "registrar-help"}
        />{#if errors.registrar}<p id="registrar-error" class="field-error">
            {$text(errors.registrar)}
          </p>{/if}
      </div>
    </div>
    <div class="setting-row">
      <div class="setting-description">
        <label for="username">{$t("settings.username")}</label>
        <p id="username-help">
          {$t("settings.username_help")}
        </p>
      </div>
      <div class="setting-control">
        <input
          id="username"
          name="username"
          bind:value={account.username}
          placeholder={$t("settings.username_placeholder")}
          autocomplete="off"
          autocapitalize="none"
          spellcheck="false"
          required
          aria-invalid={!!errors.username}
          aria-describedby={errors.username
            ? "username-error"
            : "username-help"}
        />{#if errors.username}<p id="username-error" class="field-error">
            {$text(errors.username)}
          </p>{/if}
      </div>
    </div>
    <div class="setting-row">
      <div class="setting-description">
        <label for="password">{$t("settings.password")}</label>
        <p id="password-help">
          {missingCredential
            ? $t("settings.password_missing")
            : existing || removed
              ? $t("settings.password_keep")
              : $t("settings.password_help")}
        </p>
      </div>
      <div class="setting-control">
        <input
          id="password"
          name="password"
          bind:value={account.password}
          type="password"
          placeholder={(existing || removed) && !missingCredential
            ? $t("settings.password_saved")
            : $t("settings.password_placeholder")}
          autocomplete="new-password"
          required={missingCredential || (!existing && !removed)}
          aria-invalid={!!errors.password}
          aria-describedby={errors.password
            ? "password-error"
            : "password-help"}
        />{#if errors.password}<p id="password-error" class="field-error">
            {$text(errors.password)}
          </p>{/if}
      </div>
    </div>
    <svelte:element this={embedded ? "h3" : "h2"} class="settings-group"
      >{$t("settings.connection_security")}</svelte:element
    >
    <div class="setting-row">
      <div class="setting-description">
        <label for="transport">{$t("settings.sip_transport")}</label>
        <p id="transport-help">
          {$t("settings.transport_help")}
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
        <label for="media-security">{$t("settings.media_security")}</label>
        <p id="media-security-help">
          {$t("settings.media_security_help")}
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
          ><option value="disabled">{$t("settings.disabled")}</option><option
            value="required">{$t("settings.srtp_required")}</option
          ></select
        >{#if errors.mediaSecurity}<p
            id="media-security-error"
            class="field-error"
          >
            {$text(errors.mediaSecurity)}
          </p>{/if}
      </div>
    </div>
    <p class="settings-trust-note">
      {$t("settings.certificate_help")}
    </p>
    <details class="account-advanced">
      <summary>{$t("settings.advanced_connection")}</summary>
      <div class="setting-row">
        <div class="setting-description">
          <label for="auth-username">{$t("settings.auth_username")}</label>
          <p id="auth-username-help">
            {$t("settings.auth_username_help")}
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
              {$text(errors.authUsername)}
            </p>{/if}
        </div>
      </div>
      <div class="setting-row">
        <div class="setting-description">
          <label for="outbound-proxy">{$t("settings.outbound_proxy")}</label>
          <p id="outbound-proxy-help">
            {$t("settings.outbound_proxy_help")}
          </p>
        </div>
        <div class="setting-control">
          <input
            id="outbound-proxy"
            name="outboundProxy"
            value={account.outboundProxy ?? ""}
            oninput={(event) =>
              (account.outboundProxy = event.currentTarget.value || null)}
            placeholder={$t("settings.proxy_placeholder")}
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
              {$text(errors.outboundProxy)}
            </p>{/if}
        </div>
      </div>
    </details>
    <div class="settings-form-actions settings-save-bar">
      <p>{$t("settings.apply_on_save")}</p>
      <button class="primary-button" type="submit"
        >{removed
          ? $t("settings.account_removed")
          : busy
            ? $t("settings.saving")
            : existing
              ? $t("settings.save_connect")
              : $t("settings.add_account")}</button
      >
    </div>
  </fieldset>
</form>
{#if existing}<div class="account-removal">
    {#if confirmRemove}<p id="remove-question">
        {$t("settings.remove_account_question", { name: existing.name })}
      </p>
      <div class="removal-actions">
        <button
          class="secondary-button"
          type="button"
          onclick={() => (confirmRemove = false)}
          >{$t("settings.cancel")}</button
        ><button
          class="danger-button"
          type="button"
          disabled={disabled || busy || existing.removing}
          aria-describedby="remove-question"
          onclick={onremove}>{$t("settings.remove_account")}</button
        >
      </div>
    {:else}<button
        class="text-button remove-link"
        type="button"
        disabled={disabled || busy || existing.removing}
        onclick={() => (confirmRemove = true)}
        >{$t("settings.remove_account_action")}</button
      >{/if}
  </div>{/if}
