use crate::config;
use crate::error::Result;
use crate::resolve;
use colored::Colorize;
use std::collections::HashMap;
use std::path::Path;

pub fn run(config_path: Option<&Path>) -> Result<()> {
    let (cfg, cfg_path) = config::load(config_path)?;
    let config_dir = cfg_path.parent().unwrap();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    // Check dependencies resolve (no cycles, no missing refs)
    let pkg_refs: HashMap<String, &config::PackageConfig> =
        cfg.packages.iter().map(|(k, v)| (k.clone(), v)).collect();
    let all_names: Vec<String> = cfg.packages.keys().cloned().collect();

    // Use a dummy platform — we just want structural validation
    let dummy = crate::platform::Platform::new("macos");
    if let Err(e) = resolve::resolve_order(&pkg_refs, &all_names, &dummy) {
        errors.push(format!("{}", e));
    }

    // Check source files exist
    for (name, pkg) in &cfg.packages {
        if let Some(ref files) = pkg.files {
            for src in files.keys() {
                let source = config_dir.join(src);
                if !source.exists() {
                    warnings.push(format!("{}: source not found: {}", name, src));
                }
            }
        }
    }

    // Report
    if errors.is_empty() && warnings.is_empty() {
        println!("{} Config is valid ({})", "✓".green(), cfg_path.display());
        println!(
            "  {} package(s), {} manager(s)",
            cfg.packages.len().to_string().bold(),
            cfg.managers.len()
        );
    } else {
        if !errors.is_empty() {
            println!("{}:", "Errors".red().bold());
            for e in &errors {
                println!("  {} {}", "✗".red(), e);
            }
        }
        if !warnings.is_empty() {
            println!("{}:", "Warnings".yellow().bold());
            for w in &warnings {
                println!("  {} {}", "⚠".yellow(), w);
            }
        }
    }

    Ok(())
}
