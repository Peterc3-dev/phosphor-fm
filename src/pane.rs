use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::entry::{sort_entries, FileEntry, SortMode};

#[derive(Debug)]
pub struct Pane {
    pub path: PathBuf,
    pub entries: Vec<FileEntry>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub selected: HashSet<usize>,
    pub show_hidden: bool,
    pub sort_mode: SortMode,
    pub filter: String,
}

impl Pane {
    pub fn new(path: PathBuf) -> Self {
        let mut pane = Pane {
            path: path.clone(),
            entries: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            selected: HashSet::new(),
            show_hidden: false,
            sort_mode: SortMode::Name,
            filter: String::new(),
        };
        pane.load_dir(&path);
        pane
    }

    pub fn load_dir(&mut self, path: &Path) {
        let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        self.path = canonical;
        self.entries.clear();
        self.selected.clear();
        self.filter.clear();

        if let Ok(rd) = fs::read_dir(&self.path) {
            for entry in rd.flatten() {
                if let Some(fe) = FileEntry::from_path(&entry.path()) {
                    if !self.show_hidden && fe.is_hidden {
                        continue;
                    }
                    self.entries.push(fe);
                }
            }
        }

        sort_entries(&mut self.entries, self.sort_mode);
        self.cursor = 0;
        self.scroll_offset = 0;
    }

    pub fn reload(&mut self) {
        let path = self.path.clone();
        let old_cursor = self.cursor;
        self.load_dir(&path);
        self.cursor = old_cursor.min(self.entries.len().saturating_sub(1));
    }

    pub fn filtered_entries(&self) -> Vec<(usize, &FileEntry)> {
        if self.filter.is_empty() {
            self.entries.iter().enumerate().collect()
        } else {
            let lower = self.filter.to_lowercase();
            self.entries
                .iter()
                .enumerate()
                .filter(|(_, e)| e.name.to_lowercase().contains(&lower))
                .collect()
        }
    }

    pub fn current_entry(&self) -> Option<&FileEntry> {
        let filtered = self.filtered_entries();
        filtered.get(self.cursor).map(|(_, e)| *e)
    }

    pub fn move_cursor(&mut self, delta: isize) {
        let len = self.filtered_entries().len();
        if len == 0 {
            self.cursor = 0;
            return;
        }
        if delta < 0 {
            self.cursor = self.cursor.saturating_sub((-delta) as usize);
        } else {
            self.cursor = (self.cursor + delta as usize).min(len - 1);
        }
    }

    pub fn ensure_visible(&mut self, height: usize) {
        if height == 0 {
            return;
        }
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + height {
            self.scroll_offset = self.cursor - height + 1;
        }
    }

    pub fn enter_dir(&mut self) {
        if let Some(entry) = self.current_entry().cloned() {
            if entry.is_dir {
                self.load_dir(&entry.path);
            }
        }
    }

    pub fn go_parent(&mut self) {
        if let Some(parent) = self.path.parent().map(|p| p.to_path_buf()) {
            let current_name = self
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string());
            self.load_dir(&parent);
            // Try to position cursor on the directory we came from
            if let Some(name) = current_name {
                if let Some(pos) = self.entries.iter().position(|e| e.name == name) {
                    self.cursor = pos;
                }
            }
        }
    }

    pub fn go_home(&mut self) {
        if let Some(home) = dirs_home() {
            self.load_dir(&home);
        }
    }

    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.reload();
    }

    pub fn cycle_sort(&mut self) {
        self.sort_mode = self.sort_mode.next();
        self.reload();
    }

    pub fn toggle_select(&mut self) {
        let filtered = self.filtered_entries();
        if let Some(&(real_idx, _)) = filtered.get(self.cursor) {
            if self.selected.contains(&real_idx) {
                self.selected.remove(&real_idx);
            } else {
                self.selected.insert(real_idx);
            }
        }
    }

    pub fn select_all(&mut self) {
        let filtered = self.filtered_entries();
        if self.selected.len() == filtered.len() {
            self.selected.clear();
        } else {
            self.selected = filtered.iter().map(|(idx, _)| *idx).collect();
        }
    }

    pub fn selected_entries(&self) -> Vec<FileEntry> {
        self.selected
            .iter()
            .filter_map(|&idx| self.entries.get(idx).cloned())
            .collect()
    }

    /// Returns the single current entry if nothing is selected, otherwise returns all selected entries.
    pub fn action_entries(&self) -> Vec<FileEntry> {
        let sel = self.selected_entries();
        if sel.is_empty() {
            self.current_entry().cloned().into_iter().collect()
        } else {
            sel
        }
    }

    pub fn file_count(&self) -> usize {
        self.entries.len()
    }

    pub fn selected_count(&self) -> usize {
        self.selected.len()
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}
