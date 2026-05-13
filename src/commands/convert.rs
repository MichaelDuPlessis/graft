use crate::cli::ConvertArgs;
use crate::config;
use crate::error::{GraftError, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

pub fn run(args: &ConvertArgs, config_path: Option<&Path>) -> Result<()> {
    let (cfg, source_path) = config::load(config_path)?;

    let target_ext = match args.format.as_str() {
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        _ => return Err(GraftError::ConfigParse(format!("unsupported format: {}", args.format))),
    };

    let source_ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    if source_ext == target_ext || (source_ext == "yml" && target_ext == "yaml") {
        println!("Config is already in {} format.", target_ext);
        return Ok(());
    }

    let target_filename = format!("graft.{}", target_ext);
    let target_path = source_path.parent().unwrap_or(Path::new(".")).join(&target_filename);

    if target_path.exists() {
        return Err(GraftError::ConfigAlreadyExists(target_filename));
    }

    let output = config::serialize(&cfg, target_ext)?;
    fs::write(&target_path, output)?;
    fs::remove_file(&source_path)?;

    println!(
        "{} Converted {} → {}",
        "✓".green(),
        source_path.file_name().unwrap().to_string_lossy().dimmed(),
        target_filename.bold()
    );
    Ok(())
}
