use colored::Colorize;
use std::path::{Path, PathBuf};

use crate::backup::{backup_file, default_backup_dir};
use crate::config::{expand_tilde, LinkMethod, Options};
use crate::error::DotmError;

/// Estado de un mapping individual
#[derive(Debug)]
pub enum LinkStatus {
    /// El symlink/copia está correcto y apunta al source correcto
    Ok,
    /// El destino no existe (nunca se instaló)
    Missing,
    /// El destino existe pero no es un symlink (archivo real del usuario)
    Exists,
    /// Es un symlink pero apunta a otra ubicación (conflicto)
    Conflict { points_to: PathBuf },
    /// El source no existe en el repo/carpeta
    SourceMissing,
}

/// Resultado de aplicar un mapping
#[derive(Debug)]
#[expect(dead_code)]
pub struct LinkResult {
    /// Source path in the dotfiles repository
    pub src: PathBuf,
    /// Destination path on the system
    pub dest: PathBuf,
    /// Current status of the link
    pub status: LinkStatus,
    /// Human-readable action description for UI output
    pub action: String,
}

/// Aplica un mapping individual (src → dest) según las opciones configuradas
pub fn apply_mapping(
    source_root: &Path,
    src_rel: &str,
    dest_str: &str,
    options: &Options,
) -> Result<LinkResult, DotmError> {
    let src = source_root.join(src_rel);
    let dest = expand_tilde(dest_str)?;

    // El source debe existir
    if !src.exists() {
        return Ok(LinkResult {
            src: src.clone(),
            dest: dest.clone(),
            status: LinkStatus::SourceMissing,
            action: format!("{} Source missing: {}", "✗".red(), src.display()),
        });
    }

    let backup_dir = match &options.backup_dir {
        Some(p) => expand_tilde(p)?,
        None => default_backup_dir()?,
    };

    // Determinar qué hay en el destino actualmente
    let dest_status = inspect_dest(&dest, &src)?;

    match dest_status {
        LinkStatus::Ok => {
            return Ok(LinkResult {
                src: src.clone(),
                dest: dest.clone(),
                status: LinkStatus::Ok,
                action: format!("{} Already linked: {}", "✓".green(), dest.display()),
            });
        }
        LinkStatus::Conflict { ref points_to } => {
            println!(
                "  {} Conflict at {} (points to {})",
                "!".yellow(),
                dest.display(),
                points_to.display()
            );
            if options.backup {
                // Elimina el symlink conflictivo
                if !options.dry_run {
                    std::fs::remove_file(&dest).map_err(|e| DotmError::io(&dest, e))?;
                }
            }
        }
        LinkStatus::Exists => {
            if options.backup {
                backup_file(&dest, &backup_dir, options.dry_run)?;
            }
            if !options.dry_run {
                std::fs::remove_file(&dest).map_err(|e| DotmError::io(&dest, e))?;
            }
        }
        LinkStatus::Missing => {
            // Crear directorios padre si no existen
            if let Some(parent) = dest.parent() {
                if !parent.exists() && !options.dry_run {
                    std::fs::create_dir_all(parent).map_err(|e| DotmError::io(parent, e))?;
                }
            }
        }
        LinkStatus::SourceMissing => unreachable!(),
    }

    // Aplicar el link/copia
    let action = match options.method {
        LinkMethod::Symlink => {
            apply_symlink(&src, &dest, options.dry_run)?;
            format!(
                "{} Linked: {} → {}",
                "✓".green(),
                dest.display(),
                src.display()
            )
        }
        LinkMethod::Copy => {
            apply_copy(&src, &dest, options.dry_run)?;
            format!(
                "{} Copied: {} → {}",
                "✓".green(),
                src.display(),
                dest.display()
            )
        }
    };

    Ok(LinkResult {
        src,
        dest,
        status: LinkStatus::Ok,
        action,
    })
}

/// Muestra el estado actual de un mapping sin hacer cambios
pub fn check_mapping(
    source_root: &Path,
    src_rel: &str,
    dest_str: &str,
) -> Result<LinkResult, DotmError> {
    let src = source_root.join(src_rel);
    let dest = expand_tilde(dest_str)?;

    if !src.exists() {
        return Ok(LinkResult {
            src: src.clone(),
            dest: dest.clone(),
            status: LinkStatus::SourceMissing,
            action: format!("{} Source missing: {}", "✗".red(), src.display()),
        });
    }

    let status = inspect_dest(&dest, &src)?;
    let action = status_display(&status, &dest);

    Ok(LinkResult {
        src,
        dest,
        status,
        action,
    })
}

/// Muestra las diferencias entre el archivo en el source y el destino actual
pub fn diff_mapping(source_root: &Path, src_rel: &str, dest_str: &str) -> Result<(), DotmError> {
    let src = source_root.join(src_rel);
    let dest = expand_tilde(dest_str)?;

    if !src.exists() {
        println!("  {} Source not found: {}", "✗".red(), src.display());
        return Ok(());
    }

    if !dest.exists() {
        println!(
            "  {} Destination not installed: {}",
            "!".yellow(),
            dest.display()
        );
        return Ok(());
    }

    // Si el destino es un symlink que apunta al source, son idénticos
    if dest.is_symlink() {
        if let Ok(target) = std::fs::read_link(&dest) {
            if target == src {
                println!(
                    "  {} {} is symlinked (identical)",
                    "✓".green(),
                    dest.display()
                );
                return Ok(());
            }
        }
    }

    // Lee ambos archivos y hace diff línea a línea básico
    let src_content = std::fs::read_to_string(&src).map_err(|e| DotmError::io(&src, e))?;
    let dest_content = std::fs::read_to_string(&dest).map_err(|e| DotmError::io(&dest, e))?;

    if src_content == dest_content {
        println!("  {} {} is identical", "✓".green(), dest.display());
    } else {
        println!("  {} Differences in {}:", "~".yellow(), dest.display());
        print_diff(&src_content, &dest_content);
    }

    Ok(())
}

// ─── Helpers privados ────────────────────────────────────────────────────────

fn inspect_dest(dest: &Path, src: &Path) -> Result<LinkStatus, DotmError> {
    if !dest.exists() && !dest.is_symlink() {
        return Ok(LinkStatus::Missing);
    }

    if dest.is_symlink() {
        let target = std::fs::read_link(dest).map_err(|e| DotmError::io(dest, e))?;
        if target == src {
            return Ok(LinkStatus::Ok);
        } else {
            return Ok(LinkStatus::Conflict { points_to: target });
        }
    }

    Ok(LinkStatus::Exists)
}

fn apply_symlink(src: &Path, dest: &Path, dry_run: bool) -> Result<(), DotmError> {
    if dry_run {
        println!(
            "  {} [dry-run] Would symlink: {} → {}",
            "~".yellow(),
            dest.display(),
            src.display()
        );
        return Ok(());
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(src, dest)
        .map_err(|e| DotmError::symlink(src, dest, e.to_string()))?;

    #[cfg(windows)]
    {
        if src.is_dir() {
            std::os::windows::fs::symlink_dir(src, dest)
                .map_err(|e| DotmError::symlink(src, dest, e.to_string()))?;
        } else {
            std::os::windows::fs::symlink_file(src, dest)
                .map_err(|e| DotmError::symlink(src, dest, e.to_string()))?;
        }
    }

    Ok(())
}

fn apply_copy(src: &Path, dest: &Path, dry_run: bool) -> Result<(), DotmError> {
    if dry_run {
        println!(
            "  {} [dry-run] Would copy: {} → {}",
            "~".yellow(),
            src.display(),
            dest.display()
        );
        return Ok(());
    }

    if src.is_dir() {
        copy_dir_recursive(src, dest)?;
    } else {
        std::fs::copy(src, dest).map_err(|e| DotmError::io(src, e))?;
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), DotmError> {
    std::fs::create_dir_all(dest).map_err(|e| DotmError::io(dest, e))?;

    for entry in std::fs::read_dir(src).map_err(|e| DotmError::io(src, e))? {
        let entry = entry.map_err(|e| DotmError::io(src, e))?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path).map_err(|e| DotmError::io(&src_path, e))?;
        }
    }

    Ok(())
}

fn status_display(status: &LinkStatus, dest: &Path) -> String {
    match status {
        LinkStatus::Ok => format!("{} Linked:         {}", "✓".green(), dest.display()),
        LinkStatus::Missing => format!("{} Not installed:  {}", "○".yellow(), dest.display()),
        LinkStatus::Exists => format!("{} Unmanaged file: {}", "!".yellow(), dest.display()),
        LinkStatus::Conflict { .. } => format!("{} Wrong symlink:  {}", "✗".red(), dest.display()),
        LinkStatus::SourceMissing => format!("{} Source missing: {}", "✗".red(), dest.display()),
    }
}

fn print_diff(a: &str, b: &str) {
    let a_lines: Vec<&str> = a.lines().collect();
    let b_lines: Vec<&str> = b.lines().collect();
    let max = a_lines.len().max(b_lines.len());

    for i in 0..max {
        match (a_lines.get(i), b_lines.get(i)) {
            (Some(la), Some(lb)) if la != lb => {
                println!("    {} {}", "-".red(), la.red());
                println!("    {} {}", "+".green(), lb.green());
            }
            (Some(la), None) => println!("    {} {}", "-".red(), la.red()),
            (None, Some(lb)) => println!("    {} {}", "+".green(), lb.green()),
            _ => {}
        }
    }
}
