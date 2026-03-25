use crate::model::WorktreeInfo;
use crate::ui;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Detail(usize),
    Confirm(PendingAction),
}

#[derive(Debug, PartialEq, Eq)]
pub enum PendingAction {
    CleanArtifacts,
    DeleteWorktrees,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Size,
    Name,
    Updated,
    Artifacts,
}

impl SortField {
    fn next(self) -> Self {
        match self {
            Self::Size => Self::Name,
            Self::Name => Self::Artifacts,
            Self::Artifacts => Self::Updated,
            Self::Updated => Self::Size,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Size => "Size",
            Self::Name => "Name",
            Self::Artifacts => "Artifacts",
            Self::Updated => "Updated",
        }
    }
}

pub struct App {
    pub worktrees: Vec<WorktreeInfo>,
    pub table_index: usize,
    pub mode: AppMode,
    pub message: Option<(String, Instant)>,
    pub should_quit: bool,
    pub sort_field: SortField,
}

impl App {
    pub fn new(worktrees: Vec<WorktreeInfo>) -> Self {
        let mut app = Self {
            worktrees,
            table_index: 0,
            mode: AppMode::Normal,
            message: None,
            should_quit: false,
            sort_field: SortField::Size,
        };
        app.apply_sort();
        app
    }

    pub fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| ui::draw(f, self))?;

            if event::poll(Duration::from_millis(250))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match &self.mode {
                        AppMode::Normal => self.handle_normal_key(key.code),
                        AppMode::Detail(_) => self.handle_detail_key(key.code),
                        AppMode::Confirm(_) => self.handle_confirm_key(key.code),
                    }
                }
            }

            // Clear flash messages after 3 seconds
            if let Some((_, at)) = &self.message {
                if at.elapsed() > Duration::from_secs(3) {
                    self.message = None;
                }
            }
        }
        Ok(())
    }

    fn handle_normal_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Char(' ') => self.toggle_current(),
            KeyCode::Char('a') => self.toggle_all(),
            KeyCode::Enter => {
                if !self.worktrees.is_empty() {
                    self.mode = AppMode::Detail(self.table_index);
                }
            }
            KeyCode::Char('c') => {
                if self.has_selected() {
                    self.mode = AppMode::Confirm(PendingAction::CleanArtifacts);
                } else {
                    self.flash("No worktrees selected. Use [Space] to select.");
                }
            }
            KeyCode::Char('d') => {
                if self.has_selected() {
                    self.mode = AppMode::Confirm(PendingAction::DeleteWorktrees);
                } else {
                    self.flash("No worktrees selected. Use [Space] to select.");
                }
            }
            KeyCode::Char('s') => {
                self.cycle_sort();
            }
            KeyCode::Char('r') => {
                self.rescan();
            }
            _ => {}
        }
    }

    fn handle_detail_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                self.mode = AppMode::Normal;
            }
            _ => {}
        }
    }

    fn handle_confirm_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let action = std::mem::replace(&mut self.mode, AppMode::Normal);
                if let AppMode::Confirm(pending) = action {
                    self.execute_action(pending);
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.mode = AppMode::Normal;
            }
            _ => {}
        }
    }

    fn execute_action(&mut self, action: PendingAction) {
        let selected: Vec<usize> = self
            .worktrees
            .iter()
            .enumerate()
            .filter(|(_, w)| w.selected)
            .map(|(i, _)| i)
            .collect();

        match action {
            PendingAction::CleanArtifacts => {
                let mut total_freed: u64 = 0;
                let mut errors = 0;
                for &idx in &selected {
                    match crate::cleanup::clean_artifacts(&self.worktrees[idx]) {
                        Ok(freed) => total_freed += freed,
                        Err(_) => errors += 1,
                    }
                }
                if errors > 0 {
                    self.flash(format!(
                        "Cleaned {} worktrees, freed {}. {} errors.",
                        selected.len() - errors,
                        WorktreeInfo::display_size(total_freed),
                        errors,
                    ));
                } else {
                    self.flash(format!(
                        "Cleaned {} worktrees, freed {}.",
                        selected.len(),
                        WorktreeInfo::display_size(total_freed),
                    ));
                }
                self.rescan();
            }
            PendingAction::DeleteWorktrees => {
                let mut deleted = 0;
                let mut errors = 0;
                // Delete in reverse order so indices stay valid
                let mut selected = selected;
                selected.sort_unstable_by(|a, b| b.cmp(a));
                for idx in selected {
                    match crate::cleanup::delete_worktree(&self.worktrees[idx]) {
                        Ok(()) => {
                            self.worktrees.remove(idx);
                            deleted += 1;
                        }
                        Err(_) => errors += 1,
                    }
                }
                if self.table_index >= self.worktrees.len() && !self.worktrees.is_empty() {
                    self.table_index = self.worktrees.len() - 1;
                }
                if errors > 0 {
                    self.flash(format!("Deleted {deleted} worktrees. {errors} errors."));
                } else {
                    self.flash(format!("Deleted {deleted} worktrees."));
                }
            }
        }
        // Deselect all after action
        for wt in &mut self.worktrees {
            wt.selected = false;
        }
    }

    fn move_up(&mut self) {
        if !self.worktrees.is_empty() && self.table_index > 0 {
            self.table_index -= 1;
        }
    }

    fn move_down(&mut self) {
        if !self.worktrees.is_empty() && self.table_index < self.worktrees.len() - 1 {
            self.table_index += 1;
        }
    }

    fn toggle_current(&mut self) {
        if let Some(wt) = self.worktrees.get_mut(self.table_index) {
            wt.selected = !wt.selected;
        }
    }

    fn toggle_all(&mut self) {
        let all_selected = self.worktrees.iter().all(|w| w.selected);
        for wt in &mut self.worktrees {
            wt.selected = !all_selected;
        }
    }

    fn has_selected(&self) -> bool {
        self.worktrees.iter().any(|w| w.selected)
    }

    pub fn selected_count(&self) -> usize {
        self.worktrees.iter().filter(|w| w.selected).count()
    }

    fn flash(&mut self, msg: impl Into<String>) {
        self.message = Some((msg.into(), Instant::now()));
    }

    fn cycle_sort(&mut self) {
        self.sort_field = self.sort_field.next();
        self.apply_sort();
        self.flash(format!("Sorted by {}", self.sort_field.label()));
    }

    fn apply_sort(&mut self) {
        match self.sort_field {
            SortField::Size => self.worktrees.sort_by(|a, b| b.total_size.cmp(&a.total_size)),
            SortField::Name => self.worktrees.sort_by(|a, b| a.project_name.cmp(&b.project_name)),
            SortField::Artifacts => self
                .worktrees
                .sort_by(|a, b| b.artifact_size.cmp(&a.artifact_size)),
            SortField::Updated => self.worktrees.sort_by(|a, b| {
                let a_ts = a.updated_at.as_deref().unwrap_or("");
                let b_ts = b.updated_at.as_deref().unwrap_or("");
                b_ts.cmp(a_ts)
            }),
        }
    }

    fn rescan(&mut self) {
        match crate::scan::scan_worktrees() {
            Ok(worktrees) => {
                self.worktrees = worktrees;
                self.apply_sort();
                if self.table_index >= self.worktrees.len() {
                    self.table_index = self.worktrees.len().saturating_sub(1);
                }
                self.flash("Rescanned worktrees.");
            }
            Err(e) => {
                self.flash(format!("Rescan failed: {e}"));
            }
        }
    }
}
