use crate::cli::ListArgs;
use crate::config::{self, LinkMode};
use crate::error::Result;
use crate::platform;
use colored::Colorize;
use std::path::Path;

pub fn run(args: &ListArgs, config_path: Option<&Path>) -> Result<()> {
    let (config, _) = config::load(config_path)?;
    let current = platform::detect(args.os)?;

    let mut packages: Vec<_> = config.packages.iter().collect();
    packages.sort_by_key(|(name, _)| *name);

    if !args.tag.is_empty() {
        packages.retain(|(_, pkg)| {
            pkg.tags
                .as_ref()
                .is_some_and(|t| args.tag.iter().any(|f| t.contains(f)))
        });
    }

    let name_w = packages
        .iter()
        .map(|(n, _)| n.len())
        .max()
        .unwrap_or(7)
        .max(7);

    println!(
        "{:<name_w$}  {:>10}  {:<20}  {}",
        "Package", "Applicable", "Tags", "Link Mode"
    );
    println!("{}", "-".repeat(name_w + 2 + 10 + 2 + 20 + 2 + 9));

    for (name, pkg) in &packages {
        let applicable = match &pkg.os {
            Some(os_list) => platform::matches(os_list, &current),
            None => true,
        };

        let applicable_str = if applicable {
            "yes".green().to_string()
        } else {
            "no".dimmed().to_string()
        };

        let tags = pkg
            .tags
            .as_ref()
            .filter(|t| !t.is_empty())
            .map(|t| t.join(", "))
            .unwrap_or_else(|| "-".into());

        let link_mode = match pkg.link_mode {
            Some(LinkMode::Copy) => "copy",
            _ => "symlink",
        };

        println!(
            "{:<name_w$}  {:>10}  {:<20}  {}",
            name, applicable_str, tags, link_mode
        );
    }

    Ok(())
}
