#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Desktop executable entry point.

fn main() {
    sip_desktop_lib::run();
}
