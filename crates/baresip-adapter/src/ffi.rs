//! Private unsafe boundary: callback ownership outlives the joined C event loop.

use crate::engine::EngineState;
use phone_core::{
    AccountConfig, AudioDevice, AudioDevices, AudioMode, AudioTestKind, AudioTestSnapshot,
    CallDirection, CallEvent, CallEventKind, CallState, CallerIdentity, EngineSnapshot, ErrorCode,
    PhoneError, RegistrationState, SoundSettings,
};
use std::{
    ffi::{CStr, CString, c_void},
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    ptr::NonNull,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
};

static ENGINE_LEASE: AtomicBool = AtomicBool::new(false);

const EV_PROGRESS: i32 = 27;
const EV_HOLD: i32 = 28;
const EV_RESUME: i32 = 29;
const EV_DTMF_START: i32 = 30;
const EV_DTMF_END: i32 = 31;
const EV_TRANSFER_FAILED: i32 = 32;
const EV_REDIRECT: i32 = 33;
const EV_ANSWER_CLOSED: i32 = 34;
const EV_SELECTED: i32 = 39;
const EV_MUTED: i32 = 40;
const EV_LOCAL_HOLD_PENDING: i32 = 41;
const EV_LOCAL_HOLD_DONE: i32 = 42;
const EV_CONTROL_FAILED: i32 = 43;
const EV_TRANSFER_PENDING: i32 = 44;
const EV_TRANSFER_RESULT: i32 = 45;
const EV_CONTROL_REJECTED: i32 = 46;
const EV_RECOVERY_FAILED: i32 = 47;
const EV_RECOVERING: i32 = 48;
const EV_CONSULT_FAILED: i32 = 49;
const CALL_EVENT_CAPACITY: usize = 64;

/// Private ABI mirrored by bridge.c; every pointer is borrowed for the callback only.
#[repr(C)]
#[derive(Clone, Copy)]
struct NativeEvent {
    kind: i32,
    code: i32,
    account_id: u32,
    call_generation: u64,
    caller_name: *const std::ffi::c_char,
    caller_number: *const std::ffi::c_char,
    detail: *const std::ffi::c_char,
}

unsafe extern "C" {
    fn sip_engine_create(
        callback: extern "C" fn(*const NativeEvent, *mut c_void),
        arg: *mut c_void,
        audio_mode: i32,
        recording: *const std::ffi::c_char,
    ) -> *mut c_void;
    fn sip_engine_run(handle: *mut c_void) -> i32;
    fn sip_engine_reserve_call_generation(handle: *mut c_void) -> u64;
    fn sip_engine_request(
        handle: *mut c_void,
        operation: i32,
        account_id: u32,
        call_generation: u64,
        registrar: *const std::ffi::c_char,
        username: *const std::ffi::c_char,
        password: *const std::ffi::c_char,
        destination: *const std::ffi::c_char,
    ) -> i32;
    fn sip_audio_devices(
        callback: extern "C" fn(
            i32,
            *const std::ffi::c_char,
            *const std::ffi::c_char,
            i32,
            *mut c_void,
        ),
        arg: *mut c_void,
    ) -> i32;
    fn sip_engine_destroy(handle: *mut c_void);
}

#[derive(Clone, Copy)]
struct Handle(NonNull<c_void>);
// The C request queue synchronizes cross-thread access. Only the event-loop thread
// calls run; destroy is called only after join. No direct UA/call access is exposed.
unsafe impl Send for Handle {}
impl Handle {
    fn run(self) -> i32 {
        unsafe { sip_engine_run(self.0.as_ptr()) }
    }
}

struct CallbackContext {
    snapshot: Arc<Mutex<EngineState>>,
    ready: mpsc::Sender<Result<(), PhoneError>>,
}

extern "C" fn callback(event: *const NativeEvent, arg: *mut c_void) {
    // arg points to the owned box held until the engine thread has joined.
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if event.is_null() || arg.is_null() {
            return;
        }
        let NativeEvent {
            kind,
            code,
            account_id,
            call_generation,
            caller_name,
            caller_number,
            detail,
        } = unsafe { *event };
        let context = unsafe { &*arg.cast::<CallbackContext>() };
        if kind == 1 {
            let _ = context.ready.send(if code == 0 {
                Ok(())
            } else {
                Err(native_error(code))
            });
            return;
        }
        if let Ok(mut engine) = context.snapshot.lock() {
            if (35..=38).contains(&kind) {
                let id = call_generation.to_string();
                match kind {
                    35 => {
                        engine.snapshot.audio_test = Some(AudioTestSnapshot {
                            id,
                            kind: if account_id == 0 {
                                AudioTestKind::Microphone
                            } else {
                                AudioTestKind::Speaker
                            },
                            level: 0.0,
                        })
                    }
                    36 => {
                        if let Some(test) = engine
                            .snapshot
                            .audio_test
                            .as_mut()
                            .filter(|test| test.id == id)
                        {
                            test.level = (code.clamp(0, 1000) as f32) / 1000.0;
                        }
                    }
                    37 => {
                        if engine
                            .snapshot
                            .audio_test
                            .as_ref()
                            .is_some_and(|test| test.id == id)
                        {
                            engine.snapshot.audio_test = None;
                        }
                    }
                    38 => engine.snapshot.audio_test_error = Some(native_error(code)),
                    _ => {}
                }
                return;
            }
            // Journal events run synchronously before any UI snapshot projection.
            match kind {
                7 | 22 | 23 => {
                    // Public metadata is optimistic while an update is queued.
                    // Offers still belong to the existing native UA until its
                    // replacement is acknowledged; it may reject that update.
                    let account = match engine.pending_accounts.get(&account_id) {
                        Some(previous) => previous.clone(),
                        None => engine
                            .snapshot
                            .accounts
                            .iter()
                            .find(|a| a.id == account_id)
                            .cloned(),
                    };
                    let number = owned_caller_text(caller_number, 80)
                        .filter(|value| caller_number_is_display_safe(value));
                    let name = (kind != 23)
                        .then(|| owned_caller_text(caller_name, 128))
                        .flatten();
                    if kind == 23
                        && let Some(number) = &number
                    {
                        engine
                            .call_destinations
                            .insert(call_generation, number.clone());
                    }
                    engine.call_journal.start(
                        call_generation,
                        account,
                        if kind == 23 {
                            CallDirection::Outgoing
                        } else {
                            CallDirection::Incoming
                        },
                        number,
                        name,
                    );
                    if kind == 22 {
                        engine.call_journal.busy(call_generation);
                    }
                }
                8 => engine.call_journal.connected(call_generation),
                9 => engine.call_journal.finish(call_generation, code, false),
                24 | EV_ANSWER_CLOSED => engine.call_journal.finish(call_generation, code, true),
                25 => engine.call_journal.local_end(call_generation),
                11 | 26 => engine.call_journal.interrupt(),
                _ => {}
            }
            if kind == 24 {
                engine.call_destinations.remove(&call_generation);
            }
            if kind == 25 {
                engine.local_endings.insert(call_generation);
            }
            if kind == 26 {
                let generations: Vec<u64> = engine
                    .snapshot
                    .calls
                    .iter()
                    .filter_map(|call| call.id.parse().ok())
                    .collect();
                engine.local_endings.extend(generations);
            }
            if kind == 26 || (kind == 25 && engine.call_generation == call_generation) {
                engine.local_ending = true;
            }
            if kind == 22 || kind == 25 || kind == 26 {
                return;
            }
            if (matches!(kind, 6 | 8 | 9 | 21 | EV_ANSWER_CLOSED)
                || (EV_PROGRESS..=EV_REDIRECT).contains(&kind))
                && engine.call_generation != call_generation
                && !engine
                    .snapshot
                    .calls
                    .iter()
                    .any(|call| call.id == call_generation.to_string())
            {
                return;
            }
            // Busy offers, unrelated registration callbacks and failed dial attempts
            // must not replace the opaque token of a live call.
            if matches!(kind, 5..=8)
                || (kind == 23
                    && matches!(engine.snapshot.call, CallState::Idle | CallState::Calling))
            {
                engine.call_generation = call_generation;
            }
            if kind == 23 {
                return;
            }
            if kind == 24
                && engine.call_generation != call_generation
                && !engine
                    .snapshot
                    .calls
                    .iter()
                    .any(|call| call.id == call_generation.to_string())
            {
                return;
            }
            if kind == 17 || kind == 18 {
                engine.sound_result = Some(if kind == 17 {
                    Ok(())
                } else {
                    Err(native_error(code))
                });
                return;
            }
            if kind == 15 || kind == 16 {
                let previous = engine.pending_accounts.remove(&account_id);
                let result = if kind == 15 {
                    if let Some(account) = engine
                        .snapshot
                        .accounts
                        .iter_mut()
                        .find(|account| account.id == account_id)
                    {
                        account.enabled_changing = false;
                    }
                    engine.snapshot.sync_selected_registration();
                    Ok(())
                } else {
                    if let Some(previous) = previous {
                        engine.snapshot.accounts.retain(|a| a.id != account_id);
                        if let Some(previous) = previous {
                            engine.snapshot.accounts.push(previous);
                            engine.snapshot.accounts.sort_by_key(|a| a.id);
                        }
                        if engine.snapshot.selected_account_id == Some(account_id)
                            && !engine.snapshot.accounts.iter().any(|a| a.id == account_id)
                        {
                            engine.snapshot.selected_account_id =
                                engine.snapshot.accounts.first().map(|a| a.id);
                        }
                        engine.snapshot.sync_selected_registration();
                    }
                    Err(PhoneError::new(
                        ErrorCode::Busy,
                        "Native account configuration was rejected; the previous account is unchanged.",
                    ))
                };
                engine.account_results.insert(account_id, result);
                return;
            }
            if kind == 11 {
                engine.stopped = true;
            }
            let dialog_event = matches!(kind, 5..=9 | 21 | 24 | 27..=34);
            let previous_selection = engine.snapshot.call_id.clone();
            if dialog_event {
                engine.select_dialog(&call_generation.to_string());
                engine.call_generation = call_generation;
            }
            if (EV_SELECTED..=EV_CONSULT_FAILED).contains(&kind) {
                let id = call_generation.to_string();
                if kind == EV_SELECTED {
                    engine.select_dialog(&id);
                    return;
                }
                if let Some(call) = engine.snapshot.calls.iter_mut().find(|call| call.id == id) {
                    match kind {
                        EV_MUTED => call.muted = code != 0,
                        EV_LOCAL_HOLD_PENDING => call.hold_pending = true,
                        EV_LOCAL_HOLD_DONE => {
                            call.hold_pending = false;
                            call.local_held = code != 0;
                        }
                        EV_CONTROL_FAILED => {
                            call.hold_pending = false;
                            call.last_error = Some(
                                u16::try_from(code)
                                    .ok()
                                    .and_then(PhoneError::from_sip_status)
                                    .unwrap_or_else(|| {
                                        PhoneError::new(
                                            ErrorCode::NativeFailure,
                                            "Call control failed or timed out.",
                                        )
                                    }),
                            );
                        }
                        EV_CONSULT_FAILED => {
                            let mut error = u16::try_from(code)
                                .ok()
                                .and_then(PhoneError::from_sip_status)
                                .unwrap_or_else(|| {
                                    PhoneError::new(
                                        ErrorCode::NativeFailure,
                                        "The consultation could not be started.",
                                    )
                                });
                            error.message = format!(
                                "Consultation failed: {} The original call is retained.",
                                error.message
                            );
                            call.last_error = Some(error);
                        }
                        EV_RECOVERING => {
                            call.hold_pending = true;
                            call.last_error = Some(PhoneError::new(
                                ErrorCode::NativeFailure,
                                "The hold request timed out; SIP session recovery was required.",
                            ));
                        }
                        EV_RECOVERY_FAILED => {
                            call.hold_pending = false;
                            let mut error = u16::try_from(code)
                                .ok()
                                .and_then(PhoneError::from_sip_status)
                                .unwrap_or_else(|| PhoneError::new(ErrorCode::NativeFailure, ""));
                            error.message =
                                "The SIP hold recovery failed; the uncertain call was ended."
                                    .into();
                            call.last_error = Some(error);
                        }
                        EV_CONTROL_REJECTED => {
                            call.last_error = Some(PhoneError::new(
                                ErrorCode::Busy,
                                "A call control is still pending.",
                            ))
                        }
                        EV_TRANSFER_PENDING => call.transfer = phone_core::TransferState::Pending,
                        EV_TRANSFER_RESULT => {
                            call.transfer = if (200..300).contains(&code) {
                                phone_core::TransferState::Succeeded
                            } else {
                                phone_core::TransferState::Failed
                            };
                        }
                        _ => {}
                    }
                    let event = match kind {
                        EV_MUTED => Some(CallEventKind::MuteChanged { muted: code != 0 }),
                        EV_LOCAL_HOLD_DONE => {
                            Some(CallEventKind::LocalHoldChanged { held: code != 0 })
                        }
                        EV_CONTROL_FAILED | EV_RECOVERY_FAILED | EV_CONSULT_FAILED => {
                            Some(CallEventKind::ControlFailed)
                        }
                        EV_TRANSFER_PENDING => Some(CallEventKind::TransferStarted),
                        EV_TRANSFER_RESULT if (200..300).contains(&code) => {
                            Some(CallEventKind::TransferSucceeded)
                        }
                        _ => None,
                    };
                    if let Some(event) = event {
                        push_call_event(
                            &mut call.events,
                            &mut call.events_dropped,
                            call.id.clone(),
                            event,
                        );
                    }
                }
                if engine.snapshot.call_id.as_deref() == Some(&id) {
                    engine.select_dialog(&id);
                }
                return;
            }
            if kind == 5 || kind == 7 {
                engine.remote_hold = false;
                engine.local_ending = false;
                engine.snapshot.call_events.clear();
                engine.snapshot.call_events_dropped = 0;
                engine.snapshot.call_connected_at_ms = None;
                engine.snapshot.caller_contact = None;
                engine.snapshot.last_error = None;
            }
            if kind == EV_HOLD {
                engine.remote_hold = true;
            }
            if kind == EV_RESUME {
                engine.remote_hold = false;
            }
            let remote_hold = engine.remote_hold;
            let local_ending = engine.local_ending;
            let state = &mut engine.snapshot;
            match kind {
                2..=4 => {
                    if let Some(account) = state
                        .accounts
                        .iter_mut()
                        .find(|a| a.id == account_id && a.enabled)
                    {
                        if kind == 3 {
                            account.registered_at_ms = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .ok()
                                .and_then(|duration| u64::try_from(duration.as_millis()).ok());
                        }
                        account.registration = match kind {
                            2 => RegistrationState::Registering,
                            3 => RegistrationState::Registered,
                            _ => RegistrationState::Failed,
                        };
                        account.last_error = (kind == 4).then(|| {
                            if let Ok(status) = u16::try_from(code)
                                && let Some(error) = PhoneError::from_sip_status(status)
                            {
                                return error;
                            }
                            PhoneError::new(
                                ErrorCode::NativeFailure,
                                "Registration failed; verify the registrar and credentials.",
                            )
                        });
                    }
                    state.sync_selected_registration();
                }
                5..=8 => {
                    state.call = match kind {
                        5 => CallState::Calling,
                        6 => CallState::Ringing,
                        7 => CallState::Incoming,
                        _ if remote_hold => CallState::Held,
                        _ => CallState::Active,
                    };
                    if kind == 8 && state.call_connected_at_ms.is_none() {
                        state.call_connected_at_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .and_then(|duration| u64::try_from(duration.as_millis()).ok());
                    }
                    if kind == 7 {
                        state.caller = Some(CallerIdentity {
                            name: owned_caller_text(caller_name, 128),
                            number: owned_caller_text(caller_number, 80)
                                .filter(|value| caller_number_is_display_safe(value)),
                        });
                    } else if kind == 5 {
                        state.caller = None;
                    }
                    state.call_account_id = Some(account_id);
                    state.call_id = Some(call_generation.to_string());
                }
                9 | 24 | EV_ANSWER_CLOSED => {
                    let outgoing_setup = matches!(
                        state.call,
                        CallState::Calling | CallState::Ringing | CallState::EarlyMedia
                    );
                    state.call = CallState::Idle;
                    state.call_account_id = None;
                    state.call_id = None;
                    state.caller = None;
                    state.caller_contact = None;
                    state.call_connected_at_ms = None;
                    if kind == EV_ANSWER_CLOSED && !local_ending {
                        state.last_error.get_or_insert_with(|| {
                            PhoneError::new(
                                ErrorCode::NativeFailure,
                                "The incoming call ended before establishment after its answer was sent.",
                            )
                        });
                    } else if kind == 24 && code != 0 {
                        state.last_error = Some(native_error(code));
                    } else if !local_ending
                        && let Ok(status) = u16::try_from(code)
                        && let Some(error) = PhoneError::from_sip_status(status)
                    {
                        state.last_error = Some(error);
                    } else if !local_ending && code >= 700 {
                        // baresip uses internal non-SIP sentinels for native/media failures.
                        state.last_error = Some(native_error(code));
                    } else if !local_ending
                        && code == 0
                        && outgoing_setup
                        && state.last_error.is_none()
                    {
                        // Native decoder/transport failures may close setup without
                        // a SIP status. Preserve that failure instead of silently idling.
                        state.last_error = Some(PhoneError::new(
                            ErrorCode::NativeFailure,
                            "The outgoing call ended before establishment without a final SIP response.",
                        ));
                    }
                }
                10 => state.last_error = Some(native_error(code)),
                21 => {
                    if state.call == CallState::Answering {
                        state.call = CallState::Incoming;
                    }
                    state.last_error = Some(native_error(code));
                }
                11 => {
                    state.audio_test = None;
                    state.ringtone_playing = false;
                    state.registration = RegistrationState::Unconfigured;
                    state.call = CallState::Idle;
                    state.call_account_id = None;
                    state.call_id = None;
                    state.caller = None;
                    state.caller_contact = None;
                    state.call_connected_at_ms = None;
                    for account in &mut state.accounts {
                        account.registration = RegistrationState::Unconfigured;
                    }
                    if code != 0 {
                        state.last_error = Some(native_error(code));
                    }
                }
                12 => {
                    state.last_error = Some(PhoneError::new(
                        ErrorCode::InvalidState,
                        "The call changed before this action executed.",
                    ))
                }
                13 => {
                    state.accounts.retain(|a| a.id != account_id);
                    if state.selected_account_id == Some(account_id) {
                        state.selected_account_id = state.accounts.first().map(|a| a.id);
                    }
                    state.sync_selected_registration();
                }
                19 => state.ringtone_playing = true,
                20 => state.ringtone_playing = false,
                14 => {
                    if let Some(account) = state.accounts.iter_mut().find(|a| a.id == account_id) {
                        account.removing = false;
                    }
                    state.last_error = Some(PhoneError::new(
                        ErrorCode::Busy,
                        "An incoming call prevented account removal.",
                    ));
                    state.sync_selected_registration();
                }
                EV_PROGRESS => {
                    if matches!(
                        state.call,
                        CallState::Calling | CallState::Ringing | CallState::EarlyMedia
                    ) {
                        state.call = CallState::EarlyMedia;
                    }
                    record_call_event(state, CallEventKind::Progress);
                }
                EV_HOLD | EV_RESUME => {
                    if matches!(state.call, CallState::Active | CallState::Held) {
                        state.call = if remote_hold {
                            CallState::Held
                        } else {
                            CallState::Active
                        };
                    }
                    record_call_event(
                        state,
                        if remote_hold {
                            CallEventKind::Held
                        } else {
                            CallEventKind::Resumed
                        },
                    );
                }
                EV_DTMF_START => {
                    if !detail.is_null() {
                        // C supplies a NUL-terminated two-byte digit buffer for this event only.
                        let digit = unsafe { CStr::from_ptr(detail) }.to_bytes();
                        if digit.len() == 1 && b"0123456789*#ABCD".contains(&digit[0]) {
                            record_call_event(
                                state,
                                CallEventKind::DtmfStart {
                                    digit: char::from(digit[0]).to_string(),
                                },
                            );
                        }
                    }
                }
                EV_DTMF_END => record_call_event(state, CallEventKind::DtmfEnd),
                EV_TRANSFER_FAILED => {
                    record_call_event(state, CallEventKind::TransferFailed);
                    state.last_error = Some(
                        u16::try_from(code)
                            .ok()
                            .and_then(PhoneError::from_sip_status)
                            .unwrap_or_else(|| {
                                PhoneError::new(
                                    ErrorCode::NativeFailure,
                                    "Call transfer failed without a SIP response.",
                                )
                            }),
                    );
                }
                EV_REDIRECT => {
                    if let Ok(status @ 300..=399) = u16::try_from(code) {
                        record_call_event(state, CallEventKind::Redirected { sip_status: status });
                        state.last_error = PhoneError::from_sip_status(status);
                    }
                }
                _ => {}
            }
            if dialog_event {
                engine.finish_dialog_event(call_generation, previous_selection, kind);
            }
            if kind == 11 {
                engine.snapshot.calls.clear();
            }
        }
    }));
}

fn record_call_event(state: &mut EngineSnapshot, event: CallEventKind) {
    let Some(call_id) = state.call_id.clone() else {
        return;
    };
    push_call_event(
        &mut state.call_events,
        &mut state.call_events_dropped,
        call_id,
        event,
    );
}

fn push_call_event(
    events: &mut Vec<CallEvent>,
    dropped: &mut u32,
    call_id: String,
    event: CallEventKind,
) {
    let sequence = events
        .last()
        .map_or(1, |last| last.sequence.wrapping_add(1));
    if events.len() == CALL_EVENT_CAPACITY {
        events.remove(0);
        *dropped = dropped.saturating_add(1);
    }
    events.push(CallEvent {
        sequence,
        call_id,
        event,
    });
}

// C supplies NUL-terminated arrays bounded to limit + 1 and borrows them only
// during this synchronous callback. Copy before returning; no pointer escapes.
fn owned_caller_text(value: *const std::ffi::c_char, limit: usize) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let bytes = unsafe { std::slice::from_raw_parts(value.cast::<u8>(), limit + 1) };
    let end = bytes.iter().position(|byte| *byte == 0)?;
    sanitize_caller_text(std::str::from_utf8(&bytes[..end]).ok()?, limit)
}

fn caller_number_is_display_safe(value: &str) -> bool {
    value
        .chars()
        .all(|c| c.is_alphanumeric() || "_.+-*#".contains(c))
}

fn sanitize_caller_text(value: &str, limit: usize) -> Option<String> {
    if value.chars().any(|c| c.is_control() || matches!(c,
        '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{2028}' | '\u{2029}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
    )) {
        return None;
    }
    let value = value.trim();
    let mut end = value.len().min(limit);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    (!value[..end].is_empty()).then(|| value[..end].to_owned())
}

pub(crate) struct NativeEngine {
    handle: Option<Handle>,
    thread: Option<JoinHandle<i32>>,
    context: Option<NonNull<CallbackContext>>,
}
// Moving ownership does not change the boxed callback address. Public Engine
// operations require exclusive access; the event loop accesses only synchronized state.
unsafe impl Send for NativeEngine {}

impl NativeEngine {
    pub(crate) fn start(
        snapshot: Arc<Mutex<EngineState>>,
        audio: AudioMode,
        recording: Option<PathBuf>,
    ) -> Result<Self, PhoneError> {
        let recording = recording.as_deref().and_then(|p| p.to_str()).unwrap_or("");
        if matches!(audio, AudioMode::TestTone) && (recording.is_empty() || recording.len() > 127) {
            return Err(PhoneError::new(
                ErrorCode::InvalidInput,
                "Test audio requires a UTF-8 recording path of at most 127 bytes.",
            ));
        }
        let recording = cstring(recording)?;
        if ENGINE_LEASE.swap(true, Ordering::AcqRel) {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "Only one native engine can run in this process.",
            ));
        }
        let (ready, receiver) = mpsc::channel();
        let context = NonNull::new(Box::into_raw(Box::new(CallbackContext { snapshot, ready })))
            .expect("Box is non-null");
        let raw = unsafe {
            sip_engine_create(
                callback,
                context.as_ptr().cast(),
                i32::from(matches!(audio, AudioMode::TestTone)),
                recording.as_ptr(),
            )
        };
        let Some(pointer) = NonNull::new(raw) else {
            unsafe {
                drop(Box::from_raw(context.as_ptr()));
            }
            ENGINE_LEASE.store(false, Ordering::Release);
            return Err(native_error(-1));
        };
        let handle = Handle(pointer);
        let thread = match thread::Builder::new()
            .name("baresip-event-loop".into())
            .spawn(move || handle.run())
        {
            Ok(thread) => thread,
            Err(_) => {
                unsafe {
                    sip_engine_destroy(raw);
                    drop(Box::from_raw(context.as_ptr()));
                }
                ENGINE_LEASE.store(false, Ordering::Release);
                return Err(native_error(-1));
            }
        };
        let engine = Self {
            handle: Some(handle),
            thread: Some(thread),
            context: Some(context),
        };
        receiver.recv().map_err(|_| native_error(-1))??;
        Ok(engine)
    }

    pub(crate) fn reserve_call_generation(&self) -> Result<u64, PhoneError> {
        let handle = self
            .handle
            .ok_or_else(|| PhoneError::new(ErrorCode::EngineUnavailable, "Engine is stopped."))?;
        // The same native queue lock serializes reservations and incoming tokens.
        Ok(unsafe { sip_engine_reserve_call_generation(handle.0.as_ptr()) })
    }

    pub(crate) fn request(
        &self,
        operation: i32,
        account_id: u32,
        account: Option<&AccountConfig>,
        destination: Option<&str>,
        call_generation: u64,
    ) -> Result<(), PhoneError> {
        self.request_strings(
            operation,
            account_id,
            call_generation,
            account.map_or("", |a| a.registrar.as_str()),
            account.map_or("", |a| a.username.as_str()),
            account.map_or("", |a| a.password.as_str()),
            destination.unwrap_or(""),
        )
    }
    pub(crate) fn sound_request(
        &self,
        operation: i32,
        settings: &SoundSettings,
    ) -> Result<(), PhoneError> {
        self.request_strings(
            operation,
            0,
            0,
            &settings.input_device,
            &settings.output_device,
            &settings.ring_device,
            &format!(
                "{}:{}:{}",
                u8::from(settings.ringtone_enabled),
                match settings.ringtone {
                    phone_core::Ringtone::Classic => 0,
                    phone_core::Ringtone::Soft => 1,
                    phone_core::Ringtone::Short => 2,
                },
                settings.ringtone_volume
            ),
        )
    }
    pub(crate) fn audio_test_request(
        &self,
        operation: i32,
        id: u64,
        device: &str,
    ) -> Result<(), PhoneError> {
        self.request_strings(operation, 0, id, device, "", "", "")
    }
    pub(crate) fn preview_request(&self, operation: i32, device: &str) -> Result<(), PhoneError> {
        self.request_strings(operation, 0, 0, device, "", "", "")
    }
    #[allow(clippy::too_many_arguments)]
    fn request_strings(
        &self,
        operation: i32,
        account_id: u32,
        call_generation: u64,
        registrar: &str,
        username: &str,
        password: &str,
        destination: &str,
    ) -> Result<(), PhoneError> {
        let handle = self
            .handle
            .ok_or_else(|| PhoneError::new(ErrorCode::EngineUnavailable, "Engine is stopped."))?;
        let registrar = cstring(registrar)?;
        let username = cstring(username)?;
        let password = cstring(password)?;
        let destination = cstring(destination)?;
        let error = unsafe {
            sip_engine_request(
                handle.0.as_ptr(),
                operation,
                account_id,
                call_generation,
                registrar.as_ptr(),
                username.as_ptr(),
                password.as_ptr(),
                destination.as_ptr(),
            )
        };
        if error == 0 {
            Ok(())
        } else {
            Err(native_error(error))
        }
    }

    pub(crate) fn shutdown(&mut self) -> Result<(), PhoneError> {
        if self.handle.is_none() {
            return Ok(());
        }
        let request_result = self.request(5, 0, None, None, 0);
        let result = self
            .thread
            .take()
            .map(|t| t.join())
            .transpose()
            .map_err(|_| native_error(-1));
        if let Some(handle) = self.handle.take() {
            unsafe {
                sip_engine_destroy(handle.0.as_ptr());
            }
        }
        if let Some(context) = self.context.take() {
            unsafe {
                drop(Box::from_raw(context.as_ptr()));
            }
        }
        ENGINE_LEASE.store(false, Ordering::Release);
        match result? {
            Some(code) if code != 0 => Err(native_error(code)),
            _ => request_result,
        }
    }
}

impl Drop for NativeEngine {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn cstring(input: &str) -> Result<CString, PhoneError> {
    CString::new(input)
        .map_err(|_| PhoneError::new(ErrorCode::InvalidInput, "NUL bytes are not allowed."))
}
fn native_error(code: i32) -> PhoneError {
    PhoneError::new(
        ErrorCode::NativeFailure,
        format!("Native operation failed (code {code})."),
    )
}

extern "C" fn device_callback(
    input: i32,
    id: *const std::ffi::c_char,
    name: *const std::ffi::c_char,
    selectable: i32,
    arg: *mut c_void,
) {
    // The synchronous C inventory borrows this stack context only until it returns.
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if id.is_null() || name.is_null() {
            return;
        }
        let devices = unsafe { &mut *arg.cast::<AudioDevices>() };
        let (Ok(id), Ok(name)) = (
            unsafe { CStr::from_ptr(id) }.to_str(),
            unsafe { CStr::from_ptr(name) }.to_str(),
        ) else {
            return;
        };
        let device = AudioDevice {
            id: id.into(),
            name: name.into(),
            selectable: selectable != 0,
        };
        if input != 0 {
            devices.inputs.push(device);
        } else {
            devices.outputs.push(device);
        }
    }));
}
pub(crate) fn audio_devices() -> Result<AudioDevices, PhoneError> {
    let mut devices = AudioDevices::default();
    let error =
        unsafe { sip_audio_devices(device_callback, (&mut devices as *mut AudioDevices).cast()) };
    if error != 0 {
        return Err(native_error(error));
    }
    Ok(devices)
}

#[cfg(test)]
mod caller_tests {
    use super::*;

    #[test]
    fn caller_number_rejects_uri_and_password_delimiters() {
        for text in [
            "sip:user",
            "user:password",
            "user@host",
            "user;param",
            "user?header",
            "bad user",
        ] {
            assert!(!caller_number_is_display_safe(text), "{text}");
        }
        assert!(caller_number_is_display_safe("+4930123"));
        assert!(caller_number_is_display_safe("alice.sip_42"));
    }

    #[test]
    fn caller_text_rejects_controls_bidi_and_preserves_utf8_boundaries() {
        for text in [
            "bad\nname",
            "bad\0name",
            "safe\u{202e}evil",
            "safe\u{2066}evil",
            "safe\u{2028}evil",
            "safe\u{2029}evil",
        ] {
            assert!(sanitize_caller_text(text, 128).is_none());
        }
        assert_eq!(
            sanitize_caller_text("  Büro  ", 128).as_deref(),
            Some("Büro")
        );
        assert_eq!(sanitize_caller_text("  ", 128), None);
        assert_eq!(sanitize_caller_text("ééé", 5).as_deref(), Some("éé"));
    }
}

#[cfg(test)]
mod observation_tests {
    use super::*;
    use phone_core::{CallOutcome, ObservedCallEvent, ObservedCallKind};

    struct Harness {
        context: Box<CallbackContext>,
        events: Arc<Mutex<Vec<ObservedCallEvent>>>,
    }
    impl Harness {
        fn new() -> Self {
            let events = Arc::new(Mutex::new(Vec::new()));
            let sink = Arc::clone(&events);
            let mut state = EngineState::default();
            state.call_journal.observer =
                Some(Arc::new(move |event| sink.lock().unwrap().push(event)));
            Self {
                context: Box::new(CallbackContext {
                    snapshot: Arc::new(Mutex::new(state)),
                    ready: mpsc::channel().0,
                }),
                events,
            }
        }
        fn event(&mut self, kind: i32, code: i32, token: u64, name: &str, number: &str) {
            let mut caller_name = [0u8; 129];
            let mut caller_number = [0u8; 81];
            caller_name[..name.len()].copy_from_slice(name.as_bytes());
            caller_number[..number.len()].copy_from_slice(number.as_bytes());
            let event = NativeEvent {
                kind,
                code,
                account_id: 42,
                call_generation: token,
                caller_name: caller_name.as_ptr().cast(),
                caller_number: caller_number.as_ptr().cast(),
                detail: std::ptr::null(),
            };
            callback(&event, (&mut *self.context as *mut CallbackContext).cast());
        }
    }
    #[test]
    fn activation_rejection_restores_prior_identity_and_confirmed_registration() {
        let mut h = Harness::new();
        let previous = phone_core::AccountSnapshot {
            id: 42,
            name: "Original".into(),
            registrar: "pbx.test".into(),
            username: "synthetic".into(),
            enabled: true,
            enabled_changing: false,
            registered_at_ms: Some(123),
            registration: RegistrationState::Registered,
            last_error: None,
            removing: false,
        };
        {
            let mut state = h.context.snapshot.lock().unwrap();
            state.snapshot.selected_account_id = Some(42);
            state.pending_accounts.insert(42, Some(previous.clone()));
            state.snapshot.accounts.push(phone_core::AccountSnapshot {
                enabled: false,
                enabled_changing: true,
                registered_at_ms: None,
                registration: RegistrationState::Unconfigured,
                ..previous
            });
        }
        h.event(16, 16, 0, "", "");
        let mut state = h.context.snapshot.lock().unwrap();
        assert!(state.snapshot.accounts[0].enabled);
        assert!(!state.snapshot.accounts[0].enabled_changing);
        assert_eq!(state.snapshot.accounts[0].registered_at_ms, Some(123));
        assert_eq!(state.snapshot.selected_account_id, Some(42));
        assert_eq!(
            state.account_results.remove(&42).unwrap().unwrap_err().code,
            ErrorCode::Busy
        );
        state.snapshot.accounts[0].enabled = false;
        state.snapshot.accounts[0].registered_at_ms = None;
        state.snapshot.accounts[0].registration = RegistrationState::Unconfigured;
        drop(state);
        h.event(3, 0, 0, "", "");
        let state = h.context.snapshot.lock().unwrap();
        assert!(state.snapshot.accounts[0].registered_at_ms.is_none());
        assert_eq!(
            state.snapshot.accounts[0].registration,
            RegistrationState::Unconfigured
        );
    }

    #[test]
    fn hold_resume_preserves_caller_and_native_connection_time() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "Lab Caller", "123");
        h.event(8, 0, 10, "", "");
        let connected = h
            .context
            .snapshot
            .lock()
            .unwrap()
            .snapshot
            .call_connected_at_ms;
        assert!(connected.is_some());
        h.event(28, 0, 10, "", "");
        {
            let state = h.context.snapshot.lock().unwrap();
            assert_eq!(state.snapshot.call, CallState::Held);
            assert_eq!(
                state.snapshot.caller.as_ref().unwrap().number.as_deref(),
                Some("123")
            );
        }
        h.event(29, 0, 10, "", "");
        let state = h.context.snapshot.lock().unwrap();
        assert_eq!(state.snapshot.call, CallState::Active);
        assert_eq!(state.snapshot.call_connected_at_ms, connected);
        assert_eq!(state.snapshot.call_events.len(), 2);
    }

    #[test]
    fn late_close_and_hold_cannot_replace_a_newer_call() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "Old", "100");
        h.event(9, 0, 10, "", "");
        h.event(7, 0, 11, "New", "200");
        h.event(28, 0, 10, "", "");
        h.event(9, 486, 10, "", "");
        let state = h.context.snapshot.lock().unwrap();
        assert_eq!(state.snapshot.call, CallState::Incoming);
        assert_eq!(state.snapshot.call_id.as_deref(), Some("11"));
        assert_eq!(
            state.snapshot.caller.as_ref().unwrap().number.as_deref(),
            Some("200")
        );
        assert!(state.snapshot.last_error.is_none());
    }

    #[test]
    fn event_retention_is_bounded_and_explicit_instead_of_collapsing_signals() {
        let mut h = Harness::new();
        h.event(5, 0, 10, "", "");
        for _ in 0..80 {
            h.event(27, 0, 10, "", "");
        }
        let state = h.context.snapshot.lock().unwrap();
        assert_eq!(state.snapshot.call, CallState::EarlyMedia);
        assert_eq!(state.snapshot.call_events.len(), 64);
        assert_eq!(state.snapshot.call_events_dropped, 16);
        assert_eq!(state.snapshot.call_events[0].sequence, 17);
        assert_eq!(state.snapshot.call_events[63].sequence, 80);
    }

    #[test]
    fn final_protocol_failures_are_separate_from_errno_and_local_rejection() {
        let mut h = Harness::new();
        h.event(5, 0, 10, "", "");
        h.event(9, 486, 10, "", "");
        {
            let state = h.context.snapshot.lock().unwrap();
            let error = state.snapshot.last_error.as_ref().unwrap();
            assert_eq!(error.code, ErrorCode::SipBusy);
            assert_eq!(error.sip_status, Some(486));
        }
        h.event(7, 0, 11, "", "100");
        h.event(25, 0, 11, "", "");
        h.event(9, 486, 11, "", "");
        assert!(
            h.context
                .snapshot
                .lock()
                .unwrap()
                .snapshot
                .last_error
                .is_none()
        );
        h.event(10, 503, 11, "", "");
        let state = h.context.snapshot.lock().unwrap();
        let error = state.snapshot.last_error.as_ref().unwrap();
        assert_eq!(error.code, ErrorCode::NativeFailure);
        assert_eq!(error.sip_status, None);
    }

    #[test]
    fn transfer_failure_without_a_response_does_not_invent_a_sip_status() {
        let mut h = Harness::new();
        h.event(5, 0, 10, "", "");
        h.event(8, 0, 10, "", "");
        h.event(32, 0, 10, "", "");
        let state = h.context.snapshot.lock().unwrap();
        let error = state.snapshot.last_error.as_ref().unwrap();
        assert_eq!(error.code, ErrorCode::NativeFailure);
        assert_eq!(error.sip_status, None);
        assert_eq!(state.snapshot.call, CallState::Active);
        assert!(matches!(
            state.snapshot.call_events.last().unwrap().event,
            CallEventKind::TransferFailed
        ));
    }

    #[test]
    fn unestablished_outgoing_close_without_a_response_is_a_native_failure() {
        for progress in [None, Some(6), Some(EV_PROGRESS)] {
            let mut h = Harness::new();
            h.event(5, 0, 10, "", "");
            if let Some(progress) = progress {
                h.event(progress, 0, 10, "", "");
            }
            h.event(9, 0, 10, "", "");
            let state = h.context.snapshot.lock().unwrap();
            let error = state.snapshot.last_error.as_ref().expect("setup failure");
            assert_eq!(error.code, ErrorCode::NativeFailure);
            assert_eq!(error.sip_status, None);
            assert_eq!(state.snapshot.call, CallState::Idle);
        }
    }

    #[test]
    fn expected_zero_status_closures_do_not_invent_a_setup_failure() {
        for (start, established, ending) in [
            (7, false, None),     // Remote cancellation of an incoming offer.
            (5, false, Some(25)), // Accepted local outgoing cancellation.
            (5, false, Some(26)), // Engine shutdown interrupts a pending attempt.
            (5, true, None),      // Remote termination of an established call.
        ] {
            let mut h = Harness::new();
            h.event(start, 0, 10, "", "");
            if established {
                h.event(8, 0, 10, "", "");
            }
            if let Some(ending) = ending {
                h.event(ending, 0, 10, "", "");
            }
            h.event(9, 0, 10, "", "");
            let state = h.context.snapshot.lock().unwrap();
            assert!(state.snapshot.last_error.is_none());
            assert_eq!(state.snapshot.call, CallState::Idle);
        }
    }

    #[test]
    fn a_sent_answer_closure_reports_failure_but_a_queued_answer_cancellation_does_not() {
        for (closed, expected_error) in [(9, false), (EV_ANSWER_CLOSED, true)] {
            let mut h = Harness::new();
            h.event(7, 0, 10, "", "100");
            h.context.snapshot.lock().unwrap().snapshot.call = CallState::Answering;
            h.event(closed, 0, 10, "", "");
            let state = h.context.snapshot.lock().unwrap();
            assert_eq!(state.snapshot.call, CallState::Idle);
            assert_eq!(state.snapshot.last_error.is_some(), expected_error);
            if let Some(error) = &state.snapshot.last_error {
                assert_eq!(error.code, ErrorCode::NativeFailure);
                assert_eq!(error.sip_status, None);
            }
            let events = h.events.lock().unwrap();
            assert!(matches!(events.last().unwrap().kind,
                ObservedCallKind::Finished { outcome, sip_status: None, .. }
                if outcome == if expected_error { CallOutcome::Failed } else { CallOutcome::Missed }
            ));
        }
    }

    #[test]
    fn an_old_failed_answer_cannot_close_a_new_offer() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "", "100");
        h.event(9, 0, 10, "", "");
        h.event(7, 0, 11, "", "200");
        h.event(EV_ANSWER_CLOSED, 0, 10, "", "");
        let state = h.context.snapshot.lock().unwrap();
        assert_eq!(state.snapshot.call, CallState::Incoming);
        assert_eq!(state.snapshot.call_id.as_deref(), Some("11"));
        assert!(state.snapshot.last_error.is_none());
    }

    #[test]
    fn native_call_close_failure_is_reported_without_claiming_a_sip_response() {
        let mut h = Harness::new();
        h.event(5, 0, 10, "", "");
        // Pinned baresip reports native RTP stream failures with its internal 701 sentinel.
        h.event(9, 701, 10, "", "");
        let state = h.context.snapshot.lock().unwrap();
        let error = state
            .snapshot
            .last_error
            .as_ref()
            .expect("native close failure must be visible");
        assert_eq!(error.code, ErrorCode::NativeFailure);
        assert_eq!(error.sip_status, None);
        assert_eq!(state.snapshot.call, CallState::Idle);
    }
    #[test]
    fn incoming_offers_keep_effective_account_identity_while_replacement_is_pending() {
        for kind in [7, 22] {
            let mut h = Harness::new();
            let original = phone_core::AccountSnapshot {
                id: 42,
                name: "Original".into(),
                registrar: "old.example.test".into(),
                username: "old-user".into(),
                registration: RegistrationState::Registered,
                last_error: None,
                removing: false,
                enabled: true,
                enabled_changing: false,
                registered_at_ms: None,
            };
            {
                let mut state = h.context.snapshot.lock().unwrap();
                state.pending_accounts.insert(42, Some(original.clone()));
                state.snapshot.accounts.push(phone_core::AccountSnapshot {
                    name: "Replacement".into(),
                    registrar: "new.example.test".into(),
                    username: "new-user".into(),
                    registration: RegistrationState::Registering,
                    ..original
                });
            }
            h.event(kind, 0, 10, "Caller", "100");
            h.event(16, 16, 10, "", "");
            let events = h.events.lock().unwrap();
            match &events[0].kind {
                ObservedCallKind::Started {
                    account: Some(account),
                    ..
                } => {
                    assert_eq!(account.name, "Original");
                    assert_eq!(account.registrar, "old.example.test");
                    assert_eq!(account.username, "old-user");
                }
                _ => panic!("Missing effective account identity"),
            }
        }
    }

    #[test]
    fn consultation_background_events_and_end_intent_keep_dialog_ownership() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "Original", "100");
        h.event(8, 0, 10, "", "");
        h.event(23, 0, 20, "", "200");
        h.event(5, 0, 20, "", "");
        h.event(8, 0, 20, "", "");
        h.event(EV_HOLD, 0, 10, "", "");
        {
            let state = h.context.snapshot.lock().unwrap();
            assert_eq!(state.snapshot.call_id.as_deref(), Some("20"));
            assert_eq!(state.snapshot.call, CallState::Active);
            assert_eq!(state.snapshot.calls.len(), 2);
            assert!(state.snapshot.calls[0].remote_held);
            assert_eq!(
                state.snapshot.calls[0]
                    .caller
                    .as_ref()
                    .unwrap()
                    .name
                    .as_deref(),
                Some("Original")
            );
            assert_eq!(state.snapshot.calls[0].account_id, 42);
            assert_eq!(state.snapshot.calls[1].destination.as_deref(), Some("200"));
            assert_eq!(state.snapshot.calls[0].events.len(), 1);
            assert!(state.snapshot.calls[1].events.is_empty());
        }
        h.event(25, 0, 20, "", "");
        h.event(EV_RESUME, 0, 10, "", "");
        h.event(9, 0, 20, "", "");
        {
            let state = h.context.snapshot.lock().unwrap();
            assert_eq!(state.snapshot.call_id.as_deref(), Some("10"));
            assert_eq!(state.snapshot.calls.len(), 1);
            assert_eq!(state.snapshot.call_events.len(), 2);
            assert!(state.snapshot.last_error.is_none());
        }
        h.event(9, 0, 10, "", "");
        let events = h.events.lock().unwrap();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event.kind, ObservedCallKind::Started { .. }))
                .count(),
            2
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(
                    event.kind,
                    ObservedCallKind::Finished {
                        outcome: CallOutcome::Completed,
                        ..
                    }
                ))
                .count(),
            2
        );
    }

    #[test]
    fn busy_offer_preserves_active_snapshot_token_and_identity() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "First", "100");
        h.event(8, 0, 10, "", "");
        h.event(22, 0, 11, "Second", "200");
        let state = h.context.snapshot.lock().unwrap();
        assert_eq!(state.call_generation, 10);
        assert_eq!(state.snapshot.call_id.as_deref(), Some("10"));
        assert_eq!(state.snapshot.call, CallState::Active);
        assert_eq!(
            state.snapshot.caller.as_ref().unwrap().number.as_deref(),
            Some("100")
        );
        let events = h.events.lock().unwrap();
        assert_eq!(events.len(), 4);
        assert!(matches!(
            events[3].kind,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Busy,
                ..
            }
        ));
    }
    #[test]
    fn failed_dial_attempt_does_not_close_an_incoming_call_that_won_the_race() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "First", "100");
        h.event(23, 0, 11, "", "4321");
        h.event(24, 500, 11, "", "");
        let state = h.context.snapshot.lock().unwrap();
        assert_eq!(state.snapshot.call, CallState::Incoming);
        assert_eq!(state.call_generation, 10);
        assert!(state.snapshot.last_error.is_none());
        let events = h.events.lock().unwrap();
        assert!(
            matches!(&events[1].kind, ObservedCallKind::Started { direction: CallDirection::Outgoing, number: Some(number), .. } if number == "4321")
        );
        assert!(matches!(
            events[2].kind,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Failed,
                sip_status: None,
                ..
            }
        ));
    }
    #[test]
    fn a_failed_current_dial_still_reports_its_native_error() {
        let mut h = Harness::new();
        h.event(23, 0, 10, "", "4321");
        h.event(24, 16, 10, "", "");
        let state = h.context.snapshot.lock().unwrap();
        assert_eq!(state.snapshot.call, CallState::Idle);
        let error = state.snapshot.last_error.as_ref().expect("dial failure");
        assert_eq!(error.code, ErrorCode::NativeFailure);
        assert_eq!(error.sip_status, None);
    }

    #[test]
    fn unaccepted_stale_action_cannot_change_a_remote_cancel_to_local_rejection() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "", "100");
        h.event(12, 0, 10, "", "");
        h.event(9, 487, 10, "", "");
        let events = h.events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[1].kind,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Missed,
                ..
            }
        ));
    }
    #[test]
    fn invalid_caller_text_is_absent_from_history_as_well_as_the_snapshot() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "name\u{202e}private", "user:secret@host");
        h.event(9, 487, 10, "", "");
        let events = h.events.lock().unwrap();
        assert!(matches!(
            events[0].kind,
            ObservedCallKind::Started {
                remote_name: None,
                number: None,
                ..
            }
        ));
    }
    #[test]
    fn accepted_local_intent_and_engine_stop_are_observed_before_close() {
        let mut h = Harness::new();
        h.event(7, 0, 10, "", "100");
        h.event(25, 0, 10, "", "");
        h.event(9, 486, 10, "", "");
        h.event(23, 0, 11, "", "4321");
        h.event(5, 0, 11, "", "");
        h.event(26, 0, 11, "", "");
        h.event(9, 0, 11, "", "");
        h.event(11, 0, 11, "", "");
        let events = h.events.lock().unwrap();
        assert_eq!(events.len(), 4);
        assert!(matches!(
            events[1].kind,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Rejected,
                ..
            }
        ));
        assert!(matches!(
            events[3].kind,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Interrupted,
                ..
            }
        ));
    }
}

#[cfg(test)]
mod audio_dsp_tests {
    unsafe extern "C" {
        fn sip_audio_peak_s16(samples: *const i16, count: usize) -> u32;
        fn sip_ringtone_sample(variant: i32, volume: u32, index: u32) -> i16;
    }
    #[test]
    fn synthetic_meter_handles_silence_half_scale_and_minimum_sample() {
        for (samples, expected) in [
            ([0, 0, 0], 0),
            ([0, 16384, -100], 500),
            ([0, -32768, 0], 1000),
        ] {
            assert_eq!(
                unsafe { sip_audio_peak_s16(samples.as_ptr(), samples.len()) },
                expected
            );
        }
    }
    #[test]
    fn pcm_ringtones_preserve_classic_and_scale_amplitude_without_clipping() {
        let mut peaks = [0_i32; 3];
        for i in 0..96000 {
            let seconds = f64::from(i) / 48000.0;
            let local = seconds % 0.6;
            let expected = if seconds < 1.0 && local < 0.4 {
                (2200.0
                    * 1.0_f64.min((local / 0.01).min((0.4 - local) / 0.01))
                    * ((2.0 * std::f64::consts::PI * 440.0 * seconds).sin()
                        + (2.0 * std::f64::consts::PI * 480.0 * seconds).sin()))
                    as i16
            } else {
                0
            };
            assert_eq!(unsafe { sip_ringtone_sample(0, 100, i) }, expected);
            for variant in 0..3 {
                let sample = unsafe { sip_ringtone_sample(variant, 100, i) };
                peaks[variant as usize] = peaks[variant as usize].max(i32::from(sample).abs());
                assert_eq!(unsafe { sip_ringtone_sample(variant, 0, i) }, 0);
                assert!(
                    (i32::from(unsafe { sip_ringtone_sample(variant, 50, i) }) * 2
                        - i32::from(sample))
                    .abs()
                        <= 1
                );
            }
            if i >= 12000 {
                assert_eq!(unsafe { sip_ringtone_sample(2, 100, i) }, 0);
            }
        }
        assert!(peaks[0] <= 4400 && peaks[0] > 4000);
        assert!(peaks[1] < peaks[0]);
        assert!(peaks[2] > 0);
    }
}
