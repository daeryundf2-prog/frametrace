use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn resolved_external_tool_cannot_be_forged_by_downstream_crates() {
    let root = unique_temp_dir("tool-policy-api-forge");
    let src_dir = root.join("src");
    fs::create_dir_all(&src_dir).expect("temporary crate src directory should be created");
    fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"[package]
name = "tool_policy_api_forge_attempt"
version = "0.0.0"
edition = "2024"

[dependencies]
frametrace = {{ path = "{}" }}
"#,
            env!("CARGO_MANIFEST_DIR")
        ),
    )
    .expect("temporary Cargo.toml should be written");
    fs::write(
        src_dir.join("main.rs"),
        r#"use frametrace::tool_policy::ResolvedExternalTool;

fn main() {
    let _forged = ResolvedExternalTool {
        name: "ffmpeg".to_string(),
        path: "/tmp/ffmpeg".to_string(),
        version: "ffmpeg fake 1.0".to_string(),
    };
}
"#,
    )
    .expect("temporary forge attempt should be written");

    let output = Command::new("cargo")
        .args([
            "check",
            "--quiet",
            "--manifest-path",
            path(&root.join("Cargo.toml")),
        ])
        .output()
        .expect("cargo check should run for temporary crate");

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = fs::remove_dir_all(root);
    assert!(
        !output.status.success(),
        "forged ResolvedExternalTool unexpectedly compiled\n{combined}"
    );
    assert!(
        combined.contains("private field") || combined.contains("cannot construct"),
        "expected compile error to prove fields are not constructible\n{combined}"
    );
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after UNIX epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("frametrace-{name}-{}-{nanos}", std::process::id()))
}

fn path(path: &Path) -> &str {
    path.to_str().expect("test path should be utf-8")
}
