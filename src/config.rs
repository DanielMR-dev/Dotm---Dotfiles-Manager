use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::DotmError;

// ─── Tipos públicos ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum SourceConfig {
    /// Repositorio de GitHub clonado localmente
    GitHub {
        url: String,
        #[serde(default = "default_branch")]
        branch: String,
    },
    /// Carpeta local con los dotfiles
    Local { path: String },
}

fn default_branch() -> String {
    "main".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LinkMethod {
    #[default]
    Symlink,
    Copy,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Options {
    /// Crear backup de archivos existentes antes de sobreescribir
    #[serde(default = "bool_true")]
    pub backup: bool,

    /// Método de aplicación: symlink (por defecto) o copy
    #[serde(default)]
    pub method: LinkMethod,

    /// Simula todas las operaciones sin escribir nada al disco
    #[serde(default)]
    pub dry_run: bool,

    /// Directorio de backups (por defecto ~/.local/share/dotm/backups)
    pub backup_dir: Option<String>,
}

fn bool_true() -> bool {
    true
}

impl Default for Options {
    fn default() -> Self {
        Self {
            backup: true,
            method: LinkMethod::Symlink,
            dry_run: false,
            backup_dir: None,
        }
    }
}

/// Configuración completa de dotm (~/.config/dotm/config.toml)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub source: SourceConfig,

    /// Mappings: "ruta/en/dotfiles" => "~/.destino/en/sistema"
    #[serde(default)]
    pub mappings: HashMap<String, String>,

    #[serde(default)]
    pub options: Options,
}

// ─── Implementación ──────────────────────────────────────────────────────────

impl Config {
    /// Carga la config desde el path estándar ~/.config/dotm/config.toml
    pub fn load_default() -> Result<Self, DotmError> {
        let path = default_config_path()?;
        Self::load_from(&path)
    }

    /// Carga la config desde un path específico
    pub fn load_from(path: &Path) -> Result<Self, DotmError> {
        let raw = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(DotmError::ConfigNotFound(path.to_path_buf()));
            }
            Err(e) => return Err(DotmError::io(path, e)),
        };

        let config: Config = toml::from_str(&raw)?;
        Ok(config)
    }

    /// Escribe una config de ejemplo al path estándar si no existe
    pub fn write_example(path: &Path) -> Result<(), DotmError> {
        let example = r#"# dotm configuration file
# Documentation: https://github.com/DanielMR-dev/dotm

[source]
type   = "github"
url    = "https://github.com/TU_USUARIO/dotfiles"
branch = "main"

# Para usar una carpeta local en su lugar:
# [source]
# type = "local"
# path = "~/dotfiles"

# Mappings: "ruta relativa en tu repo" = "destino absoluto en el sistema"
[mappings]
"zsh/.zshrc"              = "~/.zshrc"
"zsh/.zsh_aliases"        = "~/.zsh_aliases"
"hypr/hyprland.conf"      = "~/.config/hypr/hyprland.conf"
"ghostty/config"          = "~/.config/ghostty/config"
"nvim/"                   = "~/.config/nvim/"
"git/.gitconfig"          = "~/.gitconfig"

[options]
backup  = true        # Hace backup antes de sobreescribir
method  = "symlink"   # "symlink" o "copy"
dry_run = false       # true = simula sin escribir nada
"#;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DotmError::io(parent, e))?;
        }

        std::fs::write(path, example).map_err(|e| DotmError::io(path, e))?;

        Ok(())
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Devuelve la ruta por defecto de la config del usuario
pub fn default_config_path() -> Result<PathBuf, DotmError> {
    let home = dirs::home_dir().ok_or(DotmError::NoHomeDir)?;
    Ok(home.join(".config/dotm/config.toml"))
}

/// Expande "~" al home real del usuario en una ruta
pub fn expand_tilde(path: &str) -> Result<PathBuf, DotmError> {
    if let Some(stripped) = path.strip_prefix("~/") {
        let home = dirs::home_dir().ok_or(DotmError::NoHomeDir)?;
        Ok(home.join(stripped))
    } else if path == "~" {
        dirs::home_dir().ok_or(DotmError::NoHomeDir)
    } else {
        Ok(PathBuf::from(path))
    }
}
