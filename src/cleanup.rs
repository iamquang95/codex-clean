use crate::model::WorktreeInfo;
use anyhow::{Context, Result};
use std::fs;

pub fn clean_artifacts(wt: &WorktreeInfo) -> Result<u64> {
    let mut total_freed: u64 = 0;
    for dir_name in wt.project_type.artifact_dirs() {
        // __pycache__ can appear at any nesting level
        if *dir_name == "__pycache__" {
            total_freed += remove_recursive(&wt.project_path, "__pycache__")?;
            continue;
        }
        let path = wt.project_path.join(dir_name);
        let size = crate::scan::dir_size(&path);
        if size > 0 {
            match fs::remove_dir_all(&path) {
                Ok(()) => total_freed += size,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e).with_context(|| format!("Failed to remove {}", path.display())),
            }
        }
    }
    Ok(total_freed)
}

fn remove_recursive(path: &std::path::Path, target_name: &str) -> Result<u64> {
    let mut freed = 0;
    let Ok(entries) = fs::read_dir(path) else {
        return Ok(0);
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_dir() {
            if p.file_name().map(|n| n == target_name).unwrap_or(false) {
                let size = crate::scan::dir_size(&p);
                match fs::remove_dir_all(&p) {
                    Ok(()) => freed += size,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e).with_context(|| format!("Failed to remove {}", p.display())),
                }
            } else {
                freed += remove_recursive(&p, target_name)?;
            }
        }
    }
    Ok(freed)
}

pub fn delete_worktree(wt: &WorktreeInfo) -> Result<()> {
    if let Some(git_wt_path) = &wt.git_worktree_path {
        match fs::remove_dir_all(git_wt_path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e).with_context(|| format!("Failed to remove git worktree {}", git_wt_path.display())),
        }
    }

    fs::remove_dir_all(&wt.path)
        .with_context(|| format!("Failed to remove {}", wt.path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ProjectType;

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
