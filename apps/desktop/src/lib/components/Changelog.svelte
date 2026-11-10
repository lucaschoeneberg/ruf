<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import { locale, t, getLocale } from "../i18n";
  import type { AppLocale } from "../generated/AppInfo";
  import bundledEnglish from "../../../../../CHANGELOG.md?raw";
  import bundledGerman from "../../../../../CHANGELOG.de.md?raw";
  import { loadChangelog, parseChangelog } from "../changelog";

  const bundled = {
    de: parseChangelog(bundledGerman),
    en: parseChangelog(bundledEnglish),
  };
  const source = $derived(
    $locale === "de"
      ? import.meta.env.VITE_RUF_CHANGELOG_URL_DE
      : import.meta.env.VITE_RUF_CHANGELOG_URL,
  );
  let open = $state(false);
  let entries = $state(bundled[getLocale()]);
  let loading = $state(false);
  let failed = $state(false);
  let currentLocale: AppLocale = getLocale();
  let requested = false;
  let disposed = false;
  let active: AbortController | null = null;

  $effect(() => {
    const language = $locale;
    const expanded = open;
    const url = source;
    untrack(() => {
      if (language !== currentLocale) {
        active?.abort();
        active = null;
        currentLocale = language;
        entries = bundled[language];
        loading = false;
        failed = false;
        requested = false;
      }
      if (expanded && url && !requested) {
        requested = true;
        void refresh();
      }
    });
  });
  onDestroy(() => {
    disposed = true;
    active?.abort();
  });

  async function refresh() {
    const url = source;
    const language = $locale;
    if (!url || loading) return;
    loading = true;
    failed = false;
    const controller = new AbortController();
    active = controller;
    const timeout = setTimeout(() => controller.abort(), 8000);
    try {
      const updated = await loadChangelog(url, controller.signal);
      if (!disposed && active === controller && getLocale() === language)
        entries = updated;
    } catch {
      if (!disposed && active === controller && getLocale() === language) {
        entries = bundled[language];
        failed = true;
      }
    } finally {
      clearTimeout(timeout);
      if (!disposed && active === controller && getLocale() === language) {
        active = null;
        loading = false;
      }
    }
  }
</script>

<details class="settings-lab changelog" bind:open>
  <summary>{$t("settings.whats_new")}</summary>
  {#if open}
    <div class="changelog-status" role="status">
      {#if loading}<p>{$t("settings.changelog_loading")}</p>{/if}
      {#if failed}<p>
          {$t("settings.changelog_failed")}
        </p>{/if}
    </div>
    {#each entries as entry}
      <h3>{entry.title}</h3>
      <ul>
        {#each entry.changes as change}<li>{change}</li>{/each}
      </ul>
    {/each}
    {#if source}<button class="text-button" disabled={loading} onclick={refresh}
        >{$t("settings.refresh")}</button
      >{/if}
  {/if}
</details>

<style>
  summary:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
  }
  h3 {
    margin: 14px 0 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
  }
  ul {
    padding-left: 18px;
    margin: 0 0 12px;
    color: var(--text-secondary);
  }
  li + li {
    margin-top: 5px;
  }
  .changelog-status p {
    margin: 10px 0;
  }
</style>
