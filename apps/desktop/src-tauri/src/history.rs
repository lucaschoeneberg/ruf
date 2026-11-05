//! Ordered native call observation storage and password-free history commands.
use crate::phone::PhoneState;
use phone_core::{
    AccountSnapshot, CallDirection, CallOutcome, CallState, Contact, EngineSnapshot, ErrorCode,
    HistoryEntry, HistoryFilter, HistoryPage, HistoryPreferences, HistoryQuery, ObservedCallEvent,
    ObservedCallKind, PhoneError, RegistrationState, contact_dial_target, contact_match_key,
    resolve_contact,
};
use std::sync::{Arc, Mutex, mpsc};
use tauri::{Emitter, Manager};

mod database;

fn storage_error() -> PhoneError {
    PhoneError::new(
        ErrorCode::StorageFailure,
        "Call history storage is unavailable. Check access to the application data folder.",
    )
}

fn validate_query(query: &HistoryQuery) -> Result<(), PhoneError> {
    if query.search.chars().count() > 200
        || query.search.chars().any(char::is_control)
        || !(1..=100).contains(&query.limit.unwrap_or(50))
        || query.contact_numbers.len() > 100
        || query
            .contact_numbers
            .iter()
            .any(|number| contact_match_key(number).is_none())
    {
        return Err(PhoneError::new(
            ErrorCode::InvalidInput,
            "Invalid history search, page size or contact numbers.",
        ));
    }
    Ok(())
}

fn callback_account(entry: &HistoryEntry, accounts: &[AccountSnapshot]) -> Option<u32> {
    contact_dial_target(entry.number.as_deref()?)?;
    let registrar = entry.account_registrar.as_deref()?;
    let username = entry.account_username.as_deref()?;
    let mut matches = accounts.iter().filter(|account| {
        account.registrar.eq_ignore_ascii_case(registrar)
            && account.username == username
            && account.registration == RegistrationState::Registered
            && !account.removing
    });
    let id = matches.next()?.id;
    matches.next().is_none().then_some(id)
}

fn project_page(
    entries: Vec<HistoryEntry>,
    query: &HistoryQuery,
    contacts: &[Contact],
    accounts: &[AccountSnapshot],
    recording_error: Option<PhoneError>,
) -> Result<HistoryPage, PhoneError> {
    validate_query(query)?;
    let search = query.search.trim().to_lowercase();
    let numbers: Vec<String> = query
        .contact_numbers
        .iter()
        .filter_map(|n| contact_match_key(n))
        .collect();
    let entries: Vec<HistoryEntry> = entries
        .into_iter()
        .filter_map(|mut entry| {
            if !numbers.is_empty()
                && !entry
                    .number
                    .as_deref()
                    .and_then(contact_match_key)
                    .is_some_and(|key| numbers.contains(&key))
            {
                return None;
            }
            entry.contact = entry
                .number
                .as_deref()
                .and_then(|number| resolve_contact(contacts, number));
            entry.callback_account_id = callback_account(&entry, accounts);
            let matches = search.is_empty()
                || [
                    Some(entry.account_name.as_str()),
                    entry.account_registrar.as_deref(),
                    entry.account_username.as_deref(),
                    entry.number.as_deref(),
                    entry.remote_name.as_deref(),
                    entry.contact.as_ref().map(|contact| contact.name.as_str()),
                ]
                .into_iter()
                .flatten()
                .any(|text| text.to_lowercase().contains(&search));
            matches.then_some(entry)
        })
        .collect();
    let total = u32::try_from(entries.len()).map_err(|_| storage_error())?;
    let limit = query.limit.unwrap_or(50) as usize;
    let has_more =
        u64::from(query.offset) + u64::from(query.limit.unwrap_or(50)) < u64::from(total);
    Ok(HistoryPage {
        entries: entries
            .into_iter()
            .skip(query.offset as usize)
            .take(limit)
            .collect(),
        total,
        has_more,
        recording_error,
    })
}

/// Latch each call's collection policy at Started; ignored calls can never be
/// reconstructed by a later Connected/Finished event or a repeated Started.
struct RecordingPolicy {
    enabled: bool,
    choices: std::collections::HashMap<u64, bool>,
}
impl RecordingPolicy {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            choices: Default::default(),
        }
    }
    fn record(&mut self, event: &ObservedCallEvent) -> bool {
        if matches!(event.kind, ObservedCallKind::Started { .. }) {
            *self
                .choices
                .entry(event.observation_id)
                .or_insert(self.enabled)
        } else {
            self.choices
                .get(&event.observation_id)
                .copied()
                .unwrap_or(false)
        }
    }
}

type Reply<T> = mpsc::Sender<Result<T, PhoneError>>;
enum Message {
    Observe(Box<ObservedCallEvent>),
    Entries(HistoryFilter, Reply<Vec<HistoryEntry>>),
    Page(HistoryFilter, u32, u32, Reply<(Vec<HistoryEntry>, u32)>),
    Entry(String, Reply<HistoryEntry>),
    Delete(String, Reply<()>),
    Clear(Reply<()>),
    Configure(
        HistoryPreferences,
        bool,
        Box<dyn FnOnce() -> Result<(), PhoneError> + Send>,
        Reply<()>,
    ),
    Stop,
}

/// One queue orders native lifecycle writes before subsequent history commands.
struct HistoryState {
    sender: mpsc::Sender<Message>,
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
    error: Arc<Mutex<Option<PhoneError>>>,
}
impl HistoryState {
    fn request<T>(&self, message: impl FnOnce(Reply<T>) -> Message) -> Result<T, PhoneError> {
        let (sender, receiver) = mpsc::channel();
        self.sender
            .send(message(sender))
            .map_err(|_| storage_error())?;
        receiver.recv().map_err(|_| storage_error())?
    }
}

fn mark_error(error: &Mutex<Option<PhoneError>>) {
    if let Ok(mut error) = error.lock() {
        *error = Some(storage_error());
    }
}

fn worker(
    app: tauri::AppHandle,
    path: Result<std::path::PathBuf, PhoneError>,
    receiver: mpsc::Receiver<Message>,
    error: Arc<Mutex<Option<PhoneError>>>,
    initial_preferences: Result<HistoryPreferences, PhoneError>,
) {
    let initial_failed = initial_preferences.is_err();
    let mut preferences = initial_preferences.unwrap_or(HistoryPreferences {
        recording_enabled: false,
        retention_days: None,
    });
    let mut policy = RecordingPolicy::new(preferences.recording_enabled);
    let mut database = path.and_then(|path| database::HistoryDb::open(&path));
    if database.is_err() || initial_failed {
        mark_error(&error);
        let _ = app.emit("history-changed", ());
    }
    let mut last_prune = std::time::Instant::now();
    if let Ok(db) = &mut database
        && prune(db, &preferences).is_err()
    {
        mark_error(&error);
    }
    loop {
        let message = match receiver.recv_timeout(std::time::Duration::from_secs(60)) {
            Ok(message) => Some(message),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        let mut changed = false;
        if last_prune.elapsed() >= std::time::Duration::from_secs(24 * 60 * 60) {
            if let Ok(db) = &mut database {
                match prune(db, &preferences) {
                    Ok(update) => changed |= update,
                    Err(_) => {
                        mark_error(&error);
                        changed = true;
                    }
                }
            }
            last_prune = std::time::Instant::now();
        }
        let Some(message) = message else {
            if changed {
                let _ = app.emit("history-changed", ());
            }
            continue;
        };
        match message {
            Message::Observe(event) if !policy.record(&event) => {}
            Message::Observe(event) => match database
                .as_mut()
                .map_err(|_| storage_error())
                .and_then(|db| db.observe(*event))
            {
                Ok(update) => changed |= update,
                Err(_) => {
                    mark_error(&error);
                    changed = true;
                }
            },
            Message::Entries(filter, reply) => {
                let result = database
                    .as_ref()
                    .map_err(|_| storage_error())
                    .and_then(|db| db.entries(filter));
                if result
                    .as_ref()
                    .is_err_and(|e| e.code == ErrorCode::StorageFailure)
                {
                    mark_error(&error);
                }
                let _ = reply.send(result);
            }
            Message::Page(filter, offset, limit, reply) => {
                let result = database
                    .as_ref()
                    .map_err(|_| storage_error())
                    .and_then(|db| db.page(filter, offset, limit));
                if result
                    .as_ref()
                    .is_err_and(|e| e.code == ErrorCode::StorageFailure)
                {
                    mark_error(&error);
                }
                let _ = reply.send(result);
            }
            Message::Entry(id, reply) => {
                let result = database
                    .as_ref()
                    .map_err(|_| storage_error())
                    .and_then(|db| db.get(&id));
                if result
                    .as_ref()
                    .is_err_and(|e| e.code == ErrorCode::StorageFailure)
                {
                    mark_error(&error);
                }
                let _ = reply.send(result);
            }
            Message::Delete(id, reply) => {
                let result = database
                    .as_mut()
                    .map_err(|_| storage_error())
                    .and_then(|db| db.delete(&id));
                changed |= result.as_ref().is_ok_and(|changed| *changed);
                if result
                    .as_ref()
                    .is_err_and(|e| e.code == ErrorCode::StorageFailure)
                {
                    mark_error(&error);
                }
                let _ = reply.send(result.map(|_| ()));
            }
            Message::Clear(reply) => {
                let result = database
                    .as_mut()
                    .map_err(|_| storage_error())
                    .and_then(database::HistoryDb::clear);
                changed |= result.as_ref().is_ok_and(|changed| *changed);
                if result
                    .as_ref()
                    .is_err_and(|e| e.code == ErrorCode::StorageFailure)
                {
                    mark_error(&error);
                }
                let _ = reply.send(result.map(|_| ()));
            }
            Message::Configure(next, confirmed, persist, reply) => {
                let result = database
                    .as_mut()
                    .map_err(|_| storage_error())
                    .and_then(|db| db.configure(&preferences, &next, confirmed, now_ms(), persist));
                if let Ok(update) = &result {
                    changed |= *update;
                    policy.enabled = next.recording_enabled;
                    preferences = next;
                    last_prune = std::time::Instant::now();
                } else if result
                    .as_ref()
                    .is_err_and(|e| e.code == ErrorCode::StorageFailure)
                {
                    mark_error(&error);
                    changed = true;
                }
                let _ = reply.send(result.map(|_| ()));
            }
            Message::Stop => break,
        }
        if changed {
            let _ = app.emit("history-changed", ());
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

fn retention_cutoff(preferences: &HistoryPreferences, now_ms: u64) -> Option<u64> {
    preferences
        .retention_days
        .map(|days| now_ms.saturating_sub(u64::from(days) * 24 * 60 * 60 * 1000))
}

fn prune(
    db: &mut database::HistoryDb,
    preferences: &HistoryPreferences,
) -> Result<bool, PhoneError> {
    retention_cutoff(preferences, now_ms()).map_or(Ok(false), |cutoff| db.prune_before(cutoff))
}

/// Serialize policy changes and metadata persistence with native lifecycle writes.
/// Deletion is rolled back if metadata cannot be saved; callers restore metadata
/// if a SQLite commit fails after persistence.
pub(crate) fn configure_preferences(
    app: &tauri::AppHandle,
    preferences: HistoryPreferences,
    confirmed: bool,
    persist: impl FnOnce() -> Result<(), PhoneError> + Send + 'static,
) -> Result<(), PhoneError> {
    app.try_state::<HistoryState>()
        .ok_or_else(storage_error)?
        .request(|reply| Message::Configure(preferences, confirmed, Box::new(persist), reply))
}

/// Initialize independent history storage before account restoration starts SIP.
/// A storage failure is exposed in history/engine state and does not stop telephony.
pub(crate) fn setup(app: &tauri::AppHandle) {
    if app.try_state::<HistoryState>().is_some() {
        return;
    }
    let directory = crate::store::data_directory(app);
    let initial_preferences = directory
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|directory| {
            crate::store::AccountStore {
                path: directory.join("accounts.json"),
            }
            .load()
            .map(|settings| settings.preferences.history)
        });
    let path = directory.map(|path| path.join("history.sqlite3"));
    let (sender, receiver) = mpsc::channel();
    let error = Arc::new(Mutex::new(None));
    let worker_app = app.clone();
    let worker_error = Arc::clone(&error);
    let worker = std::thread::Builder::new()
        .name("call-history".into())
        .spawn(move || {
            worker(
                worker_app,
                path,
                receiver,
                worker_error,
                initial_preferences,
            )
        });
    let worker = match worker {
        Ok(worker) => Some(worker),
        Err(_) => {
            mark_error(&error);
            None
        }
    };
    let sink = sender.clone();
    app.manage(HistoryState {
        sender,
        worker: Mutex::new(worker),
        error: Arc::clone(&error),
    });
    if app
        .state::<PhoneState>()
        .with(|session| {
            session.set_call_observer(Some(Arc::new(move |event| {
                let _ = sink.send(Message::Observe(Box::new(event)));
            })));
            Ok(())
        })
        .is_err()
    {
        mark_error(&error);
    }
}

/// Drain queued writes after the native engine has stopped producing observations.
pub(crate) fn shutdown(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<HistoryState>() else {
        return;
    };
    let _ = state.sender.send(Message::Stop);
    if let Ok(mut worker) = state.worker.lock()
        && let Some(worker) = worker.take()
        && worker.join().is_err()
    {
        mark_error(&state.error);
    }
}

/// Return a safe sticky storage warning without accessing disk or blocking SIP.
pub(crate) fn recording_error(app: &tauri::AppHandle) -> Option<PhoneError> {
    let state = app.try_state::<HistoryState>()?;
    state
        .error
        .lock()
        .map_or_else(|_| Some(storage_error()), |error| error.clone())
}

/// Read a newest-first page; authorized contact and account matches are transient.
#[tauri::command]
pub(crate) async fn get_history(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    query: HistoryQuery,
) -> Result<HistoryPage, PhoneError> {
    validate_query(&query)?;
    let phone = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let history = app.try_state::<HistoryState>().ok_or_else(storage_error)?;
        let accounts =
            phone.with(|session| session.snapshot().map(|snapshot| snapshot.accounts))?;
        let contacts = crate::contacts::cached_contacts(&app);
        if query.search.trim().is_empty() && query.contact_numbers.is_empty() {
            let limit = query.limit.unwrap_or(50);
            let (entries, total) =
                history.request(|reply| Message::Page(query.filter, query.offset, limit, reply))?;
            let has_more = u64::from(query.offset) + u64::from(limit) < u64::from(total);
            let mut page = project_page(
                entries,
                &HistoryQuery { offset: 0, ..query },
                &contacts,
                &accounts,
                recording_error(&app),
            )?;
            page.total = total;
            page.has_more = has_more;
            Ok(page)
        } else {
            let entries = history.request(|reply| Message::Entries(query.filter, reply))?;
            project_page(entries, &query, &contacts, &accounts, recording_error(&app))
        }
    })
    .await
    .map_err(|_| storage_error())?
}

/// Delete a completed record. Unknown IDs are invalid; ongoing calls return Busy.
#[tauri::command]
pub(crate) async fn delete_history_entry(
    app: tauri::AppHandle,
    id: String,
) -> Result<(), PhoneError> {
    tauri::async_runtime::spawn_blocking(move || {
        app.try_state::<HistoryState>()
            .ok_or_else(storage_error)?
            .request(|reply| Message::Delete(id, reply))
    })
    .await
    .map_err(|_| storage_error())?
}

/// Clear completed history while preserving every ongoing call.
#[tauri::command]
pub(crate) async fn clear_history(app: tauri::AppHandle) -> Result<(), PhoneError> {
    tauri::async_runtime::spawn_blocking(move || {
        app.try_state::<HistoryState>()
            .ok_or_else(storage_error)?
            .request(Message::Clear)
    })
    .await
    .map_err(|_| storage_error())?
}

/// Call the recorded target on its original, currently registered SIP identity.
/// Requires an idle phone; deleted/unregistered identities never fall back.
#[tauri::command]
pub(crate) async fn callback_history(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    id: String,
) -> Result<EngineSnapshot, PhoneError> {
    let phone = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let entry = app
            .try_state::<HistoryState>()
            .ok_or_else(storage_error)?
            .request(|reply| Message::Entry(id, reply))?;
        let snapshot = phone.with(|session| {
            let snapshot = session.snapshot()?;
            if snapshot.call != CallState::Idle {
                return Err(PhoneError::new(
                    ErrorCode::Busy,
                    "End the current call before calling from history.",
                ));
            }
            let number = entry
                .number
                .as_deref()
                .and_then(contact_dial_target)
                .ok_or_else(|| {
                    PhoneError::new(
                        ErrorCode::InvalidInput,
                        "This history entry has no supported number.",
                    )
                })?;
            let account_id = callback_account(&entry, &snapshot.accounts).ok_or_else(|| {
                PhoneError::new(
                    ErrorCode::NotRegistered,
                    "The original account is no longer available and registered.",
                )
            })?;
            session.select(account_id)?;
            session.engine()?.dial(&number)?;
            session.snapshot()
        })?;
        let mut snapshot = snapshot;
        crate::contacts::enrich_snapshot(&app, &mut snapshot);
        Ok(snapshot)
    })
    .await
    .map_err(|_| storage_error())?
}
#[cfg(test)]
mod query_tests {
    use super::*;
    use phone_core::{
        AccountSnapshot, Contact, ContactPhone, ContactSource, HistoryQuery, RegistrationState,
    };
    fn entry() -> HistoryEntry {
        HistoryEntry {
            id: "synthetic".into(),
            account_name: "Work".into(),
            account_registrar: Some("PBX.test".into()),
            account_username: Some("100".into()),
            direction: CallDirection::Incoming,
            number: Some("+49 123".into()),
            remote_name: Some("Literal %_".into()),
            started_at_ms: 100,
            answered_at_ms: None,
            ended_at_ms: Some(101),
            duration_ms: None,
            outcome: CallOutcome::Missed,
            sip_status: None,
            contact: None,
            callback_account_id: None,
        }
    }
    fn account() -> AccountSnapshot {
        AccountSnapshot {
            transport: Default::default(),
            media_security: Default::default(),
            auth_username: None,
            outbound_proxy: None,
            id: 7,
            name: "Renamed".into(),
            registrar: "pbx.TEST".into(),
            username: "100".into(),
            registration: RegistrationState::Registered,
            last_error: None,
            removing: false,
            enabled: true,
            enabled_changing: false,
            registered_at_ms: None,
        }
    }
    fn contact() -> Contact {
        Contact {
            id: "test-contact".into(),
            source: ContactSource::Macos,
            name: "New name".into(),
            organization: String::new(),
            phones: vec![ContactPhone {
                label: String::new(),
                number: "0049123".into(),
                destination: Some("0049123".into()),
            }],
            favorite: false,
        }
    }
    #[test]
    fn callback_uses_original_identity_and_requires_registered_nonremoving_account() {
        let row = entry();
        let mut a = account();
        assert_eq!(callback_account(&row, &[a.clone()]), Some(7));
        a.username = "other".into();
        assert_eq!(callback_account(&row, &[a.clone()]), None);
        a = account();
        a.registration = RegistrationState::Failed;
        assert_eq!(callback_account(&row, &[a.clone()]), None);
        a = account();
        a.removing = true;
        assert_eq!(callback_account(&row, &[a]), None);
        let mut hidden = row;
        hidden.number = None;
        assert_eq!(callback_account(&hidden, &[account()]), None);
    }
    #[test]
    fn literal_search_current_contact_names_contact_scope_and_paging() {
        let mut second = entry();
        second.id = "older".into();
        second.number = Some("555".into());
        second.remote_name = None;
        let rows = vec![entry(), second];
        let contacts = vec![contact()];
        let query = HistoryQuery {
            search: "%_".into(),
            ..Default::default()
        };
        assert_eq!(
            project_page(rows.clone(), &query, &contacts, &[account()], None)
                .unwrap()
                .total,
            1
        );
        let query = HistoryQuery {
            search: "new NAME".into(),
            ..Default::default()
        };
        let page = project_page(rows.clone(), &query, &contacts, &[account()], None).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.entries[0].contact.as_ref().unwrap().name, "New name");
        assert_eq!(page.entries[0].callback_account_id, Some(7));
        assert_eq!(
            project_page(rows.clone(), &query, &[], &[account()], None)
                .unwrap()
                .total,
            0
        );
        let query = HistoryQuery {
            contact_numbers: vec!["0049123".into()],
            ..Default::default()
        };
        assert_eq!(
            project_page(rows.clone(), &query, &contacts, &[], None)
                .unwrap()
                .total,
            1
        );
        let query = HistoryQuery {
            limit: Some(1),
            ..Default::default()
        };
        let page = project_page(rows.clone(), &query, &contacts, &[], None).unwrap();
        assert!(page.has_more);
        assert_eq!(page.total, 2);
        let query = HistoryQuery { offset: 1, ..query };
        let page = project_page(rows, &query, &[], &[], None).unwrap();
        assert!(!page.has_more);
        assert_eq!(page.entries[0].id, "older");
    }
    #[test]
    fn invalid_query_bounds_are_rejected() {
        for query in [
            HistoryQuery {
                limit: Some(0),
                ..Default::default()
            },
            HistoryQuery {
                limit: Some(101),
                ..Default::default()
            },
            HistoryQuery {
                search: "a".repeat(201),
                ..Default::default()
            },
            HistoryQuery {
                contact_numbers: vec!["1".into(); 101],
                ..Default::default()
            },
            HistoryQuery {
                contact_numbers: vec!["sip:other@test".into()],
                ..Default::default()
            },
        ] {
            assert_eq!(
                validate_query(&query).unwrap_err().code,
                ErrorCode::InvalidInput
            );
        }
    }
    #[test]
    fn recording_choice_is_latched_at_start_without_retroactive_fill() {
        let mut policy = RecordingPolicy::new(false);
        let start = |id| ObservedCallEvent {
            observation_id: id,
            at_ms: 0,
            kind: ObservedCallKind::Started {
                account: None,
                direction: CallDirection::Incoming,
                number: None,
                remote_name: None,
            },
        };
        assert!(!policy.record(&start(1)));
        policy.enabled = true;
        assert!(!policy.record(&start(1)));
        assert!(!policy.record(&ObservedCallEvent {
            observation_id: 1,
            at_ms: 1,
            kind: ObservedCallKind::Connected
        }));
        assert!(policy.record(&start(2)));
        policy.enabled = false;
        assert!(policy.record(&ObservedCallEvent {
            observation_id: 2,
            at_ms: 2,
            kind: ObservedCallKind::Connected
        }));
        assert!(!policy.record(&start(3)));
    }
}
