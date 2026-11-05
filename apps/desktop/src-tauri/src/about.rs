//! Fixed About-page links opened with the operating system's URL handler.

use phone_core::{AboutLink, ErrorCode, PhoneError};

fn url(link: AboutLink) -> &'static str {
    match link {
        AboutLink::Company => "https://www.mardu.de/",
        AboutLink::Imprint => "https://www.mardu.de/publisher",
        AboutLink::Privacy => "https://www.mardu.de/privacy",
    }
}

/// Open one of three fixed HTTPS company pages in the default browser.
/// Unknown enum values are rejected by IPC deserialization before dispatch.
/// Returns NativeFailure if the operating system cannot open the browser.
#[tauri::command]
pub(crate) async fn open_about_link(link: AboutLink) -> Result<(), PhoneError> {
    tauri::async_runtime::spawn_blocking(move || open_browser(url(link)))
        .await
        .map_err(|_| browser_error())?
}

fn browser_error() -> PhoneError {
    PhoneError::new(
        ErrorCode::NativeFailure,
        "The company page could not be opened in the default browser.",
    )
}

fn open_browser(url: &str) -> Result<(), PhoneError> {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("/usr/bin/open");
    #[cfg(target_os = "windows")]
    let mut command = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut command = std::process::Command::new("rundll32.exe");
        command.arg("url.dll,FileProtocolHandler");
        command.creation_flags(CREATE_NO_WINDOW);
        command
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = std::process::Command::new("xdg-open");
    if command
        .arg(url)
        .status()
        .is_ok_and(|status| status.success())
    {
        Ok(())
    } else {
        Err(browser_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destinations_are_fixed_company_https_pages() {
        assert_eq!(url(AboutLink::Company), "https://www.mardu.de/");
        assert_eq!(url(AboutLink::Imprint), "https://www.mardu.de/publisher");
        assert_eq!(url(AboutLink::Privacy), "https://www.mardu.de/privacy");
    }

    #[test]
    fn ipc_cannot_select_arbitrary_urls_or_protocols() {
        assert_eq!(
            serde_json::from_str::<AboutLink>("\"imprint\"").unwrap(),
            AboutLink::Imprint
        );
        for value in [
            "https://example.com",
            "file:///tmp/test",
            "javascript:alert(1)",
            "",
        ] {
            assert!(serde_json::from_value::<AboutLink>(serde_json::json!(value)).is_err());
        }
    }
}

/// Read the exact compiled CycloneDX JSON, without reading accounts or starting SIP.
#[tauri::command]
pub(crate) fn get_software_bill() -> Result<String, PhoneError> {
    crate::software_bill().map(str::to_owned).ok_or_else(|| {
        PhoneError::new(
            ErrorCode::EngineUnavailable,
            "No packaged software inventory is available in this development build.",
        )
    })
}
