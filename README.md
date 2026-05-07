# phosphor-fm

Dual-pane terminal file manager with symlink-safe operations.

## Features

- Side-by-side directory panes with Tab to switch focus
- Copy, move, rename, delete between panes — all ops use `symlink_metadata` to avoid following symlinks into unintended targets
- Recursive directory copy preserves symlinks as symlinks
- File preview panel (text, hex dump with format detection for PNG/JPEG/GIF/WebP/ELF, directory stats)
- Inline search filter (`/`) to narrow file listings
- Go-to-path prompt (`g`) for direct navigation
- Create new files (`n`) and directories (`N`)
- Toggle hidden files (`.`), cycle sort modes (`s`)
- Multi-select with Space, select-all with `a`
- Delete confirmation prompt (y/N)

## Install

```
cargo build --release
# binary at target/release/phosphor-fm
```

## Usage

```
# open current directory
phosphor-fm

# open a specific path
phosphor-fm /var/log
```

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `k` or arrows | Move cursor |
| `Enter` | Enter directory |
| `Backspace` | Parent directory |
| `~` | Go to home |
| `Tab` | Switch pane |
| `Space` | Toggle selection |
| `a` | Select all |
| `c` | Copy selected to other pane |
| `m` | Move selected to other pane |
| `d` | Delete (with confirmation) |
| `r` | Rename |
| `n` / `N` | New file / new directory |
| `/` | Search filter |
| `g` | Go to path |
| `.` | Toggle hidden files |
| `s` | Cycle sort mode |
| `p` | Toggle preview panel |
| `G` | Jump to last entry |
| `PgUp` / `PgDn` | Page scroll |
| `q` / `Ctrl-C` | Quit |

---
Built with Rust + ratatui
