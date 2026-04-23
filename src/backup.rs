use colored::Colorize;
use std::path::{Path, PathBuf};

use crate::error::DotmError;

/// Directorio de backups por defecto
pub fn default_backup_dir() -> Result<PathBuf, DotmError> {
    let home = dirs::home_dir().ok_or(DotmError::NoHomeDir)?;
    Ok(home.join(".local/share/dotm/backups"))
}

/// Hace backup de un archivo existente antes de sobreescribirlo.
/// El backup se guarda con la fecha y hora actuales como sufijo.
/// Returns Ok(None) if the destination doesn't exist or is already a dotm symlink.
pub fn backup_file(
    dest: &Path,
    backup_dir: &Path,
    dry_run: bool,
) -> Result<Option<PathBuf>, DotmError> {
    // Quick check - don't back up if already a symlink (likely from dotm)
    // Note: This is a best-effort check. The actual copy will fail if file doesn't exist.
    if dest.is_symlink() {
        return Ok(None);
    }

    let timestamp = chrono_now();
    let filename = dest
        .file_name()
        .ok_or_else(|| DotmError::Other("Cannot determine filename for backup".into()))?
        .to_string_lossy();

    let backup_filename = format!("{}.{}.bak", filename, timestamp);

    // Preserva la estructura de directorios del destino dentro del backup_dir
    let relative = dest
        .strip_prefix(dirs::home_dir().unwrap_or_default())
        .unwrap_or(dest);

    let backup_path = backup_dir
        .join(relative.parent().unwrap_or(Path::new("")))
        .join(&backup_filename);

    if dry_run {
        println!(
            "  {} [dry-run] Would backup: {} → {}",
            "~".yellow(),
            dest.display(),
            backup_path.display()
        );
        return Ok(Some(backup_path));
    }

    if let Some(parent) = backup_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DotmError::io(parent, e))?;
    }

    // Attempt the copy - will succeed (file exists) or fail (file gone/missing).
    // If the file doesn't exist, return None to indicate no backup was made.
    match std::fs::copy(dest, &backup_path) {
        Ok(_) => {
            println!(
                "  {} Backed up: {} → {}",
                "~".yellow(),
                dest.display(),
                backup_path.display()
            );
            Ok(Some(backup_path))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(DotmError::io(dest, e)),
    }
}

/// Restaura todos los backups del último snapshot disponible
pub fn restore_latest(backup_dir: &Path, dry_run: bool) -> Result<u32, DotmError> {
    if !backup_dir.exists() {
        println!(
            "  {} No backups found at {}",
            "!".yellow(),
            backup_dir.display()
        );
        return Ok(0);
    }

    let mut count = 0u32;

    for entry in walkdir(backup_dir)? {
        let backup_path = entry;
        if !backup_path.is_file() {
            continue;
        }

        // Reconstruye la ruta original desde el backup
        let relative = backup_path.strip_prefix(backup_dir).unwrap_or(&backup_path);

        // Elimina el sufijo ".TIMESTAMP.bak"
        let original_name = relative.to_string_lossy();
        if let Some(pos) = original_name.rfind('.') {
            let without_bak = &original_name[..pos]; // elimina .bak
            if let Some(pos2) = without_bak.rfind('.') {
                let original_rel = &without_bak[..pos2]; // elimina .TIMESTAMP
                let home = dirs::home_dir().ok_or(DotmError::NoHomeDir)?;
                let dest = home.join(original_rel);

                if dry_run {
                    println!(
                        "  {} [dry-run] Would restore: {} → {}",
                        "↩".cyan(),
                        backup_path.display(),
                        dest.display()
                    );
                } else {
                    if let Some(parent) = dest.parent() {
                        std::fs::create_dir_all(parent).map_err(|e| DotmError::io(parent, e))?;
                    }
                    std::fs::copy(&backup_path, &dest)
                        .map_err(|e| DotmError::io(&backup_path, e))?;
                    println!("  {} Restored: {}", "↩".cyan(), dest.display());
                }
                count += 1;
            }
        }
    }

    Ok(count)
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn chrono_now() -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

fn walkdir(dir: &Path) -> Result<Vec<PathBuf>, DotmError> {
    let mut results = Vec::new();
    collect_files(dir, &mut results)?;
    Ok(results)
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), DotmError> {
    for entry in std::fs::read_dir(dir).map_err(|e| DotmError::io(dir, e))? {
        let entry = entry.map_err(|e| DotmError::io(dir, e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}
