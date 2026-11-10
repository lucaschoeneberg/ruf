<script lang="ts">
  import { t, text, message, locale, type MessageValue } from "../i18n";
  import { onMount, tick, untrack } from "svelte";
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
    focusSearchRequest = 0,
    searchQuery = null,
    focusEntryId = null,
    canDial,
    oncallback,
    oncontact,
    contactScope = null,
    onclearscope = () => {},
  }: {
    focusSearchRequest?: number;
    searchQuery?: string | null;
    focusEntryId?: string | null;
    canDial: boolean;
    oncallback: (id: string, number: string | null) => void;
    oncontact: (contact: ContactReference) => void;
    contactScope?: { name: string; numbers: string[] } | null;
    onclearscope?: () => void;
  } = $props();

  const pageSize = 50;
  const searchDelayMs = 200;
  const filters = $derived<{ value: HistoryFilter; label: string }[]>([
    { value: "all", label: $t("phone.history.all") },
    { value: "missed", label: $t("phone.history.missed") },
    { value: "incoming", label: $t("phone.history.incoming") },
    { value: "outgoing", label: $t("phone.history.outgoing") },
  ]);
  const time = $derived(
    new Intl.DateTimeFormat($locale === "de" ? "de-DE" : "en-GB", {
      hour: "2-digit",
      minute: "2-digit",
    }),
  );
  const fullTime = $derived(
    new Intl.DateTimeFormat($locale === "de" ? "de-DE" : "en-GB", {
      dateStyle: "medium",
      timeStyle: "medium",
    }),
  );
  let searchInput = $state<HTMLInputElement>();
  let appliedSearchFocus = 0;
  $effect(() => {
    if (focusSearchRequest > appliedSearchFocus && searchInput) {
      appliedSearchFocus = focusSearchRequest;
      if (searchQuery !== null) query = searchQuery;
      searchInput.focus();
      searchInput.select();
    }
  });
  let desktop = $state(true);
  let mounted = $state(false);
  let query = $state("");
  let filter = $state<HistoryFilter>("all");
  let page = $state<HistoryPage | null>(null);
  let nextOffset = $state(0);
  let loading = $state(false);
  let expandedId = $state<string | null>(null);
  let confirmation = $state<string | null>(null);
  let mutating = $state(false);
  let error = $state<MessageValue | null>(null);
  let actionError = $state<MessageValue | null>(null);
  let copiedId = $state<string | null>(null);
  let requestId = 0;
  let failedRequest: {
    query: HistoryQuery;
    append: boolean;
    retainCount: number;
  } | null = null;
  let requestedSearch = "";
  let appliedEntryFocus: string | null = null;
  $effect(() => {
    const id = focusEntryId;
    if (!id) {
      appliedEntryFocus = null;
      return;
    }
    if (
      id === appliedEntryFocus ||
      loading ||
      currentQuery.search !== requestedSearch ||
      !page?.entries.some((entry) => entry.id === id)
    )
      return;
    untrack(() => {
      appliedEntryFocus = id;
      expandedId = id;
      void tick().then(() => {
        if (!mounted || focusEntryId !== id) return;
        const row = [
          ...document.querySelectorAll<HTMLElement>("[data-history-id]"),
        ].find((row) => row.dataset.historyId === id);
        row?.scrollIntoView({ block: "nearest" });
        row
          ?.querySelector<HTMLButtonElement>(".history-main")
          ?.focus({ preventScroll: true });
      });
    });
  });
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
      // Server progress counts every consumed row, including overlapping IDs.
      nextOffset = request.offset + next.entries.length;
      // New calls can shift offset pages between reads. Keep one current record
      // per ID so an overlapping page cannot break Svelte's keyed call rows.
      const entries =
        append && page ? [...page.entries, ...next.entries] : next.entries;
      page = {
        ...next,
        entries: [
          ...new Map(entries.map((entry) => [entry.id, entry])).values(),
        ],
      };
      if (expandedId && !page.entries.some((entry) => entry.id === expandedId))
        expandedId = null;
    } catch {
      if (mounted && id === requestId) {
        failedRequest = { query: request, append, retainCount };
        error = message("phone.history.load_error");
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
      if (mounted) actionError = message("phone.history.delete_error");
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
      if (mounted) actionError = message("phone.contacts.copy_error");
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
    <h1 id="history-heading">{$t("phone.history.title")}</h1>
    {#if page}<span class="history-count" role="status"
        >{page.total}<span class="sr-only">
          {$t("phone.history.count_label", { count: page.total })}</span
        ></span
      >{/if}
    {#if desktop}
      <button
        class="icon-button refresh-history"
        type="button"
        aria-label={$t("phone.history.refresh")}
        title={$t("phone.history.refresh")}
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
        }}>{$t("phone.history.clear_prompt")}</button
      >
    {/if}
  </div>

  {#if !desktop}
    <div class="empty-state">
      <Icon name="history" size={22} />
      <h2>{$t("phone.history.desktop_title")}</h2>
      <p>{$t("phone.history.desktop_hint")}</p>
    </div>
  {:else}
    {#if contactScope}
      <div class="contact-scope">
        <span>{$t("phone.history.scope", { name: contactScope.name })}</span
        ><button class="text-button" type="button" onclick={onclearscope}
          >{$t("phone.history.all_calls")}</button
        >
      </div>
    {/if}
    <label class="history-search">
      <Icon name="search" size={15} />
      <span class="sr-only">{$t("phone.history.search")}</span>
      <input
        bind:this={searchInput}
        type="search"
        placeholder={$t("phone.history.search_placeholder")}
        bind:value={query}
        maxlength="200"
        autocomplete="off"
        spellcheck="false"
      />
    </label>
    <div
      class="history-filters"
      role="group"
      aria-label={$t("phone.history.filter")}
    >
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
        aria-label={$t("phone.history.confirm_clear")}
      >
        <p>
          {$t("phone.history.clear_warning")}
        </p>
        <div class="actions">
          <button
            class="danger-button"
            type="button"
            disabled={mutating}
            onclick={() => void remove(null)}
            >{$t("phone.history.clear")}</button
          ><button
            class="secondary-button"
            type="button"
            disabled={mutating}
            onclick={() => (confirmation = null)}
            >{$t("phone.common.cancel")}</button
          >
        </div>
      </div>
    {/if}
    {#if actionError}<p class="history-note history-error" role="alert">
        {$text(actionError)}
      </p>{/if}
    {#if page?.recordingError}<p
        class="history-note history-error"
        role="alert"
      >
        {$t("phone.history.recording_error")}
      </p>{/if}
    {#if error}
      <div class="history-note history-error">
        <p role="alert">{$text(error)}</p>
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
          }}>{$t("phone.common.retry")}</button
        >
      </div>
    {/if}
    {#if loading}
      <p class="history-note" class:sr-only={page !== null} role="status">
        {$t("phone.history.loading")}
      </p>
    {/if}
    {#if page && page.entries.length === 0 && !loading}
      <div class="empty-state">
        <Icon name="history" size={22} />
        <h2>
          {query.trim() || filter !== "all" || contactScope
            ? $t("phone.history.no_matches")
            : $t("phone.history.empty")}
        </h2>
        <p>
          {query.trim() || filter !== "all"
            ? $t("phone.history.search_hint")
            : contactScope
              ? $t("phone.history.scope_empty")
              : $t("phone.history.empty_hint")}
        </p>
        {#if query.trim() || filter !== "all"}<button
            class="secondary-button"
            type="button"
            onclick={() => {
              query = "";
              filter = "all";
            }}>{$t("phone.history.reset")}</button
          >{/if}
      </div>
    {:else if page}
      {#each groups as group (group.key)}
        <h2 class="day-heading">
          {historyDayLabel(group.atMs, Date.now(), $locale)}
        </h2>
        <ul
          class="history-list"
          aria-label={historyDayLabel(group.atMs, Date.now(), $locale)}
        >
          {#each group.entries as entry (entry.id)}
            {@const expanded = expandedId === entry.id}
            {@const title = historyTitle(entry, $locale)}
            {@const unavailable = callbackUnavailable(entry, canDial)}
            {@const duration = historyDuration(entry.durationMs)}
            <li
              class="history-entry"
              data-history-id={entry.id}
              class:missed={entry.outcome === "missed"}
            >
              <button
                class="history-main"
                type="button"
                aria-expanded={expanded}
                aria-label={$t(
                  expanded
                    ? "phone.history.hide_details"
                    : "phone.history.show_details",
                  { name: title },
                )}
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
                      ? $t("phone.history.incoming")
                      : $t("phone.history.outgoing")} ·
                    <span class="outcome"
                      >{$text(historyOutcomes[entry.outcome])}</span
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
                  aria-label={$t("phone.history.details", { name: title })}
                >
                  {#if entry.contact}<button
                      class="text-button contact-link"
                      type="button"
                      onclick={() => entry.contact && oncontact(entry.contact)}
                      >{$t("phone.history.open_contact", {
                        name: entry.contact.name,
                      })}</button
                    >{/if}
                  <dl>
                    <div>
                      <dt>{$t("phone.history.number")}</dt>
                      <dd class="selectable-number">
                        {entry.number ?? $t("phone.history.unknown_number")}
                      </dd>
                    </div>
                    <div>
                      <dt>{$t("phone.history.account")}</dt>
                      <dd>
                        {entry.accountName}{entry.accountUsername &&
                        entry.accountRegistrar
                          ? ` · ${entry.accountUsername}@${entry.accountRegistrar}`
                          : ""}
                      </dd>
                    </div>
                    <div>
                      <dt>{$t("phone.history.started")}</dt>
                      <dd>{fullTime.format(entry.startedAtMs)}</dd>
                    </div>
                    {#if entry.answeredAtMs !== null}<div>
                        <dt>{$t("phone.history.connected")}</dt>
                        <dd>{fullTime.format(entry.answeredAtMs)}</dd>
                      </div>{/if}
                    {#if entry.endedAtMs !== null}<div>
                        <dt>{$t("phone.history.ended")}</dt>
                        <dd>{fullTime.format(entry.endedAtMs)}</dd>
                      </div>{/if}
                    <div>
                      <dt>{$t("phone.history.outcome")}</dt>
                      <dd>
                        {$text(
                          historyOutcomes[entry.outcome],
                        )}{entry.sipStatus !== null
                          ? ` · SIP ${entry.sipStatus}`
                          : ""}
                      </dd>
                    </div>
                    {#if duration !== null}<div>
                        <dt>{$t("phone.history.duration")}</dt>
                        <dd>{duration}</dd>
                      </div>{/if}
                    {#if entry.outcome === "interrupted" && entry.durationMs === null}<div
                      >
                        <dt>{$t("phone.history.duration")}</dt>
                        <dd>{$t("phone.history.unknown")}</dd>
                      </div>{/if}
                  </dl>
                  {#if unavailable}<p class="history-note">
                      {$text(unavailable)}
                    </p>{/if}
                  <div class="actions">
                    <button
                      class="secondary-button"
                      type="button"
                      disabled={!!unavailable || mutating}
                      title={unavailable
                        ? $text(unavailable)
                        : $t("phone.history.callback_account", {
                            name: entry.accountName,
                          })}
                      onclick={() => {
                        if (!callbackUnavailable(entry, canDial))
                          oncallback(entry.id, entry.number);
                      }}>{$t("phone.history.callback")}</button
                    >
                    {#if entry.number}<button
                        class="secondary-button"
                        type="button"
                        onclick={() => void copyNumber(entry)}
                        >{copiedId === entry.id
                          ? $t("phone.common.copied")
                          : $t("phone.history.copy")}</button
                      >{/if}
                    <button
                      class="text-button delete-entry"
                      type="button"
                      disabled={entry.outcome === "inProgress" || mutating}
                      onclick={() => (confirmation = entry.id)}
                      >{$t("phone.history.delete_prompt")}</button
                    >
                  </div>
                  {#if entry.outcome === "inProgress"}<p class="history-note">
                      {$t("phone.history.active_retained")}
                    </p>{/if}
                  {#if copiedId === entry.id}<span class="sr-only" role="status"
                      >{$t("phone.contacts.copied")}</span
                    >{/if}
                  {#if confirmation === entry.id}
                    <div
                      class="confirmation"
                      role="group"
                      aria-label={$t("phone.history.confirm_delete")}
                    >
                      <p>
                        {$t("phone.history.delete_warning")}
                      </p>
                      <div class="actions">
                        <button
                          class="danger-button"
                          type="button"
                          disabled={mutating}
                          onclick={() => void remove(entry.id)}
                          >{$t("phone.history.delete")}</button
                        ><button
                          class="secondary-button"
                          type="button"
                          disabled={mutating}
                          onclick={() => (confirmation = null)}
                          >{$t("phone.common.cancel")}</button
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
            void load({ ...currentQuery, offset: nextOffset }, true)}
          >{loading
            ? $t("phone.history.loading_more")
            : $t("phone.history.more")}</button
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
