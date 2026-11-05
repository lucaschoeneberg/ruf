// Shared native-shaped projection for legacy browser regression scenarios only.
// Serialized into context.addInitScript: keep this installer free of module closures.
export function installDialogFixtures() {
  window.__rufTestSyncDialogs = (snapshot) => {
    snapshot.lastCallFailure ??= null;
    if (snapshot.call === "idle" || !snapshot.callId) snapshot.calls = [];
    else {
      const previous = snapshot.calls?.find(
        (call) => call.id === snapshot.callId,
      );
      const selected = {
        id: snapshot.callId,
        accountId: snapshot.callAccountId,
        state: snapshot.call,
        destination: snapshot.caller
          ? null
          : (snapshot.destination ?? previous?.destination ?? "1234"),
        caller: snapshot.caller ?? null,
        connectedAtMs: snapshot.callConnectedAtMs ?? null,
        muted: false,
        localHeld: false,
        remoteHeld: snapshot.call === "held",
        holdPending: false,
        transfer: "idle",
        consultation: false,
        events: snapshot.callEvents ?? [],
        eventsDropped: 0,
        lastError: null,
        security: {
          signalingTransport: null,
          tlsPeerVerified: false,
          media: "unknown",
        },
        ...previous,
        state: snapshot.call,
        caller: snapshot.caller ?? null,
        connectedAtMs: snapshot.callConnectedAtMs ?? null,
        events: snapshot.callEvents ?? [],
      };
      snapshot.calls = [selected];
    }
    return snapshot;
  };
}
