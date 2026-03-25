use crate::model::{CodexThread, ProjectType, SessionRecord, WorktreeInfo};
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn scan_worktrees() -> Result<Vec<WorktreeInfo>> {
    scan_worktrees_in(default_worktrees_dir()?, default_session_index_path()?)
}

pub fn scan_worktrees_in(
    worktrees_dir: PathBuf,
    session_index_path: PathBuf,
) -> Result<Vec<WorktreeInfo>> {
    let sessions = load_session_index(&session_index_path);

    let entries: Vec<_> = fs::read_dir(&worktrees_dir)
        .with_context(|| format!("Cannot read {}", worktrees_dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .collect();

    let mut worktrees: Vec<WorktreeInfo> = entries
        .par_iter()
        .filter_map(|entry| scan_single_worktree(entry.path(), &sessions).ok())
        .collect();

    worktrees.sort_by(|a, b| b.total_size.cmp(&a.total_size));
    Ok(worktrees)
}

fn scan_single_worktree(
    path: PathBuf,
    sessions: &HashMap<String, SessionRecord>,
) -> Result<WorktreeInfo> {
    let codex_id = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let project_entry = fs::read_dir(&path)?
        .filter_map(|e| e.ok())
        .find(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false));

    let Some(project_entry) = project_entry else {
        return Ok(WorktreeInfo {
            codex_id,
            path: path.clone(),
            project_name: "(empty)".to_string(),
            project_path: path,
            git_worktree_path: None,
            branch: None,
            thread_id: None,
            thread_name: None,
            updated_at: None,
            total_size: 0,
            artifact_size: 0,
            project_type: ProjectType::Unknown,
            selected: false,
        });
    };

    let project_path = project_entry.path();
    let project_name = project_entry.file_name().to_string_lossy().to_string();

    let git_meta = resolve_git_metadata(&project_path);
    let project_type = detect_project_type(&project_path);
    let (total_size, artifact_size) = compute_sizes(&project_path, &project_type);

    let (thread_name, updated_at) = git_meta
        .as_ref()
        .and_then(|m| m.thread_id.as_ref())
        .and_then(|tid| sessions.get(tid))
        .map(|s| (s.thread_name.clone(), s.updated_at.clone()))
        .unwrap_or((None, None));

    Ok(WorktreeInfo {
        codex_id,
        path,
        project_name,
        project_path,
        git_worktree_path: git_meta.as_ref().and_then(|m| m.git_worktree_path.clone()),
        branch: git_meta.as_ref().and_then(|m| m.branch.clone()),
        thread_id: git_meta.and_then(|m| m.thread_id),
        thread_name,
        updated_at,
        total_size,
        artifact_size,
        project_type,
        selected: false,
    })
}

struct GitMeta {
    git_worktree_path: Option<PathBuf>,
    branch: Option<String>,
    thread_id: Option<String>,
}

fn resolve_git_metadata(project_path: &Path) -> Option<GitMeta> {
    let git_file = project_path.join(".git");
    let content = fs::read_to_string(&git_file).ok()?;

    let gitdir = content.trim().strip_prefix("gitdir: ")?;
    let git_worktree_path = PathBuf::from(gitdir);

    let head_path = git_worktree_path.join("HEAD");
    let branch = fs::read_to_string(&head_path).ok().map(|h| {
        let h = h.trim();
        if let Some(refname) = h.strip_prefix("ref: refs/heads/") {
            refname.to_string()
        } else if h.len() >= 8 {
            format!("({})", &h[..8])
        } else {
            h.to_string()
        }
    });

    let thread_path = git_worktree_path.join("codex-thread.json");
    let thread_id = fs::read_to_string(&thread_path)
        .ok()
        .and_then(|content| {
            serde_json::from_str::<CodexThread>(&content)
                .ok()
                .map(|t| t.owner_thread_id)
        });

    Some(GitMeta {
        git_worktree_path: Some(git_worktree_path),
        branch,
        thread_id,
    })
}

pub fn detect_project_type(project_path: &Path) -> ProjectType {
    if project_path.join("Cargo.toml").exists() {
        ProjectType::Rust
    } else if project_path.join("go.mod").exists() {
        ProjectType::Go
    } else if project_path.join("package.json").exists() {
        ProjectType::Node
    } else if project_path.join("pyproject.toml").exists()
        || project_path.join("setup.py").exists()
        || project_path.join("requirements.txt").exists()
    {
        ProjectType::Python
    } else {
        ProjectType::Unknown
    }
}

pub fn compute_sizes(project_path: &Path, project_type: &ProjectType) -> (u64, u64) {
    let artifact_names: Vec<&str> = project_type.artifact_dirs().to_vec();
    let mut total_size: u64 = 0;
    let mut artifact_size: u64 = 0;

    // Single-pass walk: accumulate total and artifact sizes together
    compute_sizes_recursive(project_path, &artifact_names, true, &mut total_size, &mut artifact_size);
    (total_size, artifact_size)
}

fn compute_sizes_recursive(
    path: &Path,
    artifact_names: &[&str],
    check_artifacts: bool,
    total: &mut u64,
    artifacts: &mut u64,
) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

        if is_dir {
            let name = entry.file_name();
            let is_artifact = check_artifacts
                && artifact_names.iter().any(|a| *a == name.to_string_lossy().as_ref());

            if is_artifact {
                let size = dir_size(&entry.path());
                *total += size;
                *artifacts += size;
            } else {
                compute_sizes_recursive(&entry.path(), artifact_names, check_artifacts, total, artifacts);
            }
        } else {
            *total += len;
        }
    }
}

pub fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    };

    entries
        .filter_map(|e| e.ok())
        .map(|entry| {
            let Ok(meta) = entry.metadata().or_else(|_| fs::symlink_metadata(entry.path())) else {
                return 0;
            };
            if meta.is_dir() {
                dir_size(&entry.path())
            } else {
                meta.len()
            }
        })
        .sum()
}

fn default_worktrees_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    Ok(home.join(".codex").join("worktrees"))
}

fn default_session_index_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    Ok(home.join(".codex").join("session_index.jsonl"))
}

fn load_session_index(path: &Path) -> HashMap<String, SessionRecord> {
    let mut map = HashMap::new();
    let Ok(content) = fs::read_to_string(path) else {
        return map;
    };
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(record) = serde_json::from_str::<SessionRecord>(line) {
            let id = record.id.clone();
            map.insert(id, record);
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_project_type_rust() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Rust);
    }

    #[test]
    fn test_detect_project_type_go() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("go.mod"), "module foo").unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Go);
    }

    #[test]
    fn test_detect_project_type_node() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Node);
    }

    #[test]
    fn test_detect_project_type_python() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Python);
    }

    #[test]
    fn test_detect_project_type_unknown() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Unknown);
    }

    #[test]
    fn test_compute_sizes() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        let target = dir.path().join("target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("big_file"), vec![0u8; 1024]).unwrap();

        let (total, artifacts) = compute_sizes(dir.path(), &ProjectType::Rust);
        assert!(total > 0);
        assert!(artifacts >= 1024);
    }

    #[test]
    fn test_dir_size() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "hello").unwrap();
        fs::write(dir.path().join("b.txt"), "world!").unwrap();
        let size = dir_size(dir.path());
        assert_eq!(size, 11);
    }

    #[test]
    fn test_scan_worktrees_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let session_file = dir.path().join("sessions.jsonl");
        fs::write(&session_file, "").unwrap();
        let result = scan_worktrees_in(dir.path().to_path_buf(), session_file).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_scan_worktrees_with_mock_structure() {
        let dir = tempfile::tempdir().unwrap();
        let worktrees_dir = dir.path().join("worktrees");
        fs::create_dir(&worktrees_dir).unwrap();

        let wt_dir = worktrees_dir.join("ab12");
        fs::create_dir(&wt_dir).unwrap();
        let project_dir = wt_dir.join("myproject");
        fs::create_dir(&project_dir).unwrap();
        fs::write(project_dir.join("Cargo.toml"), "[package]").unwrap();
        let target = project_dir.join("target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("artifact"), vec![0u8; 2048]).unwrap();

        let session_file = dir.path().join("sessions.jsonl");
        fs::write(&session_file, "").unwrap();

        let result = scan_worktrees_in(worktrees_dir, session_file).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].codex_id, "ab12");
        assert_eq!(result[0].project_name, "myproject");
        assert_eq!(result[0].project_type, ProjectType::Rust);
        assert!(result[0].total_size > 0);
        assert!(result[0].artifact_size >= 2048);
    }

    #[test]
    fn test_scan_empty_worktree() {
        let dir = tempfile::tempdir().unwrap();
        let worktrees_dir = dir.path().join("worktrees");
        fs::create_dir(&worktrees_dir).unwrap();

        let wt_dir = worktrees_dir.join("f67f");
        fs::create_dir(&wt_dir).unwrap();

        let session_file = dir.path().join("sessions.jsonl");
        fs::write(&session_file, "").unwrap();

        let result = scan_worktrees_in(worktrees_dir, session_file).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].project_name, "(empty)");
        assert!(result[0].is_empty());
    }

    #[test]
    fn test_load_session_index() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.jsonl");
        fs::write(
            &path,
            r#"{"id":"abc123","thread_name":"Fix bug","updated_at":"2025-03-20T10:00:00Z"}
{"id":"def456","thread_name":"Add feature","updated_at":"2025-03-21T12:00:00Z"}
"#,
        )
        .unwrap();

        let sessions = load_session_index(&path);
        assert_eq!(sessions.len(), 2);
        assert_eq!(
            sessions["abc123"].thread_name.as_deref(),
            Some("Fix bug")
        );
        assert_eq!(
            sessions["def456"].thread_name.as_deref(),
            Some("Add feature")
        );
    }
}
