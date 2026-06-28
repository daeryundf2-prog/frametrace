use crate::util::{json_escape, write_text};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const REQUIRED_RELEASE_TOOLS: &[&str] = &["rustc", "cargo", "ffmpeg", "ffprobe", "dotnet"];
pub(crate) const WINUI_PROJECT_DIR: &str = "gui/winui";
const WINUI_BUILD_RECEIPT: &str = "winui-build.json";

#[derive(Debug, Clone)]
pub(crate) struct WindowsPrerequisites {
    host_os: String,
    missing_tools: Vec<String>,
    pub(crate) winui_project_files: Vec<String>,
    pub(crate) winui_project_present: bool,
    winui_build_receipt_present: Option<bool>,
    winui_build_receipt_path: Option<PathBuf>,
    pub(crate) blockers: Vec<String>,
}

impl WindowsPrerequisites {
    fn current() -> Self {
        Self::from_repo_root(Path::new(env!("CARGO_MANIFEST_DIR")))
    }

    pub(crate) fn from_repo_root(repo_root: &Path) -> Self {
        let host_os = env::consts::OS.to_string();
        let missing_tools = REQUIRED_RELEASE_TOOLS
            .iter()
            .filter(|tool| !command_available(tool))
            .map(|tool| (*tool).to_string())
            .collect::<Vec<_>>();
        let winui_project_files = discover_winui_project_files(repo_root);
        let winui_project_present = !winui_project_files.is_empty();
        let mut blockers = Vec::new();
        if host_os != "windows" {
            blockers.push("unsupported-host".to_string());
        }
        blockers.extend(
            missing_tools
                .iter()
                .map(|tool| format!("missing-tool:{tool}")),
        );
        if !winui_project_present {
            blockers.push("missing-winui-project".to_string());
        }
        Self {
            host_os,
            missing_tools,
            winui_project_files,
            winui_project_present,
            winui_build_receipt_present: None,
            winui_build_receipt_path: None,
            blockers,
        }
    }

    fn require_build_receipt(&mut self, output_dir: &Path) {
        let receipt_path = output_dir.join(WINUI_BUILD_RECEIPT);
        let receipt_present = winui_build_receipt_is_valid(&receipt_path);
        self.winui_build_receipt_path = Some(receipt_path);
        self.winui_build_receipt_present = Some(receipt_present);
        if !receipt_present {
            self.blockers
                .push("missing-winui-build-receipt".to_string());
        }
    }

    fn release_validation_host_ready(&self) -> bool {
        self.blockers.is_empty()
    }

    fn json(&self) -> String {
        let receipt_json = match (
            self.winui_build_receipt_present,
            self.winui_build_receipt_path.as_ref(),
        ) {
            (Some(present), Some(path)) => format!(
                ",\"winui_build_receipt_present\":{},\"winui_build_receipt_path\":\"{}\"",
                present,
                json_escape(&path.to_string_lossy())
            ),
            _ => String::new(),
        };
        format!(
            "{{\"host_os\":\"{}\",\"release_validation_host_ready\":{},\
\"required_tools\":{},\"missing_tools\":{},\"winui_project_present\":{},\
\"winui_project_dir\":\"{}\",\"winui_project_files\":{}{},\"blockers\":{}}}",
            json_escape(&self.host_os),
            self.release_validation_host_ready(),
            string_array_json(REQUIRED_RELEASE_TOOLS),
            owned_string_array_json(&self.missing_tools),
            self.winui_project_present,
            WINUI_PROJECT_DIR,
            owned_string_array_json(&self.winui_project_files),
            receipt_json,
            owned_string_array_json(&self.blockers)
        )
    }
}

pub fn status_json() -> String {
    WindowsPrerequisites::current().json()
}

pub fn release_validation_check(output_dir: &Path) -> Result<PathBuf, String> {
    let mut prerequisites = WindowsPrerequisites::current();
    prerequisites.require_build_receipt(output_dir);
    let output_path = output_dir.join("windows-prerequisites.json");
    write_text(&output_path, &prerequisites.json())
        .map_err(|err| format!("failed to write Windows prerequisite evidence: {err}"))?;
    if prerequisites.release_validation_host_ready() {
        Ok(output_path)
    } else {
        Err(format!(
            "windows_prerequisites failed: {}; see {}",
            prerequisites.blockers.join(", "),
            output_path.display()
        ))
    }
}

fn discover_winui_project_files(repo_root: &Path) -> Vec<String> {
    let project_dir = repo_root.join(WINUI_PROJECT_DIR);
    let mut files = Vec::new();
    collect_project_files(repo_root, &project_dir, &mut files);
    files.sort();
    files
}

fn collect_project_files(repo_root: &Path, dir: &Path, files: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_project_files(repo_root, &path, files);
            continue;
        }
        if has_winui_project_extension(&path) {
            files.push(relative_path_string(repo_root, &path));
        }
    }
}

fn has_winui_project_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("sln" | "csproj")
    )
}

fn relative_path_string(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn winui_build_receipt_is_valid(path: &Path) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let compact = text
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    compact.contains("\"dotnet_build\":\"pass\"") && compact.contains("\"dotnet_test\":\"pass\"")
}

pub(crate) fn command_available(command: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path).any(|dir| {
        command_candidates(&dir, command)
            .iter()
            .any(|path| command_candidate_available(path))
    })
}

fn command_candidate_available(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    executable_file(path)
}

#[cfg(unix)]
fn executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.metadata()
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn executable_file(path: &Path) -> bool {
    path.is_file()
}

fn command_candidates(dir: &Path, command: &str) -> Vec<PathBuf> {
    let base = dir.join(command);
    if cfg!(windows) {
        let mut candidates = vec![base.clone()];
        if let Some(exts) = env::var_os("PATHEXT") {
            candidates.extend(env::split_paths(&exts).map(|ext| {
                let suffix = ext.to_string_lossy();
                dir.join(format!("{command}{suffix}"))
            }));
        }
        candidates
    } else {
        vec![base]
    }
}

fn string_array_json(values: &[&str]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| format!("\"{}\"", json_escape(value)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn owned_string_array_json(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| format!("\"{}\"", json_escape(value)))
            .collect::<Vec<_>>()
            .join(",")
    )
}
