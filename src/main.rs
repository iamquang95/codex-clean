mod app;
mod cleanup;
mod model;
mod scan;
mod ui;

use anyhow::{Context, Result};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::path::PathBuf;

fn resolve_codex_home() -> Result<PathBuf> {
    let args: Vec<String> = std::env::args().collect();

    // --help / -h
    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!("Usage: worktree-cleanup [--codex-home <path>]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --codex-home <path>  Path to Codex home directory");
        eprintln!("                       (default: $CODEX_HOME or ~/.codex)");
        std::process::exit(0);
    }

    // --codex-home <path>
    if let Some(pos) = args.iter().position(|a| a == "--codex-home") {
        let path = args
            .get(pos + 1)
            .context("--codex-home requires a path argument")?;
        return Ok(PathBuf::from(path));
    }

    // $CODEX_HOME
    if let Ok(val) = std::env::var("CODEX_HOME") {
        if !val.is_empty() {
            return Ok(PathBuf::from(val));
        }
    }

    // Default: ~/.codex
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    Ok(home.join(".codex"))
}

fn main() -> Result<()> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stderr(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    let codex_home = resolve_codex_home()?;
    let worktrees = scan::scan_worktrees(&codex_home)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new(worktrees, codex_home);
    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}
