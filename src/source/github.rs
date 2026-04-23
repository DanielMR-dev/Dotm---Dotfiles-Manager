use colored::Colorize;
use std::path::PathBuf;

use super::Source;
use crate::error::DotmError;

/// Carpeta local donde se clona el repo: ~/.local/share/dotm/repo
fn local_clone_dir() -> Result<PathBuf, DotmError> {
    let home = dirs::home_dir().ok_or(DotmError::NoHomeDir)?;
    Ok(home.join(".local/share/dotm/repo"))
}

pub struct GitHubSource {
    pub url: String,
    pub branch: String,
}

impl Source for GitHubSource {
    fn fetch(&self, dry_run: bool) -> Result<PathBuf, DotmError> {
        let dest = local_clone_dir()?;

        if dry_run {
            println!(
                "  {} [dry-run] Would clone/pull {} → {}",
                "→".cyan(),
                self.url,
                dest.display()
            );
            return Ok(dest);
        }

        if dest.exists() {
            self.pull(&dest)
        } else {
            self.clone(&dest)
        }
    }

    fn describe(&self) -> String {
        format!("GitHub: {} (branch: {})", self.url, self.branch)
    }
}

impl GitHubSource {
    /// Clona el repositorio por primera vez
    #[cfg(feature = "git")]
    fn clone(&self, dest: &PathBuf) -> Result<PathBuf, DotmError> {
        use indicatif::{ProgressBar, ProgressStyle};

        println!("  {} Cloning {} ...", "↓".cyan(), self.url);

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("  {spinner:.cyan} {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.set_message("Cloning repository...");

        std::fs::create_dir_all(dest).map_err(|e| DotmError::io(dest, e))?;

        // Opciones de clone con checkout de la rama correcta
        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.depth(1); // shallow clone — más rápido

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_opts);
        builder.branch(&self.branch);

        builder
            .clone(&self.url, dest)
            .map_err(|e| DotmError::Git(e.message().to_string()))?;

        pb.finish_with_message(format!("{} Repository cloned", "✓".green()));
        Ok(dest.clone())
    }

    /// Actualiza el repositorio ya clonado (git pull)
    #[cfg(feature = "git")]
    fn pull(&self, dest: &PathBuf) -> Result<PathBuf, DotmError> {
        use git2::{MergeAnalysis, Repository};

        println!(
            "  {} Pulling latest changes from {} ...",
            "↑".cyan(),
            self.url
        );

        let repo = Repository::open(dest).map_err(|e| DotmError::Git(e.message().to_string()))?;

        // 1. fetch origin
        {
            let mut remote = repo
                .find_remote("origin")
                .map_err(|e| DotmError::Git(e.message().to_string()))?;

            remote
                .fetch(&[&self.branch], None, None)
                .map_err(|e| DotmError::Git(e.message().to_string()))?;
        }

        // 2. fast-forward HEAD
        let fetch_head = repo
            .find_reference("FETCH_HEAD")
            .map_err(|e| DotmError::Git(e.message().to_string()))?;

        let fetch_commit = repo
            .reference_to_annotated_commit(&fetch_head)
            .map_err(|e| DotmError::Git(e.message().to_string()))?;

        let (analysis, _) = repo
            .merge_analysis(&[&fetch_commit])
            .map_err(|e| DotmError::Git(e.message().to_string()))?;

        if analysis.contains(MergeAnalysis::ANALYSIS_UP_TO_DATE) {
            println!("  {} Already up to date.", "✓".green());
        } else if analysis.contains(MergeAnalysis::ANALYSIS_FASTFORWARD) {
            let mut reference = repo
                .find_reference(&format!("refs/heads/{}", self.branch))
                .map_err(|e| DotmError::Git(e.message().to_string()))?;

            reference
                .set_target(fetch_commit.id(), "Fast-forward")
                .map_err(|e| DotmError::Git(e.message().to_string()))?;

            repo.set_head(&format!("refs/heads/{}", self.branch))
                .map_err(|e| DotmError::Git(e.message().to_string()))?;

            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .map_err(|e| DotmError::Git(e.message().to_string()))?;

            println!("  {} Repository updated.", "✓".green());
        } else {
            return Err(DotmError::Git(
                "Cannot fast-forward. Please resolve manually.".into(),
            ));
        }

        Ok(dest.clone())
    }

    // Stubs cuando el feature git no está habilitado
    #[cfg(not(feature = "git"))]
    fn clone(&self, dest: &PathBuf) -> Result<PathBuf, DotmError> {
        Err(DotmError::Git("git feature not enabled".into()))
    }

    #[cfg(not(feature = "git"))]
    fn pull(&self, dest: &PathBuf) -> Result<PathBuf, DotmError> {
        Err(DotmError::Git("git feature not enabled".into()))
    }
}
