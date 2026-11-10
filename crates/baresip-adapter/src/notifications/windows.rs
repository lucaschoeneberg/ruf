//! Unpackaged desktop toasts with a COM activator for Notification Center actions.
//!
//! COM and WinRT objects stay on one MTA worker. OS callbacks only enqueue an
//! opaque action; the desktop service must authorize it against the current call.
use super::{CallNotificationAction, NotificationPermission, windows_payload};
use phone_core::{ErrorCode, PhoneError};
use std::{
    collections::HashMap,
    ffi::c_void,
    sync::{
        Mutex, OnceLock,
        mpsc::{self, Receiver, Sender, SyncSender},
    },
    time::Duration,
};
use windows::{
    Data::Xml::Dom::XmlDocument,
    Foundation::{DateTime, IReference, PropertyValue, TypedEventHandler},
    UI::Notifications::{
        NotificationSetting, ToastActivatedEventArgs, ToastNotification, ToastNotificationManager,
        ToastNotifier,
    },
    Win32::{
        Foundation::{CLASS_E_NOAGGREGATION, E_POINTER},
        System::{
            Com::{
                CLSCTX_LOCAL_SERVER, CoRegisterClassObject, CoRevokeClassObject, IClassFactory,
                IClassFactory_Impl, REGCLS_MULTIPLEUSE,
            },
            Registry::{
                HKEY, HKEY_CURRENT_USER, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
                RegCreateKeyExW, RegSetValueExW,
            },
            WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
        },
        UI::Notifications::{
            INotificationActivationCallback, INotificationActivationCallback_Impl,
            NOTIFICATION_USER_INPUT_DATA,
        },
    },
    core::{BOOL, GUID, HSTRING, IInspectable, IUnknown, Interface, PCWSTR, Ref, implement},
};

const ACTIVATOR: GUID = GUID::from_u128(0xa14373db_08e7_45ea_9e24_26cb70bfa7b1);
const ACTIVATOR_TEXT: &str = "{A14373DB-08E7-45EA-9E24-26CB70BFA7B1}";
const GROUP: &str = "sip-calls";
const TEST_GROUP: &str = "sip-settings";
const TIMEOUT: Duration = Duration::from_secs(5);
const MAX_TOASTS: usize = 32;
static INIT_LOCK: Mutex<()> = Mutex::new(());
static SERVICE: OnceLock<Option<SyncSender<Command>>> = OnceLock::new();
type Reply<T> = Sender<Result<T, PhoneError>>;

enum Command {
    Permission(Reply<NotificationPermission>),
    Test {
        title: String,
        body: String,
        reply: Reply<()>,
    },
    Show {
        token: String,
        title: String,
        body: String,
        answer: String,
        reject: String,
        reply: Reply<()>,
    },
    Dismiss {
        token: String,
        reply: Reply<()>,
    },
}

pub(super) fn init(
    sender: Sender<CallNotificationAction>,
    app_id: &str,
    register_activation: bool,
) -> Result<(), PhoneError> {
    let _lock = INIT_LOCK.lock().map_err(|_| failure())?;
    if SERVICE.get().is_some() {
        return Ok(());
    }
    if !register_activation {
        let _ = SERVICE.set(None);
        return Ok(());
    }
    if app_id.is_empty()
        || app_id.len() > 128
        || !app_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
    {
        return Err(failure());
    }
    let app_id = app_id.to_owned();
    let (commands, receiver) = mpsc::sync_channel(16);
    let (ready, initialized) = mpsc::channel();
    std::thread::Builder::new()
        .name("call-notifications".into())
        .spawn(move || {
            let result = Worker::new(sender, &app_id);
            match result {
                Ok(mut worker) => {
                    if ready.send(Ok(())).is_ok() {
                        worker.run(receiver);
                    }
                }
                Err(_) => {
                    let _ = ready.send(Err(failure()));
                }
            }
        })
        .map_err(|_| failure())?;
    initialized.recv_timeout(TIMEOUT).map_err(|_| failure())??;
    SERVICE.set(Some(commands)).map_err(|_| failure())
}

pub(super) fn permission(_request: bool) -> Result<NotificationPermission, PhoneError> {
    if matches!(SERVICE.get(), Some(None)) {
        return Ok(NotificationPermission::Unsupported);
    }
    request(Command::Permission)
}

pub(super) fn show(
    token: &str,
    title: &str,
    body: &str,
    answer: &str,
    reject: &str,
) -> Result<(), PhoneError> {
    // Toast tags have a 16-character limit. The desktop issues 16 hex digits.
    if token.len() > 16 {
        return Err(PhoneError::new(
            ErrorCode::InvalidInput,
            "Windows call notification tokens must not exceed 16 characters.",
        ));
    }
    request(|reply| Command::Show {
        token: token.into(),
        title: title.into(),
        body: body.into(),
        answer: answer.into(),
        reject: reject.into(),
        reply,
    })
}

pub(super) fn show_test(title: &str, body: &str) -> Result<(), PhoneError> {
    if SERVICE.get().and_then(Option::as_ref).is_none() {
        return Err(PhoneError::new(
            ErrorCode::EngineUnavailable,
            "Native test notifications are unavailable in this launch context.",
        ));
    }
    request(|reply| Command::Test {
        title: title.into(),
        body: body.into(),
        reply,
    })
}

pub(super) fn dismiss(token: &str) -> Result<(), PhoneError> {
    if matches!(SERVICE.get(), Some(None)) {
        return Ok(());
    }
    request(|reply| Command::Dismiss {
        token: token.into(),
        reply,
    })
}

fn request<T>(command: impl FnOnce(Reply<T>) -> Command) -> Result<T, PhoneError> {
    let service = SERVICE.get().and_then(Option::as_ref).ok_or_else(failure)?;
    let (reply, result) = mpsc::channel();
    service.try_send(command(reply)).map_err(|_| failure())?;
    result.recv_timeout(TIMEOUT).map_err(|_| failure())?
}

fn failure() -> PhoneError {
    PhoneError::new(
        ErrorCode::NativeFailure,
        "Windows call notifications are unavailable.",
    )
}

// RoUninitialize must run after every COM object has been released on this thread.
struct Apartment;
impl Apartment {
    fn new() -> windows::core::Result<Self> {
        // SAFETY: this dedicated thread has no earlier apartment initialization.
        unsafe {
            RoInitialize(RO_INIT_MULTITHREADED)?;
        }
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}
struct Registration(u32);
impl Drop for Registration {
    fn drop(&mut self) {
        let _ = unsafe { CoRevokeClassObject(self.0) };
    }
}
struct ActiveToast {
    toast: ToastNotification,
    activated: i64,
}
impl Drop for ActiveToast {
    fn drop(&mut self) {
        let _ = self.toast.RemoveActivated(self.activated);
    }
}
struct Worker {
    notifier: ToastNotifier,
    app_id: HSTRING,
    actions: Sender<CallNotificationAction>,
    toasts: HashMap<String, ActiveToast>,
    _registration: Registration,
    _apartment: Apartment,
}
impl Worker {
    fn new(actions: Sender<CallNotificationAction>, app_id: &str) -> windows::core::Result<Self> {
        let apartment = Apartment::new()?;
        register_installed_app(app_id)?;
        let factory: IClassFactory = ActivationFactory {
            actions: actions.clone(),
            app_id: app_id.into(),
        }
        .into();
        // SAFETY: the factory is a valid reference-counted COM object. COM retains
        // its own reference until the registration guard revokes this cookie.
        let cookie = unsafe {
            CoRegisterClassObject(
                &ACTIVATOR,
                &factory,
                CLSCTX_LOCAL_SERVER,
                REGCLS_MULTIPLEUSE,
            )?
        };
        let registration = Registration(cookie);
        let app_id = HSTRING::from(app_id);
        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&app_id)?;
        Ok(Self {
            notifier,
            app_id,
            actions,
            toasts: HashMap::new(),
            _registration: registration,
            _apartment: apartment,
        })
    }
    fn run(&mut self, commands: Receiver<Command>) {
        while let Ok(command) = commands.recv() {
            match command {
                Command::Permission(reply) => {
                    let result = self
                        .notifier
                        .Setting()
                        .map(|setting| {
                            if setting == NotificationSetting::Enabled {
                                NotificationPermission::Authorized
                            } else {
                                NotificationPermission::Denied
                            }
                        })
                        .map_err(|_| failure());
                    let _ = reply.send(result);
                }
                Command::Test { title, body, reply } => {
                    let _ = reply.send(self.show_test(&title, &body).map_err(|_| failure()));
                }
                Command::Show {
                    token,
                    title,
                    body,
                    answer,
                    reject,
                    reply,
                } => {
                    let _ = reply.send(
                        self.show(&token, &title, &body, &answer, &reject)
                            .map_err(|_| failure()),
                    );
                }
                Command::Dismiss { token, reply } => {
                    let _ = reply.send(self.dismiss(&token).map_err(|_| failure()));
                }
            }
        }
        for token in self.toasts.keys().cloned().collect::<Vec<_>>() {
            let _ = self.dismiss(&token);
        }
    }
    fn show(
        &mut self,
        token: &str,
        title: &str,
        body: &str,
        answer: &str,
        reject: &str,
    ) -> windows::core::Result<()> {
        if self.toasts.len() >= MAX_TOASTS && !self.toasts.contains_key(token) {
            return Err(windows::core::Error::from_hresult(
                windows::Win32::Foundation::E_OUTOFMEMORY,
            ));
        }
        self.dismiss(token)?;
        let document = XmlDocument::new()?;
        document.LoadXml(&HSTRING::from(windows_payload::toast_xml(
            token, title, body, answer, reject,
        )))?;
        let toast = ToastNotification::CreateToastNotification(&document)?;
        toast.SetTag(&HSTRING::from(token))?;
        toast.SetGroup(&HSTRING::from(GROUP))?;
        toast.SetExpiresOnReboot(true)?;
        // A call offer cannot remain actionable indefinitely after a process crash.
        let unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let expiry = DateTime {
            UniversalTime: ((unix.as_secs() + 300 + 11_644_473_600) * 10_000_000) as i64,
        };
        let expiration: IReference<DateTime> = PropertyValue::CreateDateTime(expiry)?.cast()?;
        toast.SetExpirationTime(&expiration)?;
        let actions = self.actions.clone();
        let activated = toast.Activated(
            &TypedEventHandler::<ToastNotification, IInspectable>::new(move |_, args| {
                if let Some(args) = args
                    .as_ref()
                    .and_then(|args| args.cast::<ToastActivatedEventArgs>().ok())
                    && let Ok(arguments) = args.Arguments()
                {
                    dispatch(&actions, &arguments.to_string_lossy());
                }
                Ok(())
            }),
        )?;
        let active = ActiveToast { toast, activated };
        self.notifier.Show(&active.toast)?;
        self.toasts.insert(token.to_owned(), active);
        Ok(())
    }
    fn show_test(&self, title: &str, body: &str) -> windows::core::Result<()> {
        // Recheck at delivery time without requesting access. This toast has no
        // call action handler and never replaces entries in self.toasts.
        if self.notifier.Setting()? != NotificationSetting::Enabled {
            return Err(windows::core::Error::from_hresult(
                windows::Win32::Foundation::E_ACCESSDENIED,
            ));
        }
        let document = XmlDocument::new()?;
        document.LoadXml(&HSTRING::from(windows_payload::test_toast_xml(title, body)))?;
        let toast = ToastNotification::CreateToastNotification(&document)?;
        toast.SetTag(&HSTRING::from("settings-test"))?;
        toast.SetGroup(&HSTRING::from(TEST_GROUP))?;
        toast.SetExpiresOnReboot(true)?;
        self.notifier.Show(&toast)
    }
    fn dismiss(&mut self, token: &str) -> windows::core::Result<()> {
        let hide = if let Some(active) = self.toasts.remove(token) {
            self.notifier.Hide(&active.toast)
        } else {
            Ok(())
        };
        // Hide only affects the banner. History removal also revokes delivered
        // Notification Center buttons, including those from an earlier process.
        let history = ToastNotificationManager::History()?.RemoveGroupedTagWithId(
            &HSTRING::from(token),
            &HSTRING::from(GROUP),
            &self.app_id,
        );
        hide.and(history)
    }
}

fn dispatch(sender: &Sender<CallNotificationAction>, arguments: &str) {
    if let Some(action) = windows_payload::parse_activation(arguments) {
        let _ = sender.send(action);
    }
}

#[implement(INotificationActivationCallback)]
struct ActivationCallback {
    actions: Sender<CallNotificationAction>,
    app_id: String,
}
impl INotificationActivationCallback_Impl for ActivationCallback_Impl {
    fn Activate(
        &self,
        app_id: &PCWSTR,
        invoked_args: &PCWSTR,
        _data: *const NOTIFICATION_USER_INPUT_DATA,
        _count: u32,
    ) -> windows::core::Result<()> {
        // SAFETY: COM supplies valid null-terminated strings for the duration of
        // this callback; no pointer or user input data escapes this invocation.
        unsafe {
            if !app_id.is_null()
                && !invoked_args.is_null()
                && app_id.to_string().ok().as_deref() == Some(&self.app_id)
                && let Ok(arguments) = invoked_args.to_string()
            {
                dispatch(&self.actions, &arguments);
            }
        }
        Ok(())
    }
}
#[implement(IClassFactory)]
struct ActivationFactory {
    actions: Sender<CallNotificationAction>,
    app_id: String,
}
impl IClassFactory_Impl for ActivationFactory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<IUnknown>,
        iid: *const GUID,
        result: *mut *mut c_void,
    ) -> windows::core::Result<()> {
        if result.is_null() || iid.is_null() {
            return Err(windows::core::Error::from_hresult(E_POINTER));
        }
        // SAFETY: COM guarantees result points to writable interface storage.
        unsafe {
            *result = std::ptr::null_mut();
        }
        if outer.is_some() {
            return Err(windows::core::Error::from_hresult(CLASS_E_NOAGGREGATION));
        }
        let callback: INotificationActivationCallback = ActivationCallback {
            actions: self.actions.clone(),
            app_id: self.app_id.clone(),
        }
        .into();
        // QueryInterface adds the caller's owned reference before the temporary
        // callback is released, including when a different interface is requested.
        unsafe { callback.query(iid, result).ok() }
    }
    fn LockServer(&self, _lock: BOOL) -> windows::core::Result<()> {
        Ok(())
    }
}

struct RegistryKey(HKEY);
impl Drop for RegistryKey {
    fn drop(&mut self) {
        let _ = unsafe { RegCloseKey(self.0) };
    }
}
fn set_registry_values(path: &str, values: &[(&str, &str)]) -> windows::core::Result<()> {
    let path = HSTRING::from(path);
    let mut key = HKEY::default();
    // SAFETY: strings remain alive, the output is valid, and HKCU requires no
    // elevation. The key guard closes the handle on every return path.
    unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(path.as_ptr()),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut key,
            None,
        )
        .ok()?;
    }
    let key = RegistryKey(key);
    for (name, value) in values {
        let name = HSTRING::from(*name);
        let data: Vec<u8> = value
            .encode_utf16()
            .chain(Some(0))
            .flat_map(u16::to_le_bytes)
            .collect();
        unsafe {
            RegSetValueExW(key.0, PCWSTR(name.as_ptr()), None, REG_SZ, Some(&data)).ok()?;
        }
    }
    Ok(())
}
fn register_installed_app(app_id: &str) -> windows::core::Result<()> {
    let executable = std::env::current_exe()
        .map_err(|_| windows::core::Error::from_hresult(windows::Win32::Foundation::E_FAIL))?;
    let executable = executable
        .to_str()
        .ok_or_else(|| windows::core::Error::from_hresult(windows::Win32::Foundation::E_FAIL))?;
    let command = format!("\"{executable}\" --notification-activation");
    set_registry_values(
        &format!("Software\\Classes\\CLSID\\{ACTIVATOR_TEXT}\\LocalServer32"),
        &[("", &command)],
    )?;
    set_registry_values(
        &format!("Software\\Classes\\AppUserModelId\\{app_id}"),
        &[("DisplayName", "Ruf"), ("CustomActivator", ACTIVATOR_TEXT)],
    )
}
