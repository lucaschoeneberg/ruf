<script lang="ts">
  import { onDestroy } from "svelte";
  import bundled from "../../../../../CHANGELOG.md?raw";
  import { loadChangelog, parseChangelog } from "../changelog";

  const local = parseChangelog(bundled);
  const source = import.meta.env.VITE_RUF_CHANGELOG_URL;
  let open = $state(false);
  let entries = $state(local);
  let loading = $state(false);
  let failed = $state(false);
  let requested = false;
  let disposed = false;
  let active: AbortController | null = null;

  $effect(() => {
    if (open && source && !requested) {
      requested = true;
      void refresh();
    }
  });
  onDestroy(() => {
    disposed = true;
    active?.abort();
  });

  async function refresh() {
    if (!source || loading) return;
    loading = true;
    failed = false;
    const controller = new AbortController();
    active = controller;
    const timeout = setTimeout(() => controller.abort(), 8000);
    try {
      const updated = await loadChangelog(source, controller.signal);
      if (!disposed) entries = updated;
    } catch {
      if (!disposed) {
        entries = local;
        failed = true;
      }
    } finally {
      clearTimeout(timeout);
      if (!disposed) loading = false;
    }
  }
</script>

<details class="settings-lab changelog" bind:open>
  <summary>Was ist neu?</summary>
  {#if open}
    <div class="changelog-status" role="status">
      {#if loading}<p>Aktuelle Änderungen werden geladen …</p>{/if}
      {#if failed}<p>
          Online-Änderungen konnten nicht geladen werden. Hier siehst du den
          Stand dieser Version.
        </p>{/if}
    </div>
    {#each entries as entry}
      <h3>{entry.title}</h3>
      <ul>
        {#each entry.changes as change}<li>{change}</li>{/each}
      </ul>
    {/each}
    {#if source}<button class="text-button" disabled={loading} onclick={refresh}
        >Aktualisieren</button
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
