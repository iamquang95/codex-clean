use crate::app::{App, AppMode, PendingAction};
use crate::model::format_size;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(3),
    ])
    .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_table(f, app, chunks[1]);
    draw_help_bar(f, chunks[2]);

    match &app.mode {
        AppMode::Detail(idx) => draw_detail_popup(f, app, *idx),
        AppMode::Confirm(action) => draw_confirm_popup(f, app, action),
        AppMode::Normal => {}
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let total: u64 = app.worktrees.iter().map(|w| w.total_size).sum();
    let total_artifacts: u64 = app.worktrees.iter().map(|w| w.artifact_size).sum();
    let selected = app.selected_count();

    let mut info = format!(
        " {} worktrees  |  Total: {}  |  Artifacts: {}  |  Sort: {}",
        app.worktrees.len(),
        format_size(total),
        format_size(total_artifacts),
        app.sort_field.label(),
    );
    if selected > 0 {
        info.push_str(&format!("  |  Selected: {selected}"));
    }

    if let Some((msg, _)) = &app.message {
        info = format!(" {msg}");
    }

    let block = Block::default()
        .title(" Worktree Cleanup ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(info).block(block);
    f.render_widget(paragraph, area);
}

fn draw_table(f: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from(""),
        Cell::from("ID"),
        Cell::from("Project"),
        Cell::from("Branch"),
        Cell::from("Size"),
        Cell::from("Artifacts"),
        Cell::from("Updated"),
        Cell::from("Thread"),
    ])
    .style(Style::default().fg(Color::Yellow).bold())
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .worktrees
        .iter()
        .enumerate()
        .map(|(i, wt)| {
            let sel = if wt.selected { "●" } else { " " };
            let sz_color = size_color(wt.total_size);
            let art_color = size_color(wt.artifact_size);

            let style = if i == app.table_index {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(sel).style(Style::default().fg(Color::Green)),
                Cell::from(wt.codex_id.as_str()),
                Cell::from(wt.project_name.as_str()),
                Cell::from(wt.display_branch()),
                Cell::from(format_size(wt.total_size))
                    .style(Style::default().fg(sz_color)),
                Cell::from(format_size(wt.artifact_size))
                    .style(Style::default().fg(art_color)),
                Cell::from(wt.display_updated_at()),
                Cell::from(truncate_str(wt.display_thread(), 25)),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(2),
        Constraint::Length(6),
        Constraint::Length(14),
        Constraint::Length(18),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Min(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_widget(table, area);
}

fn draw_help_bar(f: &mut Frame, area: Rect) {
    let help = vec![
        Span::styled("[↑↓]", Style::default().fg(Color::Yellow)),
        Span::raw(" Navigate  "),
        Span::styled("[Space]", Style::default().fg(Color::Yellow)),
        Span::raw(" Select  "),
        Span::styled("[a]", Style::default().fg(Color::Yellow)),
        Span::raw(" All  "),
        Span::styled("[c]", Style::default().fg(Color::Yellow)),
        Span::raw(" Clean  "),
        Span::styled("[d]", Style::default().fg(Color::Yellow)),
        Span::raw(" Delete  "),
        Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
        Span::raw(" Details  "),
        Span::styled("[r]", Style::default().fg(Color::Yellow)),
        Span::raw(" Rescan  "),
        Span::styled("[q]", Style::default().fg(Color::Yellow)),
        Span::raw(" Quit"),
    ];

    let paragraph = Paragraph::new(Line::from(help))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(paragraph, area);
}

fn draw_detail_popup(f: &mut Frame, app: &App, idx: usize) {
    let Some(wt) = app.worktrees.get(idx) else {
        return;
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("ID:          ", Style::default().fg(Color::Yellow)),
            Span::raw(&wt.codex_id),
        ]),
        Line::from(vec![
            Span::styled("Project:     ", Style::default().fg(Color::Yellow)),
            Span::raw(&wt.project_name),
        ]),
        Line::from(vec![
            Span::styled("Type:        ", Style::default().fg(Color::Yellow)),
            Span::raw(wt.project_type.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Branch:      ", Style::default().fg(Color::Yellow)),
            Span::raw(wt.display_branch()),
        ]),
        Line::from(vec![
            Span::styled("Total Size:  ", Style::default().fg(Color::Yellow)),
            Span::raw(format_size(wt.total_size)),
        ]),
        Line::from(vec![
            Span::styled("Artifacts:   ", Style::default().fg(Color::Yellow)),
            Span::raw(format_size(wt.artifact_size)),
        ]),
        Line::from(vec![
            Span::styled("Updated:     ", Style::default().fg(Color::Yellow)),
            Span::raw(wt.display_updated_at()),
        ]),
        Line::from(vec![
            Span::styled("Thread:      ", Style::default().fg(Color::Yellow)),
            Span::raw(wt.display_thread()),
        ]),
        Line::from(vec![
            Span::styled("Thread ID:   ", Style::default().fg(Color::Yellow)),
            Span::raw(wt.thread_id.as_deref().unwrap_or("(none)")),
        ]),
        Line::from(vec![
            Span::styled("Path:        ", Style::default().fg(Color::Yellow)),
            Span::raw(wt.project_path.display().to_string()),
        ]),
        Line::from(vec![
            Span::styled("Git Worktree:", Style::default().fg(Color::Yellow)),
            Span::raw(
                wt.git_worktree_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "(none)".to_string()),
            ),
        ]),
        Line::default(),
        Line::from(Span::styled(
            "Press [Esc] to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let popup_area = centered_rect(70, 60, f.area());
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Worktree Details ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, popup_area);
}

fn draw_confirm_popup(f: &mut Frame, app: &App, action: &PendingAction) {
    let selected_count = app.selected_count();
    let selected_size: u64 = app
        .worktrees
        .iter()
        .filter(|w| w.selected)
        .map(|w| match action {
            PendingAction::CleanArtifacts => w.artifact_size,
            PendingAction::DeleteWorktrees => w.total_size,
        })
        .sum();

    let action_text = match action {
        PendingAction::CleanArtifacts => "CLEAN ARTIFACTS",
        PendingAction::DeleteWorktrees => "DELETE WORKTREES",
    };

    let lines = vec![
        Line::default(),
        Line::from(format!(
            "Are you sure you want to {action_text} for {selected_count} worktree(s)?"
        )),
        Line::from(format!(
            "This will free approximately {}.",
            format_size(selected_size)
        )),
        Line::default(),
        Line::from(vec![
            Span::styled("[y]", Style::default().fg(Color::Green).bold()),
            Span::raw(" Yes   "),
            Span::styled("[n]", Style::default().fg(Color::Red).bold()),
            Span::raw(" No"),
        ]),
    ];

    let popup_area = centered_rect(50, 30, f.area());
    f.render_widget(Clear, popup_area);

    let border_color = match action {
        PendingAction::CleanArtifacts => Color::Yellow,
        PendingAction::DeleteWorktrees => Color::Red,
    };

    let block = Block::default()
        .title(format!(" Confirm {action_text} "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(block);
    f.render_widget(paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
}

fn size_color(bytes: u64) -> Color {
    if bytes >= 1_073_741_824 {
        Color::Red
    } else if bytes >= 104_857_600 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let end = s
            .char_indices()
            .nth(max - 3)
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        format!("{}...", &s[..end])
    }
}
