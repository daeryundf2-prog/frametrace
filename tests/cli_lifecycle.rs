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
fn case_lifecycle_smoke_test_uses_real_binary() {
    let root = unique_temp_dir("cli-lifecycle");
    let case_dir = root.join("case");
    let media_dir = root.join("media");
    fs::create_dir_all(&media_dir).expect("media dir should be created");
    fs::write(media_dir.join("clip.mp4"), b"\0\0\0\x18ftypmp42payload")
        .expect("fixture video should be written");

    assert_success(&run(&[
        "init-case",
        path(&case_dir),
        "--title",
        "Smoke Case",
    ]));
    assert_success(&run(&[
        "scan-folder",
        path(&case_dir),
        path(&media_dir),
        "--no-ffprobe",
    ]));
    assert_success(&run(&["make-review", path(&case_dir)]));
    assert_success(&run(&["make-report", path(&case_dir)]));
    assert_workstation_contract(&case_dir);

    let corpus_manifest = root.join("corpus.tsv");
    write_corpus_manifest(&corpus_manifest, &media_dir);
    let incomplete_review_manifest = root.join("release-review-incomplete.txt");
    fs::write(
        &incomplete_review_manifest,
        "technical_review=pass\nsecurity_review=pass\nmigration_validation=pass\noperator_review=pass\nlegal_review=pass\n",
    )
    .expect("incomplete release review manifest should be written");

    assert_success(&run(&[
        "qa",
        "accuracy",
        path(&case_dir),
        path(&corpus_manifest),
    ]));
    assert_success(&run(&[
        "qa",
        "reproducibility",
        path(&case_dir),
        path(&case_dir),
    ]));
    assert_success(&run(&["qa", "report-defense", path(&case_dir)]));
    assert_success(&run(&[
        "qa",
        "performance",
        path(&root.join("qa-performance")),
        "--rows",
        "1000",
    ]));
    assert_release_blocks_without_manifest(&root, &case_dir, &corpus_manifest);
    assert_release_blocks_without_all_gates(
        &root,
        &case_dir,
        &corpus_manifest,
        &incomplete_review_manifest,
    );

    let review_manifest = root.join("release-review.txt");
    fs::write(&review_manifest, full_release_review_manifest())
        .expect("release review manifest should be written");
    assert_release_blocks_without_windows_prereqs(
        &root,
        &case_dir,
        &corpus_manifest,
        &review_manifest,
    );

    assert_success(&run(&["package-case", path(&case_dir)]));
    assert_success(&run(&["inspect", path(&case_dir)]));
    assert_lifecycle_outputs_exist(&root, &case_dir);

    let _ = fs::remove_dir_all(root);
}

fn assert_workstation_contract(case_dir: &Path) {
    let output = run(&["workstation-status", path(case_dir)]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"view\":\"workstation-status\""));
    assert!(stdout.contains("\"engine_source_of_truth\":true"));
    assert!(stdout.contains("\"gui_durable_state_allowed\":false"));
    assert!(stdout.contains("\"full_json_load_allowed\":false"));
    assert!(stdout.contains("\"confirm-playback\""));
    assert!(stdout.contains("\"ffprobe_and_playback_are_separate_states\":true"));
    assert!(stdout.contains("\"windows_prerequisites\":{"));
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

fn assert_release_blocks_without_manifest(root: &Path, case_dir: &Path, corpus_manifest: &Path) {
    assert_failure_contains(
        &run(&[
            "qa",
            "release",
            path(case_dir),
            "--corpus-manifest",
            path(corpus_manifest),
            "--comparison-case",
            path(case_dir),
            "--performance-output-dir",
            path(&root.join("qa-release-blocked-performance")),
            "--performance-rows",
            "1000",
        ]),
        "missing --review-manifest",
    );
}

fn assert_release_blocks_without_all_gates(
    root: &Path,
    case_dir: &Path,
    corpus_manifest: &Path,
    review_manifest: &Path,
) {
    assert_failure_contains(
        &run(&[
            "qa",
            "release",
            path(case_dir),
            "--corpus-manifest",
            path(corpus_manifest),
            "--comparison-case",
            path(case_dir),
            "--review-manifest",
            path(review_manifest),
            "--performance-output-dir",
            path(&root.join("qa-release-incomplete-gates-performance")),
            "--performance-rows",
            "1000",
        ]),
        "privacy_review",
    );
}

fn assert_release_blocks_without_windows_prereqs(
    root: &Path,
    case_dir: &Path,
    corpus_manifest: &Path,
    review_manifest: &Path,
) {
    assert_failure_contains(
        &run(&[
            "qa",
            "release",
            path(case_dir),
            "--corpus-manifest",
            path(corpus_manifest),
            "--comparison-case",
            path(case_dir),
            "--review-manifest",
            path(review_manifest),
            "--performance-output-dir",
            path(&root.join("qa-release-performance")),
            "--performance-rows",
            "1000",
        ]),
        "windows_prerequisites",
    );
}

fn assert_lifecycle_outputs_exist(root: &Path, case_dir: &Path) {
    assert!(case_dir.join("db/video_index.json").is_file());
    assert!(case_dir.join("review/index.html").is_file());
    assert!(case_dir.join("review/evidence-viewer.html").is_file());
    assert!(case_dir.join("reports/case-report.html").is_file());
    assert!(case_dir.join("reports/qa/accuracy-report.json").is_file());
    assert!(
        case_dir
            .join("reports/qa/reproducibility-report.json")
            .is_file()
    );
    assert!(
        case_dir
            .join("reports/qa/report-defense-checklist.md")
            .is_file()
    );
    assert!(
        root.join("qa-performance/performance-report.json")
            .is_file()
    );
    assert!(case_dir.join("reports/qa/release-readiness.json").is_file());
    let release_readiness =
        fs::read_to_string(case_dir.join("reports/qa/release-readiness.json")).unwrap();
    assert!(release_readiness.contains("\"name\":\"workstation_shell_contract\""));
    assert!(release_readiness.contains("\"name\":\"windows_prerequisites\""));
    assert!(
        case_dir
            .join("reports/qa/workstation-status.json")
            .is_file()
    );
    assert!(
        root.join("qa-release-performance/performance-report.json")
            .is_file()
    );
}

fn path(path: &Path) -> &str {
    path.to_str().expect("test paths should be UTF-8")
}

fn full_release_review_manifest() -> &'static str {
    "technical_review=pass\nsecurity_review=pass\nprivacy_review=pass\nsupply_chain_review=pass\naccuracy_validation=pass\nreproducibility_validation=pass\nperformance_validation=pass\nmigration_validation=pass\noperator_review=pass\nreport_defensibility_review=pass\nlegal_wording_review=pass\ninstaller_package_validation=pass\nwindows_workstation_validation=pass\nknown_limitations_review=pass\nrelease_notes_review=pass\nsupport_triage_policy=pass\nhotfix_policy=pass\nincident_response_plan=pass\ncorpus_governance=pass\nfeature_intake_governance=pass\npost_ga_monitoring=pass\nexternal_review_readiness=pass\nregression_schedule=pass\n"
}
