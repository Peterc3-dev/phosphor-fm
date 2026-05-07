use std::fs;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub is_executable: bool,
    pub is_hidden: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub permissions: u32,
    pub owner_uid: u32,
}

impl FileEntry {
    pub fn from_path(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().to_string();
        let is_hidden = name.starts_with('.');
        let symlink_meta = fs::symlink_metadata(path).ok()?;
        let is_symlink = symlink_meta.file_type().is_symlink();
        let meta = fs::metadata(path).ok().unwrap_or_else(|| symlink_meta.clone());
        let is_dir = meta.is_dir();
        let permissions = meta.permissions().mode();
        let is_executable = !is_dir && (permissions & 0o111 != 0);
        let size = if is_dir { 0 } else { meta.len() };
        let modified = meta.modified().ok();
        let owner_uid = meta.uid();

        Some(FileEntry {
            name,
            path: path.to_path_buf(),
            is_dir,
            is_symlink,
            is_executable,
            is_hidden,
            size,
            modified,
            permissions,
            owner_uid,
        })
    }

    pub fn type_indicator(&self) -> &str {
        if self.is_symlink {
            "~>"
        } else if self.is_dir {
            " /"
        } else if self.is_executable {
            " *"
        } else {
            "  "
        }
    }

    pub fn format_size(&self) -> String {
        if self.is_dir {
            return "<DIR>".to_string();
        }
        let size = self.size;
        if size < 1024 {
            format!("{}B", size)
        } else if size < 1024 * 1024 {
            format!("{:.1}K", size as f64 / 1024.0)
        } else if size < 1024 * 1024 * 1024 {
            format!("{:.1}M", size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1}G", size as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }

    pub fn format_modified(&self) -> String {
        match self.modified {
            Some(time) => {
                let duration = time
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default();
                let secs = duration.as_secs() as i64;
                let dt = chrono::DateTime::from_timestamp(secs, 0);
                match dt {
                    Some(dt) => dt.format("%Y-%m-%d %H:%M").to_string(),
                    None => "????-??-?? ??:??".to_string(),
                }
            }
            None => "????-??-?? ??:??".to_string(),
        }
    }

    pub fn format_permissions(&self) -> String {
        let mode = self.permissions;
        let mut s = String::with_capacity(10);
        s.push(if self.is_dir {
            'd'
        } else if self.is_symlink {
            'l'
        } else {
            '-'
        });
        for shift in [6, 3, 0] {
            let bits = (mode >> shift) & 0o7;
            s.push(if bits & 4 != 0 { 'r' } else { '-' });
            s.push(if bits & 2 != 0 { 'w' } else { '-' });
            s.push(if bits & 1 != 0 { 'x' } else { '-' });
        }
        s
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortMode {
    Name,
    Size,
    Date,
    Type,
}

impl SortMode {
    pub fn next(self) -> Self {
        match self {
            SortMode::Name => SortMode::Size,
            SortMode::Size => SortMode::Date,
            SortMode::Date => SortMode::Type,
            SortMode::Type => SortMode::Name,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SortMode::Name => "Name",
            SortMode::Size => "Size",
            SortMode::Date => "Date",
            SortMode::Type => "Type",
        }
    }
}

pub fn sort_entries(entries: &mut [FileEntry], mode: SortMode) {
    entries.sort_by(|a, b| {
        // Directories always first
        match (a.is_dir, b.is_dir) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }
        match mode {
            SortMode::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortMode::Size => a.size.cmp(&b.size).reverse(),
            SortMode::Date => {
                let a_t = a.modified.unwrap_or(SystemTime::UNIX_EPOCH);
                let b_t = b.modified.unwrap_or(SystemTime::UNIX_EPOCH);
                a_t.cmp(&b_t).reverse()
            }
            SortMode::Type => {
                let a_ext = a
                    .path
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                let b_ext = b
                    .path
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                a_ext.cmp(&b_ext).then_with(|| a.name.cmp(&b.name))
            }
        }
    });
}
