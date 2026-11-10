<script lang="ts">
  import { t, text, message, locale, type MessageValue } from "../i18n";
  import { onMount, tick } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import type {
    CallSnapshot,
    CallState,
    AccountSnapshot,
    ContactReference,
    ContactsSnapshot,
  } from "../generated/AppInfo";
  import { errorMessage } from "../api";
  import { getContacts } from "../contacts-api";
  import { searchContacts } from "../contacts";
  import DtmfPad from "./DtmfPad.svelte";
  import { normalizeDialNumber } from "../dialing";
  import Icon from "./Icon.svelte";
  import { phoneActions, shortcutLabel, type CallAction } from "../shortcuts";
  const hint = (id: CallAction) =>
    shortcutLabel(phoneActions.find((action) => action.id === id)!);
  let {
    call,
    calls = [call],
    callerContact = null,
    accounts = [],
    elapsed,
    busy,
    compact,
    testAudio = false,
    audioUnknown = false,
    receivedDtmf = null,
    matchGermanNumbers = true,
    onsenddtmf = () => {},
    onanswer,
    onhangup,
    onmute,
    onhold,
    ontransfer,
    onconsult,
    onswitch,
    oncancelconsult,
    oncompletetransfer,
  }: {
    call: CallSnapshot;
    calls?: readonly CallSnapshot[];
    callerContact?: ContactReference | null;
    accounts?: readonly AccountSnapshot[];
    elapsed: string;
    busy: boolean;
    compact: boolean;
    testAudio?: boolean;
    audioUnknown?: boolean;
    receivedDtmf?: string | null;
    matchGermanNumbers?: boolean;
    onsenddtmf?: (id: string, digit: string) => void;
    onanswer: (id: string) => void;
    onhangup: (id: string) => void;
    onmute: (id: string, muted: boolean) => Promise<boolean>;
    onhold: (id: string, held: boolean) => void;
    ontransfer: (id: string, destination: string) => void;
    onconsult: (id: string, destination: string) => void;
    onswitch: (id: string) => void;
    oncancelconsult: (id: string) => void;
    oncompletetransfer: (id: string) => void;
  } = $props();
  const labels = $derived<Record<CallState, string>>({
    idle: $t("phone.call.idle"),
    calling: $t("phone.call.calling"),
    ringing: $t("phone.call.ringing"),
    earlyMedia: $t("phone.call.early_media"),
    incoming: $t("phone.call.incoming"),
    answering: $t("phone.call.answering"),
    active: $t("phone.call.active"),
    held: $t("phone.call.remote_held"),
  });
  const established = (item: CallSnapshot) =>
    item.state === "active" || item.state === "held";
  const peer = (item: CallSnapshot) =>
    item.contact?.name ??
    item.destination ??
    item.caller?.name ??
    item.caller?.number ??
    $t("phone.call.unknown_caller");
  const selectedPeer = $derived(
    call.contact?.name ?? callerContact?.name ?? peer(call),
  );
  const pending = $derived(
    calls.some((item) => item.holdPending || item.transfer === "pending"),
  );
  const child = $derived(calls.find((item) => item.consultation));
  const canChange = $derived(!busy && !audioUnknown && !pending);
  const securityLabel = $derived.by(() => {
    const security = call.security;
    if (!security.signalingTransport)
      return $t("phone.call.security_unconfirmed");
    let signaling = security.signalingTransport.toUpperCase();
    if (security.signalingTransport === "tls")
      signaling = security.tlsPeerVerified
        ? $t("phone.call.tls_verified")
        : $t("phone.call.tls_unconfirmed");
    let media = $t("phone.call.media_unknown");
    if (security.media === "sdesSrtp") media = "SRTP";
    else if (security.media === "unprotected")
      media = $t("phone.call.media_unprotected");
    return `${signaling} · ${media}`;
  });
  let muteRequest = $state<{
    id: string;
    desired: boolean;
    priorError: string;
  } | null>(null);
  const mutePending = $derived(muteRequest?.id === call.id);
  $effect(() => {
    const request = muteRequest;
    if (
      request &&
      (call.id !== request.id ||
        call.muted === request.desired ||
        (call.lastError !== null &&
          JSON.stringify(call.lastError) !== request.priorError))
    )
      muteRequest = null;
  });
  async function toggleMute() {
    if (busy || audioUnknown || mutePending) return;
    const request = {
      id: call.id,
      desired: !call.muted,
      priorError: JSON.stringify(call.lastError),
    };
    muteRequest = request;
    if (
      !(await onmute(request.id, request.desired)) &&
      muteRequest?.id === request.id
    )
      muteRequest = null;
  }
  let tonesOpen = $state(false);
  let tonesFor: string | undefined;
  $effect(() => {
    if (tonesFor !== call.id) {
      tonesFor = call.id;
      tonesOpen = false;
    }
  });
  const canSendTone = $derived(
    !busy &&
      !audioUnknown &&
      call.state === "active" &&
      !call.localHeld &&
      !call.remoteHeld &&
      !pending,
  );
  let transferOpen = $state(false);
  let target = $state("");
  let targetError = $state<MessageValue>("");
  let targetInput = $state<HTMLInputElement>();
  let transferButton = $state<HTMLButtonElement>();
  let contactsOpen = $state(false);
  let contacts = $state<ContactsSnapshot | null>(null);
  let contactsLoading = $state(false);
  let contactsError = $state<MessageValue>("");
  let query = $state("");
  let mounted = false;
  let contactsRequest = 0;
  const results = $derived(
    searchContacts(
      contacts?.permission === "authorized" ? contacts.contacts : [],
      query,
      false,
      true,
      matchGermanNumbers,
      $locale,
    ).slice(0, 30),
  );
  async function refreshContacts() {
    const request = ++contactsRequest;
    contactsLoading = true;
    contactsError = "";
    try {
      const next = await getContacts();
      if (mounted && request === contactsRequest)
        contacts =
          next.permission === "authorized" ? next : { ...next, contacts: [] };
    } catch {
      if (mounted && request === contactsRequest) {
        contacts = null;
        contactsError = message("phone.call.contacts_error");
      }
    } finally {
      if (mounted && request === contactsRequest) contactsLoading = false;
    }
  }
  onMount(() => {
    mounted = true;
    let unlisten: UnlistenFn | undefined;
    const refresh = () => {
      if (contactsOpen) void refreshContacts();
    };
    window.addEventListener("focus", refresh);
    if (isTauri())
      void listen("contacts-changed", refresh)
        .then((next) => {
          if (mounted) unlisten = next;
          else next();
        })
        .catch(() => {
          /* Focus refresh also revalidates OS authorization. */
        });
    return () => {
      mounted = false;
      ++contactsRequest;
      unlisten?.();
      window.removeEventListener("focus", refresh);
    };
  });
  function openTransfer() {
    tonesOpen = false;
    transferOpen = true;
    void tick().then(() => targetInput?.focus());
  }
  function closeTransfer() {
    transferOpen = false;
    void tick().then(() => transferButton?.focus());
  }
  /** The menu, action search and keyboard reuse the visible controls' guards. */
  export function availableActions(): Record<CallAction, boolean> {
    return {
      answer: call.state === "incoming" && !busy && !audioUnknown,
      end: call.state !== "idle" && !busy,
      mute: established(call) && !busy && !audioUnknown && !mutePending,
      hold: established(call) && canChange,
      transfer: established(call) && canChange && calls.length === 1,
      tones: established(call),
    };
  }
  function toggleTones() {
    tonesOpen = !tonesOpen;
    if (tonesOpen) transferOpen = false;
  }
  export function runAction(action: CallAction) {
    if (!availableActions()[action]) return;
    switch (action) {
      case "answer":
        onanswer(call.id);
        break;
      case "end":
        onhangup(call.id);
        break;
      case "mute":
        void toggleMute();
        break;
      case "hold":
        onhold(call.id, !call.localHeld);
        break;
      case "transfer":
        openTransfer();
        break;
      case "tones":
        toggleTones();
        break;
    }
  }
  function submitTarget(consult: boolean) {
    if (!canChange || !established(call) || calls.length !== 1) return;
    const normalized = normalizeDialNumber(target);
    if (!normalized) {
      targetError = message("phone.call.target_invalid");
      targetInput?.focus();
      return;
    }
    targetError = "";
    // Queue acceptance is not a native outcome. Preserve the draft through failure.
    (consult ? onconsult : ontransfer)(call.id, normalized);
  }
</script>

<section
  class="call-panel"
  class:compact-call={compact}
  class:has-controls={established(call) || calls.length > 1}
  aria-label={$t("phone.call.current")}
>
  {#if calls.length > 1}
    <div class="conversation-list" aria-label={$t("phone.call.conversations")}>
      {#each calls as item (item.id)}
        <div
          class="conversation-row"
          class:selected-conversation={item.id === call.id}
        >
          <div class="conversation-copy">
            <strong>{item.id === call.id ? selectedPeer : peer(item)}</strong>
            {#if (item.destination ?? item.caller?.number) && (item.destination ?? item.caller?.number) !== peer(item)}<span
                >{item.destination ?? item.caller?.number}</span
              >{/if}
            <span
              >{item.consultation
                ? $t("phone.call.consultation")
                : $t("phone.call.original")} · {accounts.find(
                (account) => account.id === item.accountId,
              )?.name ?? $t("phone.call.account")}</span
            >
            <span
              >{item.id === call.id
                ? $t("phone.call.current")
                : labels[item.state]}{item.localHeld
                ? ` · ${$t("phone.call.local_held")}`
                : ""}{item.remoteHeld
                ? ` · ${$t("phone.call.remote_held")}`
                : ""}</span
            >
            {#if item.id !== call.id && item.transfer === "failed"}<p
                class="call-error"
                role="alert"
              >
                {$t("phone.call.transfer_original_failed")}
                {#if item.lastError}{$text(errorMessage(item.lastError))}{/if}
              </p>{:else if item.id !== call.id && item.lastError}<p
                class="call-error"
                role="alert"
              >
                {$text(errorMessage(item.lastError))}
              </p>{/if}
          </div>
          {#if item.id !== call.id}<button
              class="secondary-button"
              type="button"
              disabled={!canChange || !established(item)}
              onclick={() => onswitch(item.id)}
              aria-label={$t("phone.call.switch_to", { name: peer(item) })}
              >{$t("phone.call.switch")}</button
            >{/if}
        </div>
      {/each}
    </div>
  {:else}
    <div class="call-details" role="status" aria-live="polite">
      <div class="call-symbol">
        <Icon name="phone" size={compact ? 16 : 22} />
      </div>
      <p class="call-state">
        <span>{labels[call.state]}</span>{accounts.find(
          (account) => account.id === call.accountId,
        )?.name
          ? ` · ${accounts.find((account) => account.id === call.accountId)?.name}`
          : ""}
      </p>
      <h1 class="call-peer">{selectedPeer}</h1>
      {#if (call.destination ?? call.caller?.number) && (call.destination ?? call.caller?.number) !== selectedPeer}<p
          class="call-number"
        >
          {call.destination ?? call.caller?.number}
        </p>{/if}
    </div>
  {/if}
  {#if established(call)}<p
      class="call-timer"
      aria-label={$t("phone.call.duration")}
    >
      {elapsed}
    </p>{/if}
  <div class="call-status" role="status" aria-live="polite">
    {#if call.localHeld && calls.length === 1}<p>
        {$t("phone.call.local_held")}
      </p>{/if}
    {#if call.remoteHeld && call.state !== "held" && calls.length === 1}<p>
        {$t("phone.call.remote_held")}
      </p>{/if}
    {#if pending}<p>
        {calls.some((item) => item.transfer === "pending")
          ? $t("phone.call.transfer_pending")
          : $t("phone.call.change_pending")}
      </p>{/if}
    {#if mutePending}<p>{$t("phone.call.mute_pending")}</p>{/if}
    {#if call.transfer === "failed"}<p>
        {$t("phone.call.transfer_failed")}
      </p>{/if}
    {#if call.transfer === "succeeded"}<p>
        {$t("phone.call.transfer_succeeded")}
      </p>{/if}
    {#if call.lastError}<p class="call-error">
        {$text(errorMessage(call.lastError))}
      </p>{/if}
  </div>
  <details class="call-security">
    <summary>{securityLabel}</summary>
    <p>
      {$t("phone.call.security_hint")}
    </p>
  </details>
  {#if receivedDtmf}<p class="call-test-note">
      {$t("phone.call.received_dtmf", { digit: receivedDtmf })}
    </p>{/if}
  {#if audioUnknown}<p class="call-test-note">
      {$t("phone.call.audio_unknown")}
    </p>{/if}
  {#if testAudio}<p class="call-test-note">
      {$t("phone.call.test_audio")}
    </p>{/if}
  <div class="call-actions">
    {#if call.state === "incoming"}<button
        class="primary-button"
        disabled={busy || audioUnknown}
        title={$t("phone.call.answer_shortcut", { shortcut: hint("answer") })}
        onclick={() => onanswer(call.id)}
        ><Icon name="phone" size={18} />{$t("phone.call.answer")}</button
      >{/if}
    {#if established(call)}
      <button
        class="secondary-button"
        type="button"
        disabled={busy || audioUnknown || mutePending}
        aria-pressed={call.muted}
        aria-busy={mutePending}
        title={$t("phone.call.mute_shortcut", { shortcut: hint("mute") })}
        onclick={toggleMute}
        >{call.muted ? $t("phone.call.unmute") : $t("phone.call.mute")}</button
      >
      <button
        class="secondary-button"
        type="button"
        disabled={!canChange}
        aria-pressed={call.localHeld}
        title={$t("phone.call.hold_shortcut", { shortcut: hint("hold") })}
        onclick={() => onhold(call.id, !call.localHeld)}
        >{call.localHeld
          ? $t("phone.call.resume")
          : $t("phone.call.hold")}</button
      >
      {#if calls.length === 1}<button
          bind:this={transferButton}
          class="secondary-button"
          type="button"
          disabled={!canChange}
          aria-expanded={transferOpen}
          title={$t("phone.call.transfer_shortcut", {
            shortcut: hint("transfer"),
          })}
          onclick={openTransfer}>{$t("phone.call.transfer")}</button
        >{/if}
    {/if}
    <button
      class="danger-button"
      disabled={busy}
      title={$t("phone.call.end_shortcut", { shortcut: hint("end") })}
      onclick={() => onhangup(call.id)}
      ><Icon name="endCall" size={20} />{call.state === "incoming"
        ? $t("phone.call.reject")
        : $t("phone.call.hang_up")}</button
    >
  </div>
  {#if established(call)}
    <button
      class="secondary-button tone-toggle"
      type="button"
      aria-expanded={tonesOpen}
      title={$t("phone.call.tones_shortcut", { shortcut: hint("tones") })}
      onclick={toggleTones}
      ><Icon name="keypad" size={16} />{tonesOpen
        ? $t("phone.call.hide_tones")
        : $t("phone.call.show_tones")}</button
    >
    {#if tonesOpen}<DtmfPad
        disabled={!canSendTone}
        onsend={(digit) => onsenddtmf(call.id, digit)}
      />{/if}
  {/if}
  {#if child && calls.length === 2}
    <div class="consultation-actions">
      <button
        class="secondary-button"
        type="button"
        disabled={!canChange}
        onclick={() => oncancelconsult(child.id)}
        >{$t("phone.call.cancel_consultation")}</button
      >
      <button
        class="primary-button"
        type="button"
        disabled={!canChange || !calls.every(established)}
        onclick={() => oncompletetransfer(child.id)}
        >{$t("phone.call.complete_transfer")}</button
      >
    </div>
  {/if}
  {#if transferOpen && calls.length === 1}
    <form class="transfer-form" onsubmit={(event) => event.preventDefault()}>
      <label for="transfer-target">{$t("phone.call.target")}</label>
      <input
        bind:this={targetInput}
        id="transfer-target"
        name="transfer-target"
        type="tel"
        inputmode="tel"
        maxlength="128"
        autocomplete="off"
        bind:value={target}
        aria-invalid={!!targetError}
        aria-describedby={targetError ? "transfer-error" : "transfer-help"}
        disabled={!canChange}
      />
      <p id="transfer-help">
        {$t("phone.call.transfer_hint")}
      </p>
      {#if targetError}<p id="transfer-error" class="field-error">
          {$text(targetError)}
        </p>{/if}
      <button
        class="text-button"
        type="button"
        aria-expanded={contactsOpen}
        onclick={() => {
          contactsOpen = !contactsOpen;
          if (contactsOpen) void refreshContacts();
        }}>{$t("phone.call.choose_contact")}</button
      >
      {#if contactsOpen}
        <div class="transfer-contacts">
          {#if contactsLoading}<p role="status">
              {$t("phone.call.contacts_loading")}
            </p>
          {:else if contactsError}<p role="status">{$text(contactsError)}</p>
          {:else if contacts?.permission !== "authorized"}<p>
              {$t("phone.call.contacts_denied")}
            </p>
          {:else}
            <label class="sr-only" for="transfer-contact-search"
              >{$t("phone.call.search_contact")}</label
            >
            <input
              id="transfer-contact-search"
              type="search"
              placeholder={$t("phone.call.search_placeholder")}
              bind:value={query}
            />
            <ul>
              {#each results as contact (contact.id)}{#each contact.phones as phone}<li
                  >
                    <button
                      class="text-button"
                      type="button"
                      disabled={!phone.destination || !canChange}
                      onclick={() => {
                        if (phone.destination) {
                          target = phone.destination;
                          targetError = "";
                          targetInput?.focus();
                        }
                      }}>{contact.name} · {phone.label}: {phone.number}</button
                    >
                  </li>{/each}{/each}
            </ul>
            {#if !results.length}<p>{$t("phone.call.no_contact")}</p>{/if}
          {/if}
        </div>
      {/if}
      <div class="transfer-actions">
        <button
          class="secondary-button"
          type="button"
          disabled={!canChange}
          onclick={() => submitTarget(false)}
          >{$t("phone.call.direct_transfer")}</button
        >
        <button
          class="primary-button"
          type="button"
          disabled={!canChange}
          onclick={() => submitTarget(true)}
          >{$t("phone.call.start_consultation")}</button
        >
        <button class="text-button" type="button" onclick={closeTransfer}
          >{$t("phone.call.close_transfer")}</button
        >
      </div>
    </form>
  {/if}
</section>
