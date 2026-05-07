use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use crate::app::{App, InputMode};
use crate::ops;

pub fn handle_events(app: &mut App) -> std::io::Result<bool> {
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Normal => handle_normal(app, key),
                InputMode::Search => handle_search(app, key),
                InputMode::GoPath => handle_go_path(app, key),
                InputMode::Rename => handle_rename(app, key),
                InputMode::NewFile => handle_new_file(app, key),
                InputMode::NewDir => handle_new_dir(app, key),
                InputMode::ConfirmDelete => handle_confirm_delete(app, key),
            }
        }
    }
    Ok(app.should_quit)
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    // Ctrl-C or q to quit
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match key.code {
        KeyCode::Char('q') => app.should_quit = true,

        // Navigation
        KeyCode::Char('j') | KeyCode::Down => app.active_pane_mut().move_cursor(1),
        KeyCode::Char('k') | KeyCode::Up => app.active_pane_mut().move_cursor(-1),
        KeyCode::Char('G') => {
            let len = app.active_pane().filtered_entries().len();
            if len > 0 {
                app.active_pane_mut().cursor = len - 1;
            }
        }
        KeyCode::PageDown => app.active_pane_mut().move_cursor(20),
        KeyCode::PageUp => app.active_pane_mut().move_cursor(-20),
        KeyCode::Enter => {
            if let Some(entry) = app.active_pane().current_entry().cloned() {
                if entry.is_dir {
                    app.active_pane_mut().enter_dir();
                }
                // For files, if in preview mode, no action needed — preview updates automatically
            }
        }
        KeyCode::Backspace => app.active_pane_mut().go_parent(),
        KeyCode::Char('~') => app.active_pane_mut().go_home(),
        KeyCode::Tab => app.toggle_pane(),

        // Search
        KeyCode::Char('/') => app.start_search(),

        // Go to path
        KeyCode::Char('g') => app.start_go_path(),

        // File operations
        KeyCode::Char('c') => do_copy(app),
        KeyCode::Char('m') => do_move(app),
        KeyCode::Char('d') => app.start_delete_confirm(),
        KeyCode::Char('r') => app.start_rename(),
        KeyCode::Char('n') => app.start_new_file(),
        KeyCode::Char('N') => app.start_new_dir(),

        // Selection
        KeyCode::Char(' ') => {
            app.active_pane_mut().toggle_select();
            app.active_pane_mut().move_cursor(1);
        }
        KeyCode::Char('a') => app.active_pane_mut().select_all(),

        // View toggles
        KeyCode::Char('p') => app.toggle_preview(),
        KeyCode::Char('.') => app.active_pane_mut().toggle_hidden(),
        KeyCode::Char('s') => app.active_pane_mut().cycle_sort(),

        KeyCode::Esc => app.set_status(String::new()),

        _ => {}
    }
}

fn handle_search(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter | KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            // Keep the filter applied on Enter, clear on Esc
            if key.code == KeyCode::Esc {
                app.active_pane_mut().filter.clear();
                app.input_buffer.clear();
            }
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
            app.active_pane_mut().filter = app.input_buffer.clone();
            app.active_pane_mut().cursor = 0;
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
            app.active_pane_mut().filter = app.input_buffer.clone();
            app.active_pane_mut().cursor = 0;
        }
        _ => {}
    }
}

fn handle_go_path(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let path = std::path::PathBuf::from(&app.input_buffer);
            if path.is_dir() {
                app.active_pane_mut().load_dir(&path);
                app.set_status(format!("Navigated to {}", path.display()));
            } else {
                app.set_status(format!("Not a directory: {}", app.input_buffer));
            }
            app.input_mode = InputMode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Esc => app.cancel_input(),
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
}

fn handle_rename(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let new_name = app.input_buffer.clone();
            if let Some(entry) = app.active_pane().current_entry().cloned() {
                match ops::rename_entry(&entry.path, &new_name) {
                    Ok(_) => {
                        app.set_status(format!("Renamed to '{}'", new_name));
                        app.active_pane_mut().reload();
                    }
                    Err(e) => app.set_status(format!("Rename failed: {}", e)),
                }
            }
            app.input_mode = InputMode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Esc => app.cancel_input(),
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
}

fn handle_new_file(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let name = app.input_buffer.clone();
            let dir = app.active_pane().path.clone();
            match ops::create_file(&dir, &name) {
                Ok(_) => {
                    app.set_status(format!("Created file '{}'", name));
                    app.active_pane_mut().reload();
                }
                Err(e) => app.set_status(format!("Create file failed: {}", e)),
            }
            app.input_mode = InputMode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Esc => app.cancel_input(),
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
}

fn handle_new_dir(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let name = app.input_buffer.clone();
            let dir = app.active_pane().path.clone();
            match ops::create_directory(&dir, &name) {
                Ok(_) => {
                    app.set_status(format!("Created directory '{}'", name));
                    app.active_pane_mut().reload();
                }
                Err(e) => app.set_status(format!("Create directory failed: {}", e)),
            }
            app.input_mode = InputMode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Esc => app.cancel_input(),
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
}

fn handle_confirm_delete(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let entries = app.active_pane().action_entries();
            let total = entries.len();
            let mut done = 0;
            let mut errors = Vec::new();
            for entry in &entries {
                match ops::delete_entry(&entry.path) {
                    Ok(()) => done += 1,
                    Err(e) => errors.push(format!("{}: {}", entry.name, e)),
                }
                app.progress = Some((done, total));
            }
            app.progress = None;
            if errors.is_empty() {
                app.set_status(format!("Deleted {} item(s)", done));
            } else {
                app.set_status(format!(
                    "Deleted {}/{}, errors: {}",
                    done,
                    total,
                    errors.join(", ")
                ));
            }
            app.active_pane_mut().reload();
            app.input_mode = InputMode::Normal;
        }
        _ => {
            app.set_status("Delete cancelled".to_string());
            app.input_mode = InputMode::Normal;
        }
    }
}

fn do_copy(app: &mut App) {
    let entries = app.active_pane().action_entries();
    if entries.is_empty() {
        app.set_status("Nothing to copy".to_string());
        return;
    }
    let dest_dir = app.other_pane().path.clone();
    let total = entries.len();
    let mut done = 0;
    let mut errors = Vec::new();
    for entry in &entries {
        match ops::copy_entry(&entry.path, &dest_dir) {
            Ok(()) => done += 1,
            Err(e) => errors.push(format!("{}: {}", entry.name, e)),
        }
        app.progress = Some((done, total));
    }
    app.progress = None;
    if errors.is_empty() {
        app.set_status(format!("Copied {} item(s) to {}", done, dest_dir.display()));
    } else {
        app.set_status(format!(
            "Copied {}/{}, errors: {}",
            done,
            total,
            errors.join(", ")
        ));
    }
    // Reload both panes
    app.left.reload();
    app.right.reload();
    app.active_pane_mut().selected.clear();
}

fn do_move(app: &mut App) {
    let entries = app.active_pane().action_entries();
    if entries.is_empty() {
        app.set_status("Nothing to move".to_string());
        return;
    }
    let dest_dir = app.other_pane().path.clone();
    let total = entries.len();
    let mut done = 0;
    let mut errors = Vec::new();
    for entry in &entries {
        match ops::move_entry(&entry.path, &dest_dir) {
            Ok(()) => done += 1,
            Err(e) => errors.push(format!("{}: {}", entry.name, e)),
        }
        app.progress = Some((done, total));
    }
    app.progress = None;
    if errors.is_empty() {
        app.set_status(format!("Moved {} item(s) to {}", done, dest_dir.display()));
    } else {
        app.set_status(format!(
            "Moved {}/{}, errors: {}",
            done,
            total,
            errors.join(", ")
        ));
    }
    app.left.reload();
    app.right.reload();
    app.active_pane_mut().selected.clear();
}
