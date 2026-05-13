use crate::platform::{self, Platform};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "graft", version, about = "OS-aware dotfile and tool manager")]
pub struct Cli {
    /// Path to config file (overrides auto-detection)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate a starter config file in the current directory
    Init(InitArgs),
    /// Convert config to a different format
    Convert(ConvertArgs),
    /// Validate the config file
    Check(CheckArgs),
    /// Deploy packages (install tools + link files)
    Apply(ApplyArgs),
    /// Remove deployed files (unlink/delete)
    Remove(RemoveArgs),
    /// Add a new package entry to the config file
    Add(AddArgs),
    /// Scan a directory for configs and import them into the graft repo
    Scan(ScanArgs),
    /// Show deployment state of packages
    Status(StatusArgs),
    /// List available packages with OS/tag info
    List(ListArgs),
}

#[derive(Debug, clap::Args)]
pub struct InitArgs {
    /// Config format to generate: toml, yaml, json (default: toml)
    #[arg(long)]
    pub format: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct ConvertArgs {
    /// Target format: toml, yaml, json
    pub format: String,
}

#[derive(Debug, clap::Args)]
pub struct CheckArgs {}

#[derive(Debug, clap::Args)]
pub struct ApplyArgs {
    /// Package names to deploy. If omitted, deploy all applicable.
    pub packages: Vec<String>,

    /// Only deploy packages with this tag (repeatable)
    #[arg(long)]
    pub tag: Vec<String>,

    /// Override OS detection
    #[arg(long, value_parser = platform::parse_platform)]
    pub os: Option<Platform>,

    /// Skip confirmation prompts for installs
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Overwrite existing files without prompting
    #[arg(short = 'f', long)]
    pub force: bool,

    /// Show what would be done without doing it
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, clap::Args)]
pub struct RemoveArgs {
    /// Package names to remove. If omitted, remove all deployed.
    pub packages: Vec<String>,

    /// Only remove packages with this tag (repeatable)
    #[arg(long)]
    pub tag: Vec<String>,

    /// Override OS detection
    #[arg(long, value_parser = platform::parse_platform)]
    pub os: Option<Platform>,

    /// Show what would be done without doing it
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, clap::Args)]
pub struct AddArgs {
    /// Name for the new package entry
    pub name: String,

    /// Platforms this package applies to (repeatable)
    #[arg(long, value_parser = platform::parse_platform)]
    pub os: Vec<Platform>,

    /// Package name for the system package manager
    #[arg(long)]
    pub install: Option<String>,

    /// File mapping as source:destination (repeatable)
    #[arg(long)]
    pub files: Vec<String>,

    /// "symlink" or "copy"
    #[arg(long)]
    pub link_mode: Option<String>,

    /// Tag for the package (repeatable)
    #[arg(long)]
    pub tag: Vec<String>,

    /// Dependency on another package (repeatable)
    #[arg(long)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub struct StatusArgs {
    /// Override OS detection
    #[arg(long, value_parser = platform::parse_platform)]
    pub os: Option<Platform>,
}

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Filter by tag (repeatable)
    #[arg(long)]
    pub tag: Vec<String>,

    /// Override OS detection
    #[arg(long, value_parser = platform::parse_platform)]
    pub os: Option<Platform>,
}

#[derive(Debug, clap::Args)]
pub struct ScanArgs {
    /// Directory to scan for config files/directories
    pub path: std::path::PathBuf,

    /// Import all discovered items without prompting
    #[arg(short = 'a', long)]
    pub all: bool,

    /// Prompt for tags, OS, and link mode per item
    #[arg(short = 'd', long)]
    pub detailed: bool,

    /// Tag all imported packages with this tag (repeatable)
    #[arg(long)]
    pub tag: Vec<String>,

    /// Set the os field on all imported packages (repeatable)
    #[arg(long, value_parser = platform::parse_platform)]
    pub os: Vec<Platform>,

    /// Link mode for imported packages: "symlink" or "copy" (default: symlink)
    #[arg(long)]
    pub link_mode: Option<String>,
}
