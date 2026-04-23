mod backup;
mod cli;
mod config;
mod error;
mod linker;
mod manager;
mod source;

use clap::Parser;
use colored::Colorize;

use cli::{Cli, Commands};
use config::Config;

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("\n  {} {}\n", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    // Inicializar no necesita config
    if let Commands::Init = &cli.command {
        manager::init()?;
        return Ok(());
    }

    // Cargar configuración
    let mut config = match &cli.config {
        Some(path) => Config::load_from(std::path::Path::new(path))?,
        None => Config::load_default()?,
    };

    // El flag --dry-run de CLI tiene precedencia sobre config
    if cli.dry_run {
        config.options.dry_run = true;
    }

    // Despachar al comando correspondiente
    match &cli.command {
        Commands::Init => unreachable!(),
        Commands::Install => manager::install(&config)?,
        Commands::Sync => manager::sync(&config)?,
        Commands::Status => manager::status(&config)?,
        Commands::Diff { filter } => manager::diff(&config, filter.as_deref())?,
        Commands::Add { file } => manager::add(&config, file)?,
        Commands::Backup => {
            let backup_dir = match &config.options.backup_dir {
                Some(p) => config::expand_tilde(p)?,
                None => backup::default_backup_dir()?,
            };
            println!("\n{}", "dotm backup".bold());
            let mut count = 0u32;
            // Note: source_root is intentionally not used - we only need dest paths for backup
            let _source_root = source_local_path(&config)?;
            for dest_str in config.mappings.values() {
                let dest = config::expand_tilde(dest_str)?;
                if let Ok(Some(_)) = backup::backup_file(&dest, &backup_dir, config.options.dry_run)
                {
                    count += 1;
                }
            }
            println!(
                "\n  {} {} files backed up to {}",
                "✓".green(),
                count,
                backup_dir.display()
            );
        }
        Commands::Restore => {
            let backup_dir = match &config.options.backup_dir {
                Some(p) => config::expand_tilde(p)?,
                None => backup::default_backup_dir()?,
            };
            println!("\n{}", "dotm restore".bold());
            let count = backup::restore_latest(&backup_dir, config.options.dry_run)?;
            println!("\n  {} {} files restored", "✓".green(), count);
        }
    }

    println!();
    Ok(())
}

fn source_local_path(config: &Config) -> Result<std::path::PathBuf, error::DotmError> {
    match &config.source {
        config::SourceConfig::GitHub { .. } => {
            let home = dirs::home_dir().ok_or(error::DotmError::NoHomeDir)?;
            Ok(home.join(".local/share/dotm/repo"))
        }
        config::SourceConfig::Local { path } => config::expand_tilde(path),
    }
}
