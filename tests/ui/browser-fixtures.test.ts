import { expect, test } from "bun:test";
import { runInNewContext } from "node:vm";
import { installDialogFixtures } from "./browser/dialog-fixtures.ts";
import type { EngineSnapshot } from "../../apps/desktop/src/lib/generated/AppInfo";

// Exercise the same closure-free serialized installer the CLI registers in the browser.
function installedProjection() {
  const context = { window: {} };
  runInNewContext(`(${installDialogFixtures.toString()})();`, context);
  return (
    context.window as {
      __rufTestSyncDialogs: <T extends Partial<EngineSnapshot>>(
        snapshot: T,
      ) => T & Pick<EngineSnapshot, "calls" | "lastCallFailure">;
    }
  ).__rufTestSyncDialogs;
}

test("serialized fixture installer projects owner, identity and safe unobserved defaults", () => {
  const sync = installedProjection();
  const snapshot = {
    call: "active" as const,
    callId: "original",
    callAccountId: 2,
    caller: null,
    destination: "4567",
    callConnectedAtMs: 123,
    callEvents: [
      {
        sequence: 1,
        callId: "original",
        event: { kind: "muteChanged" as const, muted: true },
      },
    ],
  };
  const result = sync(snapshot);
  expect(result === snapshot).toBe(true);
  expect(result.calls).toEqual([
    {
      contact: null,
      id: "original",
      accountId: 2,
      state: "active",
      destination: "4567",
      caller: null,
      connectedAtMs: 123,
      muted: false,
      localHeld: false,
      remoteHeld: false,
      holdPending: false,
      transfer: "idle",
      consultation: false,
      events: snapshot.callEvents,
      eventsDropped: 0,
      lastError: null,
      security: {
        signalingTransport: null,
        tlsPeerVerified: false,
        media: "unknown",
      },
    },
  ]);
  expect(result.lastCallFailure).toBeNull();
});

test("fixture projection preserves confirmed fields and terminal ownership while refreshing legacy selection", () => {
  const sync = installedProjection();
  const result = sync({ call: "active", callId: "original", callAccountId: 2 });
  const current = result.calls[0]!;
  Object.assign(current, {
    muted: true,
    localHeld: true,
    holdPending: true,
    transfer: "pending",
    security: {
      signalingTransport: "tls",
      tlsPeerVerified: true,
      media: "sdesSrtp",
    },
  });
  const failure = {
    callId: "ended-child",
    accountId: 3,
    error: { code: "sipBusy" as const, sipStatus: 486, message: "safe" },
  };
  const caller = { name: "Incoming", number: "678" };
  const next = sync({
    ...result,
    call: "held",
    caller,
    callConnectedAtMs: 456,
    callEvents: [],
    lastCallFailure: failure,
  });
  expect(next.calls[0]).toMatchObject({
    state: "held",
    caller,
    connectedAtMs: 456,
    events: [],
    muted: true,
    localHeld: true,
    holdPending: true,
    transfer: "pending",
    security: {
      signalingTransport: "tls",
      tlsPeerVerified: true,
      media: "sdesSrtp",
    },
  });
  expect(next.lastCallFailure).toBe(failure);
  expect(sync({ ...next, call: "idle", callId: null }).calls).toEqual([]);
});
