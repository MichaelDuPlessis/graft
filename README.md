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
# From source
git clone https://github.com/user/graft.git
cd graft
cargo install --path .
```

## Quick Start

```bash
mkdir ~/dotfiles && cd ~/dotfiles
git init
graft init
```

Edit the generated `graft.toml`:

```toml
[managers]
macos = "brew install"
arch = "pacman -S --noconfirm"
ubuntu = "sudo apt install -y"

[packages.neovim]
os = ["macos", "linux"]
install = "neovim"
files = { "nvim/" = "~/.config/nvim" }
tags = ["editor"]

[packages.zsh]
install = { macos = "zsh", ubuntu = "zsh", arch = "zsh" }
files = { "zsh/.zshrc" = "~/.zshrc", "zsh/.zshenv" = "~/.zshenv" }
tags = ["shell"]
```

Then run:

```bash
graft apply
```

---

## CLI Reference

### Global Options

These options work with any command:

| Flag | Description |
|------|-------------|
| `--config <PATH>` | Path to config file. Overrides auto-detection (which searches for `graft.toml`, `graft.yaml`, `graft.json` in the current directory). |
| `-h, --help` | Show help |
| `-V, --version` | Show version |

```bash
graft --config ~/dotfiles/graft.toml apply
graft --config /path/to/graft.yaml status
```

---

### `graft init`

Generate a starter config file in the current directory.

```
graft init [OPTIONS]
```

**Options:**

| Flag | Description |
|------|-------------|
| `--format <FORMAT>` | Config format to generate: `toml`, `yaml`, or `json`. Defaults to `toml`. |

**Examples:**

```bash
graft init                # Creates graft.toml with commented-out examples
graft init --format yaml  # Creates graft.yaml instead
```

**Behavior:**

- Creates a starter config with commented-out examples showing the structure
- Errors if any config file already exists (`graft.toml`, `graft.yaml`, or `graft.json`) — prevents accidental dual configs

---

### `graft apply`

Deploy packages — installs tools and links/copies files.

```
graft apply [PACKAGES...] [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `[PACKAGES...]` | Optional. Package names to deploy. If omitted, deploys all packages applicable to the current OS. |

**Options:**

| Flag | Short | Description |
|------|-------|-------------|
| `--tag <TAG>` | | Only deploy packages with this tag. Repeatable for multiple tags. |
| `--os <PLATFORM>` | | Override automatic OS detection. Use any platform string (e.g., `macos`, `arch`, `ubuntu`, `fedora`). |
| `--yes` | `-y` | Skip confirmation prompts for package installations. |
| `--force` | `-f` | Overwrite existing files at destinations without prompting. |
| `--dry-run` | | Show what would be done without making any changes. |

**Examples:**

```bash
graft apply                          # Deploy all applicable packages
graft apply neovim zsh               # Deploy specific packages only
graft apply --tag editor             # Deploy packages tagged "editor"
graft apply --tag shell --tag editor # Deploy packages with either tag
graft apply --os arch                # Pretend we're on Arch Linux
graft apply --yes --force            # No prompts, overwrite everything
graft apply --dry-run                # Preview without making changes
graft apply waybar                   # Auto-includes dependencies (e.g., hyprland)
```

**Behavior:**

1. Filters packages by OS and tags
2. Resolves dependencies (topological sort). Auto-includes required dependencies.
3. For each package in dependency order:
   - Checks if the tool is installed via `which <package_name>`
   - If not installed, resolves and runs the install command (with confirmation unless `--yes`)
   - Deploys files via symlink or copy
4. Prints a summary of successes and failures

---

### `graft remove`

Remove deployed files — unlinks symlinks and deletes copied files.

```
graft remove [PACKAGES...] [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `[PACKAGES...]` | Optional. Package names to remove. If omitted, removes all deployed files. |

**Options:**

| Flag | Description |
|------|-------------|
| `--tag <TAG>` | Only remove packages with this tag. Repeatable. |
| `--os <PLATFORM>` | Override OS detection. |
| `--dry-run` | Show what would be removed without doing it. |

**Examples:**

```bash
graft remove                   # Remove all deployed files
graft remove neovim            # Remove only neovim's files
graft remove --tag wm          # Remove all window manager packages
graft remove --os arch         # Remove packages applicable to Arch
graft remove --dry-run         # Preview what would be removed
```

**Behavior:**

- Symlink mode: removes the symlink if it points to the expected source
- Copy mode: removes the file/directory at the destination
- If a file exists but isn't ours (different symlink target, unexpected file): warns and skips
- If destination doesn't exist: skips silently

---

### `graft add`

Add a new package entry to the config file. Supports CLI flags for scripting and interactive mode for manual use.

```
graft add <PACKAGE_NAME> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<PACKAGE_NAME>` | Required. Name for the new package entry. |

**Options:**

| Flag | Description |
|------|-------------|
| `--os <PLATFORM>` | Platforms this package applies to. Repeatable. |
| `--install <NAME>` | Package name for the system package manager. |
| `--files <SRC:DEST>` | File mapping as `source:destination`. Repeatable. |
| `--link-mode <MODE>` | `"symlink"` or `"copy"`. Defaults to symlink. |
| `--tag <TAG>` | Tag for the package. Repeatable. |
| `--depends-on <PKG>` | Dependency on another package. Repeatable. |

**Examples:**

```bash
# Full CLI usage
graft add neovim --os macos --os linux --install neovim --files "nvim/:~/.config/nvim" --tag editor

# Multiple file mappings
graft add zsh --install zsh --files "zsh/.zshrc:~/.zshrc" --files "zsh/.zshenv:~/.zshenv" --tag shell

# With dependencies
graft add waybar --os arch --install waybar --depends-on hyprland --tag wm

# Interactive mode (prompts for each field)
graft add my-package
```

**Behavior:**

- If no options are provided: enters interactive mode, prompting for each field
- If some options are provided: uses them directly without prompting
- Appends to the existing config file (or creates `graft.toml` if none exists)
- Preserves existing config file formatting (for TOML, appends a new section)

---

### `graft scan`

Scan a directory for config files/directories and import them into your graft repo.

```
graft scan <PATH> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<PATH>` | Directory to scan (e.g., `~/.config`). |

**Options:**

| Flag | Short | Description |
|------|-------|-------------|
| `--all` | `-a` | Import all discovered items without prompting. |
| `--detailed` | `-d` | Prompt for tags, OS, and link mode per item. |
| `--tag <TAG>` | | Tag all imported packages with this tag. Repeatable. |
| `--os <PLATFORM>` | | Set the OS field on all imported packages. Repeatable. |
| `--link-mode <MODE>` | | Link mode for imported packages: `"symlink"` or `"copy"`. Defaults to symlink. |

**Examples:**

```bash
graft scan ~/.config                          # Interactive multi-select
graft scan ~/.config --all --tag desktop      # Import everything, tag them
graft scan ~/.config --detailed --os macos    # Per-item prompts for tags/OS/mode
graft scan ~/dotfiles-old --all --link-mode copy
```

**Behavior:**

1. Lists immediate children of the target directory (one level deep)
2. Presents an interactive multi-select (arrow keys to navigate, space to toggle, type to filter)
3. For each selected item:
   - Infers a package name from the filename (strips leading dots and extensions)
   - Prompts for a name if there's a conflict with an existing package
   - Copies the file/directory into the graft repo
   - Appends a config entry to `graft.toml`
4. With `--detailed`: also prompts for OS, tags, and link mode per item

---

### `graft doctor`

Show the detected platform.

```
graft doctor
```

**Output:**

```
✓ Detected platform: macos
```

Useful for verifying what graft thinks your OS is, especially on Linux where it reads `/etc/os-release`. If detection fails, it tells you why.

---

### `graft check`

Validate the config file without making any changes.

```
graft check
```

**Examples:**

```bash
graft check
graft check --config ~/dotfiles/graft.toml
```

**Validates:**

- Config file parses correctly (syntax)
- No dependency cycles
- No references to packages that don't exist in the config
- Source files/directories referenced in `files` mappings exist in the repo

**Output:**

```
✓ Config is valid (graft.toml)
  4 package(s), 3 manager(s)
```

Or with issues:

```
Errors:
  ✗ Dependency cycle detected: waybar → hyprland → waybar
Warnings:
  ⚠ neovim: source not found: nvim/
```

---

### `graft convert`

Convert the config file to a different format.

```
graft convert <FORMAT>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<FORMAT>` | Target format: `toml`, `yaml`, or `json`. |

**Examples:**

```bash
graft convert yaml    # graft.toml → graft.yaml
graft convert json    # graft.yaml → graft.json
graft convert toml    # graft.json → graft.toml
```

**Behavior:**

- Loads the existing config, serializes it to the target format, writes the new file, and removes the old one.
- Errors if the target file already exists.
- No-op if the config is already in the requested format.

---

### `graft status`

Show the deployment state of all applicable packages.

```
graft status [OPTIONS]
```

**Options:**

| Flag | Description |
|------|-------------|
| `--os <PLATFORM>` | Override OS detection. |

**Example:**

```bash
graft status
graft status --os ubuntu
```

**Output shows per package:**

- **Install status**: ✓ (green) if the binary is found via `which`, ✗ (red) if not. Only shown for packages with an `install` or `install_command` field.
- **File status** for each mapping:
  - `linked` (green) — symlink exists and points to the correct source
  - `copied` (green) — file exists at destination (copy mode)
  - `missing` (yellow) — destination doesn't exist
  - `conflict` (red) — destination exists but isn't our symlink

---

### `graft list`

List all available packages with their platform applicability, tags, and link mode.

```
graft list [OPTIONS]
```

**Options:**

| Flag | Description |
|------|-------------|
| `--tag <TAG>` | Filter to only show packages with this tag. Repeatable. |
| `--os <PLATFORM>` | Override OS detection (affects the "Applicable" column). |

**Examples:**

```bash
graft list                     # List all packages
graft list --tag editor        # Only show packages tagged "editor"
graft list --os fedora         # Show applicability as if on Fedora
```

**Output:** A table with columns: Package, Applicable (yes/no), Tags, Link Mode.

---

## Config Reference

Graft looks for a config file in the current directory: `graft.toml`, `graft.yaml`, or `graft.json` (first match wins). Override with `--config`.

### Managers Section

Defines the install command prefix for each platform. If omitted, built-in defaults are used.

```toml
[managers]
macos = "brew install"
arch = "pacman -S --noconfirm"
ubuntu = "sudo apt install -y"
fedora = "sudo dnf install -y"
void = "sudo xbps-install -S"
nixos = "nix-env -iA nixpkgs."
```

**Built-in defaults** (used when `[managers]` is absent or a platform isn't listed):

| Platform | Command |
|----------|---------|
| `macos` | `brew install` |
| `arch` | `pacman -S --noconfirm` |
| `ubuntu` | `sudo apt install -y` |

### Package Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `os` | list of strings | all platforms | Which platforms this package applies to. Use `"linux"` as a catch-all for any Linux distro. |
| `depends_on` | list of strings | none | Package names that must be applied before this one. |
| `install` | string or map | none | Package name for the system package manager. String = same name on all platforms. Map = per-platform names. |
| `install_command` | string | none | Arbitrary shell command run verbatim (e.g., a curl pipe). Takes precedence over `install` if both are set. |
| `files` | map of string → string | none | Source path (relative to config dir) → destination path. `~` is expanded to `$HOME`. |
| `link_mode` | `"symlink"` or `"copy"` | `"symlink"` | How files are deployed. |
| `tags` | list of strings | none | Tags for filtering with `--tag`. |

### Platform Matching

- If `os` is omitted, the package applies to **all** platforms
- `"linux"` in the `os` list matches **any** Linux distro
- Specific distro names (e.g., `"arch"`, `"fedora"`, `"ubuntu"`) match only that distro
- Platform is auto-detected from `/etc/os-release` on Linux and `uname` on macOS
- Use `--os` to override detection

### Install Logic

1. Graft checks if the tool is already installed via `which <package_name>` (the config key)
2. If `install_command` is set, it's run verbatim (no prefix)
3. Otherwise, the `install` field provides the package name, and the platform's manager prefix is prepended
4. If the `install` field is a map and the current platform isn't in it, installation is skipped (not an error)

### Dependency Resolution

- Dependencies are processed before dependents (topological sort)
- Circular dependencies produce a hard error with the cycle path
- When you request a specific package, its dependencies are automatically included
- If a dependency doesn't exist in the config, graft errors out

### File Deployment

- **Symlink mode** (default): creates a symlink at the destination pointing to the source
- **Copy mode**: copies the file or directory to the destination
- Directories are linked/copied as a whole (not individual files within)
- Parent directories are created automatically
- Existing files at the destination are skipped unless `--force` is used

---

## Full Example Config

```toml
[managers]
macos = "brew install"
arch = "pacman -S --noconfirm"
ubuntu = "sudo apt install -y"
fedora = "sudo dnf install -y"

[packages.neovim]
os = ["macos", "linux"]
install = "neovim"
files = { "nvim/" = "~/.config/nvim" }
tags = ["editor"]

[packages.ripgrep]
os = ["macos", "arch", "ubuntu"]
install = { macos = "ripgrep", arch = "ripgrep", ubuntu = "ripgrep" }
link_mode = "copy"
files = { "ripgrep/config" = "~/.config/ripgrep/config" }
tags = ["search", "cli"]

[packages.zsh]
install = { macos = "zsh", ubuntu = "zsh", arch = "zsh" }
files = { "zsh/.zshrc" = "~/.zshrc", "zsh/.zshenv" = "~/.zshenv" }
tags = ["shell"]

[packages.rust]
os = ["macos", "linux"]
install_command = "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
files = { "cargo/config.toml" = "~/.cargo/config.toml" }
tags = ["dev"]

[packages.hyprland]
os = ["arch"]
install = "hyprland"
files = { "hyprland/" = "~/.config/hypr" }
tags = ["wm"]

[packages.waybar]
os = ["arch"]
depends_on = ["hyprland"]
install = "waybar"
files = { "waybar/" = "~/.config/waybar" }
tags = ["wm", "bar"]
```

---

## Disclaimer

This project was AI-generated. All code was reviewed by a human, so any issues are my fault.
