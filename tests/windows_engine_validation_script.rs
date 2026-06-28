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
fn engine_validation_script_stays_ascii_for_windows_powershell_51() {
    // Given: Windows PowerShell 5.1 reads UTF-8 without BOM through the local
    // ANSI code page, which can corrupt non-ASCII string literals before parse.
    let script = fs::read("scripts/windows/validate-engine.ps1")
        .expect("Windows engine script should be readable");

    // When/Then: keep the committed script source ASCII-only and construct
    // Unicode validation paths at runtime.
    assert!(
        script.is_ascii(),
        "validate-engine.ps1 must stay ASCII-only for Windows PowerShell 5.1 parsing"
    );
}

#[test]
fn engine_validation_native_step_does_not_shadow_scriptblock_variable() {
    let script = fs::read_to_string("scripts/windows/validate-engine.ps1")
        .expect("Windows engine script should be readable");

    assert!(script.contains("[scriptblock]$StepAction"));
    assert!(script.contains("[scriptblock]$NativeAction"));
    assert!(script.contains(".GetNewClosure()"));
    assert!(!script.contains("[scriptblock]$Script"));
}

#[test]
fn engine_validation_script_is_engine_only_and_fail_closed() {
    let script = fs::read_to_string("scripts/windows/validate-engine.ps1")
        .expect("Windows engine script should be readable");

    assert!(script.contains("Windows engine validation must run on Windows 10/11 x64."));
    assert!(script.contains("host: .*windows-msvc"));
    assert!(
        script.contains(r#"foreach ($CommandName in @("rustc", "cargo", "ffmpeg", "ffprobe"))"#)
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
fn engine_validation_creates_qa_output_dir_before_writing_receipt_artifacts() {
    let script = fs::read_to_string("scripts/windows/validate-engine.ps1")
        .expect("Windows engine script should be readable");

    let qa_dir = find_required(&script, r#"$QaReportDir = Join-Path $CaseDir "reports\qa""#);
    let create_dir = find_required(
        &script,
        "New-Item -ItemType Directory -Force -Path $QaReportDir",
    );
    let status_out = find_required(
        &script,
        r#"$StatusPath = Join-Path $QaReportDir "workstation-status.json""#,
    );
    let inventory_out = find_required(
        &script,
        r#"$InventoryPath = Join-Path $QaReportDir "inventory-page.json""#,
    );
    let page_size_assertion = find_required(
        &script,
        r#"Assert-Contains $InventoryPath '"page_size":10'"#,
    );

    assert!(qa_dir < create_dir);
    assert!(create_dir < status_out);
    assert!(create_dir < inventory_out);
    assert!(inventory_out < page_size_assertion);
    assert!(!script.contains(r#"Assert-Contains $InventoryPath '"limit":10'"#));
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
    assert!(script.contains("function Test-FullyQualifiedPath"));
    assert!(guard.contains("Test-FullyQualifiedPath $ExpandedPath"));
    assert!(!guard.contains("IsPathFullyQualified"));
    assert!(guard.contains(r#"$LeafName -notlike "frametrace-*""#));
    assert!(guard.contains("[System.IO.Path]::GetPathRoot($FullPath)"));
    assert!(guard.contains("Test-SamePath $FullPath $RepoRoot"));
    assert!(
        guard.contains("[Environment+SpecialFolder]::UserProfile"),
        "guard should reject user profile roots; unsafe examples include {unsafe_case_roots:?}"
    );
    assert!(guard.contains("[System.IO.Path]::GetTempPath()"));
}
