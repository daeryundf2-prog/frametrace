#![cfg(unix)]

use std::fs;
use std::os::unix::fs::symlink;
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

fn run_with_path(args: &[&str], path_env: &str) -> Output {
    Command::new(frametrace())
        .args(args)
        .env("PATH", path_env)
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
fn inspect_e01_rejects_symlinked_logs_directory_without_writing_outside() {
    let fixture = E01Fixture::new("cli-inspect-e01-logs-dir-symlink");
    fixture.init();
    symlink(&fixture.source_dir, fixture.case_dir.join("evidence/logs"))
        .expect("symlink should be created");

    let output = fixture.inspect_e01();

    assert_failure_contains(&output, "inside the case directory");
    assert!(!fixture.source_dir.join("e01-audit.jsonl").exists());
    assert_no_matching_outside_file(&fixture.source_dir, "e01-info-");
    let _ = fs::remove_dir_all(fixture.root);
}

#[test]
fn import_e01_rejects_symlinked_logs_directory_without_writing_outside() {
    let fixture = E01Fixture::new("cli-import-e01-logs-dir-symlink");
    fixture.init();
    symlink(&fixture.source_dir, fixture.case_dir.join("evidence/logs"))
        .expect("symlink should be created");

    let output = fixture.import_e01();

    assert_failure_contains(&output, "inside the case directory");
    assert!(!fixture.source_dir.join("e01-audit.jsonl").exists());
    assert_no_matching_outside_file(&fixture.source_dir, "e01-info-");
    assert_no_matching_outside_file(&fixture.source_dir, "e01-verify-");
    assert_no_matching_outside_file(&fixture.source_dir, "e01-export-");
    let _ = fs::remove_dir_all(fixture.root);
}

#[test]
fn validate_artifact_rejects_symlinked_logs_directory_without_writing_outside() {
    let root = unique_temp_dir("cli-validate-logs-dir-symlink");
    let case_dir = root.join("case");
    let media_dir = root.join("media");
    let outside = root.join("outside-validation-logs");
    let source_file = media_dir.join("clip.mp4");
    fs::create_dir_all(&media_dir).expect("media dir should exist");
    fs::write(&source_file, b"\0\0\0\x18ftypmp42payload").expect("fixture video should be written");
    init_case(&case_dir);
    fs::remove_dir_all(case_dir.join("evidence/logs")).expect("logs dir should be removed");
    fs::create_dir_all(&outside).expect("outside logs dir should exist");
    symlink(&outside, case_dir.join("evidence/logs")).expect("symlink should be created");

    let output = run(&["validate-artifact", path(&case_dir), path(&source_file)]);

    assert_failure_contains(&output, "inside the case directory");
    assert!(!outside.join("validation-log.jsonl").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn confirm_playback_rejects_symlinked_logs_directory_without_appending_outside() {
    let root = unique_temp_dir("cli-playback-logs-dir-symlink");
    let case_dir = root.join("case");
    let outside = root.join("outside-validation-logs");
    let target = root.join("clip.mp4");
    fs::create_dir_all(&root).expect("root dir should exist");
    fs::write(&target, b"\0\0\0\x18ftypmp42payload").expect("fixture video should be written");
    init_case(&case_dir);
    fs::remove_dir_all(case_dir.join("evidence/logs")).expect("logs dir should be removed");
    fs::create_dir_all(&outside).expect("outside logs dir should exist");
    seed_validation_log(&outside.join("validation-log.jsonl"), &target);
    symlink(&outside, case_dir.join("evidence/logs")).expect("symlink should be created");

    let output = run(&["confirm-playback", path(&case_dir), "vid_000001"]);

    assert_failure_contains(&output, "inside the case directory");
    let validation_log =
        fs::read_to_string(outside.join("validation-log.jsonl")).expect("seed log should exist");
    assert_eq!(validation_log.lines().count(), 1);
    let _ = fs::remove_dir_all(root);
}

struct E01Fixture {
    root: PathBuf,
    case_dir: PathBuf,
    source_dir: PathBuf,
    fake_bin: PathBuf,
    source_file: PathBuf,
}

impl E01Fixture {
    fn new(name: &str) -> Self {
        let root = unique_temp_dir(name);
        let case_dir = root.join("case");
        let source_dir = root.join("source-evidence");
        let fake_bin = root.join("fake-bin");
        let source_file = source_dir.join("blackbox.E01");
        Self {
            root,
            case_dir,
            source_dir,
            fake_bin,
            source_file,
        }
    }

    fn init(&self) {
        fs::create_dir_all(&self.source_dir).expect("source evidence dir should exist");
        fs::write(&self.source_file, b"e01 image").expect("source E01 should be written");
        init_case(&self.case_dir);
        write_fake_libewf(&self.fake_bin);
        fs::remove_dir_all(self.case_dir.join("evidence/logs"))
            .expect("logs dir should be removed");
    }

    fn inspect_e01(&self) -> Output {
        run_with_path(
            &[
                "inspect-e01",
                path(&self.case_dir),
                path(&self.source_file),
                "--ewfinfo",
                path(&self.fake_bin.join("ewfinfo")),
            ],
            &test_path_with_fake_bin(&self.fake_bin),
        )
    }

    fn import_e01(&self) -> Output {
        run_with_path(
            &[
                "import-e01",
                path(&self.case_dir),
                path(&self.source_file),
                "--ewfinfo",
                path(&self.fake_bin.join("ewfinfo")),
                "--ewfverify",
                path(&self.fake_bin.join("ewfverify")),
                "--ewfexport",
                path(&self.fake_bin.join("ewfexport")),
            ],
            &test_path_with_fake_bin(&self.fake_bin),
        )
    }
}

fn init_case(case_dir: &Path) {
    assert_success(&run(&[
        "init-case",
        path(case_dir),
        "--title",
        "Output Policy",
    ]));
}

fn seed_validation_log(log_path: &Path, target: &Path) {
    fs::write(
        log_path,
        format!(
            "{{\"selector\":\"vid_000001\",\"target_path\":\"{}\",\"target_sha256\":\"abc\",\"validation_status\":\"ffprobe-video-stream-confirmed\"}}\n",
            target.display()
        ),
    )
    .expect("outside validation log should be seeded");
}

fn write_fake_libewf(fake_bin: &Path) {
    fs::create_dir_all(fake_bin).expect("fake bin should be created");
    write_executable(
        &fake_bin.join("ewfinfo"),
        "#!/bin/sh\ncase \"$1\" in -V|--version|-version) echo 'ewfinfo fake 1.0'; exit 0 ;; esac\necho 'EWF information'\n",
    );
    write_executable(
        &fake_bin.join("ewfverify"),
        "#!/bin/sh\ncase \"$1\" in -V|--version|-version) echo 'ewfverify fake 1.0'; exit 0 ;; esac\nlog=''\nwhile [ $# -gt 0 ]; do if [ \"$1\" = '-l' ]; then shift; log=\"$1\"; fi; shift || break; done\n[ -n \"$log\" ] && printf 'verified\\n' > \"$log\"\n",
    );
    write_executable(
        &fake_bin.join("ewfexport"),
        "#!/bin/sh\ncase \"$1\" in -V|--version|-version) echo 'ewfexport fake 1.0'; exit 0 ;; esac\nlog=''\ntarget=''\nwhile [ $# -gt 0 ]; do case \"$1\" in -l) shift; log=\"$1\" ;; -t) shift; target=\"$1\" ;; esac; shift || break; done\n[ -n \"$log\" ] && printf 'exported\\n' > \"$log\"\n[ -n \"$target\" ] && printf 'raw image' > \"${target}.raw\"\n",
    );
}

fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("fake executable should be written");
    let mut permissions = fs::metadata(path)
        .expect("fake executable metadata should exist")
        .permissions();
    use std::os::unix::fs::PermissionsExt;
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("fake executable should be executable");
}

fn test_path_with_fake_bin(fake_bin: &Path) -> String {
    let current = std::env::var_os("PATH")
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    format!("{}:{current}", fake_bin.display())
}

fn assert_no_matching_outside_file(outside: &Path, prefix: &str) {
    let matches = fs::read_dir(outside)
        .expect("outside dir should be readable")
        .collect::<Result<Vec<_>, _>>()
        .expect("outside entries should be readable")
        .into_iter()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(prefix))
        .collect::<Vec<_>>();
    assert!(
        matches.is_empty(),
        "outside directory should not contain files starting with {prefix:?}: {matches:?}"
    );
}

fn path(path: &Path) -> &str {
    path.to_str().expect("test paths should be UTF-8")
}
