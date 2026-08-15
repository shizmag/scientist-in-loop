//! MCP project-root and caller-path confinement.

use camino::{Utf8Path, Utf8PathBuf};
use sil_core::{Config, ProjectPaths};

/// Explicit, canonical project context used by an MCP server.
#[derive(Debug, Clone)]
pub struct McpContext {
    /// Canonical project root.
    pub root: Utf8PathBuf,
    /// Canonical roots declared by project configuration.
    pub allowed_roots: Vec<Utf8PathBuf>,
    /// Whether the root was discovered from CWD for an interactive fallback.
    pub discovered_from_cwd: bool,
}

impl McpContext {
    /// Validate and construct a context from an explicit project root.
    pub fn from_root(root: impl AsRef<Utf8Path>) -> Result<Self, String> {
        Self::build(root.as_ref(), false)
    }

    /// Discover a project root from CWD. This is retained only for direct interactive use.
    pub fn from_cwd() -> Result<Self, String> {
        let root =
            sil_core::project_root_from_cwd().map_err(|e| format!("Not in a sil project: {e}"))?;
        Self::build(&root, true)
    }

    fn build(root: &Utf8Path, discovered_from_cwd: bool) -> Result<Self, String> {
        let canonical = root
            .canonicalize()
            .map_err(|e| format!("Invalid MCP project root '{root}': {e}"))
            .and_then(|path| {
                Utf8PathBuf::from_path_buf(path)
                    .map_err(|_| format!("Invalid MCP project root '{root}': non-UTF-8 path"))
            })?;
        let paths = ProjectPaths::new(&canonical);
        if !paths.config().is_file() {
            return Err(format!(
                "Invalid MCP project root '{root}': missing .sil/config.yaml"
            ));
        }
        // Keep the existing MCP fixture/project rule: a config file establishes a project even
        // when an older minimal config cannot be decoded by the typed application config.
        let config = Config::load(&paths.config()).unwrap_or_default();

        let mut allowed_roots = vec![canonical.clone()];
        for configured in [
            &config.paths.sources,
            &config.paths.data,
            &config.paths.figures,
            &config.paths.agent,
            &config.latex.main,
        ] {
            if configured.is_absolute()
                && let Ok(path) = configured.canonicalize()
                && let Ok(path) = Utf8PathBuf::from_path_buf(path)
                && !allowed_roots.iter().any(|root| root == &path)
            {
                allowed_roots.push(path);
            }
        }

        Ok(Self {
            root: canonical,
            allowed_roots,
            discovered_from_cwd,
        })
    }

    /// Resolve an existing caller path and require it to remain under an allowlisted root.
    pub fn confine_existing(&self, raw: &str) -> Result<Utf8PathBuf, String> {
        let path = Utf8PathBuf::from(raw);
        if path
            .components()
            .any(|component| component.as_str() == "..")
        {
            return Err(format!("Rejected path traversal: {raw}"));
        }
        let candidate = if path.is_absolute() {
            path
        } else {
            self.root.join(path)
        };
        let canonical = candidate
            .canonicalize()
            .map_err(|e| format!("Path not found: {raw}: {e}"))
            .and_then(|path| {
                Utf8PathBuf::from_path_buf(path)
                    .map_err(|_| format!("Path is not valid UTF-8: {raw}"))
            })?;
        if self
            .allowed_roots
            .iter()
            .any(|root| canonical == *root || canonical.starts_with(root))
        {
            Ok(canonical)
        } else {
            Err(format!("Rejected path outside MCP allowlist: {raw}"))
        }
    }

    /// Validate a registry skill identifier and resolve its file beneath `agent/skills`.
    pub fn skill_path(&self, name: &str) -> Result<Utf8PathBuf, String> {
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || name.contains("..")
            || Utf8Path::new(name).is_absolute()
            || !name.ends_with(".md")
        {
            return Err(format!("Rejected skill identifier: {name}"));
        }
        let skills_root = self.root.join("agent/skills").canonicalize().map_err(|e| {
            format!(
                "Skill registry is unavailable at {}: {e}",
                self.root.join("agent/skills")
            )
        })?;
        let skills_root = Utf8PathBuf::from_path_buf(skills_root)
            .map_err(|_| "Skill registry path is not valid UTF-8".to_string())?;
        let path = skills_root.join(name);
        let canonical = path
            .canonicalize()
            .map_err(|e| format!("Skill '{name}' is not declared in the registry: {e}"))
            .and_then(|path| {
                Utf8PathBuf::from_path_buf(path)
                    .map_err(|_| format!("Skill '{name}' path is not valid UTF-8"))
            })?;
        if canonical.parent() == Some(skills_root.as_path()) && canonical.is_file() {
            Ok(canonical)
        } else {
            Err(format!("Skill '{name}' is not declared in the registry"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn confines_paths_and_rejects_traversal_and_escape() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
        fs::create_dir_all(root.join(".sil")).unwrap();
        fs::write(root.join(".sil/config.yaml"), "version: 1\n").unwrap();
        fs::write(root.join("inside.txt"), "ok").unwrap();
        let ctx = McpContext::from_root(&root).unwrap();
        assert!(ctx.confine_existing("inside.txt").is_ok());
        assert!(ctx.confine_existing("../outside.txt").is_err());
        assert!(ctx.confine_existing("/tmp").is_err());
    }

    #[test]
    fn skill_registry_rejects_absolute_traversal_and_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
        fs::create_dir_all(root.join(".sil")).unwrap();
        fs::create_dir_all(root.join("agent/skills")).unwrap();
        fs::write(root.join(".sil/config.yaml"), "version: 1\n").unwrap();
        fs::write(root.join("agent/skills/SYSTEM.md"), "system").unwrap();
        let ctx = McpContext::from_root(&root).unwrap();
        assert!(ctx.skill_path("SYSTEM.md").is_ok());
        assert!(ctx.skill_path("../secret.md").is_err());
        assert!(ctx.skill_path("/tmp/secret.md").is_err());
    }

    #[test]
    fn accepts_configured_absolute_external_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
        let external = Utf8PathBuf::from_path_buf(dir.path().join("external")).unwrap();
        fs::create_dir_all(root.join(".sil")).unwrap();
        fs::create_dir_all(&external).unwrap();
        let mut config = Config::default().to_yaml().unwrap();
        config = config.replace("sources: ./sources", &format!("sources: {external}"));
        fs::write(root.join(".sil/config.yaml"), config).unwrap();
        fs::write(external.join("source.pdf"), "pdf").unwrap();
        let ctx = McpContext::from_root(&root).unwrap();
        assert!(
            ctx.confine_existing(external.join("source.pdf").as_str())
                .is_ok()
        );
        let external_canonical =
            Utf8PathBuf::from_path_buf(external.canonicalize().unwrap()).unwrap();
        assert!(
            ctx.allowed_roots
                .iter()
                .any(|path| path == &external_canonical)
        );
    }
}
