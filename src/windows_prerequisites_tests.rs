use crate::windows_prerequisites::{WINUI_PROJECT_DIR, WindowsPrerequisites, status_json};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn status_reports_host_and_release_readiness() {
    let json = status_json();

    assert!(json.contains(&format!("\"host_os\":\"{}\"", std::env::consts::OS)));
    assert!(json.contains("\"release_validation_host_ready\":"));
    assert!(json.contains("\"required_tools\":["));
    assert!(json.contains("\"winui_project_present\":"));
    assert!(json.contains("\"winui_project_files\":["));
    assert!(json.contains("\"blockers\":["));
}

#[test]
fn empty_winui_directory_is_not_a_project() {
    let root = unique_temp_dir("empty-winui");
    fs::create_dir_all(root.join(WINUI_PROJECT_DIR)).expect("fixture dir should be created");

    let prerequisites = WindowsPrerequisites::from_repo_root(&root);

    assert!(!prerequisites.winui_project_present);
    assert!(
        prerequisites
            .blockers
            .iter()
            .any(|blocker| blocker == "missing-winui-project")
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn winui_solution_or_project_file_is_required_project_evidence() {
    let root = unique_temp_dir("real-winui");
    let winui_dir = root.join(WINUI_PROJECT_DIR).join("FrameTrace");
    fs::create_dir_all(&winui_dir).expect("fixture dir should be created");
    fs::write(winui_dir.join("FrameTrace.csproj"), "<Project />")
        .expect("fixture project should be written");

    let prerequisites = WindowsPrerequisites::from_repo_root(&root);

    assert!(prerequisites.winui_project_present);
    assert_eq!(
        prerequisites.winui_project_files,
        vec!["gui/winui/FrameTrace/FrameTrace.csproj"]
    );
    let _ = fs::remove_dir_all(root);
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after UNIX epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "frametrace-windows-prereq-{name}-{}-{nanos}",
        std::process::id()
    ))
}
