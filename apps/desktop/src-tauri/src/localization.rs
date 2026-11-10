//! Process-local language state, resolved before menus without loading SIP or credentials.
use crate::store;
use phone_core::{AppLocale, AppPreferences, LanguagePreference, LocalizationState, PhoneError};
use std::{borrow::Cow, sync::Mutex};
use tauri::{Emitter, Manager};

#[derive(Clone, Copy)]
pub(crate) enum NativeMessage {
    Loading,
    Dialer,
    Contacts,
    History,
    Number,
    Find,
    Search,
    QuitSettingsFailure,
    Quit,
    QuitActive,
    QuitActiveConfirm,
    Back,
    OutgoingAccount,
    Connected,
    Disconnected,
    Call,
    Answer,
    Reject,
    Hangup,
    Unmute,
    Mute,
    Resume,
    Hold,
    Transfer,
    Tones,
    IncomingStatus,
    AnsweringStatus,
    ActiveStatus,
    HeldStatus,
    EarlyMediaStatus,
    ConnectingStatus,
    ErrorStatus,
    NoAccountStatus,
    AccountsStatus,
    Open,
    Accounts,
    Audio,
    Settings,
    File,
    About,
    Edit,
    View,
    Window,
    OpenPhone,
    Help,
    Shortcuts,
    GettingStarted,
    Diagnostics,
    SettingsTitle,
    #[cfg(target_os = "macos")]
    Services,
    #[cfg(target_os = "macos")]
    Hide,
    #[cfg(target_os = "macos")]
    HideOthers,
    #[cfg(target_os = "macos")]
    ShowAll,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Minimize,
    CloseWindow,
    Incoming,
    IncomingBody,
    CallerTitle,
    UnknownCaller,
    TestTitle,
    TestBody,
}
impl NativeMessage {
    fn key(self) -> &'static str {
        match self {
            Self::Loading => "native.loading",
            Self::Dialer => "native.dialer",
            Self::Contacts => "native.contacts",
            Self::History => "native.history",
            Self::Number => "native.number",
            Self::Find => "native.find",
            Self::Search => "native.search",
            Self::QuitSettingsFailure => "native.quitSettingsFailure",
            Self::Quit => "native.quit",
            Self::QuitActive => "native.quitActive",
            Self::QuitActiveConfirm => "native.quitActiveConfirm",
            Self::Back => "native.back",
            Self::OutgoingAccount => "native.outgoingAccount",
            Self::Connected => "native.connected",
            Self::Disconnected => "native.disconnected",
            Self::Call => "native.call",
            Self::Answer => "native.answer",
            Self::Reject => "native.reject",
            Self::Hangup => "native.hangup",
            Self::Unmute => "native.unmute",
            Self::Mute => "native.mute",
            Self::Resume => "native.resume",
            Self::Hold => "native.hold",
            Self::Transfer => "native.transfer",
            Self::Tones => "native.tones",
            Self::IncomingStatus => "native.incomingStatus",
            Self::AnsweringStatus => "native.answeringStatus",
            Self::ActiveStatus => "native.activeStatus",
            Self::HeldStatus => "native.heldStatus",
            Self::EarlyMediaStatus => "native.earlyMediaStatus",
            Self::ConnectingStatus => "native.connectingStatus",
            Self::ErrorStatus => "native.errorStatus",
            Self::NoAccountStatus => "native.noAccountStatus",
            Self::AccountsStatus => "native.accountsStatus",
            Self::Open => "native.open",
            Self::Accounts => "native.accounts",
            Self::Audio => "native.audio",
            Self::Settings => "native.settings",
            Self::File => "native.file",
            Self::About => "native.about",
            Self::Edit => "native.edit",
            Self::View => "native.view",
            Self::Window => "native.window",
            Self::OpenPhone => "native.openPhone",
            Self::Help => "native.help",
            Self::Shortcuts => "native.shortcuts",
            Self::GettingStarted => "native.gettingStarted",
            Self::Diagnostics => "native.diagnostics",
            Self::SettingsTitle => "native.settingsTitle",
            #[cfg(target_os = "macos")]
            Self::Services => "native.services",
            #[cfg(target_os = "macos")]
            Self::Hide => "native.hide",
            #[cfg(target_os = "macos")]
            Self::HideOthers => "native.hideOthers",
            #[cfg(target_os = "macos")]
            Self::ShowAll => "native.showAll",
            Self::Undo => "native.undo",
            Self::Redo => "native.redo",
            Self::Cut => "native.cut",
            Self::Copy => "native.copy",
            Self::Paste => "native.paste",
            Self::SelectAll => "native.selectAll",
            Self::Minimize => "native.minimize",
            Self::CloseWindow => "native.closeWindow",
            Self::Incoming => "native.incoming",
            Self::IncomingBody => "native.incomingBody",
            Self::CallerTitle => "native.callerTitle",
            Self::UnknownCaller => "native.unknownCaller",
            Self::TestTitle => "native.testTitle",
            Self::TestBody => "native.testBody",
        }
    }
}

pub(crate) struct Localization(Mutex<LocalizationState>);

fn resolve(
    preference: LanguagePreference,
    preferred: impl IntoIterator<Item = impl AsRef<str>>,
) -> AppLocale {
    match preference {
        LanguagePreference::De => AppLocale::De,
        LanguagePreference::En => AppLocale::En,
        LanguagePreference::System => preferred
            .into_iter()
            .find_map(|language| {
                match language
                    .as_ref()
                    .split(['-', '_', '.', '@'])
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "de" => Some(AppLocale::De),
                    "en" => Some(AppLocale::En),
                    _ => None,
                }
            })
            .unwrap_or(AppLocale::En),
    }
}

fn initial_state(
    store: Result<store::AccountStore, PhoneError>,
    preferred: impl IntoIterator<Item = impl AsRef<str>>,
) -> LocalizationState {
    // Read password-free metadata only. Corruption must remain observable by
    // get_preferences and must never cause a fallback value to be written.
    let preference = store
        .and_then(|store| store.load())
        .map(|settings| settings.preferences.language)
        .unwrap_or_default();
    LocalizationState {
        preference,
        locale: resolve(preference, preferred),
        revision: 0,
    }
}

pub(crate) fn initialize(app: &tauri::AppHandle) {
    if app.try_state::<Localization>().is_some() {
        return;
    }
    let store = store::data_directory(app).map(|directory| store::AccountStore {
        path: directory.join("accounts.json"),
    });
    app.manage(Localization(Mutex::new(initial_state(
        store,
        sys_locale::get_locales(),
    ))));
}

/// Read the same language/revision used by native menus; no telephony or credential access.
#[tauri::command]
pub(crate) fn get_localization(app: tauri::AppHandle) -> LocalizationState {
    state(&app)
}

pub(crate) fn state(app: &tauri::AppHandle) -> LocalizationState {
    app.state::<Localization>()
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

fn apply(
    state: &mut LocalizationState,
    preference: LanguagePreference,
    preferred: impl IntoIterator<Item = impl AsRef<str>>,
) -> bool {
    if state.preference == preference {
        return false;
    }
    state.preference = preference;
    state.locale = resolve(preference, preferred);
    state.revision = state.revision.saturating_add(1);
    true
}

fn persisted(
    state: &mut LocalizationState,
    result: Result<AppPreferences, PhoneError>,
    preferred: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<(AppPreferences, Option<LocalizationState>), PhoneError> {
    let preferences = result?;
    let changed = apply(state, preferences.language, preferred).then(|| state.clone());
    Ok((preferences, changed))
}

/// Apply only after the complete preference transaction has persisted successfully.
pub(crate) fn saved(
    app: &tauri::AppHandle,
    result: Result<AppPreferences, PhoneError>,
) -> Result<AppPreferences, PhoneError> {
    let localization = app.state::<Localization>();
    let (preferences, changed) = {
        let mut state = localization
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        persisted(&mut state, result, sys_locale::get_locales())?
    };
    if let Some(state) = changed {
        crate::shell::refresh_localization(app);
        let _ = app.emit_to("main", "localization-changed", &state);
        let _ = app.emit_to("settings", "localization-changed", &state);
    }
    Ok(preferences)
}

pub(crate) fn translate(locale: AppLocale, key: NativeMessage) -> &'static str {
    let locale = match locale {
        AppLocale::De => "de",
        AppLocale::En => "en",
    };
    // The macro-generated backend contains borrowed, compile-time catalog data.
    // Explicit locales keep the managed application state authoritative.
    match crate::_rust_i18n_backend()
        .translate(locale, key.key())
        .expect("validated native message key")
    {
        Cow::Borrowed(message) => message,
        Cow::Owned(_) => unreachable!("compiled catalogs contain static messages"),
    }
}
pub(crate) fn text(app: &tauri::AppHandle, key: NativeMessage) -> &'static str {
    translate(state(app).locale, key)
}
pub(crate) fn format(
    app: &tauri::AppHandle,
    key: NativeMessage,
    params: &[(&str, String)],
) -> String {
    render(state(app).locale, key, params)
}
pub(crate) fn render(locale: AppLocale, key: NativeMessage, params: &[(&str, String)]) -> String {
    let (names, values): (Vec<_>, Vec<_>) = params.iter().cloned().unzip();
    rust_i18n::replace_patterns(translate(locale, key), &names, &values)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_interpolation_uses_the_selected_locale_and_preserves_raw_values() {
        for (locale, expected) in [
            (AppLocale::De, "Ruf · %{total}/9 Konten verbunden"),
            (AppLocale::En, "Ruf · %{total}/9 accounts connected"),
        ] {
            assert_eq!(
                render(
                    locale,
                    NativeMessage::AccountsStatus,
                    &[("ready", "%{total}".into()), ("total", "9".into())],
                ),
                expected
            );
        }
        assert_eq!(
            render(
                AppLocale::En,
                NativeMessage::CallerTitle,
                &[("name", "<Ada> & %{name}".into())],
            ),
            "<Ada> & %{name} is calling"
        );
    }

    #[test]
    fn native_interpolation_leaves_missing_variables_visible() {
        assert_eq!(
            render(
                AppLocale::En,
                NativeMessage::AccountsStatus,
                &[("ready", "2".into())],
            ),
            "Ruf · 2/%{total} accounts connected"
        );
    }

    #[test]
    fn startup_fallback_keeps_corrupt_metadata_and_its_preferences_error() {
        let directory =
            std::env::temp_dir().join(format!("ruf-locale-corruption-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("accounts.json");
        let original = b"{corrupt-password-free-metadata";
        std::fs::write(&path, original).unwrap();
        let state = initial_state(Ok(store::AccountStore { path: path.clone() }), ["de-DE"]);
        assert_eq!(state.preference, LanguagePreference::System);
        assert_eq!(state.locale, AppLocale::De);
        assert_eq!(state.revision, 0);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(
            store::AccountStore { path }.load().err().unwrap().code,
            phone_core::ErrorCode::StorageFailure
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn failed_saves_never_switch_language_or_increment_revision() {
        let mut state = LocalizationState {
            preference: LanguagePreference::De,
            locale: AppLocale::De,
            revision: 7,
        };
        let before = state.clone();
        for code in [
            phone_core::ErrorCode::StorageFailure,
            phone_core::ErrorCode::InvalidInput,
            phone_core::ErrorCode::Busy,
        ] {
            assert!(
                persisted(
                    &mut state,
                    Err(PhoneError::new(code, "Synthetic failure")),
                    ["en-US"]
                )
                .is_err()
            );
            assert_eq!(state, before);
        }
        let preferences = AppPreferences {
            language: LanguagePreference::En,
            ..Default::default()
        };
        let (_, changed) = persisted(&mut state, Ok(preferences), ["de-DE"]).unwrap();
        assert_eq!(changed.unwrap(), state);
        assert_eq!(state.revision, 8);
        assert_eq!(state.locale, AppLocale::En);
    }

    #[test]
    fn every_native_message_is_present_in_both_shared_catalogs() {
        for locale in [AppLocale::De, AppLocale::En] {
            for key in [
                NativeMessage::Loading,
                NativeMessage::Dialer,
                NativeMessage::Contacts,
                NativeMessage::History,
                NativeMessage::Number,
                NativeMessage::Find,
                NativeMessage::Search,
                NativeMessage::QuitSettingsFailure,
                NativeMessage::Quit,
                NativeMessage::QuitActive,
                NativeMessage::QuitActiveConfirm,
                NativeMessage::Back,
                NativeMessage::OutgoingAccount,
                NativeMessage::Connected,
                NativeMessage::Disconnected,
                NativeMessage::Call,
                NativeMessage::Answer,
                NativeMessage::Reject,
                NativeMessage::Hangup,
                NativeMessage::Unmute,
                NativeMessage::Mute,
                NativeMessage::Resume,
                NativeMessage::Hold,
                NativeMessage::Transfer,
                NativeMessage::Tones,
                NativeMessage::IncomingStatus,
                NativeMessage::AnsweringStatus,
                NativeMessage::ActiveStatus,
                NativeMessage::HeldStatus,
                NativeMessage::EarlyMediaStatus,
                NativeMessage::ConnectingStatus,
                NativeMessage::ErrorStatus,
                NativeMessage::NoAccountStatus,
                NativeMessage::AccountsStatus,
                NativeMessage::Open,
                NativeMessage::Accounts,
                NativeMessage::Audio,
                NativeMessage::Settings,
                NativeMessage::File,
                NativeMessage::About,
                NativeMessage::Edit,
                NativeMessage::View,
                NativeMessage::Window,
                NativeMessage::OpenPhone,
                NativeMessage::Help,
                NativeMessage::Shortcuts,
                NativeMessage::GettingStarted,
                NativeMessage::Diagnostics,
                NativeMessage::SettingsTitle,
                #[cfg(target_os = "macos")]
                NativeMessage::Services,
                #[cfg(target_os = "macos")]
                NativeMessage::Hide,
                #[cfg(target_os = "macos")]
                NativeMessage::HideOthers,
                #[cfg(target_os = "macos")]
                NativeMessage::ShowAll,
                NativeMessage::Undo,
                NativeMessage::Redo,
                NativeMessage::Cut,
                NativeMessage::Copy,
                NativeMessage::Paste,
                NativeMessage::SelectAll,
                NativeMessage::Minimize,
                NativeMessage::CloseWindow,
                NativeMessage::Incoming,
                NativeMessage::IncomingBody,
                NativeMessage::CallerTitle,
                NativeMessage::UnknownCaller,
                NativeMessage::TestTitle,
                NativeMessage::TestBody,
            ] {
                assert!(!translate(locale, key).is_empty());
            }
        }
    }

    #[test]
    fn native_language_resources_and_system_selected_installer_languages_are_bundled() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(
            config["bundle"]["windows"]["nsis"]["languages"],
            serde_json::json!(["English", "German"])
        );
        assert_eq!(
            config["bundle"]["windows"]["nsis"]["displayLanguageSelector"],
            false
        );
        for (locale, source) in [
            (
                "de",
                include_str!("../localizations/de.lproj/InfoPlist.strings"),
            ),
            (
                "en",
                include_str!("../localizations/en.lproj/InfoPlist.strings"),
            ),
        ] {
            assert!(source.contains("NSContactsUsageDescription"));
            assert!(source.contains("NSMicrophoneUsageDescription"));
            assert_eq!(
                config["bundle"]["macOS"]["files"]
                    [format!("Resources/{locale}.lproj/InfoPlist.strings")],
                format!("localizations/{locale}.lproj/InfoPlist.strings")
            );
        }
    }

    #[test]
    fn preferred_languages_are_ordered_base_matched_and_fall_back_to_english() {
        for (languages, expected) in [
            (vec!["fr-FR", "de-CH", "en-US"], AppLocale::De),
            (vec!["en-GB", "de-DE"], AppLocale::En),
            (vec!["DE_at.UTF-8"], AppLocale::De),
            (vec!["fr-FR"], AppLocale::En),
            (vec![], AppLocale::En),
        ] {
            assert_eq!(resolve(LanguagePreference::System, languages), expected);
        }
        assert_eq!(resolve(LanguagePreference::De, ["en-US"]), AppLocale::De);
        assert_eq!(resolve(LanguagePreference::En, ["de-DE"]), AppLocale::En);
    }
    #[test]
    fn revisions_change_only_with_saved_choices_and_system_is_resolved_on_selection() {
        let mut state = LocalizationState {
            preference: LanguagePreference::System,
            locale: AppLocale::De,
            revision: 0,
        };
        assert!(!apply(&mut state, LanguagePreference::System, ["en-US"]));
        assert_eq!(state.locale, AppLocale::De);
        assert!(apply(&mut state, LanguagePreference::En, ["de-DE"]));
        assert_eq!(state.revision, 1);
        assert!(apply(&mut state, LanguagePreference::System, ["de-DE"]));
        assert_eq!(state.locale, AppLocale::De);
        assert_eq!(state.revision, 2);
    }
}
