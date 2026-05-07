use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Copy a file or directory recursively to dest_dir, preserving the name.
/// Uses symlink_metadata to avoid following symlinks into unexpected targets.
pub fn copy_entry(src: &Path, dest_dir: &Path) -> io::Result<()> {
    let name = src
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no filename"))?;
    let dest = dest_dir.join(name);
    let meta = fs::symlink_metadata(src)?;
    if meta.is_dir() {
        copy_dir_recursive(src, &dest)
    } else {
        fs::copy(src, &dest)?;
        Ok(())
    }
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let meta = fs::symlink_metadata(&src_path)?;
        if meta.is_symlink() {
            let target = fs::read_link(&src_path)?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&target, &dest_path)?;
        } else if meta.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

/// Move (rename) a file or directory to dest_dir, preserving the name.
pub fn move_entry(src: &Path, dest_dir: &Path) -> io::Result<()> {
    let name = src
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no filename"))?;
    let dest = dest_dir.join(name);
    // Try rename first (same filesystem), fall back to copy+delete
    match fs::rename(src, &dest) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_entry(src, dest_dir)?;
            delete_entry(src)
        }
    }
}

/// Delete a file or directory recursively.
/// Uses symlink_metadata so symlinks are removed, not their targets.
pub fn delete_entry(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Rename an entry (within the same directory).
pub fn rename_entry(path: &Path, new_name: &str) -> io::Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no parent"))?;
    let new_path = parent.join(new_name);
    fs::rename(path, &new_path)?;
    Ok(new_path)
}

/// Create a new empty file.
pub fn create_file(dir: &Path, name: &str) -> io::Result<PathBuf> {
    let path = dir.join(name);
    fs::File::create(&path)?;
    Ok(path)
}

/// Create a new directory.
pub fn create_directory(dir: &Path, name: &str) -> io::Result<PathBuf> {
    let path = dir.join(name);
    fs::create_dir(&path)?;
    Ok(path)
}

/// Generate a preview for a file.
pub fn file_preview(path: &Path, max_lines: usize) -> Vec<String> {
    if path.is_dir() {
        return dir_preview(path);
    }

    // Check if it looks like a text file by reading first bytes
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return vec![format!("Error reading file: {}", e)],
    };

    if data.is_empty() {
        return vec!["(empty file)".to_string()];
    }

    // Check if binary: if more than 10% non-text bytes in first 512 bytes, treat as binary
    let check_len = data.len().min(512);
    let non_text = data[..check_len]
        .iter()
        .filter(|&&b| b < 0x08 || (b > 0x0D && b < 0x20 && b != 0x1B))
        .count();
    let is_binary = non_text > check_len / 10;

    if is_binary {
        return hex_preview(&data);
    }

    // Text preview
    let text = String::from_utf8_lossy(&data);
    let mut lines: Vec<String> = text.lines().take(max_lines).map(|l| l.to_string()).collect();
    let total_lines = text.lines().count();
    if total_lines > max_lines {
        lines.push(format!("... ({} more lines)", total_lines - max_lines));
    }
    lines
}

fn dir_preview(path: &Path) -> Vec<String> {
    let mut lines = Vec::new();
    let mut file_count: u64 = 0;
    let mut dir_count: u64 = 0;
    let mut total_size: u64 = 0;

    if let Ok(rd) = fs::read_dir(path) {
        for entry in rd.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    dir_count += 1;
                } else {
                    file_count += 1;
                    total_size += meta.len();
                }
            }
        }
    }

    lines.push(format!("Directory: {}", path.display()));
    lines.push(String::new());
    lines.push(format!("  Files:       {}", file_count));
    lines.push(format!("  Directories: {}", dir_count));
    lines.push(format!("  Total size:  {}", format_bytes(total_size)));
    lines
}

fn hex_preview(data: &[u8]) -> Vec<String> {
    let mut lines = Vec::new();
    let len = data.len().min(256);

    // Try to detect image formats
    if data.len() >= 8 {
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            lines.push("Format: PNG image".to_string());
            if data.len() >= 24 {
                let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
                let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
                lines.push(format!("Dimensions: {}x{}", w, h));
            }
            lines.push(String::new());
        } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            lines.push("Format: JPEG image".to_string());
            lines.push(String::new());
        } else if data.starts_with(b"GIF8") {
            lines.push("Format: GIF image".to_string());
            if data.len() >= 10 {
                let w = u16::from_le_bytes([data[6], data[7]]);
                let h = u16::from_le_bytes([data[8], data[9]]);
                lines.push(format!("Dimensions: {}x{}", w, h));
            }
            lines.push(String::new());
        } else if data.starts_with(b"RIFF") && data.len() >= 12 && &data[8..12] == b"WEBP" {
            lines.push("Format: WebP image".to_string());
            lines.push(String::new());
        } else if data.starts_with(b"\x7fELF") {
            lines.push("Format: ELF binary".to_string());
            lines.push(String::new());
        }
    }

    lines.push(format!("Hex dump (first {} bytes):", len));
    lines.push(String::new());

    for (i, chunk) in data[..len].chunks(16).enumerate() {
        let offset = i * 16;
        let hex: Vec<String> = chunk.iter().map(|b| format!("{:02x}", b)).collect();
        let ascii: String = chunk
            .iter()
            .map(|&b| if (0x20..=0x7E).contains(&b) { b as char } else { '.' })
            .collect();
        let hex_str = if hex.len() == 16 {
            format!("{} {}", hex[..8].join(" "), hex[8..].join(" "))
        } else if hex.len() > 8 {
            format!(
                "{} {}{}",
                hex[..8].join(" "),
                hex[8..].join(" "),
                " ".repeat((16 - hex.len()) * 3)
            )
        } else {
            format!(
                "{}{}",
                hex.join(" "),
                " ".repeat((16 - hex.len()) * 3 + 1)
            )
        };
        lines.push(format!("{:08x}  {}  |{}|", offset, hex_str, ascii));
    }

    if data.len() > 256 {
        lines.push(format!("... ({} more bytes)", data.len() - 256));
    }

    lines
}

fn format_bytes(size: u64) -> String {
    if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
