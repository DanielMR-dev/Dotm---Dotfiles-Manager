# dotm — Dotfiles Manager

> A fast, portable dotfiles manager written in Rust. Supports GitHub repos and local folders.

## Installation

### From crates.io
```bash
cargo install dotm
```

### From prebuilt binary (Arch Linux)
```bash
# Download the latest release
curl -L https://github.com/DanielMR-dev/dotm/releases/latest/download/dotm-linux-x86_64 \
  -o ~/.local/bin/dotm
chmod +x ~/.local/bin/dotm
```

### Build from source
```bash
git clone https://github.com/DanielMR-dev/dotm
cd dotm
cargo build --release
sudo cp target/release/dotm /usr/local/bin/
```

---

## Quick start

```bash
# 1. Create a sample config
dotm init

# 2. Edit ~/.config/dotm/config.toml to point to your dotfiles

# 3. Apply everything
dotm install
```

---

## Configuration (`~/.config/dotm/config.toml`)

```toml
# Source: GitHub repository
[source]
type   = "github"
url    = "https://github.com/YOUR_USER/dotfiles"
branch = "main"

# Or a local folder:
# [source]
# type = "local"
# path = "~/dotfiles"

# Mappings: "path in repo" = "destination on system"
[mappings]
"zsh/.zshrc"           = "~/.zshrc"
"hypr/hyprland.conf"   = "~/.config/hypr/hyprland.conf"
"ghostty/config"       = "~/.config/ghostty/config"
"nvim/"                = "~/.config/nvim/"
"git/.gitconfig"       = "~/.gitconfig"

[options]
backup  = true       # Backup existing files before overwriting
method  = "symlink"  # "symlink" (default) or "copy"
dry_run = false      # true = simulate only, no disk writes
```

---

## Commands

| Command | Description |
|---|---|
| `dotm init` | Create sample config at `~/.config/dotm/config.toml` |
| `dotm install` | Clone/copy source and apply all mappings |
| `dotm sync` | Update source (git pull) and re-apply mappings |
| `dotm status` | Show status of all mappings |
| `dotm diff [filter]` | Show differences between repo and system files |
| `dotm add <file>` | Add a system file to the dotfiles repo |
| `dotm backup` | Manually backup all destination files |
| `dotm restore` | Restore the latest backup |

### Global flags

```bash
dotm --dry-run install    # Simulate without writing
dotm --config /path/to/config.toml install
```

---

## How it works

```
GitHub repo / Local folder
         │
         ▼
   dotm fetches source
         │
         ▼
   Reads config.toml mappings
         │
         ▼
   For each mapping:
     1. Backup existing destination file (if any)
     2. Create symlink (or copy) → destination
         │
         ▼
   ~/.zshrc → ~/dotfiles/zsh/.zshrc
   ~/.config/hypr/hyprland.conf → ~/dotfiles/hypr/hyprland.conf
   ...
```

---

## Roadmap

- [x] Local folder source
- [x] GitHub source via git clone/pull
- [x] Symlink and copy methods
- [x] Automatic backup before overwriting
- [x] Status, diff, add commands
- [x] Multi-platform binaries via GitHub Actions
- [ ] Distro detection (pacman / apt / dnf) for dependency installs
- [ ] Profile support (work / home / server)
- [ ] Shell completions (bash, zsh, fish)
- [ ] `dotm pull-request` to push changes back to GitHub

---

## License

MIT — Daniel MR
