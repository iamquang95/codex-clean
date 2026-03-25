use crate::model::{ProjectType, WorktreeInfo};
use anyhow::{Context, Result};
use std::fs;

pub fn clean_artifacts(wt: &WorktreeInfo) -> Result<u64> {
    let artifact_dirs: Vec<&str> = match wt.project_type {
        ProjectType::Rust => vec!["target"],
        ProjectType::Go => vec!["vendor"],
        ProjectType::Node => vec!["node_modules"],
        ProjectType::Python => vec![".venv"],
        ProjectType::Unknown => vec!["target", "node_modules", ".venv", "build", "dist"],
    };

    let mut total_freed: u64 = 0;
    for dir_name in artifact_dirs {
        let path = wt.project_path.join(dir_name);
        if path.exists() && path.is_dir() {
            let size = crate::scan::dir_size(&path);
            fs::remove_dir_all(&path)
                .with_context(|| format!("Failed to remove {}", path.display()))?;
            total_freed += size;
        }
    }

    // Python: also recursively find and remove __pycache__ dirs
    if wt.project_type == ProjectType::Python {
        total_freed += remove_pycache_recursive(&wt.project_path)?;
    }

    Ok(total_freed)
}

fn remove_pycache_recursive(path: &std::path::Path) -> Result<u64> {
    let mut freed = 0;
    let Ok(entries) = fs::read_dir(path) else {
        return Ok(0);
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_dir() {
            if p.file_name().map(|n| n == "__pycache__").unwrap_or(false) {
                freed += crate::scan::dir_size(&p);
                fs::remove_dir_all(&p)?;
            } else {
                freed += remove_pycache_recursive(&p)?;
            }
        }
    }
    Ok(freed)
}

pub fn delete_worktree(wt: &WorktreeInfo) -> Result<()> {
    // Remove the git worktree metadata dir if it exists
    if let Some(git_wt_path) = &wt.git_worktree_path {
        if git_wt_path.exists() {
            fs::remove_dir_all(git_wt_path)
                .with_context(|| format!("Failed to remove git worktree {}", git_wt_path.display()))?;
        }
    }

    // Remove the codex worktree directory
    fs::remove_dir_all(&wt.path)
        .with_context(|| format!("Failed to remove {}", wt.path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_worktree(dir: &std::path::Path, project_type: ProjectType) -> WorktreeInfo {
        WorktreeInfo {
            codex_id: "test".to_string(),
            path: dir.to_path_buf(),
            project_name: "testproject".to_string(),
            project_path: dir.to_path_buf(),
            git_worktree_path: None,
            branch: None,
            thread_id: None,
            thread_name: None,
            updated_at: None,
            total_size: 0,
            artifact_size: 0,
            project_type,
            selected: false,
        }
    }

    #[test]
    fn test_clean_rust_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("debug_bin"), vec![0u8; 4096]).unwrap();

        let wt = make_worktree(dir.path(), ProjectType::Rust);
        let freed = clean_artifacts(&wt).unwrap();
        assert!(freed >= 4096);
        assert!(!target.exists());
    }

    #[test]
    fn test_clean_node_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let nm = dir.path().join("node_modules");
        fs::create_dir(&nm).unwrap();
        fs::write(nm.join("dep.js"), vec![0u8; 2048]).unwrap();

        let wt = make_worktree(dir.path(), ProjectType::Node);
        let freed = clean_artifacts(&wt).unwrap();
        assert!(freed >= 2048);
        assert!(!nm.exists());
    }

    #[test]
    fn test_clean_python_pycache() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = dir.path().join("pkg");
        fs::create_dir(&pkg).unwrap();
        let cache = pkg.join("__pycache__");
        fs::create_dir(&cache).unwrap();
        fs::write(cache.join("mod.pyc"), vec![0u8; 512]).unwrap();

        let wt = make_worktree(dir.path(), ProjectType::Python);
        let freed = clean_artifacts(&wt).unwrap();
        assert!(freed >= 512);
        assert!(!cache.exists());
    }

    #[test]
    fn test_clean_go_vendor() {
        let dir = tempfile::tempdir().unwrap();
        let vendor = dir.path().join("vendor");
        fs::create_dir(&vendor).unwrap();
        fs::write(vendor.join("dep.go"), vec![0u8; 1024]).unwrap();

        let wt = make_worktree(dir.path(), ProjectType::Go);
        let freed = clean_artifacts(&wt).unwrap();
        assert!(freed >= 1024);
        assert!(!vendor.exists());
    }

    #[test]
    fn test_clean_no_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let wt = make_worktree(dir.path(), ProjectType::Rust);
        let freed = clean_artifacts(&wt).unwrap();
        assert_eq!(freed, 0);
    }

    #[test]
    fn test_delete_worktree() {
        let dir = tempfile::tempdir().unwrap();
        let wt_path = dir.path().join("ab12");
        fs::create_dir(&wt_path).unwrap();
        fs::write(wt_path.join("file"), "data").unwrap();

        // Also create a mock git worktree metadata dir
        let git_wt = dir.path().join("git_worktree_meta");
        fs::create_dir(&git_wt).unwrap();
        fs::write(git_wt.join("HEAD"), "ref: refs/heads/main").unwrap();

        let wt = WorktreeInfo {
            path: wt_path.clone(),
            git_worktree_path: Some(git_wt.clone()),
            ..make_worktree(&wt_path, ProjectType::Unknown)
        };

        delete_worktree(&wt).unwrap();
        assert!(!wt_path.exists());
        assert!(!git_wt.exists());
    }

    #[test]
    fn test_delete_worktree_no_git_meta() {
        let dir = tempfile::tempdir().unwrap();
        let wt_path = dir.path().join("cd34");
        fs::create_dir(&wt_path).unwrap();

        let wt = WorktreeInfo {
            path: wt_path.clone(),
            git_worktree_path: None,
            ..make_worktree(&wt_path, ProjectType::Unknown)
        };

        delete_worktree(&wt).unwrap();
        assert!(!wt_path.exists());
    }
}
