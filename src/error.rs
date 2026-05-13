use std::fmt;
use std::io;

#[derive(Debug)]
pub enum GraftError {
    ConfigNotFound,
    ConfigParse(String),
    #[allow(dead_code)]
    OsDetectionFailed,
    CycleDetected(Vec<String>),
    MissingDependency { package: String, dependency: String },
    UnknownPackage(String),
    ConfigAlreadyExists(String),
    #[allow(dead_code)]
    SourceNotFound(String),
    InstallFailed { package: String, exit_code: i32 },
    IoError(io::Error),
}

impl fmt::Display for GraftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigNotFound => write!(f, "No config file found (searched graft.toml, graft.yaml, graft.json)"),
            Self::ConfigParse(detail) => write!(f, "Config parse error: {detail}"),
            Self::OsDetectionFailed => write!(f, "Could not detect OS. Use --os <platform> to specify."),
            Self::CycleDetected(cycle) => write!(f, "Dependency cycle detected: {}", cycle.join(" → ")),
            Self::MissingDependency { package, dependency } => write!(f, "Package \"{package}\" depends on \"{dependency}\", which is not available."),
            Self::UnknownPackage(name) => write!(f, "Unknown package: \"{name}\""),
            Self::ConfigAlreadyExists(path) => write!(f, "Config file already exists: {path}"),
            Self::SourceNotFound(path) => write!(f, "Source not found: {path}"),
            Self::InstallFailed { package, exit_code } => write!(f, "Install failed for \"{package}\" (exit code {exit_code})"),
            Self::IoError(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl std::error::Error for GraftError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for GraftError {
    fn from(e: io::Error) -> Self {
        Self::IoError(e)
    }
}

pub type Result<T> = std::result::Result<T, GraftError>;
