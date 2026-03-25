mod app;
mod cleanup;
mod model;
mod scan;
mod ui;

use anyhow::Result;

fn main() -> Result<()> {
    let worktrees = scan::scan_worktrees()?;

    // Phase 1: just print to stdout for validation
    println!("Found {} worktrees:\n", worktrees.len());
    for wt in &worktrees {
        println!(
            "  {} | {:12} | {:20} | {:>8} ({:>8} artifacts) | {} | {}",
            wt.codex_id,
            wt.project_name,
            wt.display_branch(),
            model::WorktreeInfo::display_size(wt.total_size),
            model::WorktreeInfo::display_size(wt.artifact_size),
            wt.display_updated_at(),
            wt.display_thread(),
        );
    }

    let total: u64 = worktrees.iter().map(|w| w.total_size).sum();
    let total_artifacts: u64 = worktrees.iter().map(|w| w.artifact_size).sum();
    println!(
        "\nTotal: {} ({} artifacts)",
        model::WorktreeInfo::display_size(total),
        model::WorktreeInfo::display_size(total_artifacts),
    );

    Ok(())
}
