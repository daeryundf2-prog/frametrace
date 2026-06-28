use std::fs;

fn find_required(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("expected script to contain {needle:?}"))
}

#[test]
fn release_engine_only_mode_delegates_before_winui_release_preflight() {
    let script = fs::read_to_string("scripts/windows/validate-release.ps1")
        .expect("Windows release script should be readable");
    let engine_only_block = script
        .split("if ($EngineOnly)")
        .nth(1)
        .expect("release script should branch on EngineOnly")
        .split("Remove-Item -LiteralPath $CaseRoot")
        .next()
        .expect("EngineOnly branch should be before release case setup");

    assert!(script.contains("[switch]$EngineOnly"));
    assert!(engine_only_block.contains("validate-engine.ps1"));
    assert!(!engine_only_block.contains("dotnet"));
    assert!(!engine_only_block.contains("Find-WinUiProject"));
    assert!(!engine_only_block.contains("winui-build.json"));
}

#[test]
fn engine_validation_script_is_engine_only_and_fail_closed() {
    let script = fs::read_to_string("scripts/windows/validate-engine.ps1")
        .expect("Windows engine script should be readable");

    assert!(script.contains("Windows engine validation must run on Windows 10/11 x64."));
    assert!(script.contains("host: .*windows-msvc"));
    assert!(
        script
            .contains("foreach ($CommandName in @(\"rustc\", \"cargo\", \"ffmpeg\", \"ffprobe\"))")
    );
    assert!(!script.contains("Require-Command dotnet"));
    assert!(!script.contains("gui\\winui"));
    assert!(!script.contains("winui-build.json"));
    assert!(script.contains("windows-engine-validation.json"));
    assert!(script.contains("validation_playback_separation"));
    assert!(script.contains("unicode_long_path_scan"));
    assert!(script.contains("file_lock_scan"));
    assert!(script.contains("bounded_inventory"));
    assert!(
        script
            .contains("If this status is BLOCKED, T12 must write release-decision.json as BLOCKED")
    );
}

#[test]
fn engine_validation_guards_caseroot_before_recursive_delete() {
    // Given: the Windows engine validation script is a source contract because
    // PowerShell is not guaranteed on every CI host that runs this Rust test.
    let script = fs::read_to_string("scripts/windows/validate-engine.ps1")
        .expect("Windows engine script should be readable");

    // When: locating the safe CaseRoot boundary and the destructive cleanup.
    let guard_function = find_required(&script, "function Assert-SafeCaseRoot");
    let guard_call = find_required(
        &script,
        "$CaseRoot = Assert-SafeCaseRoot $CaseRoot $RepoRoot",
    );
    let recursive_delete = find_required(
        &script,
        "Remove-Item -LiteralPath $CaseRoot -Recurse -Force",
    );
    let windows_host_check = find_required(&script, "if (-not (Test-RunningOnWindows))");
    let msvc_check = find_required(&script, "host: .*windows-msvc");

    // Then: validation is defined and invoked before any recursive delete or
    // host/toolchain checks can run.
    assert!(guard_function < guard_call);
    assert!(guard_call < recursive_delete);
    assert!(guard_call < windows_host_check);
    assert!(guard_call < msvc_check);
}

#[test]
fn engine_validation_caseroot_guard_rejects_broad_unsafe_paths() {
    // Given: unsafe examples that must never be acceptable recursive-delete roots.
    let unsafe_case_roots = ["", "C:\\", "C:\\Temp", "%USERPROFILE%", "$env:TEMP", "."];
    assert_eq!(unsafe_case_roots.len(), 6);

    let script = fs::read_to_string("scripts/windows/validate-engine.ps1")
        .expect("Windows engine script should be readable");
    let guard = script
        .split("function Assert-SafeCaseRoot")
        .nth(1)
        .expect("script should define Assert-SafeCaseRoot")
        .split("$RepoRoot = Resolve-Path")
        .next()
        .expect("guard should appear before top-level script execution");

    // When/Then: the guard structurally rejects null/empty, relative, non
    // frametrace-* leaf, drive/filesystem root, repo root, user profile, and
    // temp root values before the script can clear CaseRoot.
    assert!(guard.contains("[string]::IsNullOrWhiteSpace($Path)"));
    assert!(guard.contains("[System.IO.Path]::IsPathFullyQualified($ExpandedPath)"));
    assert!(guard.contains("$LeafName -notlike \"frametrace-*\""));
    assert!(guard.contains("[System.IO.Path]::GetPathRoot($FullPath)"));
    assert!(guard.contains("Test-SamePath $FullPath $RepoRoot"));
    assert!(
        guard.contains("[Environment+SpecialFolder]::UserProfile"),
        "guard should reject user profile roots; unsafe examples include {unsafe_case_roots:?}"
    );
    assert!(guard.contains("[System.IO.Path]::GetTempPath()"));
}
