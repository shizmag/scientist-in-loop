//! Project path helpers.

use camino::{Utf8Path, Utf8PathBuf};

use crate::config::Config;
use crate::error::SilError;

/// Well-known relative paths inside a sil project.
pub mod rel {
    /// Sil metadata directory.
    pub const SIL_DIR: &str = ".sil";
    /// Config file.
    pub const CONFIG: &str = ".sil/config.yaml";
    /// Structure file.
    pub const STRUCTURE: &str = ".sil/structure.yaml";
    /// SQLite database.
    pub const DB: &str = ".sil/db.sqlite";
    /// Skills directory.
    pub const SKILLS: &str = ".sil/skills";
    /// SYSTEM skill.
    pub const SKILL_SYSTEM: &str = ".sil/skills/SYSTEM.md";
    /// Paper skill.
    pub const SKILL_PAPER: &str = ".sil/skills/paper.md";
    /// Agent-code skill.
    pub const SKILL_AGENT_CODE: &str = ".sil/skills/agent-code.md";
    /// Draft manuscript.
    pub const PAPER_DRAFT: &str = "paper_draft.tex";
    /// Final manuscript.
    pub const PAPER_FINAL: &str = "paper.tex";
    /// Bibliography.
    pub const REFERENCES: &str = "references.bib";
    /// Sources directory.
    pub const SOURCES: &str = "sources";
    /// Data directory.
    pub const DATA: &str = "data";
    /// Figures root.
    pub const FIGURES: &str = "figures";
    /// Generated plots.
    pub const FIGURES_PLOTS: &str = "figures/plots";
    /// External images.
    pub const FIGURES_IMAGES: &str = "figures/images";
    /// Agent code.
    pub const AGENT: &str = "agent";
    /// Project README.
    pub const README: &str = "README.md";
}

/// Resolved absolute paths for a project.
#[derive(Debug, Clone)]
pub struct ProjectPaths {
    /// Project root.
    pub root: Utf8PathBuf,
}

impl ProjectPaths {
    /// Create from project root.
    pub fn new(root: impl Into<Utf8PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Join a path relative to root.
    pub fn join(&self, rel: impl AsRef<Utf8Path>) -> Utf8PathBuf {
        self.root.join(rel)
    }

    /// `.sil/` directory.
    pub fn sil_dir(&self) -> Utf8PathBuf {
        self.join(rel::SIL_DIR)
    }

    /// Config path.
    pub fn config(&self) -> Utf8PathBuf {
        self.join(rel::CONFIG)
    }

    /// Structure path.
    pub fn structure(&self) -> Utf8PathBuf {
        self.join(rel::STRUCTURE)
    }

    /// SQLite database path.
    pub fn db(&self) -> Utf8PathBuf {
        self.join(rel::DB)
    }

    /// Skills directory.
    pub fn skills_dir(&self) -> Utf8PathBuf {
        self.join(rel::SKILLS)
    }

    /// Draft tex path.
    pub fn paper_draft(&self) -> Utf8PathBuf {
        self.join(rel::PAPER_DRAFT)
    }

    /// Final tex path.
    pub fn paper_final(&self) -> Utf8PathBuf {
        self.join(rel::PAPER_FINAL)
    }

    /// Sources directory from config (resolved against root).
    pub fn sources(&self, config: &Config) -> Utf8PathBuf {
        self.resolve_config_path(&config.paths.sources)
    }

    /// Data directory from config.
    pub fn data(&self, config: &Config) -> Utf8PathBuf {
        self.resolve_config_path(&config.paths.data)
    }

    /// Figures directory from config.
    pub fn figures(&self, config: &Config) -> Utf8PathBuf {
        self.resolve_config_path(&config.paths.figures)
    }

    /// Agent directory from config.
    pub fn agent(&self, config: &Config) -> Utf8PathBuf {
        self.resolve_config_path(&config.paths.agent)
    }

    /// Resolve a config-relative path against project root.
    pub fn resolve_config_path(&self, p: &Utf8Path) -> Utf8PathBuf {
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            // Strip leading ./
            let s = p.as_str().trim_start_matches("./");
            self.root.join(s)
        }
    }

    /// Whether this root looks like a sil project.
    pub fn is_project(&self) -> bool {
        self.config().is_file()
    }
}

/// Walk upward from `start` looking for `.sil/config.yaml`.
pub fn find_project_root(start: &Utf8Path) -> Option<Utf8PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = ProjectPaths::new(&current);
        if candidate.is_project() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Resolve project root from the process current directory.
pub fn project_root_from_cwd() -> Result<Utf8PathBuf, SilError> {
    let cwd = std::env::current_dir()?;
    let cwd = Utf8PathBuf::from_path_buf(cwd).map_err(|_| {
        SilError::Message("current directory is not valid UTF-8".into())
    })?;
    find_project_root(&cwd).ok_or(SilError::NotAProject)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolve_relative_paths() {
        let paths = ProjectPaths::new("/tmp/my-paper");
        assert_eq!(paths.config().as_str(), "/tmp/my-paper/.sil/config.yaml");
        assert_eq!(paths.db().as_str(), "/tmp/my-paper/.sil/db.sqlite");
    }

    #[test]
    fn find_root_walks_up() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        fs::create_dir_all(root.join(".sil")).unwrap();
        fs::write(root.join(".sil/config.yaml"), "x: 1").unwrap();
        let nested = root.join("a/b/c");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(find_project_root(&nested).unwrap(), root);
    }
}
