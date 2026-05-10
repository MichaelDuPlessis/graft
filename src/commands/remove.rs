use crate::cli::RemoveArgs;
use crate::config::{self, LinkMode};
use crate::error::Result;
use crate::platform;
use colored::Colorize;
use std::fs;
use std::path::Path;

pub fn run(args: &RemoveArgs, config_path: Option<&Path>) -> Result<()> {
    let (cfg, config_file_path) = config::load(config_path)?;
    let config_dir = config_file_path.parent().unwrap();
    let current_platform = platform::detect(args.os)?;

    // Validate requested packages exist in config
    if !args.packages.is_empty() {
        for name in &args.packages {
            if !cfg.packages.contains_key(name) {
                eprintln!("{} unknown package: \"{}\"", "Error:".red().bold(), name);
                return Ok(());
            }
        }
    }

    let mut removed = 0u32;
    let mut skipped = 0u32;
    let mut errors: Vec<(String, String)> = Vec::new();

    for (name, pkg) in &cfg.packages {
        // Filter by name
        if !args.packages.is_empty() && !args.packages.contains(name) {
            continue;
        }
        // Filter by tag
        if !args.tag.is_empty() {
            let pkg_tags = pkg.tags.as_deref().unwrap_or(&[]);
            if !args.tag.iter().any(|t| pkg_tags.contains(t)) {
                continue;
            }
        }
        // Filter by OS
        if !platform::matches(pkg.os.as_deref().unwrap_or(&[]), &current_platform) {
            continue;
        }

        let files = match &pkg.files {
            Some(f) => f,
            None => continue,
        };
        let link_mode = pkg.link_mode.unwrap_or(LinkMode::Symlink);

        for (src, dest_str) in files {
            let source = config_dir
                .join(src)
                .canonicalize()
                .unwrap_or_else(|_| config_dir.join(src));
            let dest = config::expand_tilde(dest_str);

            if !dest.exists() && !dest.symlink_metadata().is_ok() {
                continue;
            }

            if dest
                .symlink_metadata()
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false)
            {
                let target = fs::read_link(&dest).unwrap_or_default();
                let target_canon = target.canonicalize().unwrap_or_else(|_| target.clone());
                if target_canon == source || target == source {
                    if args.dry_run {
                        println!(
                            "{} would remove symlink: {}",
                            "[dry-run]".cyan(),
                            dest.display()
                        );
                        removed += 1;
                    } else {
                        match fs::remove_file(&dest) {
                            Ok(()) => {
                                println!(
                                    "{} {} ({})",
                                    "Removed".red(),
                                    dest.display(),
                                    name.bold()
                                );
                                removed += 1;
                            }
                            Err(e) => {
                                errors.push((dest.display().to_string(), e.to_string()));
                            }
                        }
                    }
                } else {
                    eprintln!(
                        "{} {} is a symlink to {}, not ours — skipping",
                        "Warning:".yellow(),
                        dest.display(),
                        target.display()
                    );
                    skipped += 1;
                }
            } else if link_mode == LinkMode::Copy {
                if args.dry_run {
                    println!("{} would remove: {}", "[dry-run]".cyan(), dest.display());
                    removed += 1;
                } else {
                    let result = if dest.is_dir() {
                        fs::remove_dir_all(&dest)
                    } else {
                        fs::remove_file(&dest)
                    };
                    match result {
                        Ok(()) => {
                            println!("{} {} ({})", "Removed".red(), dest.display(), name.bold());
                            removed += 1;
                        }
                        Err(e) => {
                            errors.push((dest.display().to_string(), e.to_string()));
                        }
                    }
                }
            } else {
                eprintln!(
                    "{} {} exists but is not our symlink — skipping",
                    "Warning:".yellow(),
                    dest.display()
                );
                skipped += 1;
            }
        }
    }

    println!(
        "\n{} {} removed, {} skipped",
        "Done:".green().bold(),
        removed,
        skipped
    );

    if !errors.is_empty() {
        println!("\n{}:", "Errors".red().bold());
        for (path, err) in &errors {
            println!("  {} {}: {}", "✗".red(), path, err);
        }
    }

    Ok(())
}
