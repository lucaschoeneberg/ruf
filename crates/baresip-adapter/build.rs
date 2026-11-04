//! Compile the private C bridge only for an explicitly requested native build.

#[cfg(feature = "native")]
fn main() {
    use std::{env, fs, path::PathBuf};
    let target = env::var("TARGET").expect("Cargo target");
    let prefix = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../native/install")
        .join(&target);
    println!("cargo:rerun-if-changed=native/bridge.c");
    println!("cargo:rerun-if-changed=native/audio_devices.c");
    println!("cargo:rerun-if-changed=native/audio_devices.h");
    println!("cargo:rerun-if-changed=native/network_roam.c");
    println!("cargo:rerun-if-changed=native/network_roam.h");
    println!("cargo:rerun-if-changed=../../native/sources.json");
    println!("cargo:rerun-if-changed=../../native/patches/re-sip-nat.patch");
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
        manifest["patches"]["reSipNat"].as_str(),
        Some(include_str!("../../native/patches/re-sip-nat.patch")),
        "Native SIP NAT patch changed; run bun run native:build"
    );
    if target.contains("apple") {
        assert!(
            manifest["modules"]
                .as_array()
                .is_some_and(|modules| modules.iter().any(|module| module == "coreaudio")),
            "Native SDK needs coreaudio; run bun run native:build"
        );
    }
    let mut bridge = cc::Build::new();
    bridge
        .file("native/bridge.c")
        .file("native/audio_devices.c")
        .file("native/network_roam.c")
        .include(prefix.join("include"))
        .include(prefix.join("include/re"))
        .std("c11");
    if target.contains("windows") {
        bridge
            .define("WIN32", None)
            .define("_CRT_SECURE_NO_WARNINGS", None);
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
fn main() {}
