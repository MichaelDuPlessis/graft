# Graft

An OS-aware dotfile manager and tool installer. Like GNU Stow, but with package installation, dependency management, and platform-aware configuration.

Run `graft apply` on a fresh machine and get a fully configured environment.

## Features

- **OS-aware** — packages can target specific platforms (macOS, any Linux distro)
- **Tool installation** — installs packages via your system's package manager if they're missing
- **Arbitrary install commands** — use curl pipes, scripts, or anything via `install_command`
- **Dependency management** — declare dependencies between packages with topological ordering and cycle detection
- **Symlink or copy** — configurable per package, defaults to symlinks
- **Multi-format config** — TOML, YAML, or JSON (auto-detected)
- **Dynamic platform detection** — reads `/etc/os-release` on Linux, works with any distro
- **User-configurable package managers** — define command prefixes for any platform

## Installation

```bash
cargo install --path .
```

## Quick Start

Create a `graft.toml` in your dotfiles directory:

```toml
[managers]
macos = "brew install"
arch = "pacman -S --noconfirm"
ubuntu = "sudo apt install -y"
fedora = "sudo dnf install -y"

[neovim]
os = ["macos", "linux"]
install = "neovim"
files = { "nvim/" = "~/.config/nvim" }
tags = ["editor"]

[zsh]
install = { macos = "zsh", ubuntu = "zsh", arch = "zsh" }
files = { "zsh/.zshrc" = "~/.zshrc", "zsh/.zshenv" = "~/.zshenv" }
tags = ["shell"]

[rust]
install_command = "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
files = { "cargo/config.toml" = "~/.cargo/config.toml" }

[waybar]
os = ["arch"]
depends_on = ["hyprland"]
install = "waybar"
files = { "waybar/" = "~/.config/waybar" }
tags = ["wm"]
```

Then run:

```bash
graft apply
```

## Commands

### `graft apply`

Deploy packages — installs tools and links files.

```bash
graft apply                    # Deploy all applicable packages
graft apply neovim zsh         # Deploy specific packages
graft apply --tag editor       # Deploy packages with a tag
graft apply --yes --force      # Skip prompts, overwrite existing files
graft apply --dry-run          # Show what would be done
```

### `graft remove`

Remove deployed files (unlink symlinks, delete copies).

```bash
graft remove                   # Remove all deployed files
graft remove neovim            # Remove specific package files
graft remove --dry-run         # Show what would be removed
```

### `graft add`

Add a new package entry to the config file. Supports both CLI flags and interactive mode.

```bash
graft add neovim --os macos --os linux --install neovim --files "nvim/:~/.config/nvim" --tag editor
graft add zsh                  # Interactive mode (prompts for each field)
```

### `graft status`

Show deployment state of packages.

```bash
graft status
```

Displays per-package: install status (✓/✗) and file link status (linked/copied/missing/conflict).

### `graft list`

List available packages with OS applicability, tags, and link mode.

```bash
graft list
graft list --tag shell
```

## Global Options

```
--config <PATH>    Path to config file (overrides auto-detection)
--os <PLATFORM>    Override OS detection
```

## Config Reference

### Package Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `os` | list of strings | all platforms | Which platforms this package applies to |
| `depends_on` | list of strings | none | Packages that must be applied first |
| `install` | string or map | none | Package name for the system package manager |
| `install_command` | string | none | Arbitrary shell command (bypasses manager prefix) |
| `files` | map | none | Source → destination file mappings |
| `link_mode` | `"symlink"` or `"copy"` | `"symlink"` | How files are deployed |
| `tags` | list of strings | none | Tags for filtering |

### Platform Matching

- If `os` is omitted, the package applies to all platforms
- `"linux"` in the `os` list matches any Linux distro
- Specific distro names (e.g., `"arch"`, `"fedora"`) match only that distro
- Platform is detected from `/etc/os-release` on Linux, `uname` on macOS

### Managers Section

Define install command prefixes per platform. Built-in defaults:

```toml
[managers]
macos = "brew install"
arch = "pacman -S --noconfirm"
ubuntu = "sudo apt install -y"
```

Add any distro by including it in your config:

```toml
[managers]
fedora = "sudo dnf install -y"
void = "sudo xbps-install -S"
nixos = "nix-env -iA nixpkgs."
```

## How It Works

1. Detects your OS (or uses `--os` override)
2. Loads config and filters packages by platform and tags
3. Resolves dependencies (topological sort, cycle detection)
4. For each package in order:
   - Checks if the tool is installed (`which <name>`)
   - If not, runs the install command (with confirmation)
   - Deploys files via symlink or copy

## Disclaimer

This project was AI-generated. All code was reviewed by a human, so any issues are my fault.
