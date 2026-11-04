//! Private unsafe boundary: callback ownership outlives the joined C event loop.

use phone_core::{
    AccountConfig, AudioMode, CallState, EngineSnapshot, ErrorCode, PhoneError, RegistrationState,
};
use std::{
    ffi::{CString, c_void},
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
        callback: extern "C" fn(i32, i32, *mut c_void),
        arg: *mut c_void,
        audio_mode: i32,
        recording: *const std::ffi::c_char,
    ) -> *mut c_void;
    fn sip_engine_run(handle: *mut c_void) -> i32;
    fn sip_engine_request(
        handle: *mut c_void,
        operation: i32,
        registrar: *const std::ffi::c_char,
        username: *const std::ffi::c_char,
        password: *const std::ffi::c_char,
        destination: *const std::ffi::c_char,
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
    snapshot: Arc<Mutex<EngineSnapshot>>,
    ready: mpsc::Sender<Result<(), PhoneError>>,
}

extern "C" fn callback(kind: i32, code: i32, arg: *mut c_void) {
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
        if let Ok(mut state) = context.snapshot.lock() {
            match kind {
                2 => state.registration = RegistrationState::Registering,
                3 => {
                    state.registration = RegistrationState::Registered;
                    state.last_error = None;
                }
                4 => {
                    state.registration = RegistrationState::Failed;
                    state.last_error = Some(PhoneError::new(
                        ErrorCode::NativeFailure,
                        "Registration failed; verify the local registrar and credentials.",
                    ));
                }
                5 => state.call = CallState::Calling,
                6 => state.call = CallState::Ringing,
                7 => state.call = CallState::Incoming,
                8 => state.call = CallState::Active,
                9 => {
                    state.call = CallState::Idle;
                    if code >= 400 {
                        state.last_error = Some(native_error(code));
                    }
                }
                10 => state.last_error = Some(native_error(code)),
                11 => {
                    state.registration = RegistrationState::Unconfigured;
                    state.call = CallState::Idle;
                    if code != 0 {
                        state.last_error = Some(native_error(code));
                    }
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
        snapshot: Arc<Mutex<EngineSnapshot>>,
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
        account: Option<&AccountConfig>,
        destination: Option<&str>,
    ) -> Result<(), PhoneError> {
        let handle = self
            .handle
            .ok_or_else(|| PhoneError::new(ErrorCode::EngineUnavailable, "Engine is stopped."))?;
        let registrar = cstring(account.map_or("", |a| a.registrar.as_str()))?;
        let username = cstring(account.map_or("", |a| a.username.as_str()))?;
        let password = cstring(account.map_or("", |a| a.password.as_str()))?;
        let destination = cstring(destination.unwrap_or(""))?;
        let error = unsafe {
            sip_engine_request(
                handle.0.as_ptr(),
                operation,
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
        let request_result = self.request(5, None, None);
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
