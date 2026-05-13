use crate::cli::AddArgs;
use crate::config::{LinkMode, PackageConfig};
use crate::error::{GraftError, Result};
use crate::platform::Platform;
use colored::Colorize;
use dialoguer::{Input, MultiSelect, Select};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(args: &AddArgs, config_path: Option<&Path>) -> Result<()> {
    let pkg = if is_interactive(args) {
        prompt_interactive(&args.name)?
    } else {
        build_from_args(args)
    };

    let config_file = resolve_config_path(config_path)?;
    append_package(&args.name, &pkg, &config_file)?;

    println!("{} Added package '{}' to {}", "✓".green(), args.name.bold(), config_file.display());
    Ok(())
}

fn is_interactive(args: &AddArgs) -> bool {
    args.os.is_empty()
        && args.install.is_none()
        && args.files.is_empty()
        && args.link_mode.is_none()
        && args.tag.is_empty()
        && args.depends_on.is_empty()
}

fn prompt_interactive(name: &str) -> Result<PackageConfig> {
    let platform_labels = &["macos", "arch", "ubuntu", "linux"];

    let os_indices = MultiSelect::new()
        .with_prompt(format!("OS platforms for '{name}'"))
        .items(platform_labels)
        .interact()
        .map_err(|e| GraftError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    let os: Vec<Platform> = os_indices
        .into_iter()
        .map(|i| Platform::new(platform_labels[i]))
        .collect();

    let install_input: String = Input::new()
        .with_prompt("Install package name (empty to skip)")
        .allow_empty(true)
        .interact_text()
        .map_err(|e| GraftError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    let install = if install_input.is_empty() {
        None
    } else {
        Some(install_input)
    };

    let mut files = HashMap::new();
    loop {
        let mapping: String = Input::new()
            .with_prompt("File mapping (source:dest, empty to stop)")
            .allow_empty(true)
            .interact_text()
            .map_err(|e| GraftError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        if mapping.is_empty() {
            break;
        }
        if let Some((src, dest)) = mapping.split_once(':') {
            files.insert(src.to_string(), dest.to_string());
        }
    }

    let link_mode_idx = Select::new()
        .with_prompt("Link mode")
        .items(&["symlink", "copy"])
        .default(0)
        .interact()
        .map_err(|e| GraftError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    let link_mode = if link_mode_idx == 0 {
        LinkMode::Symlink
    } else {
        LinkMode::Copy
    };

    let tags_input: String = Input::new()
        .with_prompt("Tags (comma-separated, empty to skip)")
        .allow_empty(true)
        .interact_text()
        .map_err(|e| GraftError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    let tags: Vec<String> = tags_input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let deps_input: String = Input::new()
        .with_prompt("Dependencies (comma-separated, empty to skip)")
        .allow_empty(true)
        .interact_text()
        .map_err(|e| GraftError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    let depends_on: Vec<String> = deps_input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(PackageConfig {
        os: if os.is_empty() { None } else { Some(os) },
        install: install.map(crate::config::Install::Simple),
        install_command: None,
        files: if files.is_empty() { None } else { Some(files) },
        link_mode: Some(link_mode),
        tags: if tags.is_empty() { None } else { Some(tags) },
        depends_on: if depends_on.is_empty() {
            None
        } else {
            Some(depends_on)
        },
    })
}

fn build_from_args(args: &AddArgs) -> PackageConfig {
    let files: HashMap<String, String> = args
        .files
        .iter()
        .filter_map(|f| {
            f.split_once(':')
                .map(|(s, d)| (s.to_string(), d.to_string()))
        })
        .collect();

    let link_mode = args.link_mode.as_deref().map(|m| match m {
        "copy" => LinkMode::Copy,
        _ => LinkMode::Symlink,
    });

    PackageConfig {
        os: if args.os.is_empty() {
            None
        } else {
            Some(args.os.clone())
        },
        install: args.install.clone().map(crate::config::Install::Simple),
        install_command: None,
        files: if files.is_empty() { None } else { Some(files) },
        link_mode,
        tags: if args.tag.is_empty() {
            None
        } else {
            Some(args.tag.clone())
        },
        depends_on: if args.depends_on.is_empty() {
            None
        } else {
            Some(args.depends_on.clone())
        },
    }
}

fn resolve_config_path(config_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = config_path {
        return Ok(p.to_path_buf());
    }
    let cwd = std::env::current_dir()?;
    for name in ["graft.toml", "graft.yaml", "graft.json"] {
        let candidate = cwd.join(name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    // Default to creating graft.toml
    Ok(cwd.join("graft.toml"))
}

pub fn append_package_to_config(name: &str, pkg: &PackageConfig, config_file: &Path) -> Result<()> {
    append_package(name, pkg, config_file)
}

fn append_package(name: &str, pkg: &PackageConfig, config_file: &Path) -> Result<()> {
    let ext = config_file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("toml");

    match ext {
        "toml" => append_toml(name, pkg, config_file),
        "yaml" | "yml" => append_yaml(name, pkg, config_file),
        "json" => append_json(name, pkg, config_file),
        _ => append_toml(name, pkg, config_file),
    }
}

fn append_toml(name: &str, pkg: &PackageConfig, path: &Path) -> Result<()> {
    let mut lines = Vec::new();
    lines.push(format!("[packages.{}]", name));

    if let Some(ref os) = pkg.os {
        let vals: Vec<String> = os.iter().map(|p| format!("\"{}\"", p)).collect();
        lines.push(format!("os = [{}]", vals.join(", ")));
    }
    if let Some(ref deps) = pkg.depends_on {
        let vals: Vec<String> = deps.iter().map(|d| format!("\"{}\"", d)).collect();
        lines.push(format!("depends_on = [{}]", vals.join(", ")));
    }
    if let Some(ref install) = pkg.install {
        match install {
            crate::config::Install::Simple(s) => {
                lines.push(format!("install = \"{}\"", s));
            }
            crate::config::Install::PerPlatform(m) => {
                let pairs: Vec<String> = m.iter().map(|(p, v)| format!("{} = \"{}\"", p, v)).collect();
                lines.push(format!("install = {{ {} }}", pairs.join(", ")));
            }
        }
    }
    if let Some(ref install_cmd) = pkg.install_command {
        lines.push(format!("install_command = \"{}\"", install_cmd));
    }
    if let Some(ref files) = pkg.files {
        let pairs: Vec<String> = files.iter().map(|(k, v)| format!("\"{}\" = \"{}\"", k, v)).collect();
        lines.push(format!("files = {{ {} }}", pairs.join(", ")));
    }
    if let Some(link_mode) = pkg.link_mode {
        let s = match link_mode {
            LinkMode::Symlink => "symlink",
            LinkMode::Copy => "copy",
        };
        lines.push(format!("link_mode = \"{}\"", s));
    }
    if let Some(ref tags) = pkg.tags {
        let vals: Vec<String> = tags.iter().map(|t| format!("\"{}\"", t)).collect();
        lines.push(format!("tags = [{}]", vals.join(", ")));
    }

    let fragment = lines.join("\n");

    let mut content = if path.exists() {
        let existing = fs::read_to_string(path)?;
        if existing.ends_with('\n') {
            existing
        } else {
            existing + "\n"
        }
    } else {
        String::new()
    };

    content.push('\n');
    content.push_str(&fragment);
    content.push('\n');
    fs::write(path, content)?;
    Ok(())
}

fn append_yaml(name: &str, pkg: &PackageConfig, path: &Path) -> Result<()> {
    let mut doc: yaml_serde::Value = if path.exists() {
        let content = fs::read_to_string(path)?;
        yaml_serde::from_str(&content).map_err(|e| GraftError::ConfigParse(e.to_string()))?
    } else {
        yaml_serde::Value::Mapping(yaml_serde::Mapping::new())
    };

    let pkg_value =
        serde_json::to_value(pkg).map_err(|e| GraftError::ConfigParse(e.to_string()))?;
    let pkg_yaml: yaml_serde::Value =
        yaml_serde::to_value(&pkg_value).map_err(|e| GraftError::ConfigParse(e.to_string()))?;

    if let yaml_serde::Value::Mapping(ref mut map) = doc {
        let packages_key = yaml_serde::Value::String("packages".to_string());
        let packages_map = map
            .entry(packages_key)
            .or_insert_with(|| yaml_serde::Value::Mapping(yaml_serde::Mapping::new()));
        if let yaml_serde::Value::Mapping(pkgs) = packages_map {
            pkgs.insert(yaml_serde::Value::String(name.to_string()), pkg_yaml);
        }
    }

    let output = yaml_serde::to_string(&doc).map_err(|e| GraftError::ConfigParse(e.to_string()))?;
    fs::write(path, output)?;
    Ok(())
}

fn append_json(name: &str, pkg: &PackageConfig, path: &Path) -> Result<()> {
    let mut doc: serde_json::Value = if path.exists() {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(|e| GraftError::ConfigParse(e.to_string()))?
    } else {
        serde_json::Value::Object(serde_json::Map::new())
    };

    let pkg_value =
        serde_json::to_value(pkg).map_err(|e| GraftError::ConfigParse(e.to_string()))?;

    if let serde_json::Value::Object(ref mut map) = doc {
        let packages = map
            .entry("packages")
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if let serde_json::Value::Object(pkgs) = packages {
            pkgs.insert(name.to_string(), pkg_value);
        }
    }

    let output =
        serde_json::to_string_pretty(&doc).map_err(|e| GraftError::ConfigParse(e.to_string()))?;
    fs::write(path, output)?;
    Ok(())
}
