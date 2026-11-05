//! Actionable incoming-call notifications; native actions must revalidate live SIP state.
use baresip_adapter::{CallNotificationAction, NotificationPermission};
use phone_core::{CallState, EngineSnapshot, ErrorCode, PhoneError};
use std::{
    sync::{
        Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::Manager;

const MAX_TITLE_CHARACTERS: usize = 128;
const REOFFER_DELAY: Duration = Duration::from_secs(2);
const MAX_DELIVERY_ATTEMPTS: u8 = 3;

#[derive(Default)]
struct Registry {
    active: Option<Offer>,
    seen_call: Option<String>,
}
struct Offer {
    token: String,
    call_id: String,
    claimed: Option<Instant>,
    caption: (String, String),
    delivery_attempts: u8,
    retry_at: Option<Instant>,
    delivery_failed: bool,
}
#[derive(Default)]
struct Change {
    dismiss: Option<String>,
    show: Option<Banner>,
}
struct Banner {
    token: String,
    title: String,
    body: String,
}
#[derive(Debug, PartialEq, Eq)]
enum Choice {
    Answer,
    Reject,
    Open,
}
struct BoundAction {
    choice: Choice,
    call_id: String,
    token: String,
}

#[derive(Default)]
pub(crate) struct NotificationState {
    registry: Mutex<Registry>,
    delivery: Mutex<()>,
    pending: Mutex<Option<(EngineSnapshot, bool)>>,
    ready: Condvar,
    available: AtomicBool,
    stopped: AtomicBool,
}

impl NotificationState {
    fn enqueue(&self, snapshot: &EngineSnapshot, enabled: bool) {
        if let Ok(mut pending) = self.pending.lock() {
            *pending = Some((snapshot.clone(), enabled));
            self.ready.notify_one();
        }
    }

    fn deliver(
        &self,
        snapshot: &EngineSnapshot,
        enabled: bool,
        show_details: bool,
        apply: impl FnOnce(Change),
    ) {
        let Ok(_delivery) = self.delivery.lock() else {
            return;
        };
        if self.stopped.load(Ordering::Acquire) {
            return;
        }
        let change = self
            .registry
            .lock()
            .map_err(|_| error())
            .and_then(|mut registry| {
                registry.update(snapshot, enabled, show_details, Instant::now(), token)
            });
        // Never hold the action registry or phone session while calling the OS.
        if let Ok(change) = change {
            apply(change);
        }
    }

    fn reset(&self, expected_token: Option<&str>, dismiss: impl FnOnce(&str)) {
        let Ok(_delivery) = self.delivery.lock() else {
            return;
        };
        let token = self.registry.lock().ok().and_then(|mut registry| {
            if expected_token.is_some_and(|token| {
                registry
                    .active
                    .as_ref()
                    .is_none_or(|offer| offer.token != token)
            }) {
                return None;
            }
            registry.reset()
        });
        if let Some(token) = token {
            dismiss(&token);
        }
    }
}

fn token() -> Result<String, PhoneError> {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).map_err(|_| error())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}
fn error() -> PhoneError {
    PhoneError::new(
        ErrorCode::NativeFailure,
        "Call notification could not be updated.",
    )
}

impl Registry {
    fn update(
        &mut self,
        snapshot: &EngineSnapshot,
        enabled: bool,
        show_details: bool,
        now: Instant,
        make_token: impl FnOnce() -> Result<String, PhoneError>,
    ) -> Result<Change, PhoneError> {
        let mut change = Change::default();
        let incoming =
            snapshot.call == CallState::Incoming && snapshot.call_id.is_some() && enabled;
        if !incoming {
            change.dismiss = self.active.take().map(|offer| offer.token);
            self.seen_call = None;
            return Ok(change);
        }
        let id = snapshot
            .call_id
            .as_deref()
            .expect("incoming requires call id");
        if let Some(offer) = &mut self.active
            && offer.call_id == id
        {
            if !offer
                .claimed
                .is_some_and(|at| now.saturating_duration_since(at) >= REOFFER_DELAY)
            {
                let caption = banner_text(snapshot, show_details);
                if offer.claimed.is_none() && offer.caption != caption {
                    offer.caption = caption.clone();
                    change.show = Some(Banner {
                        token: offer.token.clone(),
                        title: caption.0,
                        body: caption.1,
                    });
                } else if offer.claimed.is_none() && offer.retry_at.is_some_and(|at| now >= at) {
                    offer.retry_at = None;
                    change.show = Some(Banner {
                        token: offer.token.clone(),
                        title: offer.caption.0.clone(),
                        body: offer.caption.1.clone(),
                    });
                }
                return Ok(change);
            }
            // A failed asynchronous answer may restore Incoming before the
            // observer saw Answering. Offer recovery once per user attempt.
            self.seen_call = None;
        }
        if self.seen_call.as_deref() == Some(id) {
            return Ok(change);
        }
        let token = make_token()?;
        change.dismiss = self.active.take().map(|offer| offer.token);
        let (title, body) = banner_text(snapshot, show_details);
        self.seen_call = Some(id.to_owned());
        self.active = Some(Offer {
            token: token.clone(),
            call_id: id.to_owned(),
            claimed: None,
            caption: (title.clone(), body.clone()),
            delivery_attempts: 0,
            retry_at: None,
            delivery_failed: false,
        });
        change.show = Some(Banner { token, title, body });
        Ok(change)
    }

    fn delivery_finished(&mut self, token: &str, success: bool, now: Instant) {
        if let Some(offer) = self.active.as_mut().filter(|offer| offer.token == token) {
            offer.delivery_failed = !success;
            offer.delivery_attempts = offer.delivery_attempts.saturating_add(1);
            offer.retry_at = if success || offer.delivery_attempts >= MAX_DELIVERY_ATTEMPTS {
                None
            } else {
                Some(now + REOFFER_DELAY * u32::from(offer.delivery_attempts))
            };
        }
    }

    fn retry_authorized_delivery(&mut self, now: Instant) {
        if let Some(offer) = self.active.as_mut()
            && offer.delivery_failed
            && offer.claimed.is_none()
        {
            offer.delivery_attempts = 0;
            offer.retry_at = Some(now);
        }
    }

    fn claim(
        &mut self,
        event: CallNotificationAction,
        snapshot: &EngineSnapshot,
        now: Instant,
    ) -> Option<BoundAction> {
        let (choice, token) = match event {
            CallNotificationAction::Answer(token) => (Choice::Answer, token),
            CallNotificationAction::Reject(token) => (Choice::Reject, token),
            CallNotificationAction::Open(token) => (Choice::Open, token),
        };
        let offer = self.active.as_mut()?;
        if offer.token != token
            || snapshot.call_id.as_deref() != Some(&offer.call_id)
            || snapshot.call != CallState::Incoming
            || offer.claimed.is_some()
        {
            return None;
        }
        if choice != Choice::Open {
            offer.claimed = Some(now);
        }
        Some(BoundAction {
            choice,
            token,
            call_id: offer.call_id.clone(),
        })
    }

    fn reset(&mut self) -> Option<String> {
        self.seen_call = None;
        self.active.take().map(|offer| offer.token)
    }
}

fn plain(value: &str) -> String {
    value.chars().filter(|c| !c.is_control() && !matches!(c, '\u{200e}' | '\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2066}'..='\u{2069}')).collect::<String>().trim().to_owned()
}
fn banner_text(snapshot: &EngineSnapshot, show_details: bool) -> (String, String) {
    if !show_details {
        return (
            "Eingehender Anruf".into(),
            "Ruf öffnen, um den Anruf anzusehen.".into(),
        );
    }
    let caller = snapshot.caller.as_ref();
    let name = snapshot
        .caller_contact
        .as_ref()
        .map(|contact| contact.name.as_str())
        .or_else(|| caller.and_then(|c| c.name.as_deref()))
        .map(|name| {
            plain(name)
                .chars()
                .take(MAX_TITLE_CHARACTERS.saturating_sub(8))
                .collect::<String>()
        })
        .filter(|v| !v.is_empty());
    let number = caller
        .and_then(|c| c.number.as_deref())
        .map(plain)
        .filter(|v| !v.is_empty());
    let title = name
        .map(|name| format!("{name} ruft an"))
        .or_else(|| number.clone())
        .unwrap_or_else(|| "Unbekannter Anrufer".into());
    let account = snapshot
        .accounts
        .iter()
        .find(|a| Some(a.id) == snapshot.call_account_id)
        .map(|a| plain(&a.name))
        .unwrap_or_default();
    let lead = number
        .filter(|number| *number != title)
        .unwrap_or_else(|| "Eingehender Anruf".into());
    let body = if account.is_empty() {
        lead
    } else {
        format!("{lead} · {account}")
    };
    (title, body)
}

pub(crate) fn setup(app: &tauri::AppHandle) {
    app.manage(NotificationState::default());
    let (sender, receiver) = std::sync::mpsc::channel();
    let initialized = baresip_adapter::init_call_notifications(
        sender,
        &app.config().identifier,
        crate::desktop::autostart_available(app),
    );
    app.state::<NotificationState>()
        .available
        .store(initialized.is_ok(), Ordering::Release);
    if initialized.is_err() {
        eprintln!("Native call notifications unavailable in this launch context");
    }
    let handle = app.clone();
    std::thread::Builder::new()
        .name("notification-actions".into())
        .spawn(move || {
            while !handle
                .state::<NotificationState>()
                .stopped
                .load(Ordering::Acquire)
            {
                match receiver.recv_timeout(Duration::from_millis(100)) {
                    Ok(action) => handle_action(&handle, action),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        })
        .expect("Failed to start notification action receiver");
    let handle = app.clone();
    std::thread::Builder::new()
        .name("notification-delivery".into())
        .spawn(move || {
            let state = handle.state::<NotificationState>();
            loop {
                let Ok(pending) = state.pending.lock() else {
                    break;
                };
                let Ok(mut pending) = state.ready.wait_while(pending, |pending| {
                    pending.is_none() && !state.stopped.load(Ordering::Acquire)
                }) else {
                    break;
                };
                if state.stopped.load(Ordering::Acquire) {
                    break;
                }
                let update = pending.take();
                drop(pending);
                if let Some((snapshot, enabled)) = update {
                    deliver(&handle, &snapshot, enabled);
                }
            }
            refresh_authorization(&handle);
        })
        .expect("Failed to start notification delivery worker");
}

/// Coalesce the latest state without waiting for the operating-system service.
/// Notification delivery must never delay phone state publication or tray controls.
pub(crate) fn sync(app: &tauri::AppHandle, snapshot: &EngineSnapshot, enabled: bool) {
    let state = app.state::<NotificationState>();
    if !state.available.load(Ordering::Acquire) || state.stopped.load(Ordering::Acquire) {
        return;
    }
    state.enqueue(snapshot, enabled);
}

fn deliver(app: &tauri::AppHandle, snapshot: &EngineSnapshot, enabled: bool) {
    let state = app.state::<NotificationState>();
    // A contended or not-yet-loaded session is anonymous. This try-lock read never
    // waits for SIP, loads metadata or retains a phone lock across native calls.
    let show_details = app
        .state::<crate::phone::PhoneState>()
        .notification_details_allowed();
    state.deliver(snapshot, enabled, show_details, |change| {
        if let Some(token) = change.dismiss {
            let _ = baresip_adapter::dismiss_call_notification(&token);
        }
        if let Some(banner) = change.show {
            let result =
                baresip_adapter::show_call_notification(&banner.token, &banner.title, &banner.body);
            if let Ok(mut registry) = state.registry.lock() {
                registry.delivery_finished(&banner.token, result.is_ok(), Instant::now());
            }
            if result.is_err() {
                // No caller data or opaque action token in diagnostics. Retrying
                // is bounded and never asks for authorization during an incoming call.
                eprintln!(
                    "Native call notification delivery failed; check notification authorization"
                );
            }
        }
    });
}

fn handle_action(app: &tauri::AppHandle, event: CallNotificationAction) {
    if app
        .state::<NotificationState>()
        .stopped
        .load(Ordering::Acquire)
    {
        return;
    }
    let event_token = match &event {
        CallNotificationAction::Answer(token)
        | CallNotificationAction::Reject(token)
        | CallNotificationAction::Open(token) => token.clone(),
    };
    let result = crate::desktop::with_session(app, |session| {
        let snapshot = session.snapshot()?;
        let action = app
            .state::<NotificationState>()
            .registry
            .lock()
            .map_err(|_| error())?
            .claim(event, &snapshot, Instant::now());
        let Some(action) = action else {
            return Ok(None);
        };
        match action.choice {
            Choice::Answer => session.engine()?.answer_call(&action.call_id)?,
            Choice::Reject => session.engine()?.reject_call(&action.call_id)?,
            Choice::Open => {}
        }
        Ok(Some(action))
    });
    match result {
        Ok(Some(action)) if action.choice == Choice::Open => {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || {
                let _ = crate::shell::show_phone(&handle);
            });
        }
        Ok(Some(action)) => {
            let state = app.state::<NotificationState>();
            let Ok(_delivery) = state.delivery.lock() else {
                return;
            };
            let _ = baresip_adapter::dismiss_call_notification(&action.token);
        }
        Ok(None) => {}
        Err(failure) => {
            if failure.code != ErrorCode::InvalidState {
                let state = app.state::<NotificationState>();
                state.reset(Some(&event_token), |token| {
                    let _ = baresip_adapter::dismiss_call_notification(token);
                });
            }
        }
    }
}

/// Reconsider a still-ringing call after an explicit permission decision.
pub(crate) fn refresh_authorization(app: &tauri::AppHandle) {
    app.state::<NotificationState>().reset(None, |token| {
        let _ = baresip_adapter::dismiss_call_notification(token);
    });
}

/// A foreground settings read may observe an OS grant made outside Ruf.
/// Resume a failed live offer without duplicating an already delivered notice.
pub(crate) fn authorization_observed(
    app: &tauri::AppHandle,
    permission: phone_core::NotificationPermission,
) {
    if permission == phone_core::NotificationPermission::Authorized
        && let Ok(mut registry) = app.state::<NotificationState>().registry.lock()
    {
        registry.retry_authorized_delivery(Instant::now());
    }
}
pub(crate) fn shutdown(app: &tauri::AppHandle) {
    let state = app.state::<NotificationState>();
    // Pair the stop predicate with the waiter's mutex to avoid a lost wakeup.
    if let Ok(mut pending) = state.pending.lock() {
        state.stopped.store(true, Ordering::Release);
        pending.take();
        state.ready.notify_all();
    } else {
        state.stopped.store(true, Ordering::Release);
    }
    refresh_authorization(app);
}
pub(crate) fn permission(request: bool) -> Result<phone_core::NotificationPermission, PhoneError> {
    baresip_adapter::call_notification_permission(request).map(|permission| match permission {
        NotificationPermission::NotDetermined => phone_core::NotificationPermission::NotDetermined,
        NotificationPermission::Denied => phone_core::NotificationPermission::Denied,
        NotificationPermission::Authorized => phone_core::NotificationPermission::Authorized,
        NotificationPermission::Unsupported => phone_core::NotificationPermission::Unsupported,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use phone_core::CallerIdentity;
    fn incoming(id: &str) -> EngineSnapshot {
        EngineSnapshot {
            call: CallState::Incoming,
            call_id: Some(id.into()),
            caller: Some(CallerIdentity {
                name: Some("Ada Weber".into()),
                number: Some("+4930123456".into()),
            }),
            ..Default::default()
        }
    }
    fn next(
        registry: &mut Registry,
        snapshot: &EngineSnapshot,
        now: Instant,
        token: &str,
    ) -> Change {
        registry
            .update(snapshot, true, true, now, || Ok(token.into()))
            .unwrap()
    }
    #[test]
    fn privacy_hides_all_identity_details_without_changing_bound_actions() {
        let mut registry = Registry::default();
        let now = Instant::now();
        let mut snapshot = incoming("private-call");
        snapshot.call_account_id = Some(7);
        snapshot.accounts.push(phone_core::AccountSnapshot {
            transport: Default::default(),
            media_security: Default::default(),
            auth_username: None,
            outbound_proxy: None,
            id: 7,
            name: "Private account name".into(),
            registrar: "private.example.test".into(),
            username: "private-user".into(),
            enabled: true,
            enabled_changing: false,
            registered_at_ms: None,
            registration: phone_core::RegistrationState::Registered,
            last_error: None,
            removing: false,
        });
        snapshot.caller_contact = Some(phone_core::ContactReference {
            id: "private-contact".into(),
            source: phone_core::ContactSource::Macos,
            name: "Private address book name".into(),
        });
        let first = registry
            .update(&snapshot, true, true, now, || Ok("bound".into()))
            .unwrap()
            .show
            .unwrap();
        assert_eq!(first.title, "Private address book name ruft an");
        assert_eq!(first.body, "+4930123456 · Private account name");
        let private = registry
            .update(&snapshot, true, false, now, || {
                panic!("privacy keeps the call token")
            })
            .unwrap()
            .show
            .unwrap();
        assert_eq!(private.token, "bound");
        assert_eq!(
            (private.title, private.body),
            (
                "Eingehender Anruf".into(),
                "Ruf öffnen, um den Anruf anzusehen.".into()
            )
        );
        assert!(
            registry
                .update(&snapshot, true, false, now, || panic!())
                .unwrap()
                .show
                .is_none()
        );
        let action = registry
            .claim(
                CallNotificationAction::Answer("bound".into()),
                &snapshot,
                now,
            )
            .unwrap();
        assert_eq!(action.call_id, "private-call");
        assert_eq!(action.choice, Choice::Answer);
    }

    #[test]
    fn contact_caption_changes_replace_the_bound_notice_without_rotating_actions() {
        let mut registry = Registry::default();
        let now = Instant::now();
        let mut snapshot = incoming("contact-call");
        snapshot.caller_contact = Some(phone_core::ContactReference {
            id: "synthetic".into(),
            source: phone_core::ContactSource::Macos,
            name: "Address book name".into(),
        });
        assert_eq!(
            next(&mut registry, &snapshot, now, "bound")
                .show
                .unwrap()
                .title,
            "Address book name ruft an"
        );
        snapshot.caller_contact = None;
        let replaced = next(&mut registry, &snapshot, now, "unused").show.unwrap();
        assert_eq!(replaced.token, "bound");
        assert_eq!(replaced.title, "Ada Weber ruft an");
        assert!(next(&mut registry, &snapshot, now, "unused").show.is_none());
        assert!(
            registry
                .claim(
                    CallNotificationAction::Answer("bound".into()),
                    &snapshot,
                    now
                )
                .is_some()
        );
    }

    #[test]
    fn one_banner_per_offered_call_and_dismiss_on_answer_or_cancel() {
        let mut registry = Registry::default();
        let now = Instant::now();
        let mut snapshot = incoming("1");
        let first = next(&mut registry, &snapshot, now, "a");
        assert!(first.show.is_some());
        assert!(next(&mut registry, &snapshot, now, "b").show.is_none());
        snapshot.call = CallState::Answering;
        assert_eq!(
            next(&mut registry, &snapshot, now, "b").dismiss.as_deref(),
            Some("a")
        );
        snapshot.call = CallState::Incoming;
        assert!(next(&mut registry, &snapshot, now, "b").show.is_some());
        assert_eq!(
            next(&mut registry, &EngineSnapshot::default(), now, "c")
                .dismiss
                .as_deref(),
            Some("b")
        );
    }
    #[test]
    fn failed_delivery_retries_with_backoff_and_stops_after_three_attempts() {
        let mut registry = Registry::default();
        let now = Instant::now();
        let snapshot = incoming("retry-call");
        let first = next(&mut registry, &snapshot, now, "bound").show.unwrap();
        registry.delivery_finished(&first.token, false, now);
        assert!(next(&mut registry, &snapshot, now, "unused").show.is_none());
        let second = next(
            &mut registry,
            &snapshot,
            now + Duration::from_secs(2),
            "unused",
        )
        .show
        .expect("transient delivery failure must not lose the call banner");
        assert_eq!(second.token, "bound");
        registry.delivery_finished(&second.token, false, now + Duration::from_secs(2));
        assert!(
            next(
                &mut registry,
                &snapshot,
                now + Duration::from_secs(3),
                "unused"
            )
            .show
            .is_none()
        );
        let third = next(
            &mut registry,
            &snapshot,
            now + Duration::from_secs(6),
            "unused",
        )
        .show
        .unwrap();
        registry.delivery_finished(&third.token, false, now + Duration::from_secs(6));
        assert!(
            next(
                &mut registry,
                &snapshot,
                now + Duration::from_secs(60),
                "unused"
            )
            .show
            .is_none()
        );
        assert!(
            next(&mut registry, &incoming("next-call"), now, "new")
                .show
                .is_some()
        );
    }

    #[test]
    fn successful_delivery_or_withdrawal_cancels_pending_retry() {
        let mut registry = Registry::default();
        let now = Instant::now();
        let snapshot = incoming("retry-call");
        next(&mut registry, &snapshot, now, "bound");
        registry.delivery_finished("bound", false, now);
        registry.delivery_finished("obsolete", true, now);
        let retry = next(
            &mut registry,
            &snapshot,
            now + Duration::from_secs(2),
            "unused",
        )
        .show
        .unwrap();
        registry.delivery_finished(&retry.token, true, now);
        assert!(
            next(
                &mut registry,
                &snapshot,
                now + Duration::from_secs(60),
                "unused"
            )
            .show
            .is_none()
        );
        registry.delivery_finished("bound", false, now);
        next(&mut registry, &EngineSnapshot::default(), now, "unused");
        registry.delivery_finished("bound", false, now);
        assert!(
            next(
                &mut registry,
                &EngineSnapshot::default(),
                now + Duration::from_secs(60),
                "unused"
            )
            .show
            .is_none()
        );
    }

    #[test]
    fn observing_authorized_permission_recovers_a_failed_live_offer_only() {
        let mut registry = Registry::default();
        let now = Instant::now();
        let snapshot = incoming("permission-changed");
        next(&mut registry, &snapshot, now, "bound");
        for attempt in 0..MAX_DELIVERY_ATTEMPTS {
            registry.delivery_finished("bound", false, now);
            let _ = next(
                &mut registry,
                &snapshot,
                now + REOFFER_DELAY * u32::from(attempt + 1),
                "unused",
            );
        }
        assert!(
            next(
                &mut registry,
                &snapshot,
                now + Duration::from_secs(60),
                "unused"
            )
            .show
            .is_none()
        );
        registry.retry_authorized_delivery(now);
        let retry = next(&mut registry, &snapshot, now, "unused")
            .show
            .expect("granting access in system settings must recover the ringing offer");
        assert_eq!(retry.token, "bound");
        registry.delivery_finished("bound", true, now);
        registry.retry_authorized_delivery(now);
        assert!(next(&mut registry, &snapshot, now, "unused").show.is_none());
        registry.delivery_finished("bound", false, now);
        assert!(
            registry
                .claim(
                    CallNotificationAction::Answer("bound".into()),
                    &snapshot,
                    now
                )
                .is_some()
        );
        registry.retry_authorized_delivery(now);
        assert!(next(&mut registry, &snapshot, now, "unused").show.is_none());
        next(&mut registry, &EngineSnapshot::default(), now, "unused");
        registry.retry_authorized_delivery(now);
        assert!(
            next(&mut registry, &EngineSnapshot::default(), now, "unused")
                .show
                .is_none()
        );
    }
    #[test]
    fn stale_duplicate_and_already_answered_actions_do_nothing() {
        let mut registry = Registry::default();
        let now = Instant::now();
        let a = incoming("1");
        next(&mut registry, &a, now, "old");
        let b = incoming("2");
        next(&mut registry, &b, now, "new");
        assert!(
            registry
                .claim(CallNotificationAction::Answer("old".into()), &b, now)
                .is_none()
        );
        let mut active = b.clone();
        active.call = CallState::Active;
        assert!(
            registry
                .claim(CallNotificationAction::Reject("new".into()), &active, now)
                .is_none()
        );
        let action = registry
            .claim(CallNotificationAction::Reject("new".into()), &b, now)
            .unwrap();
        assert_eq!(action.choice, Choice::Reject);
        assert_eq!(action.call_id, "2");
        assert!(
            registry
                .claim(CallNotificationAction::Answer("new".into()), &b, now)
                .is_none()
        );
    }
    #[test]
    fn tapping_only_opens_and_failed_native_answer_can_be_offered_again() {
        let mut registry = Registry::default();
        let now = Instant::now();
        let s = incoming("1");
        next(&mut registry, &s, now, "a");
        assert_eq!(
            registry
                .claim(CallNotificationAction::Open("a".into()), &s, now)
                .unwrap()
                .choice,
            Choice::Open
        );
        assert!(
            registry
                .claim(CallNotificationAction::Answer("a".into()), &s, now)
                .is_some()
        );
        assert!(
            next(&mut registry, &s, now + Duration::from_secs(1), "b")
                .show
                .is_none()
        );
        assert!(
            next(&mut registry, &s, now + REOFFER_DELAY, "b")
                .show
                .is_some()
        );
    }
    #[test]
    fn disabled_notices_never_replace_native_ringing() {
        let mut registry = Registry::default();
        let now = Instant::now();
        let s = incoming("1");
        assert!(
            registry
                .update(&s, false, true, now, || panic!("no token while disabled"))
                .unwrap()
                .show
                .is_none()
        );
        assert!(next(&mut registry, &s, now, "a").show.is_some());
        assert_eq!(
            registry
                .update(&s, false, true, now, || panic!())
                .unwrap()
                .dismiss
                .as_deref(),
            Some("a")
        );
    }

    #[test]
    fn failed_token_generation_does_not_orphan_an_existing_notice() {
        let mut registry = Registry::default();
        let now = Instant::now();
        next(&mut registry, &incoming("1"), now, "old");
        assert!(
            registry
                .update(&incoming("2"), true, true, now, || Err(error()))
                .is_err()
        );
        assert_eq!(
            next(&mut registry, &EngineSnapshot::default(), now, "unused")
                .dismiss
                .as_deref(),
            Some("old")
        );
    }
    #[test]
    fn caller_caption_is_plain_and_uses_number_or_unknown_fallback() {
        let mut s = incoming("1");
        assert_eq!(
            banner_text(&s, true),
            ("Ada Weber ruft an".into(), "+4930123456".into())
        );
        s.caller.as_mut().unwrap().name = None;
        assert_eq!(banner_text(&s, true).0, "+4930123456");
        s.caller = None;
        assert_eq!(banner_text(&s, true).0, "Unbekannter Anrufer");
        assert_eq!(plain(" Ada\n\u{202e} Weber "), "Ada Weber");
    }
    #[test]
    fn token_is_exactly_sixteen_hex_characters() {
        let a = token().unwrap();
        let b = token().unwrap();
        assert_eq!(a.len(), 16);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[test]
    fn latest_pending_state_replaces_an_obsolete_ringing_offer() {
        let state = NotificationState::default();
        // Holding action state does not stall the phone observer's handoff.
        let _registry = state.registry.lock().unwrap();
        state.enqueue(&incoming("old"), true);
        state.enqueue(&EngineSnapshot::default(), false);
        let (snapshot, enabled) = state.pending.lock().unwrap().take().unwrap();
        assert_eq!(snapshot.call, CallState::Idle);
        assert!(!enabled);
        assert!(state.pending.lock().unwrap().is_none());
    }

    #[test]
    fn withdrawing_an_inflight_offer_cannot_be_overtaken_by_show() {
        use std::sync::{Arc, mpsc};
        let state = Arc::new(NotificationState::default());
        let visible = Arc::new(Mutex::new(None));
        let (started, start) = mpsc::channel();
        let (resume, wait) = mpsc::channel();
        let (resetting, attempt) = mpsc::channel();
        let (done, completed) = mpsc::channel();
        let showing = {
            let state = Arc::clone(&state);
            let visible = Arc::clone(&visible);
            std::thread::spawn(move || {
                state.deliver(&incoming("1"), true, true, |change| {
                    started.send(()).unwrap();
                    wait.recv().unwrap();
                    *visible.lock().unwrap() = change.show.map(|banner| banner.token);
                });
            })
        };
        start.recv().unwrap();
        let withdrawing = {
            let state = Arc::clone(&state);
            let visible = Arc::clone(&visible);
            std::thread::spawn(move || {
                resetting.send(()).unwrap();
                state.reset(None, |token| {
                    let mut visible = visible.lock().unwrap();
                    if visible.as_deref() == Some(token) {
                        *visible = None;
                    }
                });
                done.send(()).unwrap();
            })
        };
        attempt.recv().unwrap();
        // Reset must wait for Show's OS acknowledgement, not forget its token first.
        let premature_reset = completed.recv_timeout(Duration::from_millis(50)).is_ok();
        resume.send(()).unwrap();
        showing.join().unwrap();
        withdrawing.join().unwrap();
        assert!(!premature_reset);
        assert!(visible.lock().unwrap().is_none());
        assert!(state.registry.lock().unwrap().active.is_none());
    }
}
