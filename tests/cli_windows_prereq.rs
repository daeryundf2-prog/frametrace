use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn frametrace() -> &'static str {
    env!("CARGO_BIN_EXE_frametrace")
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after UNIX epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("frametrace-{name}-{}-{nanos}", std::process::id()))
}

fn run(args: &[&str]) -> Output {
    Command::new(frametrace())
        .args(args)
        .output()
        .expect("frametrace binary should run")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure_contains(output: &Output, expected: &str) {
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains(expected),
        "expected output to contain {expected:?}\nactual:\n{combined}"
    );
}

#[test]
fn workstation_status_reports_windows_prerequisite_gate() {
    let root = unique_temp_dir("cli-windows-prereq");
    let case_dir = root.join("case");

    assert_success(&run(&[
        "init-case",
        path(&case_dir),
        "--title",
        "Windows prereq smoke",
    ]));

    let output = run(&["workstation-status", path(&case_dir)]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let winui_project_present = repo_has_winui_project_files();
    let expected_ready = std::env::consts::OS == "windows"
        && winui_project_present
        && ["rustc", "cargo", "ffmpeg", "ffprobe", "dotnet"]
            .iter()
            .all(|command| command_available(command));

    assert!(stdout.contains("\"windows_prerequisites\":{"));
    assert!(stdout.contains(&format!("\"host_os\":\"{}\"", std::env::consts::OS)));
    assert!(stdout.contains(&format!(
        "\"release_validation_host_ready\":{}",
        expected_ready
    )));
    assert!(stdout.contains(&format!(
        "\"winui_project_present\":{}",
        winui_project_present
    )));
    assert!(stdout.contains("\"winui_project_files\":["));
    if std::env::consts::OS != "windows" {
        assert!(stdout.contains("unsupported-host"));
    }
    if !winui_project_present {
        assert!(stdout.contains("missing-winui-project"));
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn release_readiness_blocks_when_windows_prerequisites_are_missing() {
    let root = unique_temp_dir("cli-release-prereq");
    let case_dir = root.join("case");
    let media_dir = root.join("media");
    fs::create_dir_all(&media_dir).expect("media dir should be created");
    fs::write(media_dir.join("clip.mp4"), b"\0\0\0\x18ftypmp42payload")
        .expect("fixture video should be written");

    assert_success(&run(&[
        "init-case",
        path(&case_dir),
        "--title",
        "Release prereq smoke",
    ]));
    assert_success(&run(&[
        "scan-folder",
        path(&case_dir),
        path(&media_dir),
        "--no-ffprobe",
    ]));
    assert_success(&run(&["make-review", path(&case_dir)]));
    assert_success(&run(&["make-report", path(&case_dir)]));

    let corpus_manifest = root.join("corpus.tsv");
    write_corpus_manifest(&corpus_manifest, &media_dir);
    let review_manifest = root.join("release-review.txt");
    write_release_review_manifest(&review_manifest, full_release_review_manifest_keys());

    assert_failure_contains(
        &run(&[
            "qa",
            "release",
            path(&case_dir),
            "--corpus-manifest",
            path(&corpus_manifest),
            "--comparison-case",
            path(&case_dir),
            "--review-manifest",
            path(&review_manifest),
            "--performance-output-dir",
            path(&root.join("qa-release-performance")),
            "--performance-rows",
            "1000",
        ]),
        "windows_prerequisites",
    );

    let release_readiness =
        fs::read_to_string(case_dir.join("reports/qa/release-readiness.json")).unwrap();
    assert!(release_readiness.contains("\"name\":\"windows_prerequisites\""));
    assert!(release_readiness.contains("\"status\":\"FAIL\""));
    assert!(release_readiness.contains("missing-winui-build-receipt"));
    let release_decision = fs::read_to_string(case_dir.join("reports/qa/release-decision.json"))
        .expect("release decision JSON should be written");
    let decision = serde_json::from_str::<Value>(&release_decision)
        .expect("release decision JSON should parse");
    assert_eq!(decision["decision"], "NO_GO");
    let blockers = decision["blockers"]
        .as_array()
        .expect("release decision blockers should be an array");
    assert!(
        blockers
            .iter()
            .any(|blocker| blocker["name"] == "windows_prerequisites")
    );
    let names = blockers
        .iter()
        .map(|blocker| blocker["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    let unique = names.iter().collect::<std::collections::HashSet<_>>();
    assert_eq!(
        unique.len(),
        names.len(),
        "duplicate blocker names: {names:?}"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn windows_release_script_enforces_native_exit_and_winui_receipt() {
    let script = fs::read_to_string("scripts/windows/validate-release.ps1")
        .expect("Windows release script should be readable");

    assert!(script.contains("Require-Command dotnet"));
    assert!(script.contains("missing WinUI .sln or .csproj"));
    assert!(script.contains("missing WinUI test .csproj"));
    assert!(script.contains("dotnet_build = \"pass\""));
    assert!(script.contains("dotnet_test = \"pass\""));
    assert!(script.contains("$LASTEXITCODE"));
    assert!(script.contains("[string]$ReviewManifestPath"));
    assert!(script.contains("missing -ReviewManifestPath"));
    assert!(script.contains("\"BLOCKED\""));
    assert!(!script.contains("status = \"PASS\""));
    assert!(script.contains("'\"name\":\"windows_prerequisites\",\"status\":\"PASS\"'"));
    assert!(script.contains("'\"passed\": true'"));
}

#[test]
fn windows_release_script_deduplicates_release_decision_blocker_names() {
    let script = fs::read_to_string("scripts/windows/validate-release.ps1")
        .expect("Windows release script should be readable");
    let preflight_body = powershell_function_body(&script, "Get-ReleasePreflightBlockers");
    let write_body = powershell_function_body(&script, "Write-ReleaseDecision");

    assert!(
        preflight_body.contains("foreach ($CommandName in @(")
            && preflight_body.contains("name = \"windows_prerequisites\"")
            && preflight_body.contains("missing required command: $CommandName"),
        "preflight fixture should cover loop-generated windows_prerequisites blockers"
    );
    assert!(
        count_occurrences(&preflight_body, "name = \"winui_build_test\"") > 1,
        "preflight fixture should cover repeated winui_build_test blockers"
    );
    assert!(write_body.contains("Merge-ReleaseDecisionBlockers $Blockers"));
    assert!(write_body.contains("blocker_count = $ReleaseBlockers.Count"));
    assert!(write_body.contains("blockers = @($ReleaseBlockers)"));
    assert!(!write_body.contains("blocker_count = $Blockers.Count"));
    assert!(!write_body.contains("blockers = @($Blockers)"));

    let merge_body = powershell_function_body(&script, "Merge-ReleaseDecisionBlockers");
    assert!(merge_body.contains("evidence_details"));
    assert!(merge_body.contains("-join \"; \""));
}

fn write_corpus_manifest(corpus_manifest: &Path, media_dir: &Path) {
    let indexed_source = media_dir
        .join("clip.mp4")
        .canonicalize()
        .expect("fixture path should canonicalize");
    fs::write(
        corpus_manifest,
        format!("source_path\tsha256\n{}\t\n", indexed_source.display()),
    )
    .expect("corpus manifest should be written");
}

fn path(path: &Path) -> &str {
    path.to_str().expect("test paths should be UTF-8")
}

fn powershell_function_body(script: &str, function_name: &str) -> String {
    let marker = format!("function {function_name}");
    let function_start = script
        .find(&marker)
        .unwrap_or_else(|| panic!("{function_name} should be present"));
    let body_start = script[function_start..]
        .find('{')
        .map(|offset| function_start + offset)
        .unwrap_or_else(|| panic!("{function_name} should have a body"));
    let mut depth = 0_i32;
    for (offset, byte) in script[body_start..].bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    let body_end = body_start + offset + 1;
                    return script[body_start..body_end].to_owned();
                }
            }
            _ => {}
        }
    }
    panic!("{function_name} body should close");
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

fn repo_has_winui_project_files() -> bool {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("gui/winui");
    contains_project_file(&root)
}

fn contains_project_file(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path = entry.path();
        if path.is_dir() {
            contains_project_file(&path)
        } else {
            matches!(
                path.extension().and_then(|extension| extension.to_str()),
                Some("sln" | "csproj")
            )
        }
    })
}

fn command_available(command: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        command_candidates(&dir, command)
            .iter()
            .any(|path| path.is_file())
    })
}

fn command_candidates(dir: &Path, command: &str) -> Vec<PathBuf> {
    let base = dir.join(command);
    if cfg!(windows) {
        let mut candidates = vec![base.clone()];
        if let Some(exts) = std::env::var_os("PATHEXT") {
            candidates.extend(std::env::split_paths(&exts).map(|ext| {
                let suffix = ext.to_string_lossy();
                dir.join(format!("{command}{suffix}"))
            }));
        }
        candidates
    } else {
        vec![base]
    }
}

fn write_release_review_manifest(path: &Path, keys: &[&str]) {
    let artifact = path.with_file_name("release-review-artifact.json");
    fs::write(&artifact, "{}").expect("review artifact should be written");
    let artifact_name = artifact.file_name().unwrap().to_string_lossy();
    let gates = keys
        .iter()
        .map(|key| {
            format!(
                r#"{{
      "key": "{key}",
      "status": "PASS",
      "artifact_path": "{artifact_name}",
      "tool": "cli-windows-prereq-test",
      "evidence": "cli windows prerequisite review artifact",
      "timestamp": "2026-06-24T00:00:00Z",
      "reviewer": "qa",
      "cleanup_status": "clean"
    }}"#
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    fs::write(
        path,
        format!(
            r#"{{
  "schema_version": 1,
  "gates": [
{gates}
  ]
}}"#
        ),
    )
    .expect("release review manifest should be written");
}

fn full_release_review_manifest_keys() -> &'static [&'static str] {
    &[
        "technical_review",
        "security_review",
        "privacy_review",
        "supply_chain_review",
        "accuracy_validation",
        "reproducibility_validation",
        "performance_validation",
        "migration_validation",
        "operator_review",
        "report_defensibility_review",
        "legal_wording_review",
        "installer_package_validation",
        "windows_workstation_validation",
        "known_limitations_review",
        "release_notes_review",
        "support_triage_policy",
        "hotfix_policy",
        "incident_response_plan",
        "corpus_governance",
        "feature_intake_governance",
        "post_ga_monitoring",
        "external_review_readiness",
        "regression_schedule",
    ]
}
