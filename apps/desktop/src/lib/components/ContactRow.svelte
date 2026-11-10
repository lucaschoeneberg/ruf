<script lang="ts">
  import { t, text, message, type MessageValue } from "../i18n";
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
  let actionError = $state<MessageValue | null>(null);

  async function copyNumber(number: string, index: number) {
    actionError = null;
    copiedIndex = null;
    try {
      await copyPhoneNumber(number);
      copiedIndex = index;
    } catch {
      actionError = message("phone.contacts.copy_error");
    }
  }

  async function openSource() {
    actionError = null;
    try {
      await openContactsApp(contact.id);
    } catch {
      actionError = message("phone.contacts.open_error");
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
        ? $t(
            expanded
              ? "phone.contacts.hide_numbers"
              : "phone.contacts.show_numbers",
            { name: contact.name, count: contact.phones.length },
          )
        : first
          ? $t("phone.contacts.call_number", {
              name: contact.name,
              label: first.label,
              number: first.number,
            })
          : $t("phone.contacts.named_no_number", { name: contact.name })}
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
            ? $t("phone.contacts.numbers", { count: contact.phones.length })
            : (first?.number ?? $t("phone.contacts.no_number"))}
        </span>
        {#if first && !multiple && !first.destination}<span class="unavailable"
            >{$t("phone.contacts.not_dialable")}</span
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
      aria-label={$t("phone.contacts.details", { name: contact.name })}
      aria-expanded={expanded}
      aria-controls={expanded ? numbersId : undefined}
      onclick={onexpand}><Icon name="info" size={15} /></button
    >
    <button
      class="icon-button favorite-button"
      class:is-favorite={contact.favorite}
      type="button"
      aria-label={$t(
        contact.favorite
          ? "phone.contacts.named_remove_favorite"
          : "phone.contacts.named_add_favorite",
        { name: contact.name },
      )}
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
      aria-label={$t("phone.contacts.named_details", { name: contact.name })}
    >
      {#each contact.phones as phone, index}
        <div class="number-line">
          <button
            class="contact-number"
            type="button"
            disabled={!canDial || !phone.destination}
            aria-label={$t("phone.contacts.call_number", {
              name: contact.name,
              label: phone.label,
              number: phone.number,
            })}
            onclick={() => {
              if (canDial && phone.destination) oncall(phone.destination);
            }}
          >
            <span class="number-copy"
              ><span class="phone-label">{phone.label}</span><span
                >{phone.number}</span
              >
              {#if !phone.destination}<span class="unavailable"
                  >{$t("phone.contacts.not_dialable")}</span
                >{/if}</span
            >
            <Icon name="phone" size={15} />
          </button>
          <button
            class="text-button copy-number"
            type="button"
            aria-label={$t("phone.contacts.copy_number", {
              name: contact.name,
              label: phone.label,
              number: phone.number,
            })}
            onclick={() => void copyNumber(phone.number, index)}
            >{copiedIndex === index
              ? $t("phone.common.copied")
              : $t("phone.common.copy")}</button
          >
        </div>
      {/each}
      {#if contact.phones.length === 0}<p class="contact-note">
          {$t("phone.contacts.no_phone")}
        </p>{/if}
      <div class="contact-actions">
        <button
          class="text-button"
          type="button"
          aria-label={$t("phone.contacts.named_history", {
            name: contact.name,
          })}
          disabled={!contact.phones.some((phone) => phone.destination)}
          onclick={() => onhistory(contact)}
          >{$t("phone.contacts.call_history")}</button
        ><button
          class="text-button"
          type="button"
          onclick={() => void openSource()}
          >{$t("phone.contacts.open_source")}</button
        >
      </div>
      {#if copiedIndex !== null}<span class="sr-only" role="status"
          >{$t("phone.contacts.copied")}</span
        >{/if}
      {#if actionError}<p class="contact-note contact-error" role="alert">
          {$text(actionError)}
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
