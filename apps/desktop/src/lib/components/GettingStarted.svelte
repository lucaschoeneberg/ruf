<script lang="ts">
  import { t } from "../i18n";
  import type { RegistrationState } from "../generated/AppInfo";
  import Icon from "./Icon.svelte";
  let {
    runtime,
    registration,
    hasAccount = false,
    audioUnknown,
    onsetup,
    onexplore,
  }: {
    runtime: "desktop" | "browser" | "unlinked" | "unavailable";
    registration: RegistrationState;
    hasAccount?: boolean;
    audioUnknown: boolean;
    onsetup: () => void;
    onexplore: () => void;
  } = $props();
  const ready = $derived(registration === "registered" && !audioUnknown);
</script>

<section class="getting-started" aria-labelledby="start-title">
  <div class="start-symbol"><Icon name="phone" size={22} /></div>
  <h1 id="start-title">
    {runtime === "browser"
      ? $t("phone.start.browser_title")
      : runtime === "unlinked"
        ? $t("phone.start.unlinked_title")
        : runtime === "unavailable"
          ? $t("phone.start.unavailable_title")
          : audioUnknown
            ? $t("phone.start.audio_title")
            : ready
              ? $t("phone.start.ready_title")
              : registration === "registering"
                ? $t("phone.start.registering_title")
                : registration === "failed"
                  ? $t("phone.start.failed_title")
                  : hasAccount
                    ? $t("phone.start.saved_title")
                    : $t("phone.start.connect_title")}
  </h1>
  <p class="start-description">
    {runtime === "browser"
      ? $t("phone.start.browser_hint")
      : runtime === "unlinked"
        ? $t("phone.start.unlinked_hint")
        : runtime === "unavailable"
          ? $t("phone.start.unavailable_hint")
          : audioUnknown
            ? $t("phone.start.audio_hint")
            : ready
              ? $t("phone.start.ready_hint")
              : registration === "registering"
                ? $t("phone.start.registering_hint")
                : registration === "failed"
                  ? $t("phone.start.failed_hint")
                  : hasAccount
                    ? $t("phone.start.saved_hint")
                    : $t("phone.start.connect_hint")}
  </p>
  {#if runtime === "desktop" && !ready && !audioUnknown && !hasAccount}
    <p class="start-requirements">
      <strong>{$t("phone.start.requirements_title")}</strong>
      {$t("phone.start.requirements")}
    </p>
  {:else if ready}
    <p class="start-note">
      {$t("phone.start.international_hint")}
    </p>
  {/if}
  <div class="start-actions">
    {#if runtime === "desktop" && !ready && !audioUnknown}<button
        class="primary-button"
        onclick={onsetup}
        >{registration === "failed"
          ? $t("phone.start.check_connection")
          : hasAccount
            ? $t("phone.start.account_settings")
            : $t("phone.start.setup")}</button
      ><button class="text-button" onclick={onexplore}
        >{$t("phone.start.explore_first")}</button
      >
    {:else}<button class="primary-button" onclick={onexplore}
        >{audioUnknown
          ? $t("phone.start.keypad")
          : ready
            ? $t("phone.start.dial")
            : $t("phone.start.explore")}</button
      ><button class="text-button" onclick={onsetup}
        >{$t("phone.start.account_settings")}</button
      >{/if}
  </div>
  <p class="start-note">{$t("phone.start.reopen")}</p>
</section>
