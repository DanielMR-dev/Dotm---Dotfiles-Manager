use colored::Colorize;
use std::path::PathBuf;

use super::Source;
use crate::config::expand_tilde;
use crate::error::DotmError;

pub struct LocalSource {
    pub path: String,
}

impl Source for LocalSource {
    fn fetch(&self, _dry_run: bool) -> Result<PathBuf, DotmError> {
        let resolved = expand_tilde(&self.path)?;

        if !resolved.exists() {
            return Err(DotmError::SourceNotFound(resolved));
        }

        if !resolved.is_dir() {
            return Err(DotmError::Other(format!(
                "Local source path is not a directory: {}",
                resolved.display()
            )));
        }

        println!(
            "  {} Using local source: {}",
            "→".cyan(),
            resolved.display()
        );
        Ok(resolved)
    }

    fn describe(&self) -> String {
        format!("Local: {}", self.path)
    }
}
