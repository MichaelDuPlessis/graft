use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{GraftError, Result};
use crate::platform::Platform;

#[derive(Debug, Clone, Deserialize)]
pub struct GraftConfig {
    pub managers: HashMap<Platform, String>,
    pub packages: HashMap<String, PackageConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageConfig {
    pub os: Option<Vec<Platform>>,
    pub depends_on: Option<Vec<String>>,
    pub install: Option<Install>,
    pub install_command: Option<String>,
    pub files: Option<HashMap<String, String>>,
    pub link_mode: Option<LinkMode>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Install {
    Simple(String),
    PerPlatform(HashMap<Platform, String>),
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LinkMode {
    Symlink,
    Copy,
}

pub fn default_managers() -> HashMap<Platform, String> {
    HashMap::from([
        (Platform::MacOs, "brew install".into()),
        (Platform::Arch, "pacman -S --noconfirm".into()),
        (Platform::Ubuntu, "sudo apt install -y".into()),
    ])
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix('~') {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest.strip_prefix('/').unwrap_or(rest));
        }
    }
    PathBuf::from(path)
}

pub fn load(config_path: Option<&Path>) -> Result<(GraftConfig, PathBuf)> {
    let path = match config_path {
        Some(p) => p.to_path_buf(),
        None => find_config()?,
    };
    let content = std::fs::read_to_string(&path)?;
    let config = parse_config(&content, &path)?;
    Ok((config, path))
}

fn find_config() -> Result<PathBuf> {
    let cwd = std::env::current_dir().map_err(GraftError::IoError)?;
    for name in ["graft.toml", "graft.yaml", "graft.json"] {
        let candidate = cwd.join(name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(GraftError::ConfigNotFound)
}

fn parse_config(content: &str, path: &Path) -> Result<GraftConfig> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mut raw: serde_json::Value = match ext {
        "toml" => {
            let table: toml::Table =
                toml::from_str(content).map_err(|e| GraftError::ConfigParse(e.to_string()))?;
            serde_json::to_value(table).map_err(|e| GraftError::ConfigParse(e.to_string()))?
        }
        "yaml" | "yml" => {
            yaml_serde::from_str(content).map_err(|e| GraftError::ConfigParse(e.to_string()))?
        }
        "json" => {
            serde_json::from_str(content).map_err(|e| GraftError::ConfigParse(e.to_string()))?
        }
        _ => return Err(GraftError::ConfigParse(format!("unsupported format: {ext}"))),
    };

    let managers = match raw.get("managers") {
        Some(v) => serde_json::from_value(v.clone())
            .map_err(|e| GraftError::ConfigParse(e.to_string()))?,
        None => default_managers(),
    };

    if let Some(obj) = raw.as_object_mut() {
        obj.remove("managers");
    }

    let packages: HashMap<String, PackageConfig> =
        serde_json::from_value(raw).map_err(|e| GraftError::ConfigParse(e.to_string()))?;

    Ok(GraftConfig { managers, packages })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_toml_config() {
        let toml_str = r#"
[managers]
macos = "brew install"
arch = "yay -S --noconfirm"

[neovim]
os = ["macos", "linux"]
install = "neovim"
files = { "nvim/" = "~/.config/nvim" }
tags = ["editor"]

[ripgrep]
os = ["macos", "arch"]
link_mode = "copy"
files = { "ripgrep/config" = "~/.config/ripgrep/config" }
"#;
        let path = Path::new("graft.toml");
        let config = parse_config(toml_str, path).unwrap();

        assert_eq!(config.managers.get(&Platform::MacOs).unwrap(), "brew install");
        assert_eq!(config.managers.get(&Platform::Arch).unwrap(), "yay -S --noconfirm");
        assert_eq!(config.packages.len(), 2);

        let neovim = &config.packages["neovim"];
        assert_eq!(neovim.os.as_ref().unwrap(), &[Platform::MacOs, Platform::Linux]);
        assert!(matches!(neovim.install, Some(Install::Simple(ref s)) if s == "neovim"));
        assert_eq!(neovim.tags.as_ref().unwrap(), &["editor"]);

        let ripgrep = &config.packages["ripgrep"];
        assert_eq!(ripgrep.link_mode, Some(LinkMode::Copy));
    }

    #[test]
    fn parse_per_platform_install() {
        let toml_str = r#"
[zsh]
install = { macos = "zsh", ubuntu = "zsh", arch = "zsh" }
"#;
        let path = Path::new("graft.toml");
        let config = parse_config(toml_str, path).unwrap();
        let zsh = &config.packages["zsh"];
        match &zsh.install {
            Some(Install::PerPlatform(map)) => {
                assert_eq!(map.get(&Platform::MacOs).unwrap(), "zsh");
                assert_eq!(map.len(), 3);
            }
            _ => panic!("expected PerPlatform install"),
        }
    }

    #[test]
    fn default_managers_has_expected_entries() {
        let m = default_managers();
        assert_eq!(m.get(&Platform::MacOs).unwrap(), "brew install");
        assert_eq!(m.get(&Platform::Arch).unwrap(), "pacman -S --noconfirm");
        assert_eq!(m.get(&Platform::Ubuntu).unwrap(), "sudo apt install -y");
    }

    #[test]
    fn expand_tilde_with_home() {
        let expanded = expand_tilde("~/.config/nvim");
        let home = dirs::home_dir().unwrap();
        assert_eq!(expanded, home.join(".config/nvim"));
    }

    #[test]
    fn expand_tilde_no_tilde() {
        let expanded = expand_tilde("/usr/local/bin");
        assert_eq!(expanded, PathBuf::from("/usr/local/bin"));
    }

    #[test]
    fn missing_managers_uses_defaults() {
        let toml_str = r#"
[neovim]
install = "neovim"
"#;
        let path = Path::new("graft.toml");
        let config = parse_config(toml_str, path).unwrap();
        assert_eq!(config.managers, default_managers());
    }
}
