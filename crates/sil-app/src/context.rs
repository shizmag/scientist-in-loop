//! Application context management.

use camino::Utf8PathBuf;
use sil_core::Config;
use sil_core::paths::ProjectPaths;

use crate::error::AppError;

/// Shared application context holding the project root, paths, and loaded configuration.
#[derive(Debug, Clone)]
pub struct AppContext {
    /// Root path of the project workspace.
    pub root: Utf8PathBuf,
    /// Resolved project path structure.
    pub paths: ProjectPaths,
    /// Loaded or default project configuration.
    pub config: Config,
}

impl AppContext {
    /// Create an `AppContext` from an explicit project root directory.
    pub fn from_root(root: impl Into<Utf8PathBuf>) -> Result<Self, AppError> {
        let root = root.into();
        let paths = ProjectPaths::new(&root);
        let config = Config::load(&paths.config()).unwrap_or_default();
        Ok(Self {
            root,
            paths,
            config,
        })
    }

    /// Resolve the project root from the process current working directory and create an `AppContext`.
    pub fn from_cwd() -> Result<Self, AppError> {
        let root = sil_core::project_root_from_cwd().map_err(|_| AppError::NotInProject)?;
        Self::from_root(root)
    }
}
