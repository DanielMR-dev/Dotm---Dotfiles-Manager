use colored::Colorize;

use crate::config::{Config, SourceConfig};
use crate::error::DotmError;
use crate::linker::{apply_mapping, check_mapping, diff_mapping};
use crate::source::{github::GitHubSource, local::LocalSource, Source};

// ─── install ─────────────────────────────────────────────────────────────────

/// Descarga la fuente y aplica todos los mappings
pub fn install(config: &Config) -> Result<(), DotmError> {
    println!("\n{}", "dotm install".bold());
    println!("  Source: {}", describe_source(&config.source));

    let source = build_source(&config.source);
    let source_root = source.fetch(config.options.dry_run)?;

    println!("\n  Applying {} mappings...\n", config.mappings.len());

    let mut ok = 0u32;
    let mut errors = 0u32;

    for (src_rel, dest_str) in &config.mappings {
        match apply_mapping(&source_root, src_rel, dest_str, &config.options) {
            Ok(result) => {
                println!("  {}", result.action);
                ok += 1;
            }
            Err(e) => {
                println!("  {} {}", "✗".red(), e);
                errors += 1;
            }
        }
    }

    println!("\n  {} {} applied, {} errors", "→".cyan(), ok, errors);

    if config.options.dry_run {
        println!("  {} dry-run mode: no changes were made", "!".yellow());
    }

    Ok(())
}

// ─── sync ────────────────────────────────────────────────────────────────────

/// Actualiza la fuente (git pull / re-scan) y re-aplica todos los mappings
pub fn sync(config: &Config) -> Result<(), DotmError> {
    println!("\n{}", "dotm sync".bold());

    let source = build_source(&config.source);
    let source_root = source.fetch(config.options.dry_run)?;

    println!("\n  Re-applying {} mappings...\n", config.mappings.len());

    for (src_rel, dest_str) in &config.mappings {
        match apply_mapping(&source_root, src_rel, dest_str, &config.options) {
            Ok(result) => println!("  {}", result.action),
            Err(e) => println!("  {} {}", "✗".red(), e),
        }
    }

    println!("\n  {} Sync complete.", "✓".green());
    Ok(())
}

// ─── status ──────────────────────────────────────────────────────────────────

/// Muestra el estado actual de cada mapping sin hacer cambios
pub fn status(config: &Config) -> Result<(), DotmError> {
    println!("\n{}", "dotm status".bold());
    println!("  Source: {}\n", describe_source(&config.source));

    let _source = build_source(&config.source);
    // Para status no necesitamos fetch, solo conocer la ruta local
    let source_root = source_local_path(&config.source)?;

    let mut ok = 0u32;
    let mut issues = 0u32;

    for (src_rel, dest_str) in &config.mappings {
        match check_mapping(&source_root, src_rel, dest_str) {
            Ok(result) => {
                println!("  {}", result.action);
                match result.status {
                    crate::linker::LinkStatus::Ok => ok += 1,
                    _ => issues += 1,
                }
            }
            Err(e) => {
                println!("  {} {}", "✗".red(), e);
                issues += 1;
            }
        }
    }

    println!("\n  {} {} ok, {} issues", "→".cyan(), ok, issues);
    Ok(())
}

// ─── diff ────────────────────────────────────────────────────────────────────

/// Muestra diferencias entre los dotfiles en el source y los actuales del sistema
pub fn diff(config: &Config, filter: Option<&str>) -> Result<(), DotmError> {
    println!("\n{}", "dotm diff".bold());

    let source_root = source_local_path(&config.source)?;

    for (src_rel, dest_str) in &config.mappings {
        if let Some(f) = filter {
            if !src_rel.contains(f) && !dest_str.contains(f) {
                continue;
            }
        }
        println!("\n  {}", dest_str.bold());
        diff_mapping(&source_root, src_rel, dest_str)?;
    }

    Ok(())
}

// ─── add ─────────────────────────────────────────────────────────────────────

/// Añade un archivo del sistema al repo de dotfiles y crea el symlink
pub fn add(config: &Config, file_path: &str) -> Result<(), DotmError> {
    use crate::config::expand_tilde;

    println!("\n{}", "dotm add".bold());

    let file = expand_tilde(file_path)?;

    if !file.exists() {
        return Err(DotmError::Other(format!(
            "File not found: {}",
            file.display()
        )));
    }

    let source_root = source_local_path(&config.source)?;

    // Ruta relativa dentro del repo: usa el nombre del archivo
    let file_name = file
        .file_name()
        .ok_or_else(|| DotmError::Other("Cannot determine filename".into()))?;

    let dest_in_repo = source_root.join(file_name);

    if dest_in_repo.exists() {
        println!(
            "  {} Already in repo: {}",
            "!".yellow(),
            dest_in_repo.display()
        );
        return Ok(());
    }

    if !config.options.dry_run {
        std::fs::copy(&file, &dest_in_repo).map_err(|e| DotmError::io(&file, e))?;
    }

    println!(
        "  {} Copied to repo: {}",
        "✓".green(),
        dest_in_repo.display()
    );
    println!("\n  {} Add this to your config.toml mappings:", "→".cyan());
    println!(
        "      \"{}\" = \"{}\"",
        file_name.to_string_lossy(),
        file_path
    );

    Ok(())
}

// ─── init ────────────────────────────────────────────────────────────────────

/// Inicializa un config.toml de ejemplo
pub fn init() -> Result<(), DotmError> {
    let path = crate::config::default_config_path()?;

    if path.exists() {
        println!(
            "  {} Config already exists at {}",
            "!".yellow(),
            path.display()
        );
        return Ok(());
    }

    Config::write_example(&path)?;
    println!("  {} Config created at {}", "✓".green(), path.display());
    println!("  Edit it and run: dotm install");

    Ok(())
}

// ─── Helpers privados ────────────────────────────────────────────────────────

fn build_source(source_config: &SourceConfig) -> Box<dyn Source> {
    match source_config {
        SourceConfig::GitHub { url, branch } => Box::new(GitHubSource {
            url: url.clone(),
            branch: branch.clone(),
        }),
        SourceConfig::Local { path } => Box::new(LocalSource { path: path.clone() }),
    }
}

fn describe_source(source_config: &SourceConfig) -> String {
    build_source(source_config).describe()
}

/// Devuelve la ruta local de la fuente sin hacer fetch
fn source_local_path(source_config: &SourceConfig) -> Result<std::path::PathBuf, DotmError> {
    match source_config {
        SourceConfig::GitHub { .. } => {
            let home = dirs::home_dir().ok_or(DotmError::NoHomeDir)?;
            let path = home.join(".local/share/dotm/repo");
            if !path.exists() {
                return Err(DotmError::Other(
                    "Repository not cloned yet. Run 'dotm install' first.".into(),
                ));
            }
            Ok(path)
        }
        SourceConfig::Local { path } => crate::config::expand_tilde(path),
    }
}
