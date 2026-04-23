use clap::{Parser, Subcommand};

/// dotm — Dotfiles Manager for Arch Linux and beyond
#[derive(Parser, Debug)]
#[command(
    name    = "dotm",
    version,
    author,
    about   = "A fast dotfiles manager for Arch-based Linux distributions",
    long_about = None,
)]
pub struct Cli {
    /// Path to a custom config file (default: ~/.config/dotm/config.toml)
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<String>,

    /// Simulate all operations without writing anything
    #[arg(long, global = true)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a sample config file at ~/.config/dotm/config.toml
    Init,

    /// Download source (if GitHub) and apply all mappings
    Install,

    /// Update source and re-apply all mappings
    Sync,

    /// Show the current status of all mappings
    Status,

    /// Show line-by-line diff between source and installed files
    Diff {
        /// Filter by filename or path substring
        #[arg(value_name = "FILTER")]
        filter: Option<String>,
    },

    /// Move a system file into the dotfiles repo and register the mapping
    Add {
        /// Path to the file to add (e.g. ~/.zshrc)
        #[arg(value_name = "FILE")]
        file: String,
    },

    /// Create a manual backup of all current destination files
    Backup,

    /// Restore the latest backup
    Restore,
}
