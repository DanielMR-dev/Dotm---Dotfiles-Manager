use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DotmError {
    #[error("Config file not found at {0}")]
    ConfigNotFound(PathBuf),

    #[error("Failed to parse config: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("IO error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Source path does not exist: {0}")]
    SourceNotFound(PathBuf),

    #[error("Failed to create symlink from {src} to {dest}: {reason}")]
    SymlinkFailed {
        src: PathBuf,
        dest: PathBuf,
        reason: String,
    },

    #[error("Git error: {0}")]
    Git(String),

    // NOTE: Reserved for future GitHub API integration
    #[expect(dead_code)]
    #[error("GitHub error: {0}")]
    GitHub(String),

    // NOTE: Reserved for future backup error handling
    #[expect(dead_code)]
    #[error("Backup failed: {0}")]
    BackupFailed(String),

    #[error("Home directory could not be resolved")]
    NoHomeDir,

    #[error("{0}")]
    Other(String),
}

// Helpers de construcción
impl DotmError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub fn symlink(
        src: impl Into<PathBuf>,
        dest: impl Into<PathBuf>,
        reason: impl Into<String>,
    ) -> Self {
        Self::SymlinkFailed {
            src: src.into(),
            dest: dest.into(),
            reason: reason.into(),
        }
    }
}
