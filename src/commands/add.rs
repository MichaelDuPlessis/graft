use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use dialoguer::{Input, MultiSelect, Select};

use crate::cli::AddArgs;
use crate::config::{LinkMode, PackageConfig};
use crate::error::{GraftError, Result};
use crate::platform::Platform;

pub fn run(args: &AddArgs, config_path: Option<&Path>) -> Result<()> {
    let pkg = if is_interactive(args) {
        prompt_interactive(&args.name)?
    } else {
        build_from_args(args)
    };

    let config_file = resolve_config_path(config_path)?;
    append_package(&args.name, &pkg, &config_file)?;

    println!("Added package '{}' to {}", args.name, config_file.display());
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
    let platforms = &[Platform::MacOs, Platform::Arch, Platform::Ubuntu, Platform::Linux];
    let platform_labels: Vec<String> = platforms.iter().map(|p| p.to_string()).collect();

    let os_indices = MultiSelect::new()
        .with_prompt(format!("OS platforms for '{name}'"))
        .items(&platform_labels)
        .interact()
        .map_err(|e| GraftError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    let os: Vec<Platform> = os_indices.into_iter().map(|i| platforms[i]).collect();

    let install_input: String = Input::new()
        .with_prompt("Install package name (empty to skip)")
        .allow_empty(true)
        .interact_text()
        .map_err(|e| GraftError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    let install = if install_input.is_empty() { None } else { Some(install_input) };

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
    let link_mode = if link_mode_idx == 0 { LinkMode::Symlink } else { LinkMode::Copy };

    let tags_input: String = Input::new()
        .with_prompt("Tags (comma-separated, empty to skip)")
        .allow_empty(true)
        .interact_text()
        .map_err(|e| GraftError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    let tags: Vec<String> = tags_input.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

    let deps_input: String = Input::new()
        .with_prompt("Dependencies (comma-separated, empty to skip)")
        .allow_empty(true)
        .interact_text()
        .map_err(|e| GraftError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    let depends_on: Vec<String> = deps_input.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

    Ok(PackageConfig {
        os: if os.is_empty() { None } else { Some(os) },
        install: install.map(crate::config::Install::Simple),
        install_command: None,
        files: if files.is_empty() { None } else { Some(files) },
        link_mode: Some(link_mode),
        tags: if tags.is_empty() { None } else { Some(tags) },
        depends_on: if depends_on.is_empty() { None } else { Some(depends_on) },
    })
}

fn build_from_args(args: &AddArgs) -> PackageConfig {
    let files: HashMap<String, String> = args
        .files
        .iter()
        .filter_map(|f| f.split_once(':').map(|(s, d)| (s.to_string(), d.to_string())))
        .collect();

    let link_mode = args.link_mode.as_deref().map(|m| match m {
        "copy" => LinkMode::Copy,
        _ => LinkMode::Symlink,
    });

    PackageConfig {
        os: if args.os.is_empty() { None } else { Some(args.os.clone()) },
        install: args.install.clone().map(crate::config::Install::Simple),
        install_command: None,
        files: if files.is_empty() { None } else { Some(files) },
        link_mode,
        tags: if args.tag.is_empty() { None } else { Some(args.tag.clone()) },
        depends_on: if args.depends_on.is_empty() { None } else { Some(args.depends_on.clone()) },
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

fn append_package(name: &str, pkg: &PackageConfig, config_file: &Path) -> Result<()> {
    let ext = config_file.extension().and_then(|e| e.to_str()).unwrap_or("toml");

    match ext {
        "toml" => append_toml(name, pkg, config_file),
        "yaml" | "yml" => append_yaml(name, pkg, config_file),
        "json" => append_json(name, pkg, config_file),
        _ => append_toml(name, pkg, config_file),
    }
}

fn append_toml(name: &str, pkg: &PackageConfig, path: &Path) -> Result<()> {
    // Build a table for just this package and serialize it
    let mut table = toml::Table::new();
    let pkg_value = build_toml_value(pkg);
    table.insert(name.to_string(), pkg_value);

    let fragment = toml::to_string(&table)
        .map_err(|e| GraftError::ConfigParse(e.to_string()))?;

    let mut content = if path.exists() {
        let existing = fs::read_to_string(path)?;
        if existing.ends_with('\n') { existing } else { existing + "\n" }
    } else {
        String::new()
    };

    content.push_str("\n");
    content.push_str(&fragment);
    fs::write(path, content)?;
    Ok(())
}

fn build_toml_value(pkg: &PackageConfig) -> toml::Value {
    let mut map = toml::Table::new();

    if let Some(ref os) = pkg.os {
        let arr: Vec<toml::Value> = os.iter().map(|p| toml::Value::String(p.to_string())).collect();
        map.insert("os".into(), toml::Value::Array(arr));
    }
    if let Some(ref install) = pkg.install {
        match install {
            crate::config::Install::Simple(s) => {
                map.insert("install".into(), toml::Value::String(s.clone()));
            }
            crate::config::Install::PerPlatform(m) => {
                let mut t = toml::Table::new();
                for (p, v) in m {
                    t.insert(p.to_string(), toml::Value::String(v.clone()));
                }
                map.insert("install".into(), toml::Value::Table(t));
            }
        }
    }
    if let Some(ref files) = pkg.files {
        let mut t = toml::Table::new();
        for (k, v) in files {
            t.insert(k.clone(), toml::Value::String(v.clone()));
        }
        map.insert("files".into(), toml::Value::Table(t));
    }
    if let Some(link_mode) = pkg.link_mode {
        let s = match link_mode {
            LinkMode::Symlink => "symlink",
            LinkMode::Copy => "copy",
        };
        map.insert("link_mode".into(), toml::Value::String(s.into()));
    }
    if let Some(ref tags) = pkg.tags {
        let arr: Vec<toml::Value> = tags.iter().map(|t| toml::Value::String(t.clone())).collect();
        map.insert("tags".into(), toml::Value::Array(arr));
    }
    if let Some(ref deps) = pkg.depends_on {
        let arr: Vec<toml::Value> = deps.iter().map(|d| toml::Value::String(d.clone())).collect();
        map.insert("depends_on".into(), toml::Value::Array(arr));
    }

    toml::Value::Table(map)
}

fn append_yaml(name: &str, pkg: &PackageConfig, path: &Path) -> Result<()> {
    let mut doc: yaml_serde::Value = if path.exists() {
        let content = fs::read_to_string(path)?;
        yaml_serde::from_str(&content).map_err(|e| GraftError::ConfigParse(e.to_string()))?
    } else {
        yaml_serde::Value::Mapping(yaml_serde::Mapping::new())
    };

    let pkg_value = serde_json::to_value(pkg).map_err(|e| GraftError::ConfigParse(e.to_string()))?;
    let pkg_yaml: yaml_serde::Value = yaml_serde::to_value(&pkg_value).map_err(|e| GraftError::ConfigParse(e.to_string()))?;

    if let yaml_serde::Value::Mapping(ref mut map) = doc {
        map.insert(yaml_serde::Value::String(name.to_string()), pkg_yaml);
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

    let pkg_value = serde_json::to_value(pkg).map_err(|e| GraftError::ConfigParse(e.to_string()))?;

    if let serde_json::Value::Object(ref mut map) = doc {
        map.insert(name.to_string(), pkg_value);
    }

    let output = serde_json::to_string_pretty(&doc).map_err(|e| GraftError::ConfigParse(e.to_string()))?;
    fs::write(path, output)?;
    Ok(())
}
