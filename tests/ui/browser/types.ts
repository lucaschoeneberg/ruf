import type {
  AccountConfig,
  AppLocale,
  AccountSnapshot,
  AppPreferences,
  AudioTestKind,
  CallSnapshot,
  Contact,
  ContactsPermission,
  DesktopSettings,
  EngineSnapshot,
  HistoryEntry,
  HistoryQuery,
  NotificationPermission,
  PhoneError,
  SoundSettings,
} from "../../../apps/desktop/src/lib/generated/AppInfo";

// Legacy scenarios deliberately omit newer snapshot fields to exercise compatibility.
export type FixtureAccount = Omit<
  AccountSnapshot,
  "enabled" | "enabledChanging" | "registeredAtMs"
> &
  Partial<
    Pick<AccountSnapshot, "enabled" | "enabledChanging" | "registeredAtMs">
  >;
export type FixtureCall = Omit<CallSnapshot, "contact"> &
  Partial<Pick<CallSnapshot, "contact">>;
export type FixtureSnapshot = Omit<
  EngineSnapshot,
  | "accounts"
  | "soundSettings"
  | "calls"
  | "lastCallFailure"
  | "audioTest"
  | "audioTestError"
  | "historyError"
> & {
  historyError:
    | (Omit<PhoneError, "sipStatus"> & Partial<Pick<PhoneError, "sipStatus">>)
    | null;
  accounts: FixtureAccount[];
  soundSettings: Omit<SoundSettings, "ringtone" | "ringtoneVolume"> &
    Partial<Pick<SoundSettings, "ringtone" | "ringtoneVolume">>;
  calls?: FixtureCall[];
  lastCallFailure?: EngineSnapshot["lastCallFailure"];
  audioTest?: EngineSnapshot["audioTest"];
  audioTestError?: EngineSnapshot["audioTestError"];
  destination?: string;
};
export type FixtureContact = Contact;
export type FixtureHistoryEntry = HistoryEntry;
export type FixtureCallback = (event: {
  id?: number;
  event: string;
  payload: unknown;
}) => void;
export type FixtureListener = { event: string; handler: number };
// Each synthetic dispatcher reads only the arguments belonging to its matched command.
export type FixtureArgs = Partial<{
  id: number | string;
  event: string;
  handler: number;
  callId: string;
  destination: string;
  number: string;
  favorite: boolean;
  enabled: boolean;
  name: string;
  account: AccountConfig;
  query: Omit<HistoryQuery, "limit"> & { limit: number };
  preferences: AppPreferences;
  confirmHistoryDeletion: boolean;
  kind: AudioTestKind;
  device: string;
  sound: SoundSettings;
  muted: boolean;
  held: boolean;
  digit: string;
  section: string;
  link: string;
}>;
export type FixtureCommand = { command: string; args: FixtureArgs };
export type FixtureResult = { name: string; passed: boolean; error?: string };
type Deferred = (() => void) | null;
type RejectDeferred = ((reason: unknown) => void) | null;
type FixturePreferences = Omit<AppPreferences, "contacts"> & {
  contacts: Omit<AppPreferences["contacts"], "matchGermanNumbers"> &
    Partial<Pick<AppPreferences["contacts"], "matchGermanNumbers">>;
};
type Commands = { commands: FixtureCommand[] };
interface PhoneFixture extends Commands {
  state: FixtureSnapshot;
  permission: NotificationPermission;
  notificationsEnabled: boolean;
  failPermission: boolean;
  holdPermission: boolean;
  resolvePermission: Deferred;
  failSnapshot: boolean;
  failSelect: boolean;
  holdSnapshot: boolean;
  rejectSnapshot: RejectDeferred;
  resolveSnapshot: Deferred;
  holdSelect: boolean;
  resolveSelect: Deferred;
  failCallCommand: string | null;
  nextCall: number;
  older?: FixtureSnapshot;
  emit(payload?: FixtureSnapshot): void;
}
interface HistoryFixture extends Commands {
  state: FixtureSnapshot;
  entries: HistoryEntry[];
  contact: Contact;
  contactsAuthorized: boolean;
  failLoad: boolean;
  failDelete: boolean;
  failCopy: boolean;
  copied: string | null;
  recordingError:
    | (Omit<PhoneError, "sipStatus"> & Partial<Pick<PhoneError, "sipStatus">>)
    | null;
  delayed: Deferred;
  holdContacts: boolean;
  pendingContacts: Array<() => void>;
  emit(event?: string): void;
}
interface ContactFixture extends Commands {
  preferences: FixturePreferences;
  permission: ContactsPermission;
  decision: ContactsPermission;
  failRequest: boolean;
  failPreferences: boolean;
  failCopy: boolean;
  failOpen: boolean;
  holdFavorite: boolean;
  holdRead: boolean;
  resolveFavorite: Deferred;
  resolveRead: Deferred;
  copied: string | null;
  snapshot?: FixtureSnapshot;
  contacts?: Contact[];
  emit(): void;
}
interface SettingsFixture extends Commands {
  snapshot: FixtureSnapshot;
  desktop: DesktopSettings;
  preferences: AppPreferences;
  requestPermissionResult: NotificationPermission;
  closed: boolean;
  quit?: boolean;
  failDesktopSave: boolean;
  failTestNotification: boolean;
  failOpenNotificationSettings: boolean;
  microphonePermission: ContactsPermission;
  failMicrophoneRead: boolean;
  failMicrophoneRequest: boolean;
  failOpenMicrophoneSettings: boolean;
  contactsPermission: ContactsPermission;
  failContactsRead: boolean;
  failContactsRequest: boolean;
  failOpenContactsSettings: boolean;
  failPreferences: boolean;
  deviceAttempts: number;
  holdSnapshot: boolean;
  resolveSnapshot: Deferred;
  rejectSnapshot: RejectDeferred;
  holdSave: boolean;
  resolveSave: Deferred;
  holdMicrophonePermission?: boolean;
  resolveMicrophonePermission?: () => void;
  releaseSave?: () => void;
  releaseStop?: () => void;
  older?: FixtureSnapshot;
  emit(payload: FixtureSnapshot): void;
  event(event: string, payload?: unknown): void;
}
interface SearchFixture extends Commands {
  permission: ContactsPermission;
  failHistory: boolean;
  manyContacts: boolean;
  manyHistory: boolean;
  failContacts: boolean;
  holdHistory: boolean;
  pending: Array<() => void>;
  emit(event: string): void;
  release(): void;
}
interface IdentityFixture extends Commands {
  state: FixtureSnapshot & { calls: FixtureCall[] };
  contact: Contact;
  permission: ContactsPermission;
  emit(event: string): void;
  project(): FixtureSnapshot;
}
interface ControlsFixture extends Commands {
  state: FixtureSnapshot & { calls: FixtureCall[] };
  contactsPermission: ContactsPermission;
  fail: string | null;
  dialog(id: string, overrides?: Partial<FixtureCall>): FixtureCall;
  project(id?: string | null): void;
  emit(id?: string | null): void;
  request(action: string, payload?: unknown): void;
  contactsChanged(): void;
}
interface AppStoreFixture extends Commands {
  state: FixtureSnapshot;
  contacts: Contact[];
  emit(): void;
  setCall(kind: "incoming" | "active"): void;
}
export type DialogProjection = <
  T extends Partial<Omit<FixtureSnapshot, "calls">> & { calls?: FixtureCall[] },
>(
  snapshot: T,
) => T & {
  calls: FixtureCall[];
  lastCallFailure: EngineSnapshot["lastCallFailure"];
};
declare global {
  interface Window {
    __rufTestLanguage: AppLocale;
    isTauri: boolean;
    __rufTestSyncDialogs: DialogProjection;
    __phoneFixture: PhoneFixture;
    __historyFixture: HistoryFixture;
    __contactPermissionTest: ContactFixture;
    __settingsTest: SettingsFixture;
    __globalSearch: SearchFixture;
    __identity: IdentityFixture;
    __controls: ControlsFixture;
    __appStoreFixture: AppStoreFixture;
    __TAURI_EVENT_PLUGIN_INTERNALS__: {
      unregisterListener(event: string, id: number): void;
    };
    __TAURI_INTERNALS__: {
      metadata?: {
        currentWindow: { label: string };
        currentWebview: { label: string };
      };
      transformCallback(callback: FixtureCallback): number;
      unregisterCallback?(id: number): void;
      invoke(command: string, args: FixtureArgs): Promise<unknown>;
    };
  }
}
