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
fn inspect_image_rejects_symlinked_logs_directory_without_writing_outside() {
    let root = unique_temp_dir("cli-inspect-image-logs-dir-symlink");
    let case_dir = root.join("case");
    let source_dir = root.join("source-evidence");
    let fake_bin = root.join("fake-bin");
    let source_file = source_dir.join("disk.img");
    fs::create_dir_all(&source_dir).expect("source evidence dir should exist");
    fs::write(&source_file, b"raw image").expect("source image should be written");
    init_case(&case_dir);
    write_fake_sleuthkit(&fake_bin);
    fs::remove_dir_all(case_dir.join("evidence/logs")).expect("logs dir should be removed");
    symlink(&source_dir, case_dir.join("evidence/logs")).expect("symlink should be created");

    let output = inspect_image(&case_dir, &source_file, &fake_bin);

    assert_failure_contains(&output, "inside the case directory");
    assert!(!source_dir.join("tsk-audit.jsonl").exists());
    assert_no_matching_outside_file(&source_dir, "tsk-mmls-");
    assert_no_matching_outside_file(&source_dir, "tsk-fls-");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn inspect_image_rejects_symlinked_filesystem_db_directory_without_writing_outside() {
    let root = unique_temp_dir("cli-inspect-image-dbfs-dir-symlink");
    let case_dir = root.join("case");
    let source_dir = root.join("source-evidence");
    let fake_bin = root.join("fake-bin");
    let outside = root.join("outside-dbfs");
    let source_file = source_dir.join("disk.img");
    fs::create_dir_all(&source_dir).expect("source evidence dir should exist");
    fs::write(&source_file, b"raw image").expect("source image should be written");
    init_case(&case_dir);
    write_fake_sleuthkit(&fake_bin);
    fs::remove_dir_all(case_dir.join("db/filesystem"))
        .expect("filesystem db dir should be removed");
    fs::create_dir_all(&outside).expect("outside dbfs dir should exist");
    symlink(&outside, case_dir.join("db/filesystem")).expect("symlink should be created");

    let output = inspect_image(&case_dir, &source_file, &fake_bin);

    assert_failure_contains(&output, "inside the case directory");
    assert_no_outside_files(&outside);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn recover_inode_rejects_symlinked_logs_directory_without_writing_outside() {
    let root = unique_temp_dir("cli-recover-inode-logs-dir-symlink");
    let case_dir = root.join("case");
    let source_dir = root.join("source-evidence");
    let fake_bin = root.join("fake-bin");
    let source_file = source_dir.join("disk.img");
    fs::create_dir_all(&source_dir).expect("source evidence dir should exist");
    fs::write(&source_file, b"raw image").expect("source image should be written");
    init_case(&case_dir);
    write_fake_icat(&fake_bin);
    fs::remove_dir_all(case_dir.join("evidence/logs")).expect("logs dir should be removed");
    symlink(&source_dir, case_dir.join("evidence/logs")).expect("symlink should be created");

    let output = run_with_path(
        &[
            "recover-inode",
            path(&case_dir),
            path(&source_file),
            "5",
            "--icat",
            path(&fake_bin.join("icat")),
        ],
        &test_path_with_fake_bin(&fake_bin),
    );

    assert_failure_contains(&output, "inside the case directory");
    assert!(!source_dir.join("tsk-audit.jsonl").exists());
    assert_no_matching_outside_file(&source_dir, "inode_");
    let _ = fs::remove_dir_all(root);
}

fn inspect_image(case_dir: &Path, source_file: &Path, fake_bin: &Path) -> Output {
    run_with_path(
        &[
            "inspect-image",
            path(case_dir),
            path(source_file),
            "--mmls",
            path(&fake_bin.join("mmls")),
            "--fls",
            path(&fake_bin.join("fls")),
        ],
        &test_path_with_fake_bin(fake_bin),
    )
}

fn init_case(case_dir: &Path) {
    assert_success(&run(&[
        "init-case",
        path(case_dir),
        "--title",
        "Output Policy",
    ]));
}

fn write_fake_sleuthkit(fake_bin: &Path) {
    fs::create_dir_all(fake_bin).expect("fake bin should be created");
    write_executable(
        &fake_bin.join("mmls"),
        "#!/bin/sh\ncase \"$1\" in -V|--version|-version) echo 'mmls fake 1.0'; exit 0 ;; esac\necho '000: 0000000000 0000001023 0000001024 NTFS'\n",
    );
    write_executable(
        &fake_bin.join("fls"),
        "#!/bin/sh\ncase \"$1\" in -V|--version|-version) echo 'fls fake 1.0'; exit 0 ;; esac\necho 'r/r 5: clip.mp4'\n",
    );
}

fn write_fake_icat(fake_bin: &Path) {
    fs::create_dir_all(fake_bin).expect("fake bin should be created");
    write_executable(
        &fake_bin.join("icat"),
        "#!/bin/sh\ncase \"$1\" in -V|--version|-version) echo 'icat fake 1.0'; exit 0 ;; esac\nprintf 'recovered inode'\n",
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

fn assert_no_outside_files(outside: &Path) {
    let entries = fs::read_dir(outside)
        .expect("outside dir should be readable")
        .collect::<Result<Vec<_>, _>>()
        .expect("outside entries should be readable");
    assert!(
        entries.is_empty(),
        "outside directory should remain empty: {:?}",
        entries.iter().map(|entry| entry.path()).collect::<Vec<_>>()
    );
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
