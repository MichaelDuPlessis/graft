use std::collections::HashMap;
use std::path::Path;

use colored::Colorize;

use crate::cli::ApplyArgs;
use crate::config::{self, LinkMode, PackageConfig};
use crate::error::{GraftError, Result};
use crate::install;
use crate::link;
use crate::platform;
use crate::resolve;

pub fn run(args: &ApplyArgs, config_path: Option<&Path>) -> Result<()> {
    let (config, cfg_path) = config::load(config_path)?;
    let config_dir = cfg_path.parent().unwrap();
    let current = platform::detect(args.os)?;

    // Filter by OS applicability
    let applicable: HashMap<String, &PackageConfig> = config
        .packages
        .iter()
        .filter(|(_, pkg)| {
            pkg.os
                .as_ref()
                .map(|os| platform::matches(os, &current))
                .unwrap_or(true)
        })
        .map(|(name, pkg)| (name.clone(), pkg))
        .collect();

    // Filter by tag
    let filtered: HashMap<String, &PackageConfig> = if args.tag.is_empty() {
        applicable
    } else {
        applicable
            .into_iter()
            .filter(|(_, pkg)| {
                pkg.tags
                    .as_ref()
                    .map(|t| t.iter().any(|tag| args.tag.contains(tag)))
                    .unwrap_or(false)
            })
            .collect()
    };

    // Validate requested packages exist
    if !args.packages.is_empty() {
        for name in &args.packages {
            if !filtered.contains_key(name) {
                return Err(GraftError::UnknownPackage(name.clone()));
            }
        }
    }

    // Resolve dependency order
    let requested: Vec<String> = if args.packages.is_empty() {
        filtered.keys().cloned().collect()
    } else {
        args.packages.clone()
    };

    let order = resolve::resolve_order(&filtered, &requested, &current)?;

    // Report auto-included dependencies
    if !args.packages.is_empty() {
        for dep in &order {
            if !args.packages.contains(dep) {
                println!(
                    "{}",
                    format!("Including dependency: {dep}").cyan()
                );
            }
        }
    }

    // Process packages
    let mut succeeded = 0usize;
    let mut failures: Vec<(String, String)> = Vec::new();

    for name in &order {
        let pkg = filtered[name];
        let mut failed = false;

        // Install step
        if let Some(ref install_field) = pkg.install {
            if !install::is_installed(name) {
                if let Some(cmd) =
                    install::resolve_install_command(name, install_field, &current, &config.managers)
                {
                    if let Err(e) = install::run_install(&cmd, name, args.yes, args.dry_run) {
                        failures.push((name.clone(), e.to_string()));
                        failed = true;
                    }
                }
            }
        }

        // Link step
        if !failed {
            if let Some(ref files) = pkg.files {
                let mode = pkg.link_mode.unwrap_or(LinkMode::Symlink);
                let errs = link::deploy_files(files, config_dir, mode, args.force, args.dry_run);
                if !errs.is_empty() {
                    failures.push((name.clone(), errs.join("; ")));
                    failed = true;
                }
            }
        }

        if !failed {
            succeeded += 1;
        }
    }

    // Summary
    let total = order.len();
    let failed_count = failures.len();
    println!(
        "\n{}: {} packages processed, {} succeeded, {} failed",
        "Summary".bold(),
        total,
        succeeded.to_string().green(),
        if failed_count > 0 {
            failed_count.to_string().red().to_string()
        } else {
            "0".to_string()
        }
    );

    if !failures.is_empty() {
        println!("\n{}:", "Failures".red().bold());
        for (pkg, err) in &failures {
            println!("  {} {}: {}", "✗".red(), pkg, err);
        }
    }

    Ok(())
}
