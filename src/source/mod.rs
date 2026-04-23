pub mod github;
pub mod local;

use crate::error::DotmError;
use std::path::PathBuf;

/// Trait que deben implementar todas las fuentes de dotfiles
pub trait Source {
    /// Descarga o actualiza la fuente, devuelve el path local al directorio de dotfiles
    fn fetch(&self, dry_run: bool) -> Result<PathBuf, DotmError>;

    /// Descripción legible de la fuente para mostrar al usuario
    fn describe(&self) -> String;
}
