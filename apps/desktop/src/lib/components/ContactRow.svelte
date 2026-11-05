<script lang="ts">
  import type { Contact } from "../generated/AppInfo";
  import { copyPhoneNumber, openContactsApp } from "../contacts-api";
  import Icon from "./Icon.svelte";

  let {
    contact,
    expanded,
    canDial,
    favoriteBusy,
    showOrganization = true,
    onexpand,
    onfavorite,
    oncall,
    onhistory,
  }: {
    contact: Contact;
    expanded: boolean;
    canDial: boolean;
    favoriteBusy: boolean;
    showOrganization?: boolean;
    onexpand: () => void;
    onfavorite: () => void;
    oncall: (destination: string) => void;
    onhistory: (contact: Contact) => void;
  } = $props();
  const numbersId = $props.id();
  const first = $derived(contact.phones[0]);
  const multiple = $derived(contact.phones.length > 1);
  let copiedIndex = $state<number | null>(null);
  let actionError = $state<string | null>(null);

  async function copyNumber(number: string, index: number) {
    actionError = null;
    copiedIndex = null;
    try {
      await copyPhoneNumber(number);
      copiedIndex = index;
    } catch {
      actionError =
        "Die Nummer konnte nicht kopiert werden. Versuche es erneut oder markiere und kopiere sie direkt.";
    }
  }

  async function openSource() {
    actionError = null;
    try {
      await openContactsApp();
    } catch {
      actionError =
        "Kontakte konnte nicht geöffnet werden. Öffne die macOS-App Kontakte manuell.";
    }
  }
</script>

<li class="contact-entry" data-contact-id={contact.id}>
  <div class="contact-line">
    <button
      class="contact-main"
      type="button"
      disabled={!multiple && (!canDial || !first?.destination)}
      aria-expanded={multiple ? expanded : undefined}
      aria-controls={multiple && expanded ? numbersId : undefined}
      aria-label={multiple
        ? `${contact.name}: ${contact.phones.length} Nummern ${expanded ? "ausblenden" : "anzeigen"}`
        : first
          ? `${contact.name}, ${first.label}: ${first.number} anrufen`
          : `${contact.name}: Keine Nummer`}
      onclick={() => {
        if (multiple) onexpand();
        else if (canDial && first?.destination) oncall(first.destination);
      }}
    >
      <span class="contact-copy">
        <span class="contact-name">{contact.name}</span>
        <span class="contact-summary">
          {#if showOrganization && contact.organization && contact.organization !== contact.name}{contact.organization}{" · "}{/if}
          {multiple
            ? `${contact.phones.length} Nummern`
            : (first?.number ?? "Keine Nummer")}
        </span>
        {#if first && !multiple && !first.destination}<span class="unavailable"
            >Nicht wählbar</span
          >{/if}
      </span>
      {#if first}<Icon
          name={multiple
            ? expanded
              ? "chevronDown"
              : "chevronRight"
            : "phone"}
          size={15}
        />{/if}
    </button>
    <button
      class="icon-button details-button"
      type="button"
      aria-label={`Details zu ${contact.name}`}
      aria-expanded={expanded}
      aria-controls={expanded ? numbersId : undefined}
      onclick={onexpand}><Icon name="info" size={15} /></button
    >
    <button
      class="icon-button favorite-button"
      class:is-favorite={contact.favorite}
      type="button"
      aria-label={`${contact.name}: ${contact.favorite ? "Favorit entfernen" : "Als Favorit merken"}`}
      aria-pressed={contact.favorite}
      disabled={favoriteBusy}
      onclick={onfavorite}
    >
      <Icon name="star" size={15} />
    </button>
  </div>
  {#if expanded}
    <div
      class="contact-numbers"
      id={numbersId}
      role="group"
      aria-label={`${contact.name}: Kontaktdetails`}
    >
      {#each contact.phones as phone, index}
        <div class="number-line">
          <button
            class="contact-number"
            type="button"
            disabled={!canDial || !phone.destination}
            aria-label={`${contact.name}, ${phone.label}: ${phone.number} anrufen`}
            onclick={() => {
              if (canDial && phone.destination) oncall(phone.destination);
            }}
          >
            <span class="number-copy"
              ><span class="phone-label">{phone.label}</span><span
                >{phone.number}</span
              >
              {#if !phone.destination}<span class="unavailable"
                  >Nicht wählbar</span
                >{/if}</span
            >
            <Icon name="phone" size={15} />
          </button>
          <button
            class="text-button copy-number"
            type="button"
            aria-label={`${contact.name}, ${phone.label}: ${phone.number} kopieren`}
            onclick={() => void copyNumber(phone.number, index)}
            >{copiedIndex === index ? "Kopiert" : "Kopieren"}</button
          >
        </div>
      {/each}
      {#if contact.phones.length === 0}<p class="contact-note">
          Keine Telefonnummer vorhanden.
        </p>{/if}
      <div class="contact-actions">
        <button
          class="text-button"
          type="button"
          aria-label={`Anrufverlauf von ${contact.name}`}
          disabled={!contact.phones.some((phone) => phone.destination)}
          onclick={() => onhistory(contact)}>Anrufverlauf</button
        ><button
          class="text-button"
          type="button"
          onclick={() => void openSource()}>macOS-Kontakte öffnen</button
        >
      </div>
      {#if copiedIndex !== null}<span class="sr-only" role="status"
          >Nummer kopiert</span
        >{/if}
      {#if actionError}<p class="contact-note contact-error" role="alert">
          {actionError}
        </p>{/if}
    </div>
  {/if}
</li>

<style>
  .contact-entry {
    border-bottom: 1px solid var(--border);
  }
  .contact-line {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .contact-main,
  .contact-number {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    min-width: 0;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text);
    text-align: left;
  }
  .contact-main {
    flex: 1;
    min-height: 56px;
    padding: 9px 4px;
  }
  .contact-copy {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 2px;
    overflow-wrap: anywhere;
  }
  .contact-name {
    font-size: 13px;
    font-weight: 600;
    line-height: 1.4;
  }
  .contact-summary,
  .phone-label,
  .unavailable {
    font-size: 11px;
    color: var(--muted);
    line-height: 1.45;
  }
  .contact-summary {
    font-variant-numeric: tabular-nums;
  }
  .contact-main > :global(svg),
  .contact-number > :global(svg) {
    flex-shrink: 0;
    color: var(--muted);
  }
  .contact-main:hover:not(:disabled),
  .contact-number:hover:not(:disabled) {
    background: var(--surface-hover);
  }
  .contact-main:hover:not(:disabled) > :global(svg),
  .contact-number:hover:not(:disabled) > :global(svg) {
    color: var(--accent);
  }
  .contact-main:disabled,
  .contact-number:disabled {
    cursor: default;
  }
  .favorite-button {
    flex-shrink: 0;
    width: 28px;
    min-height: 28px;
    color: var(--muted);
  }
  .favorite-button.is-favorite {
    color: var(--accent);
  }
  .favorite-button.is-favorite :global(svg) {
    fill: currentColor;
  }
  .contact-numbers {
    padding: 0 4px 8px;
  }
  .number-line {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .number-line > .contact-number {
    flex: 1;
  }
  .copy-number {
    flex-shrink: 0;
    font-size: 11px;
  }
  .contact-actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px 12px;
    margin-top: 4px;
  }
  .contact-actions button {
    padding: 4px;
    min-height: 28px;
    font-size: 11px;
  }
  .contact-note {
    font-size: 11px;
    line-height: 1.45;
    color: var(--muted);
    margin: 6px 4px;
    overflow-wrap: anywhere;
  }
  .contact-error {
    color: var(--danger);
  }
  .number-copy {
    user-select: text;
  }
  .details-button {
    flex-shrink: 0;
  }
  .contact-number {
    width: 100%;
    min-height: 38px;
    padding: 5px 8px;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .number-copy {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 3px 8px;
    min-width: 0;
    overflow-wrap: anywhere;
  }
</style>
