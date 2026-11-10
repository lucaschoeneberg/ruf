<script lang="ts">
  import { t, text, message, type MessageValue } from "../i18n";
  import type { AppPreferences } from "../generated/AppInfo";
  import { clearHistory } from "../history-api";
  import { errorMessage } from "../api";
  let {
    value = $bindable(),
    dirty,
    disabled,
    confirmRetention = $bindable(false),
    onsave,
    ondiscard,
  }: {
    value: AppPreferences;
    dirty: boolean;
    disabled: boolean;
    confirmRetention: boolean;
    onsave: () => void;
    ondiscard: () => void;
  } = $props();
  let confirmClear = $state(false),
    clearing = $state(false),
    feedback = $state<MessageValue>("");
  async function clear() {
    if (!confirmClear || clearing) return;
    clearing = true;
    feedback = "";
    try {
      await clearHistory();
      feedback = message("settings.history_cleared");
      confirmClear = false;
    } catch (e) {
      feedback = errorMessage(e);
    } finally {
      clearing = false;
    }
  }
</script>

<div class="settings-title">
  <div>
    <h1>{$t("settings.contacts_history")}</h1>
    <p>{$t("settings.data_description")}</p>
  </div>
</div>
<h2 class="settings-group" id="contacts-display" tabindex="-1">
  {$t("settings.contact_display")}
</h2>
<fieldset {disabled}>
  <div class="setting-row">
    <div class="setting-description">
      <label for="favorites-first">{$t("settings.favorites_first")}</label>
      <p>{$t("settings.favorites_first_help")}</p>
    </div>
    <input
      id="favorites-first"
      type="checkbox"
      bind:checked={value.contacts.favoritesFirst}
    />
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <label for="show-organization">{$t("settings.show_company")}</label>
      <p>{$t("settings.show_company_help")}</p>
    </div>
    <input
      id="show-organization"
      type="checkbox"
      bind:checked={value.contacts.showOrganization}
    />
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <label for="match-german-numbers"
        >{$t("settings.match_german_numbers")}</label
      >
      <p>
        {$t("settings.match_german_numbers_help")}
      </p>
    </div>
    <input
      id="match-german-numbers"
      type="checkbox"
      bind:checked={value.contacts.matchGermanNumbers}
    />
  </div>
  <h2 class="settings-group sound-group">{$t("settings.call_history")}</h2>
  <div class="setting-row">
    <div class="setting-description">
      <label for="history-recording">{$t("settings.record_history")}</label>
      <p>
        {$t("settings.record_history_help")}
      </p>
    </div>
    <input
      id="history-recording"
      type="checkbox"
      bind:checked={value.history.recordingEnabled}
    />
  </div>
  <div class="setting-row">
    <div class="setting-description">
      <label for="history-retention">{$t("settings.retention")}</label>
      <p>
        {$t("settings.retention_help")}
      </p>
    </div>
    <select
      id="history-retention"
      bind:value={value.history.retentionDays}
      onchange={() => (confirmRetention = false)}
      ><option value={null}>{$t("settings.unlimited")}</option><option
        value={30}>{$t("settings.retention_30")}</option
      ><option value={90}>{$t("settings.retention_90")}</option><option
        value={365}>{$t("settings.retention_365")}</option
      ></select
    >
  </div>
  {#if value.history.retentionDays !== null}<div class="setting-row">
      <div class="setting-description">
        <label for="confirm-retention">{$t("settings.confirm_retention")}</label
        >
        <p>
          {$t("settings.confirm_retention_help")}
        </p>
      </div>
      <input
        id="confirm-retention"
        type="checkbox"
        bind:checked={confirmRetention}
      />
    </div>{/if}
  <div class="settings-form-actions settings-save-bar">
    <p>
      {dirty ? $t("settings.unsaved_notice") : $t("settings.data_local")}
    </p>
    <div class="settings-actions">
      <button
        class="text-button"
        type="button"
        disabled={!dirty}
        onclick={ondiscard}>{$t("settings.discard")}</button
      ><button
        class="primary-button"
        type="button"
        disabled={!dirty}
        onclick={onsave}>{$t("settings.save_data")}</button
      >
    </div>
  </div>
</fieldset>
<h2 class="settings-group sound-group">{$t("settings.clear_history")}</h2>
<p class="settings-footnote">
  {$t("settings.clear_history_help")}
</p>
{#if confirmClear}<div class="settings-feedback">
    <p>{$t("settings.clear_history_question")}</p>
    <div class="settings-actions">
      <button
        type="button"
        class="secondary-button"
        disabled={clearing}
        onclick={() => (confirmClear = false)}>{$t("settings.cancel")}</button
      ><button
        type="button"
        class="danger-button"
        disabled={disabled || clearing}
        onclick={() => void clear()}
        >{$t("settings.clear_history_confirm")}</button
      >
    </div>
  </div>{:else}<button
    id="clear-history"
    type="button"
    class="secondary-button"
    disabled={disabled || clearing}
    onclick={() => (confirmClear = true)}
    >{$t("settings.clear_history_action")}</button
  >{/if}
{#if feedback}<p class="settings-feedback" role="status">
    {$text(feedback)}
  </p>{/if}
