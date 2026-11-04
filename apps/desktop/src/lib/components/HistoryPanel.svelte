<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import type {
    ContactReference,
    HistoryEntry,
    HistoryFilter,
    HistoryPage,
    HistoryQuery,
  } from "../generated/AppInfo";
  import { clearHistory, deleteHistoryEntry, getHistory } from "../history-api";
  import { copyPhoneNumber } from "../contacts-api";
  import {
    callbackUnavailable,
    groupHistory,
    historyDayLabel,
    historyDuration,
    historyOutcomes,
    historyTitle,
  } from "../history";
  import Icon from "./Icon.svelte";

  let {
    canDial,
    oncallback,
    oncontact,
    contactScope = null,
    onclearscope = () => {},
  }: {
    canDial: boolean;
    oncallback: (id: string, number: string | null) => void;
    oncontact: (contact: ContactReference) => void;
    contactScope?: { name: string; numbers: string[] } | null;
    onclearscope?: () => void;
  } = $props();

  const pageSize = 50;
  const searchDelayMs = 200;
  const filters: { value: HistoryFilter; label: string }[] = [
    { value: "all", label: "Alle" },
    { value: "missed", label: "Verpasst" },
    { value: "incoming", label: "Eingehend" },
    { value: "outgoing", label: "Ausgehend" },
  ];
  const time = new Intl.DateTimeFormat("de-DE", {
    hour: "2-digit",
    minute: "2-digit",
  });
  const fullTime = new Intl.DateTimeFormat("de-DE", {
    dateStyle: "medium",
    timeStyle: "medium",
  });
  let desktop = $state(true);
  let mounted = $state(false);
  let query = $state("");
  let filter = $state<HistoryFilter>("all");
  let page = $state<HistoryPage | null>(null);
  let loading = $state(false);
  let expandedId = $state<string | null>(null);
  let confirmation = $state<string | null>(null);
  let mutating = $state(false);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let copiedId = $state<string | null>(null);
  let requestId = 0;
  let failedRequest: {
    query: HistoryQuery;
    append: boolean;
    retainCount: number;
  } | null = null;
  let requestedSearch = "";
  const groups = $derived(groupHistory(page?.entries ?? []));
  const currentQuery = $derived<HistoryQuery>({
    search: query.trim(),
    filter,
    offset: 0,
    limit: pageSize,
    contactNumbers: contactScope?.numbers ?? [],
  });

  async function load(
    request: HistoryQuery = currentQuery,
    append = false,
    retainCount = append ? 0 : (page?.entries.length ?? 0),
  ) {
    if (!mounted || !desktop) return;
    const id = ++requestId;
    loading = true;
    error = null;
    failedRequest = null;
    try {
      let next = await getHistory(request);
      // Preserve the loaded window without exceeding the API's per-page limit.
      while (next.hasMore && next.entries.length < retainCount) {
        if (!mounted || id !== requestId) return;
        const more = await getHistory({
          ...request,
          offset: request.offset + next.entries.length,
        });
        next = { ...more, entries: [...next.entries, ...more.entries] };
        if (!more.entries.length) break;
      }
      if (!mounted || id !== requestId) return;
      page =
        append && page
          ? { ...next, entries: [...page.entries, ...next.entries] }
          : next;
      if (expandedId && !page.entries.some((entry) => entry.id === expandedId))
        expandedId = null;
    } catch {
      if (mounted && id === requestId) {
        failedRequest = { query: request, append, retainCount };
        error =
          "Der Verlauf konnte nicht geladen werden. Prüfe den Zugriff auf den App-Datenordner und versuche es erneut.";
      }
    } finally {
      if (mounted && id === requestId) loading = false;
    }
  }

  $effect(() => {
    const request = currentQuery;
    if (!mounted || !desktop) return;
    untrack(() => {
      // Invalidate in-flight results immediately, including during the debounce.
      ++requestId;
      loading = true;
      error = null;
      failedRequest = null;
      expandedId = null;
      confirmation = null;
      actionError = null;
      copiedId = null;
    });
    const delay = request.search === requestedSearch ? 0 : searchDelayMs;
    const timer = setTimeout(() => {
      requestedSearch = request.search;
      void load(request, false, 0);
    }, delay);
    return () => clearTimeout(timer);
  });

  async function remove(id: string | null) {
    if (
      mutating ||
      (id === null ? confirmation !== "all" : confirmation !== id)
    )
      return;
    mutating = true;
    actionError = null;
    try {
      if (id === null) await clearHistory();
      else await deleteHistoryEntry(id);
      if (!mounted) return;
      confirmation = null;
      expandedId = null;
      await load();
    } catch {
      if (mounted)
        actionError =
          "Löschen fehlgeschlagen. Laufende Anrufe bleiben erhalten. Versuche es erneut.";
    } finally {
      if (mounted) mutating = false;
    }
  }

  async function copyNumber(entry: HistoryEntry) {
    if (!entry.number) return;
    actionError = null;
    copiedId = null;
    try {
      await copyPhoneNumber(entry.number);
      if (mounted) copiedId = entry.id;
    } catch {
      if (mounted)
        actionError =
          "Die Nummer konnte nicht kopiert werden. Versuche es erneut oder markiere und kopiere sie direkt.";
    }
  }

  onMount(() => {
    desktop = isTauri();
    mounted = true;
    const unlisteners: UnlistenFn[] = [];
    const reload = () => {
      if (
        document.visibilityState !== "hidden" &&
        !mutating &&
        currentQuery.search === requestedSearch
      )
        void load();
    };
    const invalidateContacts = () => {
      // Permission invalidation must remove names even if the next read fails.
      ++requestId;
      loading = false;
      if (page)
        page = {
          ...page,
          entries: page.entries.map((entry) => ({ ...entry, contact: null })),
        };
      copiedId = null;
      confirmation = null;
      actionError = null;
      reload();
    };
    if (desktop) {
      for (const event of ["history-changed", "contacts-changed"])
        void listen(
          event,
          event === "contacts-changed" ? invalidateContacts : reload,
        )
          .then((unlisten) => {
            if (mounted) unlisteners.push(unlisten);
            else unlisten();
          })
          .catch(() => {
            // Focus/manual refresh and periodic reload remain available.
          });
    }
    window.addEventListener("focus", reload);
    document.addEventListener("visibilitychange", reload);
    const timer = setInterval(reload, 30000);
    return () => {
      mounted = false;
      ++requestId;
      unlisteners.forEach((unlisten) => unlisten());
      clearInterval(timer);
      window.removeEventListener("focus", reload);
      document.removeEventListener("visibilitychange", reload);
    };
  });
</script>

<section
  class="history-panel"
  aria-labelledby="history-heading"
  aria-busy={loading || mutating}
>
  <div class="history-heading">
    <h1 id="history-heading">Verlauf</h1>
    {#if page}<span class="history-count" role="status"
        >{page.total}<span class="sr-only"> Anrufe</span></span
      >{/if}
    {#if desktop}
      <button
        class="icon-button refresh-history"
        type="button"
        aria-label="Verlauf aktualisieren"
        title="Verlauf aktualisieren"
        disabled={loading || mutating}
        onclick={() => void load()}><Icon name="refresh" size={15} /></button
      >
      <button
        class="text-button clear-history"
        type="button"
        disabled={!page?.entries.length || loading || mutating}
        onclick={() => {
          confirmation = "all";
          actionError = null;
        }}>Verlauf leeren</button
      >
    {/if}
  </div>

  {#if !desktop}
    <div class="empty-state">
      <Icon name="history" size={22} />
      <h2>Verlauf in der Desktop-App</h2>
      <p>Öffne Ruf auf deinem Computer, um deine lokalen Anrufe zu sehen.</p>
    </div>
  {:else}
    {#if contactScope}
      <div class="contact-scope">
        <span>Für {contactScope.name}</span><button
          class="text-button"
          type="button"
          onclick={onclearscope}>Alle Anrufe</button
        >
      </div>
    {/if}
    <label class="history-search">
      <Icon name="search" size={15} />
      <span class="sr-only">Verlauf nach Name, Nummer oder Konto suchen</span>
      <input
        type="search"
        placeholder="Name, Nummer oder Konto"
        bind:value={query}
        maxlength="200"
        autocomplete="off"
        spellcheck="false"
      />
    </label>
    <div class="history-filters" role="group" aria-label="Anrufe filtern">
      {#each filters as option}
        <button
          type="button"
          aria-pressed={filter === option.value}
          onclick={() => (filter = option.value)}>{option.label}</button
        >
      {/each}
    </div>
    {#if confirmation === "all"}
      <div
        class="confirmation"
        role="group"
        aria-label="Verlauf leeren bestätigen"
      >
        <p>
          Gesamten Verlauf löschen? Laufende Anrufe bleiben erhalten. Dies lässt
          sich nicht rückgängig machen.
        </p>
        <div class="actions">
          <button
            class="danger-button"
            type="button"
            disabled={mutating}
            onclick={() => void remove(null)}>Verlauf löschen</button
          ><button
            class="secondary-button"
            type="button"
            disabled={mutating}
            onclick={() => (confirmation = null)}>Abbrechen</button
          >
        </div>
      </div>
    {/if}
    {#if actionError}<p class="history-note history-error" role="alert">
        {actionError}
      </p>{/if}
    {#if page?.recordingError}<p
        class="history-note history-error"
        role="alert"
      >
        Anrufe konnten nicht vollständig gespeichert werden. Prüfe den Zugriff
        auf den App-Datenordner und starte Ruf nach Beheben der Ursache neu.
      </p>{/if}
    {#if error}
      <div class="history-note history-error">
        <p role="alert">{error}</p>
        <button
          class="secondary-button"
          type="button"
          disabled={loading}
          onclick={() => {
            if (failedRequest)
              void load(
                failedRequest.query,
                failedRequest.append,
                failedRequest.retainCount,
              );
          }}>Erneut versuchen</button
        >
      </div>
    {/if}
    {#if loading}
      <p class="history-note" class:sr-only={page !== null} role="status">
        Verlauf wird geladen …
      </p>
    {/if}
    {#if page && page.entries.length === 0 && !loading}
      <div class="empty-state">
        <Icon name="history" size={22} />
        <h2>
          {query.trim() || filter !== "all" || contactScope
            ? "Keine passenden Anrufe"
            : "Noch keine Anrufe"}
        </h2>
        <p>
          {query.trim() || filter !== "all"
            ? "Ändere die Suche oder den Filter."
            : contactScope
              ? "Für diese Nummern sind keine Anrufe gespeichert."
              : "Hier erscheinen Anrufe, die du mit Ruf tätigst oder empfängst."}
        </p>
        {#if query.trim() || filter !== "all"}<button
            class="secondary-button"
            type="button"
            onclick={() => {
              query = "";
              filter = "all";
            }}>Suche und Filter zurücksetzen</button
          >{/if}
      </div>
    {:else if page}
      {#each groups as group (group.key)}
        <h2 class="day-heading">{historyDayLabel(group.atMs)}</h2>
        <ul class="history-list" aria-label={historyDayLabel(group.atMs)}>
          {#each group.entries as entry (entry.id)}
            {@const expanded = expandedId === entry.id}
            {@const title = historyTitle(entry)}
            {@const unavailable = callbackUnavailable(entry, canDial)}
            {@const duration = historyDuration(entry.durationMs)}
            <li class="history-entry" class:missed={entry.outcome === "missed"}>
              <button
                class="history-main"
                type="button"
                aria-expanded={expanded}
                aria-label={`${title}: Anrufdetails ${expanded ? "ausblenden" : "anzeigen"}`}
                onclick={() => {
                  expandedId = expanded ? null : entry.id;
                  confirmation = null;
                  copiedId = null;
                  actionError = null;
                }}
              >
                <span class="history-copy"
                  ><span class="history-name">{title}</span><span
                    class="history-summary"
                    >{entry.direction === "incoming"
                      ? "Eingehend"
                      : "Ausgehend"} ·
                    <span class="outcome">{historyOutcomes[entry.outcome]}</span
                    >{duration !== null ? ` · ${duration}` : ""}</span
                  ><span class="history-account">{entry.accountName}</span
                  ></span
                >
                <span class="history-trailing"
                  ><time datetime={new Date(entry.startedAtMs).toISOString()}
                    >{time.format(entry.startedAtMs)}</time
                  ><Icon
                    name={expanded ? "chevronDown" : "chevronRight"}
                    size={14}
                  /></span
                >
              </button>
              {#if expanded}
                <div
                  class="history-details"
                  role="group"
                  aria-label={`${title}: Anrufdetails`}
                >
                  {#if entry.contact}<button
                      class="text-button contact-link"
                      type="button"
                      onclick={() => entry.contact && oncontact(entry.contact)}
                      >Kontakt {entry.contact.name} öffnen</button
                    >{/if}
                  <dl>
                    <div>
                      <dt>Nummer</dt>
                      <dd class="selectable-number">
                        {entry.number ?? "Unbekannt / unterdrückt"}
                      </dd>
                    </div>
                    <div>
                      <dt>Konto</dt>
                      <dd>
                        {entry.accountName}{entry.accountUsername &&
                        entry.accountRegistrar
                          ? ` · ${entry.accountUsername}@${entry.accountRegistrar}`
                          : ""}
                      </dd>
                    </div>
                    <div>
                      <dt>Beginn</dt>
                      <dd>{fullTime.format(entry.startedAtMs)}</dd>
                    </div>
                    {#if entry.answeredAtMs !== null}<div>
                        <dt>Verbunden</dt>
                        <dd>{fullTime.format(entry.answeredAtMs)}</dd>
                      </div>{/if}
                    {#if entry.endedAtMs !== null}<div>
                        <dt>Ende</dt>
                        <dd>{fullTime.format(entry.endedAtMs)}</dd>
                      </div>{/if}
                    <div>
                      <dt>Ergebnis</dt>
                      <dd>
                        {historyOutcomes[entry.outcome]}{entry.sipStatus !==
                        null
                          ? ` · SIP ${entry.sipStatus}`
                          : ""}
                      </dd>
                    </div>
                    {#if duration !== null}<div>
                        <dt>Dauer</dt>
                        <dd>{duration}</dd>
                      </div>{/if}
                    {#if entry.outcome === "interrupted" && entry.durationMs === null}<div
                      >
                        <dt>Dauer</dt>
                        <dd>Unbekannt</dd>
                      </div>{/if}
                  </dl>
                  {#if unavailable}<p class="history-note">
                      {unavailable}
                    </p>{/if}
                  <div class="actions">
                    <button
                      class="secondary-button"
                      type="button"
                      disabled={!!unavailable || mutating}
                      title={unavailable ??
                        `Über ${entry.accountName} zurückrufen`}
                      onclick={() => {
                        if (!callbackUnavailable(entry, canDial))
                          oncallback(entry.id, entry.number);
                      }}>Rückruf</button
                    >
                    {#if entry.number}<button
                        class="secondary-button"
                        type="button"
                        onclick={() => void copyNumber(entry)}
                        >{copiedId === entry.id
                          ? "Kopiert"
                          : "Nummer kopieren"}</button
                      >{/if}
                    <button
                      class="text-button delete-entry"
                      type="button"
                      disabled={entry.outcome === "inProgress" || mutating}
                      onclick={() => (confirmation = entry.id)}
                      >Anruf löschen …</button
                    >
                  </div>
                  {#if entry.outcome === "inProgress"}<p class="history-note">
                      Laufende Anrufe können nicht gelöscht werden.
                    </p>{/if}
                  {#if copiedId === entry.id}<span class="sr-only" role="status"
                      >Nummer kopiert</span
                    >{/if}
                  {#if confirmation === entry.id}
                    <div
                      class="confirmation"
                      role="group"
                      aria-label="Anruf löschen bestätigen"
                    >
                      <p>Diesen Anruf endgültig löschen?</p>
                      <div class="actions">
                        <button
                          class="danger-button"
                          type="button"
                          disabled={mutating}
                          onclick={() => void remove(entry.id)}
                          >Anruf löschen</button
                        ><button
                          class="secondary-button"
                          type="button"
                          disabled={mutating}
                          onclick={() => (confirmation = null)}
                          >Abbrechen</button
                        >
                      </div>
                    </div>
                  {/if}
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      {/each}
      {#if page.hasMore}<button
          class="secondary-button more-history"
          type="button"
          disabled={loading || mutating}
          onclick={() =>
            void load({ ...currentQuery, offset: page!.entries.length }, true)}
          >{loading
            ? "Anrufe werden geladen …"
            : "Weitere Anrufe anzeigen"}</button
        >{/if}
    {/if}
  {/if}
</section>

<style>
  .history-panel {
    flex: 1;
    min-height: 0;
    min-width: 0;
    overflow-y: auto;
    padding: 10px 14px;
  }
  .history-heading {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 28px;
    margin-bottom: 8px;
  }
  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .history-count {
    font-size: 11px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .refresh-history {
    margin-left: auto;
  }
  .clear-history {
    font-size: 11px;
    padding: 4px 0;
  }
  .history-search {
    display: flex;
    align-items: center;
    gap: 8px;
    border: 1px solid var(--border-control);
    background: var(--surface-panel);
    border-radius: var(--radius);
    padding: 0 8px;
    color: var(--muted);
  }
  .history-search input {
    width: 100%;
    min-width: 0;
    min-height: 30px;
    padding: 5px 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font-size: 12px;
  }
  input::placeholder {
    color: var(--muted);
  }
  .history-search:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .history-search input:focus-visible {
    outline: none;
  }
  .history-filters {
    display: flex;
    gap: 2px;
    margin: 8px 0;
  }
  .history-filters button {
    flex: 1;
    min-width: 0;
    min-height: 28px;
    padding: 4px 3px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: transparent;
    color: var(--muted);
    font-size: 11px;
  }
  .history-filters button:hover {
    color: var(--text);
    background: var(--surface-hover);
  }
  .history-filters button[aria-pressed="true"] {
    color: var(--accent);
    background: var(--surface-selected);
    border-color: var(--border);
  }
  .day-heading {
    margin: 14px 0 4px;
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
  }
  .history-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .history-entry {
    border-bottom: 1px solid var(--border);
  }
  .history-main {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    min-height: 64px;
    padding: 8px 4px;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text);
    text-align: left;
  }
  .history-main:hover {
    background: var(--surface-hover);
  }
  .history-copy {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .history-name {
    font-size: 13px;
    font-weight: 600;
  }
  .history-summary,
  .history-account,
  .history-trailing {
    font-size: 11px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .missed .history-name,
  .missed .outcome {
    color: var(--danger);
  }
  .history-trailing {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .history-details {
    padding: 2px 4px 10px;
  }
  dl {
    margin: 4px 0 8px;
    font-size: 11px;
  }
  dl > div {
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr);
    gap: 8px;
    margin: 4px 0;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    color: var(--text-secondary);
    overflow-wrap: anywhere;
    font-variant-numeric: tabular-nums;
  }
  .selectable-number {
    user-select: text;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .actions button {
    font-size: 11px;
    min-height: 28px;
  }
  .delete-entry {
    color: var(--danger);
  }
  .delete-entry:disabled {
    color: var(--disabled);
  }
  .contact-link {
    padding: 4px 0;
    font-size: 12px;
    text-align: left;
    overflow-wrap: anywhere;
  }
  .contact-scope {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
    color: var(--text-secondary);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .contact-scope > span {
    min-width: 0;
  }
  .contact-scope button {
    flex-shrink: 0;
    font-size: 11px;
  }
  .history-note {
    margin: 6px 0;
    font-size: 11px;
    line-height: 1.45;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .history-error {
    color: var(--danger);
  }
  .confirmation {
    margin: 8px 0;
    padding: 8px;
    border: 1px solid var(--danger-border);
    border-radius: var(--radius);
    background: var(--danger-surface);
    color: var(--danger);
    font-size: 12px;
  }
  .confirmation p {
    margin: 0 0 8px;
  }
  .more-history {
    width: 100%;
    margin-top: 12px;
  }
</style>
