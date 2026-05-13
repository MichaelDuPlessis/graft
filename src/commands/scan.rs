use crate::cli::ScanArgs;
use crate::config::{LinkMode, PackageConfig};
use crate::error::{GraftError, Result};
use crate::platform::Platform;
use colored::Colorize;
use dialoguer::{Confirm, Input, MultiSelect, Select};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Items that are clearly not user configs and should be skipped.
const SKIP_NAMES: &[&str] = &[".DS_Store", ".localized", "Thumbs.db"];

struct DiscoveredItem {
    path: PathBuf,
    name: String,
    is_dir: bool,
}

pub fn run(args: &ScanArgs) -> Result<()> {
    let scan_path = crate::config::expand_tilde(args.path.to_str().unwrap_or(""));
    if !scan_path.is_dir() {
        return Err(GraftError::ConfigParse(format!(
            "Not a directory: {}",
            scan_path.display()
        )));
    }

    // Discover items (one level deep)
    let items = discover_items(&scan_path)?;
    if items.is_empty() {
        println!("No items found in {}", scan_path.display());
        return Ok(());
    }

    // Display what was found
    println!(
        "Found {} items in {}:",
        items.len(),
        scan_path.display()
    );
    for item in &items {
        let kind = if item.is_dir { "[dir] " } else { "[file]" };
        println!("  {} {}", kind.dimmed(), item.name);
    }
    println!();

    // Load existing config to check for name conflicts
    let existing_packages = load_existing_package_names();

    // Filter/confirm items and import
    let mut imported = 0;
    for item in &items {
        let accepted = if args.all {
            true
        } else {
            Confirm::new()
                .with_prompt(format!("Import {}?", item.name))
                .default(false)
                .interact()
                .unwrap_or(false)
        };

        if !accepted {
            continue;
        }

        // Infer package name
        let mut pkg_name = infer_package_name(&item.name);

        // Check for conflicts
        if existing_packages.contains(&pkg_name) || repo_path_exists(&pkg_name) {
            if args.all {
                eprintln!(
                    "{} Skipping '{}': name conflict",
                    "warning:".yellow(),
                    pkg_name
                );
                continue;
            }
            pkg_name = prompt_package_name(&pkg_name)?;
        }

        // Copy into graft repo
        let repo_source = copy_into_repo(item, &pkg_name)?;

        // Build file mapping: repo-relative source → original location as destination
        let dest_str = path_to_tilde_string(&item.path);
        let mut files = HashMap::new();
        if item.is_dir {
            files.insert(format!("{}/", repo_source), dest_str);
        } else {
            files.insert(repo_source, dest_str);
        }

        // Build config entry
        let (link_mode, os, tags) = if args.detailed {
            prompt_package_details(&args.os, &args.tag, args.link_mode.as_deref())?
        } else {
            let lm = match args.link_mode.as_deref() {
                Some("copy") => LinkMode::Copy,
                _ => LinkMode::Symlink,
            };
            (lm, args.os.clone(), args.tag.clone())
        };

        let pkg = PackageConfig {
            os: if os.is_empty() { None } else { Some(os) },
            depends_on: None,
            install: None,
            install_command: None,
            files: Some(files),
            link_mode: Some(link_mode),
            tags: if tags.is_empty() { None } else { Some(tags) },
        };

        // Append to config
        let config_file = resolve_config_path()?;
        crate::commands::add::append_package_to_config(&pkg_name, &pkg, &config_file)?;

        imported += 1;
    }

    if imported > 0 {
        println!("\n{} Imported {} package(s)", "✓".green(), imported);
    } else {
        println!("\nNo packages imported.");
    }

    Ok(())
}

fn discover_items(path: &Path) -> Result<Vec<DiscoveredItem>> {
    let mut items = Vec::new();
    let entries = fs::read_dir(path)?;

    for entry in entries {
        let entry = entry?;
        let file_name = entry.file_name().to_string_lossy().to_string();

        if SKIP_NAMES.contains(&file_name.as_str()) {
            continue;
        }

        let file_type = entry.file_type()?;
        items.push(DiscoveredItem {
            path: entry.path(),
            name: file_name,
            is_dir: file_type.is_dir(),
        });
    }

    items.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(items)
}

fn infer_package_name(filename: &str) -> String {
    let mut name = filename.to_string();
    // Strip leading dot
    if name.starts_with('.') {
        name = name[1..].to_string();
    }
    // Strip file extension for single files
    if let Some(stem) = Path::new(&name).file_stem() {
        if Path::new(&name).extension().is_some() {
            name = stem.to_string_lossy().to_string();
        }
    }
    name
}

fn prompt_package_name(conflicting: &str) -> Result<String> {
    let name: String = Input::new()
        .with_prompt(format!("Name '{}' conflicts. Enter a different name", conflicting))
        .interact_text()
        .map_err(|e| GraftError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    Ok(name)
}

fn prompt_package_details(
    default_os: &[Platform],
    default_tags: &[String],
    default_link_mode: Option<&str>,
) -> Result<(LinkMode, Vec<Platform>, Vec<String>)> {
    let platform_labels = &["macos", "arch", "ubuntu", "linux"];
    let os_indices = MultiSelect::new()
        .with_prompt("OS platforms (space to select, enter to confirm)")
        .items(platform_labels)
        .defaults(
            &platform_labels
                .iter()
                .map(|l| default_os.iter().any(|p| p.0 == *l))
                .collect::<Vec<_>>(),
        )
        .interact()
        .map_err(|e| GraftError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    let os: Vec<Platform> = os_indices
        .into_iter()
        .map(|i| Platform::new(platform_labels[i]))
        .collect();

    let default_tags_str = default_tags.join(", ");
    let tags_input: String = Input::new()
        .with_prompt("Tags (comma-separated, empty to skip)")
        .default(default_tags_str)
        .allow_empty(true)
        .interact_text()
        .map_err(|e| GraftError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    let tags: Vec<String> = tags_input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let default_idx = match default_link_mode {
        Some("copy") => 1,
        _ => 0,
    };
    let link_mode_idx = Select::new()
        .with_prompt("Link mode")
        .items(&["symlink", "copy"])
        .default(default_idx)
        .interact()
        .map_err(|e| GraftError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    let link_mode = if link_mode_idx == 0 {
        LinkMode::Symlink
    } else {
        LinkMode::Copy
    };

    Ok((link_mode, os, tags))
}

fn repo_path_exists(name: &str) -> bool {
    Path::new(name).exists()
}

fn copy_into_repo(item: &DiscoveredItem, pkg_name: &str) -> Result<String> {
    if item.is_dir {
        // Copy directory as-is into repo root with package name
        let dest = PathBuf::from(pkg_name);
        if dest.exists() {
            eprintln!(
                "{} Directory '{}' already exists in repo, skipping copy",
                "warning:".yellow(),
                pkg_name
            );
        } else {
            copy_dir_recursive(&item.path, &dest)?;
        }
        Ok(pkg_name.to_string())
    } else {
        // Single file: create wrapper directory, copy file into it
        let dest_dir = PathBuf::from(pkg_name);
        fs::create_dir_all(&dest_dir)?;
        let dest_file = dest_dir.join(&item.name);
        if dest_file.exists() {
            eprintln!(
                "{} File '{}' already exists in repo, skipping copy",
                "warning:".yellow(),
                dest_file.display()
            );
        } else {
            fs::copy(&item.path, &dest_file)?;
        }
        Ok(format!("{}/{}", pkg_name, item.name))
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dest_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

fn path_to_tilde_string(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(relative) = path.strip_prefix(&home) {
            return format!("~/{}", relative.display());
        }
    }
    path.display().to_string()
}

fn load_existing_package_names() -> Vec<String> {
    match crate::config::load(None) {
        Ok((config, _)) => config.packages.keys().cloned().collect(),
        Err(_) => Vec::new(),
    }
}

fn resolve_config_path() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    for name in ["graft.toml", "graft.yaml", "graft.json"] {
        let candidate = cwd.join(name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Ok(cwd.join("graft.toml"))
}
