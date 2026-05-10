use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Platform(pub String);

impl Platform {
    pub fn new(s: &str) -> Self {
        Platform(s.to_lowercase())
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Parse a string into a Platform (for clap).
pub fn parse_platform(s: &str) -> std::result::Result<Platform, String> {
    Ok(Platform::new(s))
}

pub fn detect(override_platform: Option<&Platform>) -> Result<Platform> {
    if let Some(p) = override_platform {
        return Ok(p.clone());
    }

    #[cfg(target_os = "macos")]
    {
        return Ok(Platform::new("macos"));
    }

    #[cfg(target_os = "linux")]
    {
        let content = std::fs::read_to_string("/etc/os-release")
            .map_err(|_| crate::error::GraftError::OsDetectionFailed)?;
        for line in content.lines() {
            if let Some(id) = line.strip_prefix("ID=") {
                let id = id.trim_matches('"');
                return Ok(Platform::new(id));
            }
        }
        return Err(crate::error::GraftError::OsDetectionFailed);
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err(crate::error::GraftError::OsDetectionFailed)
    }
}

/// Returns true if the platform represents a Linux distro (anything that's not macOS).
pub fn is_linux(platform: &Platform) -> bool {
    platform.0 != "macos"
}

pub fn matches(package_os: &[Platform], current: &Platform) -> bool {
    if package_os.is_empty() {
        return true;
    }
    if package_os.contains(current) {
        return true;
    }
    // "linux" is a catch-all for any Linux distro
    if package_os.iter().any(|p| p.0 == "linux") && is_linux(current) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_empty_os_list() {
        assert!(matches(&[], &Platform::new("macos")));
    }

    #[test]
    fn matches_exact_platform() {
        assert!(matches(&[Platform::new("arch")], &Platform::new("arch")));
        assert!(!matches(&[Platform::new("arch")], &Platform::new("ubuntu")));
    }

    #[test]
    fn matches_linux_catchall() {
        assert!(matches(&[Platform::new("linux")], &Platform::new("arch")));
        assert!(matches(&[Platform::new("linux")], &Platform::new("ubuntu")));
        assert!(matches(&[Platform::new("linux")], &Platform::new("fedora")));
        assert!(!matches(&[Platform::new("linux")], &Platform::new("macos")));
    }

    #[test]
    fn detect_with_override() {
        let p = Platform::new("arch");
        let result = detect(Some(&p)).unwrap();
        assert_eq!(result, Platform::new("arch"));
    }
}
