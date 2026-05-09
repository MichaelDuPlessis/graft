use std::fs;
use std::path::Path;

use colored::Colorize;

use crate::cli::StatusArgs;
use crate::config::{self, LinkMode};
use crate::error::Result;
use crate::install::is_installed;
use crate::platform;

pub fn run(args: &StatusArgs, config_path: Option<&Path>) -> Result<()> {
    let (cfg, config_file_path) = config::load(config_path)?;
    let config_dir = config_file_path.parent().unwrap();
    let current_platform = platform::detect(args.os)?;

    for (name, pkg) in &cfg.packages {
        if !platform::matches(pkg.os.as_deref().unwrap_or(&[]), &current_platform) {
            continue;
        }

        println!("{}", name.bold());

        if pkg.install.is_some() {
            if is_installed(name) {
                println!("  Installed: {}", "✓".green());
            } else {
                println!("  Installed: {}", "✗".red());
            }
        }

        let files = match &pkg.files {
            Some(f) => f,
            None => {
                println!();
                continue;
            }
        };
        let link_mode = pkg.link_mode.unwrap_or(LinkMode::Symlink);

        for (src, dest_str) in files {
            let source = config_dir.join(src).canonicalize().unwrap_or_else(|_| config_dir.join(src));
            let dest = config::expand_tilde(dest_str);

            let status = if dest.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
                let target = fs::read_link(&dest).unwrap_or_default();
                let target_canon = target.canonicalize().unwrap_or_else(|_| target.clone());
                if target_canon == source || target == source {
                    "linked".green()
                } else {
                    "conflict".red()
                }
            } else if dest.exists() {
                if link_mode == LinkMode::Copy {
                    "copied".green()
                } else {
                    "conflict".red()
                }
            } else {
                "missing".yellow()
            };

            println!("  {} → {} [{}]", src, dest_str, status);
        }
        println!();
    }

    Ok(())
}
