#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Desktop executable entry point.

fn main() {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() == Some(std::ffi::OsStr::new("--export-sbom")) {
        let result = args
            .next()
            .ok_or("Expected an SBOM output path.")
            .and_then(|path| {
                if args.next().is_some() {
                    return Err("Unexpected export arguments.");
                }
                let json =
                    sip_desktop_lib::software_bill().ok_or("No packaged SBOM in this build.")?;
                std::fs::write(path, json).map_err(|_| "Could not write the SBOM.")
            });
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(2);
        }
        return;
    }
    sip_desktop_lib::run();
}
