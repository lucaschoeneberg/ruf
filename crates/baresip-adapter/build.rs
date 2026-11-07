//! Compile platform contacts independently of the opt-in native SIP engine.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rerun-if-changed=native/contacts.m");
        println!("cargo:rerun-if-changed=native/contacts.h");
        cc::Build::new()
            .env("MACOSX_DEPLOYMENT_TARGET", "14.0")
            .file("native/contacts.m")
            .flag("-fobjc-arc")
            .flag("-fblocks")
            .compile("sip_contacts");
        println!("cargo:rerun-if-changed=native/microphone_permission.m");
        cc::Build::new()
            .env("MACOSX_DEPLOYMENT_TARGET", "14.0")
            .file("native/microphone_permission.m")
            .flag("-fobjc-arc")
            .flag("-fblocks")
            .compile("sip_microphone_permission");
        println!("cargo:rustc-link-lib=framework=AVFoundation");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=Contacts");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rerun-if-changed=native/call_notifications.m");
        println!("cargo:rerun-if-changed=native/call_notifications.h");
        cc::Build::new()
            .env("MACOSX_DEPLOYMENT_TARGET", "14.0")
            .file("native/call_notifications.m")
            .flag("-fobjc-arc")
            .flag("-fblocks")
            .compile("sip_call_notifications");
        println!("cargo:rustc-link-lib=framework=UserNotifications");
        println!("cargo:rerun-if-changed=native/login_item.m");
        println!("cargo:rerun-if-changed=native/login_item.h");
        cc::Build::new()
            .env("MACOSX_DEPLOYMENT_TARGET", "14.0")
            .file("native/login_item.m")
            .flag("-fobjc-arc")
            .compile("sip_login_item");
        println!("cargo:rustc-link-lib=framework=ServiceManagement");
    }
    build_telephony();
}

#[cfg(feature = "native")]
fn build_telephony() {
    use std::{env, fs, path::PathBuf};
    let target = env::var("TARGET").expect("Cargo target");
    let prefix = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../native/install")
        .join(&target);
    println!("cargo:rerun-if-changed=native/bridge.c");
    println!("cargo:rerun-if-changed=native/call_controls.h");
    println!("cargo:rerun-if-changed=native/audio_devices.c");
    println!("cargo:rerun-if-changed=native/audio_diagnostics.c");
    println!("cargo:rerun-if-changed=native/audio_diagnostics.h");
    println!("cargo:rerun-if-changed=native/audio_devices.h");
    println!("cargo:rerun-if-changed=native/network_roam.c");
    println!("cargo:rerun-if-changed=native/network_roam.h");
    println!("cargo:rerun-if-changed=../../native/sources.json");
    println!("cargo:rerun-if-changed=../../native/patches/re-sip-nat.patch");
    println!("cargo:rerun-if-changed=../../native/patches/baresip-refer-policy.patch");
    println!(
        "cargo:rerun-if-changed={}",
        prefix.join("sdk.json").display()
    );
    let manifest = fs::read_to_string(prefix.join("sdk.json"))
        .expect("Native SDK missing. Run `bun run native:build` on this platform first.");
    let manifest: serde_json::Value = serde_json::from_str(&manifest).expect("Valid SDK manifest");
    let pins: serde_json::Value =
        serde_json::from_str(include_str!("../../native/sources.json")).expect("Valid source pins");
    assert_eq!(
        manifest["sources"], pins,
        "SDK source pins changed; rebuild the native SDK"
    );
    assert_eq!(
        manifest["target"].as_str(),
        Some(target.as_str()),
        "SDK architecture mismatch"
    );
    assert_eq!(
        manifest["patches"]["baresipReferPolicy"].as_str(),
        Some(include_str!(
            "../../native/patches/baresip-refer-policy.patch"
        )),
        "Native REFER policy changed; run bun run native:build"
    );
    assert_eq!(
        manifest["patches"]["reSipNat"].as_str(),
        Some(include_str!("../../native/patches/re-sip-nat.patch")),
        "Native SIP NAT patch changed; run bun run native:build"
    );
    for (name, file) in [
        ("reSecurity", "re-security.patch"),
        ("baresipSecurity", "baresip-security.patch"),
        ("reCallControls", "re-call-controls.patch"),
        ("baresipCallControls", "baresip-call-controls.patch"),
        ("reReliability", "re-reliability.patch"),
        ("baresipReliability", "baresip-reliability.patch"),
    ] {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../native/patches")
            .join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        assert_eq!(
            manifest["patches"][name].as_str(),
            Some(fs::read_to_string(path).expect("control patch").as_str()),
            "Native control hooks changed; rebuild SDK"
        );
    }
    if target.contains("apple") {
        assert!(
            manifest["modules"]
                .as_array()
                .is_some_and(|modules| modules.iter().any(|module| module == "coreaudio")),
            "Native SDK needs coreaudio; run bun run native:build"
        );
    }
    assert!(
        manifest["modules"]
            .as_array()
            .is_some_and(|modules| modules.iter().any(|module| module == "srtp")),
        "SDK requires SRTP; rebuild native SDK"
    );
    assert!(
        prefix.join("include/openssl/ssl.h").exists(),
        "SDK requires matching OpenSSL headers"
    );
    println!("cargo:rerun-if-changed=native/security.c");
    println!("cargo:rerun-if-changed=native/security.h");
    let mut bridge = cc::Build::new();
    bridge
        .file("native/bridge.c")
        .file("native/security.c")
        .file("native/audio_devices.c")
        .file("native/audio_diagnostics.c")
        .file("native/network_roam.c")
        .include(prefix.join("include"))
        .include(prefix.join("include/re"))
        .std("c11");
    if target.contains("windows") {
        bridge
            .define("WIN32", None)
            .define("_CRT_SECURE_NO_WARNINGS", None);
    }
    if std::env::var_os("CARGO_FEATURE_NATIVE_TEST_HOOKS").is_some() {
        bridge.define("SIP_NATIVE_TEST_HOOKS", None);
    }
    bridge.compile("sip_bridge");
    println!(
        "cargo:rustc-link-search=native={}",
        prefix.join("lib").display()
    );
    for library in ["baresip", "re", "ssl", "crypto"] {
        println!("cargo:rustc-link-lib=static={library}");
    }
    if target.contains("apple") {
        for framework in [
            "CoreFoundation",
            "SystemConfiguration",
            "CoreAudio",
            "AudioToolbox",
            "Security",
        ] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
        println!("cargo:rustc-link-lib=resolv");
    } else if target.contains("windows") {
        for library in [
            "qwave", "iphlpapi", "wsock32", "ws2_32", "dbghelp", "winmm", "gdi32", "crypt32",
            "strmiids", "ole32", "oleaut32", "bcrypt", "ncrypt", "advapi32", "user32", "uuid",
            "avrt",
        ] {
            println!("cargo:rustc-link-lib={library}");
        }
    } else {
        for library in ["m", "dl", "pthread", "resolv"] {
            println!("cargo:rustc-link-lib={library}");
        }
    }
}

#[cfg(not(feature = "native"))]
fn build_telephony() {}
