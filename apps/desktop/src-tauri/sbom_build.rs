//! Bind the inventory to the actual Cargo target/features before embedding it.

pub(super) fn verify() {
    println!("cargo:rerun-if-env-changed=RUF_SBOM_ENFORCE");
    println!("cargo:rerun-if-changed=../dist/software-bill.cdx.json");
    let required = std::env::var("PROFILE").as_deref() == Ok("release")
        || std::env::var("RUF_SBOM_ENFORCE").as_deref() == Ok("1");
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let json = std::fs::read(directory.join("../dist/software-bill.cdx.json")).unwrap_or_default();
    let bill = serde_json::from_slice::<serde_json::Value>(&json).unwrap_or_default();
    let target = std::env::var("TARGET").expect("Cargo target");
    let native = std::env::var_os("CARGO_FEATURE_NATIVE").is_some();
    let expected = [
        ("ruf:build:target", target.clone()),
        ("ruf:build:native", native.to_string()),
        (
            "ruf:build:profile",
            std::env::var("PROFILE").expect("Cargo profile"),
        ),
        ("ruf:build:kind", "desktop".to_owned()),
        ("ruf:frontend:resolution", "rendered-modules".to_owned()),
    ];
    let matches = expected.iter().all(|(name, value)| {
        bill["metadata"]["properties"]
            .as_array()
            .is_some_and(|properties| {
                properties
                    .iter()
                    .any(|property| property["name"] == *name && property["value"] == *value)
            })
    });
    assert!(
        !required || matches,
        "SBOM does not match this Cargo target/features/profile; use bun run desktop:build"
    );
    if required && native {
        let sdk_path = directory.join(format!("../../../native/install/{target}/sdk.json"));
        println!("cargo:rerun-if-changed={}", sdk_path.display());
        let sdk: serde_json::Value =
            serde_json::from_slice(&std::fs::read(sdk_path).expect("Native SDK required"))
                .expect("Valid SDK JSON");
        assert!(
            bill["components"]
                .as_array()
                .expect("SBOM components")
                .iter()
                .any(|component| component["name"] == "openssl"
                    && component["version"] == sdk["openssl"]["version"]),
            "Native SBOM must contain the actual SDK OpenSSL version"
        );
    }
    let destination =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"))
            .join("software-bill.cdx.json");
    // Plain cargo checks and dev-server sessions have no packaged frontend inventory.
    std::fs::write(destination, if matches { json.as_slice() } else { b"{}" })
        .expect("Write the compiled SBOM snapshot");
}
