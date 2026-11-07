<script lang="ts">
  import { onMount, tick, untrack } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import type {
    Contact,
    ContactsPreferences,
    ContactsSnapshot,
  } from "../generated/AppInfo";
  import {
    getContacts,
    refreshContacts,
    requestContactsAccess,
    setContactFavorite,
    openContactsSettings,
  } from "../contacts-api";
  import { getPreferences } from "../api";
  import { searchContacts } from "../contacts";
  import Icon from "./Icon.svelte";
  import ContactRow from "./ContactRow.svelte";

  let {
    focusSearchRequest = 0,
    searchQuery = null,
    canDial,
    callHint,
    oncall,
    onhistory,
    focusId = null,
  }: {
    focusSearchRequest?: number;
    searchQuery?: string | null;
    canDial: boolean;
    callHint: string;
    oncall: (destination: string) => void;
    onhistory: (contact: Contact) => void;
    focusId?: string | null;
  } = $props();

  let snapshot = $state<ContactsSnapshot | null>(null);
  let preferences = $state<ContactsPreferences>({
    favoritesFirst: true,
    showOrganization: true,
    matchGermanNumbers: true,
  });
  let preferencesError = $state<string | null>(null);
  let preferencesRequest = 0;
  let query = $state("");
  let favoritesOnly = $state(false);
  let expandedId = $state<string | null>(null);
  let limit = $state(50);
  let refreshing = $state(false);
  let savingFavorite = $state<string | null>(null);
  let error = $state<string | null>(null);
  let favoriteError = $state<string | null>(null);
  let privacyError = $state<string | null>(null);
  let desktop = $state(true);
  let panel: HTMLElement;
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
  let mounted = false;
  let requestId = 0;
  let appliedFocus: string | null = null;
  const permission = $derived(snapshot?.permission);
  const accessible = $derived(permission === "authorized");
  const results = $derived(
    searchContacts(
      snapshot?.contacts ?? [],
      query,
      favoritesOnly,
      preferences.favoritesFirst,
      preferences.matchGermanNumbers,
    ),
  );
  const visible = $derived(results.slice(0, limit));

  async function refreshPreferences() {
    if (!mounted || !desktop) return;
    const request = ++preferencesRequest;
    try {
      const next = await getPreferences();
      if (mounted && request === preferencesRequest) {
        preferences = next.contacts;
        preferencesError = null;
      }
    } catch {
      if (mounted && request === preferencesRequest)
        preferencesError =
          "Die Kontaktanzeige konnte nicht geladen werden. Die zuletzt bekannte Ansicht bleibt erhalten.";
    }
  }

  async function refresh(requestAccess = false, force = true) {
    if (!mounted || !desktop) return;
    const id = ++requestId;
    refreshing = true;
    error = null;
    try {
      const next = await (requestAccess
        ? requestContactsAccess()
        : force
          ? refreshContacts()
          : getContacts());
      if (mounted && id === requestId) snapshot = next;
    } catch {
      if (mounted && id === requestId) {
        snapshot = null;
        error =
          "Kontakte konnten nicht geladen werden. Prüfe die Freigabe und versuche es erneut.";
      }
    } finally {
      if (mounted && id === requestId) refreshing = false;
    }
  }

  $effect(() => {
    const id = focusId;
    const contacts =
      snapshot?.permission === "authorized" ? snapshot.contacts : [];
    if (!id) {
      appliedFocus = null;
      return;
    }
    if (id === appliedFocus || !contacts.some((contact) => contact.id === id))
      return;
    untrack(() => {
      appliedFocus = id;
      query = "";
      favoritesOnly = false;
      const index = searchContacts(
        contacts,
        "",
        false,
        preferences.favoritesFirst,
        preferences.matchGermanNumbers,
      ).findIndex((contact) => contact.id === id);
      limit = Math.max(50, index + 1);
      expandedId = id;
      void tick().then(() => {
        if (!mounted) return;
        const row = [
          ...panel.querySelectorAll<HTMLElement>("[data-contact-id]"),
        ].find((element) => element.dataset.contactId === id);
        row?.scrollIntoView({ block: "nearest" });
        row
          ?.querySelector<HTMLButtonElement>(".details-button")
          ?.focus({ preventScroll: true });
      });
    });
  });

  function keyboard(event: KeyboardEvent) {
    if (event.defaultPrevented || document.querySelector("dialog[open]"))
      return;
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "f") {
      if (!searchInput) return;
      event.preventDefault();
      searchInput.focus();
      searchInput.select();
    } else if (event.key === "Escape") {
      query = "";
      expandedId = null;
      limit = 50;
    } else if (event.key === "ArrowDown" && event.target === searchInput) {
      const first = panel.querySelector<HTMLButtonElement>(
        ".contact-list button:not(:disabled)",
      );
      if (first) {
        event.preventDefault();
        first.focus();
      }
    } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      const target = event.target;
      if (!(target instanceof HTMLElement)) return;
      const row = target.closest<HTMLElement>("[data-contact-id]");
      if (!row) return;
      const rows = [
        ...panel.querySelectorAll<HTMLElement>("[data-contact-id]"),
      ];
      const next =
        rows[rows.indexOf(row) + (event.key === "ArrowDown" ? 1 : -1)];
      const button = next?.querySelector<HTMLButtonElement>(
        "button:not(:disabled)",
      );
      if (button) {
        event.preventDefault();
        button.focus();
      }
    }
  }

  async function toggleFavorite(contact: Contact) {
    if (refreshing || savingFavorite) return;
    savingFavorite = contact.id;
    favoriteError = null;
    const favorite = !contact.favorite;
    try {
      await setContactFavorite(contact.id, favorite);
      if (mounted && snapshot)
        snapshot = {
          ...snapshot,
          contacts: snapshot.contacts.map((item) =>
            item.id === contact.id ? { ...item, favorite } : item,
          ),
        };
      // A focus read may have captured favorites before this write completed.
      // Re-read current authorization and favorites, invalidating that result.
      if (mounted) await refresh(false, false);
    } catch {
      if (mounted)
        favoriteError = "Favorit nicht gespeichert. Versuche es erneut.";
    } finally {
      if (mounted) savingFavorite = null;
    }
  }

  async function openPrivacy() {
    privacyError = null;
    try {
      await openContactsSettings();
    } catch {
      if (mounted)
        privacyError =
          "Die Systemeinstellungen konnten nicht geöffnet werden. Öffne Datenschutz & Sicherheit → Kontakte manuell.";
    }
  }

  onMount(() => {
    mounted = true;
    desktop = isTauri();
    void refreshPreferences();
    let unlisten: UnlistenFn | undefined;
    if (desktop)
      void (async () => {
        try {
          const stop = await listen("contacts-changed", () => {
            // Remove projections immediately before rechecking current authorization.
            snapshot = null;
            void refresh(false, false);
          });
          if (mounted) unlisten = stop;
          else stop();
        } catch {
          // Manual/focus refresh stays available if event subscription fails.
        }
        if (mounted) void refresh(false, false);
      })();
    const reload = () => {
      if (document.visibilityState !== "hidden") {
        void refresh();
        void refreshPreferences();
      }
    };
    window.addEventListener("focus", reload);
    document.addEventListener("visibilitychange", reload);
    window.addEventListener("keydown", keyboard);
    return () => {
      mounted = false;
      ++requestId;
      ++preferencesRequest;
      unlisten?.();
      window.removeEventListener("keydown", keyboard);
      window.removeEventListener("focus", reload);
      document.removeEventListener("visibilitychange", reload);
    };
  });
</script>

<section
  bind:this={panel}
  class="contacts-panel"
  aria-labelledby="contacts-heading"
  aria-busy={refreshing}
>
  <div class="contacts-heading">
    <h1 id="contacts-heading">Kontakte</h1>
    {#if accessible}<span class="contacts-count" role="status"
        >{results.length}
        <span class="sr-only"
          >{results.length === 1 ? "Kontakt" : "Kontakte"}</span
        ></span
      >{/if}
    {#if desktop && accessible}<button
        class="icon-button refresh-contacts"
        type="button"
        aria-label="Kontakte aktualisieren"
        title="Kontakte aktualisieren"
        disabled={refreshing || !!savingFavorite}
        onclick={() => void refresh()}><Icon name="refresh" size={15} /></button
      >{/if}
  </div>
  {#if preferencesError}<p class="contacts-note" role="status">
      {preferencesError}
    </p>{/if}
  {#if privacyError}<p class="contacts-note contacts-error" role="alert">
      {privacyError}
    </p>{/if}

  {#if !desktop || permission === "unsupported"}
    <div class="empty-state">
      <Icon name="contacts" size={22} />
      <h2>Kontakte auf dem Mac</h2>
      <p>Öffne Ruf unter macOS, um dein Adressbuch zu verbinden.</p>
    </div>
  {:else if error}
    <div class="empty-state contacts-error">
      <h2>Kontakte nicht verfügbar</h2>
      <p role="alert">{error}</p>
      <button
        class="secondary-button"
        type="button"
        disabled={refreshing}
        onclick={() => void refresh()}>Erneut versuchen</button
      >
    </div>
  {:else if !snapshot}
    <p class="contacts-note" role="status">Kontakte werden geladen …</p>
  {:else if permission === "notDetermined"}
    <div class="empty-state">
      <Icon name="contacts" size={22} />
      <h2>Dein Adressbuch verbinden</h2>
      <p>Suche und rufe Kontakte aus deinem macOS-Adressbuch an.</p>
      <button
        class="primary-button"
        type="button"
        disabled={refreshing}
        onclick={() => void refresh(true)}>macOS-Kontakte freigeben</button
      >
      <p class="privacy-note">Dein Adressbuch bleibt unverändert.</p>
    </div>
  {:else if permission === "denied" || permission === "restricted"}
    <div class="empty-state">
      <Icon name="contacts" size={22} />
      <h2>
        {permission === "restricted"
          ? "Kontaktzugriff eingeschränkt"
          : "Kontakte nicht freigegeben"}
      </h2>
      <p>
        {permission === "restricted"
          ? "Eine Systemrichtlinie verhindert den Zugriff. Wende dich an die Verwaltung dieses Macs."
          : "Erlaube Ruf den Zugriff unter Datenschutz & Sicherheit → Kontakte."}
      </p>
      <button
        class="secondary-button"
        type="button"
        disabled={refreshing}
        onclick={() => void openPrivacy()}>Systemeinstellungen öffnen</button
      >
      <button
        class="text-button"
        type="button"
        disabled={refreshing}
        onclick={() => void refresh()}>Freigabe erneut prüfen</button
      >
    </div>
  {:else if accessible}
    <div class="contacts-tools">
      <label class="contact-search">
        <Icon name="search" size={16} />
        <span class="sr-only"
          >Kontakte nach Name, Firma oder Telefonnummer suchen</span
        >
        <input
          bind:this={searchInput}
          type="search"
          placeholder="Name, Firma oder Nummer"
          bind:value={query}
          oninput={() => {
            limit = 50;
            expandedId = null;
          }}
          autocomplete="off"
          spellcheck="false"
        />
      </label><button
        class="icon-button favorite-filter"
        type="button"
        aria-label="Nur Favoriten"
        title={favoritesOnly ? "Alle Kontakte anzeigen" : "Nur Favoriten"}
        aria-pressed={favoritesOnly}
        onclick={() => {
          favoritesOnly = !favoritesOnly;
          limit = 50;
          expandedId = null;
        }}><Icon name="star" size={16} /></button
      >
    </div>
    {#if !canDial}<p class="contacts-note call-hint">{callHint}</p>{/if}
    {#if favoriteError}<p class="contacts-note contacts-error" role="alert">
        {favoriteError}
      </p>{/if}
    {#if results.length === 0}
      <div class="empty-state">
        <h2>
          {query.trim()
            ? favoritesOnly
              ? "Keine passenden Favoriten"
              : "Keine passenden Kontakte"
            : favoritesOnly
              ? "Noch keine Favoriten"
              : "Dein Adressbuch ist leer"}
        </h2>
        <p>
          {query.trim()
            ? favoritesOnly
              ? "Ändere die Suche oder zeige alle Kontakte an."
              : "Versuche einen anderen Namen oder eine Nummer."
            : favoritesOnly
              ? "Markiere einen Kontakt mit dem Stern."
              : "Füge Kontakte in der macOS-App Kontakte hinzu."}
        </p>
        {#if query.trim() || favoritesOnly}<button
            class="secondary-button"
            type="button"
            onclick={() => {
              query = "";
              favoritesOnly = false;
              limit = 50;
            }}>Alle Kontakte anzeigen</button
          >{/if}
      </div>
    {:else}
      <ul class="contact-list" aria-label="macOS-Kontakte">
        {#each visible as contact (contact.id)}
          <ContactRow
            {contact}
            showOrganization={preferences.showOrganization}
            {canDial}
            {oncall}
            {onhistory}
            expanded={expandedId === contact.id}
            favoriteBusy={refreshing || !!savingFavorite}
            onexpand={() =>
              (expandedId = expandedId === contact.id ? null : contact.id)}
            onfavorite={() => void toggleFavorite(contact)}
          />
        {/each}
      </ul>
      {#if results.length > limit}<button
          class="secondary-button more-contacts"
          type="button"
          onclick={() => (limit += 50)}>Weitere Kontakte anzeigen</button
        >{/if}
    {/if}
  {/if}
</section>

<style>
  .contacts-panel {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 10px 14px;
  }
  .contacts-heading {
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
  .contacts-count {
    color: var(--muted);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
  .refresh-contacts {
    margin-left: auto;
    min-height: 28px;
    width: 28px;
    color: var(--muted);
  }
  .contacts-tools {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }
  .contact-search {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
    border: 1px solid var(--border-control);
    background: var(--surface-panel);
    border-radius: var(--radius);
    padding: 0 8px;
    color: var(--muted);
  }
  .contact-search input {
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
  .contact-search:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .contact-search input:focus-visible {
    outline: none;
  }
  .favorite-filter {
    width: 32px;
    min-height: 32px;
    flex-shrink: 0;
    color: var(--muted);
    border: 1px solid transparent;
  }
  .favorite-filter[aria-pressed="true"] {
    color: var(--accent);
    background: var(--surface-selected);
    border-color: var(--border);
  }
  .favorite-filter[aria-pressed="true"] :global(svg) {
    fill: currentColor;
  }
  .contact-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .contacts-note {
    font-size: 11px;
    line-height: 1.45;
    color: var(--muted);
    margin: 6px 0;
    overflow-wrap: anywhere;
  }
  .contacts-error {
    color: var(--danger);
  }
  .more-contacts {
    margin-top: 12px;
    width: 100%;
  }
  .empty-state .privacy-note {
    font-size: 11px;
    margin-top: 14px;
  }
</style>
