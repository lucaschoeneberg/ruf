//! Private unsafe boundary: callback ownership outlives the joined C event loop.

use crate::engine::EngineState;
use phone_core::{
    AccountConfig, AudioDevice, AudioDevices, AudioMode, CallState, ErrorCode, PhoneError,
    RegistrationState, SoundSettings,
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

unsafe extern "C" {
    fn sip_engine_create(
        callback: extern "C" fn(i32, i32, u32, u64, *mut c_void),
        arg: *mut c_void,
        audio_mode: i32,
        recording: *const std::ffi::c_char,
    ) -> *mut c_void;
    fn sip_engine_run(handle: *mut c_void) -> i32;
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

extern "C" fn callback(
    kind: i32,
    code: i32,
    account_id: u32,
    call_generation: u64,
    arg: *mut c_void,
) {
    // arg points to the owned box held until the engine thread has joined.
    let _ = catch_unwind(AssertUnwindSafe(|| {
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
            engine.call_generation = call_generation;
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
            let state = &mut engine.snapshot;
            match kind {
                2..=4 => {
                    if let Some(account) = state.accounts.iter_mut().find(|a| a.id == account_id) {
                        account.registration = match kind {
                            2 => RegistrationState::Registering,
                            3 => RegistrationState::Registered,
                            _ => RegistrationState::Failed,
                        };
                        account.last_error = (kind == 4).then(|| {
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
                        _ => CallState::Active,
                    };
                    state.call_account_id = Some(account_id);
                    state.call_id = Some(call_generation.to_string());
                }
                9 => {
                    state.call = CallState::Idle;
                    state.call_account_id = None;
                    state.call_id = None;
                    if code >= 400 {
                        state.last_error = Some(native_error(code));
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
                    state.registration = RegistrationState::Unconfigured;
                    state.call = CallState::Idle;
                    state.call_account_id = None;
                    state.call_id = None;
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
                _ => {}
            }
        }
    }));
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
            if settings.ringtone_enabled { "1" } else { "0" },
        )
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
