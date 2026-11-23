//! Compile the Embassy API in a separate crate with no local features.

#![cfg(feature = "embassy")]

use std::{env, fs, path::PathBuf, process::Command};

struct TempCrate(PathBuf);

impl Drop for TempCrate {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn embassy_dependency_features_need_no_caller_features() {
    let dir =
        TempCrate(env::temp_dir().join(format!("hsmc-downstream-features-{}", std::process::id())));
    fs::create_dir_all(dir.0.join("src")).unwrap();
    let mut features = vec!["embassy"];
    for (feature, enabled) in [
        ("journal", cfg!(feature = "journal")),
        ("trace-defmt", cfg!(feature = "trace-defmt")),
        ("trace-log", cfg!(feature = "trace-log")),
        ("trace-tracing", cfg!(feature = "trace-tracing")),
    ] {
        if enabled {
            features.push(feature);
        }
    }
    fs::write(
        dir.0.join("Cargo.toml"),
        format!(
            r#"[package]
name = "hsmc-downstream-features"
version = "0.0.0"
edition = "2021"

[workspace]

[dependencies]
hsmc = {{ path = {:?}, features = {:?} }}
embassy-sync = "0.8"
embassy-time = "0.5"
embassy-futures = "0.1"
{}
"#,
            env!("CARGO_MANIFEST_DIR"),
            features,
            if cfg!(feature = "trace-defmt") {
                "defmt = \"1.2.0\""
            } else {
                ""
            },
        ),
    )
    .unwrap();
    // Reuse the versions Cargo already downloaded for this test run.
    fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../Cargo.lock"),
        dir.0.join("Cargo.lock"),
    )
    .unwrap();
    let mut source = include_str!("../examples/embassy_full.rs").to_owned();
    if cfg!(feature = "trace-defmt") {
        // Match firmware that imports defmt's assert!/panic! macros globally.
        source = source.replacen(
            "use embassy_sync::",
            "#[macro_use]\nextern crate defmt;\n\nuse embassy_sync::",
            1,
        );
    }
    if cfg!(feature = "journal") {
        source.push_str(
            r#"

pub async fn check_journal() {
    let mut machine = Radio::new(RadioData { packets_seen: 0 }, &RADIO_CHAN);
    let _ = machine.step(Duration::ZERO).await;
    assert!(matches!(machine.journal()[0], hsmc::TraceEvent::Started { .. }));
    let _ = machine.take_journal();
    machine.clear_journal();
}
"#,
        );
    }
    fs::write(dir.0.join("src/lib.rs"), source).unwrap();

    // Running from the temporary crate avoids inheriting a firmware's
    // .cargo/config.toml target. Only type-checking is needed: no time
    // driver, executor or critical-section implementation is linked.
    let output = Command::new(env!("CARGO"))
        .current_dir(&dir.0)
        .env_remove("CARGO_BUILD_TARGET")
        .args(["check", "--lib", "--offline"])
        .env("RUSTFLAGS", "-Dunexpected_cfgs")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "downstream crate failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
