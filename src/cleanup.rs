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
