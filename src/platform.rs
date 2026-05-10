use crate::error::{GraftError, Result};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    MacOs,
    Arch,
    Ubuntu,
    Linux,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Platform::MacOs => write!(f, "macos"),
            Platform::Arch => write!(f, "arch"),
            Platform::Ubuntu => write!(f, "ubuntu"),
            Platform::Linux => write!(f, "linux"),
        }
    }
}

pub fn detect(override_platform: Option<Platform>) -> Result<Platform> {
    if let Some(p) = override_platform {
        return Ok(p);
    }

    #[cfg(target_os = "macos")]
    {
        let output = Command::new("uname").arg("-s").output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim() == "Darwin" {
            return Ok(Platform::MacOs);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let content = std::fs::read_to_string("/etc/os-release")?;
        for line in content.lines() {
            if let Some(id) = line.strip_prefix("ID=") {
                let id = id.trim_matches('"');
                return match id {
                    "arch" => Ok(Platform::Arch),
                    "ubuntu" => Ok(Platform::Ubuntu),
                    _ => Err(GraftError::OsDetectionFailed),
                };
            }
        }
    }

    Err(GraftError::OsDetectionFailed)
}

pub fn matches(package_os: &[Platform], current: &Platform) -> bool {
    if package_os.is_empty() {
        return true;
    }
    if package_os.contains(current) {
        return true;
    }
    if package_os.contains(&Platform::Linux) && matches!(current, Platform::Arch | Platform::Ubuntu)
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_empty_os_list() {
        assert!(matches(&[], &Platform::MacOs));
    }

    #[test]
    fn matches_exact_platform() {
        assert!(matches(&[Platform::Arch], &Platform::Arch));
        assert!(!matches(&[Platform::Arch], &Platform::Ubuntu));
    }

    #[test]
    fn matches_linux_catchall() {
        assert!(matches(&[Platform::Linux], &Platform::Arch));
        assert!(matches(&[Platform::Linux], &Platform::Ubuntu));
        assert!(!matches(&[Platform::Linux], &Platform::MacOs));
    }

    #[test]
    fn detect_with_override() {
        let result = detect(Some(Platform::Arch)).unwrap();
        assert_eq!(result, Platform::Arch);
    }
}
