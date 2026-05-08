use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{ActivePane, App, InputMode, RightMode};
use crate::ops;
use crate::pane::Pane;

// Phosphor green palette
const GREEN: Color = Color::Rgb(0, 255, 200);
const DIM_GREEN: Color = Color::Rgb(0, 128, 100);
const BRIGHT_GREEN: Color = Color::Rgb(160, 255, 230);
const CYAN: Color = Color::Rgb(0, 220, 200);
const YELLOW: Color = Color::Rgb(200, 200, 0);
const BG: Color = Color::Rgb(0, 8, 0);
const BG_HIGHLIGHT: Color = Color::Rgb(0, 30, 5);
const BG_SELECTED: Color = Color::Rgb(0, 50, 10);
const BORDER_ACTIVE: Color = GREEN;
const BORDER_INACTIVE: Color = DIM_GREEN;
const STATUS_BG: Color = Color::Rgb(0, 20, 0);

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Main layout: header + body + status + input
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(5),   // panes
            Constraint::Length(2), // info bar
            Constraint::Length(1), // status / input
        ])
        .split(size);

    draw_title_bar(f, chunks[0]);
    draw_panes(f, app, chunks[1]);
    draw_info_bar(f, app, chunks[2]);
    draw_status_bar(f, app, chunks[3]);
}

fn draw_title_bar(f: &mut Frame, area: Rect) {
    let title = Line::from(vec![
        Span::styled(
            " PHOSPHOR-FM ",
            Style::default()
                .fg(BG)
                .bg(GREEN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " dual-pane file manager ",
            Style::default().fg(DIM_GREEN).bg(BG),
        ),
        Span::styled(
            " q:quit Tab:switch /:search s:sort .:hidden p:preview ",
            Style::default().fg(DIM_GREEN).bg(BG),
        ),
    ]);
    let p = Paragraph::new(title).style(Style::default().bg(BG));
    f.render_widget(p, area);
}

fn draw_panes(f: &mut Frame, app: &mut App, area: Rect) {
    let pane_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left_active = app.active == ActivePane::Left;
    draw_file_pane(f, &mut app.left, pane_chunks[0], left_active, "LEFT");

    match app.right_mode {
        RightMode::Directory => {
            draw_file_pane(f, &mut app.right, pane_chunks[1], !left_active, "RIGHT");
        }
        RightMode::Preview => {
            draw_preview_pane(f, app, pane_chunks[1]);
        }
    }
}

fn draw_file_pane(f: &mut Frame, pane: &mut Pane, area: Rect, active: bool, label: &str) {
    let border_color = if active {
        BORDER_ACTIVE
    } else {
        BORDER_INACTIVE
    };

    let path_str = pane.path.to_string_lossy();
    let title = format!(
        " {} \u{2502} {} \u{2502} {} files \u{2502} sort:{} ",
        label,
        truncate_path(&path_str, area.width.saturating_sub(40) as usize),
        pane.file_count(),
        pane.sort_mode.label()
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            title,
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_height = inner.height as usize;
    pane.ensure_visible(visible_height);

    let filtered = pane.filtered_entries();
    let mut lines: Vec<Line> = Vec::new();

    for (display_idx, &(real_idx, entry)) in filtered.iter().enumerate() {
        if display_idx < pane.scroll_offset {
            continue;
        }
        if lines.len() >= visible_height {
            break;
        }

        let is_cursor = display_idx == pane.cursor && active;
        let is_selected = pane.selected.contains(&real_idx);

        let bg = if is_cursor && is_selected {
            BG_SELECTED
        } else if is_cursor {
            BG_HIGHLIGHT
        } else if is_selected {
            BG_SELECTED
        } else {
            BG
        };

        let name_color = if entry.is_symlink {
            YELLOW
        } else if entry.is_dir {
            BRIGHT_GREEN
        } else if entry.is_executable {
            CYAN
        } else {
            DIM_GREEN
        };

        let sel_marker = if is_selected { "\u{25cf} " } else { "  " };
        let indicator = entry.type_indicator();

        // Compute available width for filename
        let meta_width = 26; // size(8) + space + date(16) + space
        let prefix_width = 4; // sel_marker(2) + indicator(2)
        let avail = (inner.width as usize).saturating_sub(prefix_width + meta_width);
        let display_name = truncate_str(&entry.name, avail);
        let padding = avail.saturating_sub(display_name.len());

        let size_str = format!("{:>8}", entry.format_size());
        let date_str = entry.format_modified();

        let line = Line::from(vec![
            Span::styled(
                sel_marker.to_string(),
                Style::default().fg(GREEN).bg(bg),
            ),
            Span::styled(
                indicator.to_string(),
                Style::default().fg(name_color).bg(bg),
            ),
            Span::styled(
                format!("{}{}", display_name, " ".repeat(padding)),
                Style::default()
                    .fg(name_color)
                    .bg(bg)
                    .add_modifier(if is_cursor {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
            Span::styled(
                format!(" {}", size_str),
                Style::default().fg(DIM_GREEN).bg(bg),
            ),
            Span::styled(
                format!(" {}", date_str),
                Style::default().fg(DIM_GREEN).bg(bg),
            ),
        ]);

        lines.push(line);
    }

    // Fill remaining height with empty lines
    while lines.len() < visible_height {
        lines.push(Line::from(Span::styled(
            " ".repeat(inner.width as usize),
            Style::default().bg(BG),
        )));
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

fn draw_preview_pane(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_INACTIVE))
        .title(Span::styled(
            " PREVIEW ",
            Style::default()
                .fg(BORDER_INACTIVE)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let preview_lines = if let Some(entry) = app.left.current_entry() {
        ops::file_preview(&entry.path, inner.height as usize)
    } else {
        vec!["No file selected".to_string()]
    };

    let lines: Vec<Line> = preview_lines
        .iter()
        .map(|l| {
            Line::from(Span::styled(
                l.clone(),
                Style::default().fg(DIM_GREEN).bg(BG),
            ))
        })
        .collect();

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(paragraph, inner);
}

fn draw_info_bar(f: &mut Frame, app: &App, area: Rect) {
    let pane = app.active_pane();
    let path_str = pane.path.to_string_lossy().to_string();

    let file_info = if let Some(entry) = pane.current_entry() {
        format!(
            "{} uid:{} {} {} {}",
            entry.format_permissions(),
            entry.owner_uid,
            entry.format_size(),
            entry.format_modified(),
            entry.name,
        )
    } else {
        String::new()
    };

    let sel_info = if pane.selected_count() > 0 {
        format!(" | {} selected", pane.selected_count())
    } else {
        String::new()
    };

    let progress_info = if let Some((done, total)) = app.progress {
        format!(" | Progress: {}/{}", done, total)
    } else {
        String::new()
    };

    let filter_info = if !pane.filter.is_empty() {
        format!(" | filter: '{}'", pane.filter)
    } else {
        String::new()
    };

    let line1 = Line::from(vec![
        Span::styled(
            format!(" {} ", path_str),
            Style::default()
                .fg(GREEN)
                .bg(STATUS_BG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "| {} files{}{}{}",
                pane.file_count(),
                sel_info,
                progress_info,
                filter_info,
            ),
            Style::default().fg(DIM_GREEN).bg(STATUS_BG),
        ),
    ]);

    let line2 = Line::from(Span::styled(
        format!(" {}", file_info),
        Style::default().fg(DIM_GREEN).bg(STATUS_BG),
    ));

    let paragraph = Paragraph::new(vec![line1, line2]).style(Style::default().bg(STATUS_BG));
    f.render_widget(paragraph, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let content = match app.input_mode {
        InputMode::Normal => {
            if app.status_msg.is_empty() {
                Line::from(Span::styled(
                    " j/k:nav Enter:open Bksp:up c:copy m:move d:del r:rename n/N:new Space:select",
                    Style::default().fg(DIM_GREEN).bg(BG),
                ))
            } else {
                Line::from(Span::styled(
                    format!(" {}", app.status_msg),
                    Style::default()
                        .fg(YELLOW)
                        .bg(BG)
                        .add_modifier(Modifier::BOLD),
                ))
            }
        }
        InputMode::Search => Line::from(vec![
            Span::styled(
                " /search: ",
                Style::default()
                    .fg(GREEN)
                    .bg(BG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &app.input_buffer,
                Style::default().fg(BRIGHT_GREEN).bg(BG),
            ),
            Span::styled(
                "\u{2588}",
                Style::default().fg(GREEN).bg(BG),
            ),
        ]),
        InputMode::GoPath => Line::from(vec![
            Span::styled(
                " go to: ",
                Style::default()
                    .fg(GREEN)
                    .bg(BG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &app.input_buffer,
                Style::default().fg(BRIGHT_GREEN).bg(BG),
            ),
            Span::styled(
                "\u{2588}",
                Style::default().fg(GREEN).bg(BG),
            ),
        ]),
        InputMode::Rename => Line::from(vec![
            Span::styled(
                " rename: ",
                Style::default()
                    .fg(GREEN)
                    .bg(BG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &app.input_buffer,
                Style::default().fg(BRIGHT_GREEN).bg(BG),
            ),
            Span::styled(
                "\u{2588}",
                Style::default().fg(GREEN).bg(BG),
            ),
        ]),
        InputMode::NewFile => Line::from(vec![
            Span::styled(
                " new file: ",
                Style::default()
                    .fg(GREEN)
                    .bg(BG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &app.input_buffer,
                Style::default().fg(BRIGHT_GREEN).bg(BG),
            ),
            Span::styled(
                "\u{2588}",
                Style::default().fg(GREEN).bg(BG),
            ),
        ]),
        InputMode::NewDir => Line::from(vec![
            Span::styled(
                " new directory: ",
                Style::default()
                    .fg(GREEN)
                    .bg(BG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &app.input_buffer,
                Style::default().fg(BRIGHT_GREEN).bg(BG),
            ),
            Span::styled(
                "\u{2588}",
                Style::default().fg(GREEN).bg(BG),
            ),
        ]),
        InputMode::ConfirmDelete => Line::from(Span::styled(
            format!(" {}", app.status_msg),
            Style::default()
                .fg(Color::Rgb(255, 80, 80))
                .bg(BG)
                .add_modifier(Modifier::BOLD),
        )),
    };

    let paragraph = Paragraph::new(content).style(Style::default().bg(BG));
    f.render_widget(paragraph, area);
}

fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len || max_len < 4 {
        return path.to_string();
    }
    format!("...{}", &path[path.len() - (max_len - 3)..])
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len < 4 {
        s[..max_len].to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
