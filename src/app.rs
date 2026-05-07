use std::path::PathBuf;

use crate::pane::Pane;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightMode {
    Directory,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    GoPath,
    Rename,
    NewFile,
    NewDir,
    ConfirmDelete,
}

#[derive(Debug)]
pub struct App {
    pub left: Pane,
    pub right: Pane,
    pub active: ActivePane,
    pub right_mode: RightMode,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub status_msg: String,
    pub should_quit: bool,
    pub progress: Option<(usize, usize)>, // (done, total)
}

impl App {
    pub fn new(start_path: PathBuf) -> Self {
        let left = Pane::new(start_path.clone());
        let right = Pane::new(start_path);

        App {
            left,
            right,
            active: ActivePane::Left,
            right_mode: RightMode::Directory,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            status_msg: String::new(),
            should_quit: false,
            progress: None,
        }
    }

    pub fn active_pane(&self) -> &Pane {
        match self.active {
            ActivePane::Left => &self.left,
            ActivePane::Right => &self.right,
        }
    }

    pub fn active_pane_mut(&mut self) -> &mut Pane {
        match self.active {
            ActivePane::Left => &mut self.left,
            ActivePane::Right => &mut self.right,
        }
    }

    pub fn other_pane(&self) -> &Pane {
        match self.active {
            ActivePane::Left => &self.right,
            ActivePane::Right => &self.left,
        }
    }

    pub fn toggle_pane(&mut self) {
        // Only toggle if right pane is in directory mode
        if self.right_mode == RightMode::Directory {
            self.active = match self.active {
                ActivePane::Left => ActivePane::Right,
                ActivePane::Right => ActivePane::Left,
            };
        }
    }

    pub fn toggle_preview(&mut self) {
        self.right_mode = match self.right_mode {
            RightMode::Directory => {
                self.active = ActivePane::Left;
                RightMode::Preview
            }
            RightMode::Preview => RightMode::Directory,
        };
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_msg = msg;
    }

    pub fn start_search(&mut self) {
        self.input_mode = InputMode::Search;
        self.input_buffer.clear();
    }

    pub fn start_go_path(&mut self) {
        self.input_mode = InputMode::GoPath;
        self.input_buffer = self.active_pane().path.to_string_lossy().to_string();
    }

    pub fn start_rename(&mut self) {
        let name = self.active_pane().current_entry().map(|e| e.name.clone());
        if let Some(name) = name {
            self.input_mode = InputMode::Rename;
            self.input_buffer = name;
        }
    }

    pub fn start_new_file(&mut self) {
        self.input_mode = InputMode::NewFile;
        self.input_buffer.clear();
    }

    pub fn start_new_dir(&mut self) {
        self.input_mode = InputMode::NewDir;
        self.input_buffer.clear();
    }

    pub fn start_delete_confirm(&mut self) {
        let entries = self.active_pane().action_entries();
        if entries.is_empty() {
            self.set_status("Nothing to delete".to_string());
            return;
        }
        let names: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
        let msg = if names.len() == 1 {
            format!("Delete '{}'? [y/N]", names[0])
        } else {
            format!("Delete {} items? [y/N]", names.len())
        };
        self.set_status(msg);
        self.input_mode = InputMode::ConfirmDelete;
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
        self.active_pane_mut().filter.clear();
        self.set_status(String::new());
    }
}
