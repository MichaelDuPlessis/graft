use crate::config::{Install, default_managers};
use crate::error::{GraftError, Result};
use crate::platform::Platform;
use dialoguer::Confirm;
use std::collections::HashMap;
use std::process::Command;

pub fn is_installed(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn resolve_install_command(
    _pkg_name: &str,
    install: &Install,
    platform: &Platform,
    managers: &HashMap<Platform, String>,
) -> Option<String> {
    let package = match install {
        Install::Simple(s) => s.clone(),
        Install::PerPlatform(map) => map.get(platform)?.clone(),
    };

    let defaults = default_managers();
    let prefix = managers.get(platform).or_else(|| defaults.get(platform))?;

    Some(format!("{} {}", prefix, package))
}

pub fn resolve_command(
    pkg: &crate::config::PackageConfig,
    platform: &Platform,
    managers: &HashMap<Platform, String>,
) -> Option<String> {
    if let Some(ref cmd) = pkg.install_command {
        return Some(cmd.clone());
    }
    if let Some(ref install) = pkg.install {
        return resolve_install_command("", install, platform, managers);
    }
    None
}

pub fn run_install(command: &str, pkg_name: &str, yes: bool, dry_run: bool) -> Result<()> {
    if dry_run {
        println!("Would run: {command}");
        return Ok(());
    }

    if !yes {
        let confirmed = Confirm::new()
            .with_prompt(format!("Install {pkg_name} via \"{command}\"?"))
            .default(false)
            .interact()
            .unwrap_or(false);
        if !confirmed {
            return Ok(());
        }
    }

    let status = Command::new("sh").arg("-c").arg(command).status()?;

    if !status.success() {
        return Err(GraftError::InstallFailed {
            package: pkg_name.to_string(),
            exit_code: status.code().unwrap_or(1),
        });
    }

    Ok(())
}
