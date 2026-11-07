<script lang="ts">
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
  const labels: Record<CallState, string> = {
    idle: "Kein Gespräch",
    calling: "Verbindung wird aufgebaut …",
    ringing: "Es klingelt …",
    earlyMedia: "Verbindung wird vorbereitet …",
    incoming: "Eingehender Anruf",
    answering: "Wird angenommen …",
    active: "Gespräch läuft",
    held: "Gegenstelle hält das Gespräch",
  };
  const established = (item: CallSnapshot) =>
    item.state === "active" || item.state === "held";
  const peer = (item: CallSnapshot) =>
    item.contact?.name ??
    item.destination ??
    item.caller?.name ??
    item.caller?.number ??
    "Unbekannter Anrufer";
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
    if (!security.signalingTransport) return "Sicherheit noch nicht bestätigt";
    let signaling = security.signalingTransport.toUpperCase();
    if (security.signalingTransport === "tls")
      signaling = security.tlsPeerVerified
        ? "TLS geprüft"
        : "TLS nicht bestätigt";
    let media = "Audioschutz unbekannt";
    if (security.media === "sdesSrtp") media = "SRTP";
    else if (security.media === "unprotected") media = "Audio unverschlüsselt";
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
  let targetError = $state("");
  let targetInput = $state<HTMLInputElement>();
  let transferButton = $state<HTMLButtonElement>();
  let contactsOpen = $state(false);
  let contacts = $state<ContactsSnapshot | null>(null);
  let contactsLoading = $state(false);
  let contactsError = $state("");
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
        contactsError =
          "Kontakte konnten nicht gelesen werden. Gib die Zielnummer ein.";
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
      targetError = "Gib 1–64 Ziffern oder + * # ein. Keine SIP-Adresse.";
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
  aria-label="Aktuelles Gespräch"
>
  {#if calls.length > 1}
    <div class="conversation-list" aria-label="Gespräche">
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
              >{item.consultation ? "Rückfrage" : "Ursprüngliches Gespräch"} · {accounts.find(
                (account) => account.id === item.accountId,
              )?.name ?? "SIP-Konto"}</span
            >
            <span
              >{item.id === call.id
                ? "Aktuelles Gespräch"
                : labels[item.state]}{item.localHeld
                ? " · Von dir gehalten"
                : ""}{item.remoteHeld
                ? " · Gegenstelle hält das Gespräch"
                : ""}</span
            >
            {#if item.id !== call.id && item.transfer === "failed"}<p
                class="call-error"
                role="alert"
              >
                Vermittlung fehlgeschlagen. Das ursprüngliche Gespräch bleibt
                bestehen.
                {#if item.lastError}{errorMessage(item.lastError)}{/if}
              </p>{:else if item.id !== call.id && item.lastError}<p
                class="call-error"
                role="alert"
              >
                {errorMessage(item.lastError)}
              </p>{/if}
          </div>
          {#if item.id !== call.id}<button
              class="secondary-button"
              type="button"
              disabled={!canChange || !established(item)}
              onclick={() => onswitch(item.id)}
              aria-label={`Zu ${peer(item)} wechseln`}>Wechseln</button
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
  {#if established(call)}<p class="call-timer" aria-label="Gesprächsdauer">
      {elapsed}
    </p>{/if}
  <div class="call-status" role="status" aria-live="polite">
    {#if call.localHeld && calls.length === 1}<p>Von dir gehalten</p>{/if}
    {#if call.remoteHeld && call.state !== "held" && calls.length === 1}<p>
        Gegenstelle hält das Gespräch
      </p>{/if}
    {#if pending}<p>
        {calls.some((item) => item.transfer === "pending")
          ? "Vermittlung läuft …"
          : "Gesprächsänderung wird abgestimmt …"}
      </p>{/if}
    {#if mutePending}<p>Stummschaltung wird bestätigt …</p>{/if}
    {#if call.transfer === "failed"}<p>
        Vermittlung fehlgeschlagen. Das Gespräch bleibt bestehen.
      </p>{/if}
    {#if call.transfer === "succeeded"}<p>Vermittlung bestätigt.</p>{/if}
    {#if call.lastError}<p class="call-error">
        {errorMessage(call.lastError)}
      </p>{/if}
  </div>
  <details class="call-security">
    <summary>{securityLabel}</summary>
    <p>
      Schutz bis zur Telefonanlage; keine Garantie für
      Ende-zu-Ende-Verschlüsselung. Die Anzeige folgt den nativen Beobachtungen.
    </p>
  </details>
  {#if receivedDtmf}<p class="call-test-note">
      Empfangene Wähltaste: {receivedDtmf}
    </p>{/if}
  {#if audioUnknown}<p class="call-test-note">
      Audioquelle unbekannt. Es kann ein Testton mit Aufzeichnung aktiv sein.
      Beende das Gespräch und starte die App neu.
    </p>{/if}
  {#if testAudio}<p class="call-test-note">
      Testton · empfangenes Audio wird aufgezeichnet
    </p>{/if}
  <div class="call-actions">
    {#if call.state === "incoming"}<button
        class="primary-button"
        disabled={busy || audioUnknown}
        title={`Annehmen (${hint("answer")})`}
        onclick={() => onanswer(call.id)}
        ><Icon name="phone" size={18} />Annehmen</button
      >{/if}
    {#if established(call)}
      <button
        class="secondary-button"
        type="button"
        disabled={busy || audioUnknown || mutePending}
        aria-pressed={call.muted}
        aria-busy={mutePending}
        title={`Mikrofon ein-/ausschalten (${hint("mute")})`}
        onclick={toggleMute}
        >{call.muted ? "Mikrofon einschalten" : "Stummschalten"}</button
      >
      <button
        class="secondary-button"
        type="button"
        disabled={!canChange}
        aria-pressed={call.localHeld}
        title={`Halten / Fortsetzen (${hint("hold")})`}
        onclick={() => onhold(call.id, !call.localHeld)}
        >{call.localHeld ? "Fortsetzen" : "Halten"}</button
      >
      {#if calls.length === 1}<button
          bind:this={transferButton}
          class="secondary-button"
          type="button"
          disabled={!canChange}
          aria-expanded={transferOpen}
          title={`Vermitteln (${hint("transfer")})`}
          onclick={openTransfer}>Vermitteln …</button
        >{/if}
    {/if}
    <button
      class="danger-button"
      disabled={busy}
      title={`Beenden / Ablehnen (${hint("end")})`}
      onclick={() => onhangup(call.id)}
      ><Icon name="endCall" size={20} />{call.state === "incoming"
        ? "Ablehnen"
        : "Auflegen"}</button
    >
  </div>
  {#if established(call)}
    <button
      class="secondary-button tone-toggle"
      type="button"
      aria-expanded={tonesOpen}
      title={`Gesprächstastatur (${hint("tones")})`}
      onclick={toggleTones}
      ><Icon name="keypad" size={16} />{tonesOpen
        ? "Gesprächstastatur ausblenden"
        : "Gesprächstastatur einblenden"}</button
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
        >Rückfrage abbrechen & zurück</button
      >
      <button
        class="primary-button"
        type="button"
        disabled={!canChange || !calls.every(established)}
        onclick={() => oncompletetransfer(child.id)}
        >Vermittlung abschließen</button
      >
    </div>
  {/if}
  {#if transferOpen && calls.length === 1}
    <form class="transfer-form" onsubmit={(event) => event.preventDefault()}>
      <label for="transfer-target">Zielnummer</label>
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
        Direkt vermitteln übergibt das Gespräch ohne Rückfrage. Eine Rückfrage
        hält dein Gespräch, während du mit dem Ziel sprichst.
      </p>
      {#if targetError}<p id="transfer-error" class="field-error">
          {targetError}
        </p>{/if}
      <button
        class="text-button"
        type="button"
        aria-expanded={contactsOpen}
        onclick={() => {
          contactsOpen = !contactsOpen;
          if (contactsOpen) void refreshContacts();
        }}>Kontakt auswählen</button
      >
      {#if contactsOpen}
        <div class="transfer-contacts">
          {#if contactsLoading}<p role="status">Kontakte werden gelesen …</p>
          {:else if contactsError}<p role="status">{contactsError}</p>
          {:else if contacts?.permission !== "authorized"}<p>
              Kontakte sind nicht freigegeben. Gib die Zielnummer ein; Freigaben
              findest du in den Einstellungen.
            </p>
          {:else}
            <label class="sr-only" for="transfer-contact-search"
              >Kontakt suchen</label
            >
            <input
              id="transfer-contact-search"
              type="search"
              placeholder="Name oder Nummer suchen"
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
            {#if !results.length}<p>Kein passender Kontakt.</p>{/if}
          {/if}
        </div>
      {/if}
      <div class="transfer-actions">
        <button
          class="secondary-button"
          type="button"
          disabled={!canChange}
          onclick={() => submitTarget(false)}>Direkt vermitteln</button
        >
        <button
          class="primary-button"
          type="button"
          disabled={!canChange}
          onclick={() => submitTarget(true)}>Rückfrage starten</button
        >
        <button class="text-button" type="button" onclick={closeTransfer}
          >Vermittlungsziel schließen</button
        >
      </div>
    </form>
  {/if}
</section>
