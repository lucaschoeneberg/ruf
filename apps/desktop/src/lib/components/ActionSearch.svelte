<script lang="ts">
  import { onMount, tick } from "svelte";
  import { searchActions, type SearchAction } from "../shortcuts";
  import Icon from "./Icon.svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import type {
    Contact,
    ContactsSnapshot,
    ContactsPreferences,
    HistoryEntry,
    HistoryPage,
  } from "../generated/AppInfo";
  import { searchContacts } from "../contacts";
  import { getHistory } from "../history-api";
  import { normalizeDialNumber } from "../dialing";
  import { historyTitle, historyOutcomes } from "../history";
  let {
    actions,
    contacts,
    preferences,
    oncontact,
    onhistory,
    onsearchview,
    onaction,
    onclose,
  }: {
    actions: readonly SearchAction[];
    contacts: ContactsSnapshot | null;
    preferences: ContactsPreferences;
    oncontact: (contact: Contact) => void;
    onhistory: (entry: HistoryEntry, query: string) => void;
    onsearchview: (view: "contacts" | "history", query: string) => void;
    onaction: (id: string) => void;
    onclose: () => void;
  } = $props();
  let dialog: HTMLDialogElement;
  let input: HTMLInputElement;
  let query = $state("");
  let selected = $state(0);
  const previewLimit = 8;
  let desktop = $state(false);
  let mounted = false;
  let historyPage = $state<HistoryPage | null>(null);
  let historyLoading = $state(false);
  let historyError = $state(false);
  let revision = $state(0);
  let historyRequest = 0;
  const historyDate = new Intl.DateTimeFormat("de-DE", {
    dateStyle: "short",
    timeStyle: "short",
  });
  const text = $derived(query.trim());
  const historySearch = $derived(normalizeDialNumber(text) ?? text);
  const searchPeople = $derived(Array.from(text).length >= 2);
  const contactMatches = $derived(
    searchPeople && contacts?.permission === "authorized"
      ? searchContacts(
          contacts.contacts,
          text,
          false,
          preferences.favoritesFirst,
          preferences.matchGermanNumbers,
        )
      : [],
  );
  type Result = SearchAction & {
    group: string;
    detail?: string;
    choose: () => void;
  };
  const results = $derived.by((): Result[] => {
    const found: Result[] = searchActions(actions, query).map((action) => ({
      ...action,
      group: "Aktionen",
      choose: () => onaction(action.id),
    }));
    found.push(
      ...contactMatches.slice(0, previewLimit).map((contact) => ({
        id: `contact:${contact.id}`,
        label: contact.name,
        terms: "",
        group: "Kontakte",
        detail: [
          preferences.showOrganization ? contact.organization : "",
          contact.phones[0]?.number,
        ]
          .filter(Boolean)
          .join(" · "),
        choose: () => oncontact(contact),
      })),
    );
    if (contactMatches.length > previewLimit)
      found.push({
        id: "more-contacts",
        label: `Alle ${contactMatches.length} Kontakte anzeigen`,
        terms: "",
        group: "Kontakte",
        choose: () => onsearchview("contacts", text),
      });
    found.push(
      ...(historyPage?.entries ?? []).map((entry) => ({
        id: `history:${entry.id}`,
        label: historyTitle(
          contacts?.permission === "authorized"
            ? entry
            : { ...entry, contact: null },
        ),
        terms: "",
        group: "Verlauf",
        detail: [
          historyOutcomes[entry.outcome],
          historyDate.format(entry.startedAtMs),
          entry.accountName,
        ]
          .filter(Boolean)
          .join(" · "),
        choose: () => onhistory(entry, historySearch),
      })),
    );
    if (historyPage?.hasMore)
      found.push({
        id: "more-history",
        label: `Alle ${historyPage.total} Anrufe anzeigen`,
        terms: "",
        group: "Verlauf",
        choose: () => onsearchview("history", historySearch),
      });
    return found;
  });
  function refreshHistory() {
    ++historyRequest;
    historyPage = null;
    ++revision;
  }
  $effect(() => {
    const search = historySearch;
    // Re-read projections when the parent contact cache or native sources change.
    void contacts;
    void revision;
    const request = ++historyRequest;
    historyPage = null;
    historyError = false;
    historyLoading = desktop && searchPeople;
    if (!desktop || !searchPeople) return;
    const timer = setTimeout(() => {
      void getHistory({
        search,
        filter: "all",
        offset: 0,
        limit: previewLimit,
        contactNumbers: [],
      })
        .then((page) => {
          if (mounted && request === historyRequest) historyPage = page;
        })
        .catch(() => {
          if (mounted && request === historyRequest) historyError = true;
        })
        .finally(() => {
          if (mounted && request === historyRequest) historyLoading = false;
        });
    }, 200);
    return () => {
      clearTimeout(timer);
      if (request === historyRequest) ++historyRequest;
    };
  });
  const current = $derived(Math.min(selected, Math.max(0, results.length - 1)));
  onMount(() => {
    mounted = true;
    desktop = isTauri();
    const unlisteners: UnlistenFn[] = [];
    if (desktop)
      for (const event of ["history-changed", "contacts-changed"]) {
        void listen(event, refreshHistory)
          .then((unlisten) => {
            if (mounted) unlisteners.push(unlisten);
            else unlisten();
          })
          .catch(() => {
            /* Focus refresh remains available after a subscription failure. */
          });
      }
    window.addEventListener("focus", refreshHistory);
    const previous = document.activeElement;
    dialog.showModal();
    input.focus();
    return () => {
      mounted = false;
      ++historyRequest;
      unlisteners.forEach((unlisten) => unlisten());
      window.removeEventListener("focus", refreshHistory);
      dialog.close();
      if (previous instanceof HTMLElement && previous.isConnected)
        previous.focus();
    };
  });
  function choose(action?: Result) {
    if (!action || action.disabled) return;
    onclose();
    action.choose();
  }
  function keyboard(event: KeyboardEvent) {
    // Keep the phone's shortcuts and DTMF handler out of this focused search.
    event.stopPropagation();
    if (event.isComposing) return;
    if (event.key === "Escape") {
      event.preventDefault();
      onclose();
    }
    if (event.key === "Enter" && event.target === input) {
      event.preventDefault();
      choose(results[current]);
    }
    if (
      (event.key === "ArrowDown" || event.key === "ArrowUp") &&
      event.target === input
    ) {
      event.preventDefault();
      if (!results.length) return;
      selected =
        (current + (event.key === "ArrowDown" ? 1 : -1) + results.length) %
        results.length;
      void tick().then(() =>
        document
          .getElementById(`ruf-action-${selected}`)
          ?.scrollIntoView({ block: "nearest" }),
      );
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<dialog
  bind:this={dialog}
  aria-labelledby="action-search-title"
  oncancel={(event) => {
    event.preventDefault();
    onclose();
  }}
  onkeydown={keyboard}
>
  <header>
    <h2 id="action-search-title">Ruf durchsuchen</h2>
    <button
      class="icon-button"
      type="button"
      aria-label="Suche schließen"
      onclick={onclose}><Icon name="close" size={16} /></button
    >
  </header>
  <div class="search-field">
    <Icon name="search" size={16} />
    <input
      bind:this={input}
      bind:value={query}
      oninput={() => (selected = 0)}
      role="combobox"
      aria-label="Ruf durchsuchen"
      aria-expanded="true"
      aria-controls="ruf-action-list"
      aria-autocomplete="list"
      aria-activedescendant={results.length
        ? `ruf-action-${current}`
        : undefined}
      placeholder="Name, Nummer oder Aktion suchen …"
      maxlength="200"
      autocomplete="off"
      spellcheck="false"
    />
  </div>
  <div id="ruf-action-list" role="listbox" aria-label="Aktionen">
    {#each results as action, index (action.id)}
      {#if index === 0 || results[index - 1]?.group !== action.group}
        <div class="result-group" role="presentation">{action.group}</div>
      {/if}
      <div
        id={`ruf-action-${index}`}
        role="option"
        aria-selected={index === current}
        aria-disabled={!!action.disabled}
        class:selected={index === current}
      >
        <button
          type="button"
          tabindex="-1"
          disabled={action.disabled}
          onclick={() => choose(action)}
        >
          <span
            >{action.label}{#if action.detail}<small>{action.detail}</small
              >{/if}{#if action.disabled && action.reason}<small
                >{action.reason}</small
              >{/if}</span
          >
          {#if action.shortcut}<kbd>{action.shortcut}</kbd>{/if}
        </button>
      </div>
    {/each}
  </div>
  {#if searchPeople && historyLoading}<p class="empty" role="status">
      Verlauf wird durchsucht …
    </p>{/if}
  {#if searchPeople && desktop && contacts?.permission !== "authorized"}
    <p class="empty">
      {contacts === null
        ? "Kontakte sind momentan nicht verfügbar."
        : contacts.permission === "unsupported"
          ? "Kontakte sind auf dieser Plattform nicht verfügbar."
          : "Kontakte sind nicht freigegeben. Freigabe unter Einstellungen → Datenschutz."}
    </p>
  {/if}
  {#if searchPeople && !desktop}<p class="empty">
      Kontakte und Verlauf findest du in der Desktop-App.
    </p>{/if}
  {#if searchPeople && historyError}<div class="search-error" role="status">
      <p>Der Verlauf konnte nicht geladen werden.</p>
      <button class="text-button" type="button" onclick={refreshHistory}
        >Erneut versuchen</button
      >
    </div>{/if}
  {#if historyPage?.recordingError}<p class="empty">
      Der Verlauf kann momentan nicht vollständig gespeichert werden.
    </p>{/if}
  {#if !results.length && !historyLoading && !historyError}<p
      class="empty"
      role="status"
    >
      Keine passenden Treffer. Versuche einen Namen, eine Nummer oder eine
      Aktion.
    </p>{/if}
  {#if !searchPeople}<p class="empty">
      Ab 2 Zeichen auch Kontakte und Verlauf durchsuchen.
    </p>{/if}
  <footer>↑ ↓ auswählen · Enter ausführen · Esc schließen</footer>
</dialog>

<style>
  dialog {
    padding: 0;
    width: min(440px, calc(100vw - 20px));
    max-height: calc(100dvh - 24px);
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  dialog[open] {
    display: flex;
    flex-direction: column;
  }
  dialog::backdrop {
    background: rgb(1 4 9 / 65%);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px 4px 12px;
  }
  h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }
  .search-field {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 4px 12px 10px;
    padding: 6px 8px;
    border: 1px solid var(--border-control);
    border-radius: var(--radius);
    background: var(--surface-raised);
  }
  .search-field:focus-within {
    border-color: var(--accent);
  }
  input {
    width: 100%;
    min-width: 0;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text);
  }
  input:focus-visible {
    outline: none;
  }
  input::placeholder {
    color: var(--muted);
  }
  [role="listbox"] {
    overflow-y: auto;
    min-height: 0;
    border-top: 1px solid var(--border);
    padding: 4px;
  }
  .result-group {
    padding: 8px 8px 4px;
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
  }
  .search-error {
    padding: 8px 12px;
    color: var(--muted);
    font-size: 12px;
  }
  .search-error p {
    margin: 0 0 4px;
  }
  [role="option"] {
    border-radius: var(--radius);
  }
  [role="option"].selected {
    background: var(--surface-selected);
  }
  [role="option"] button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
    text-align: left;
    color: var(--text);
    background: transparent;
    border: 0;
    border-radius: var(--radius);
    padding: 8px;
  }
  [role="option"] button > span {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  [role="option"] button:enabled:hover {
    background: var(--surface-hover);
  }
  [role="option"] button:disabled {
    color: var(--disabled);
    cursor: default;
  }
  small {
    display: block;
    font-size: 11px;
    color: var(--muted);
    margin-top: 2px;
  }
  kbd {
    flex-shrink: 0;
    font: inherit;
    font-size: 11px;
    color: var(--text-secondary);
  }
  footer,
  .empty {
    flex-shrink: 0;
    margin: 0;
    padding: 8px 12px;
    font-size: 11px;
    color: var(--muted);
  }
  footer {
    flex-shrink: 0;
    border-top: 1px solid var(--border);
  }
</style>
