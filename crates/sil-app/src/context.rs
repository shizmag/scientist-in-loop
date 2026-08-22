//! Application context management.

use camino::{Utf8Path, Utf8PathBuf};
use sil_agent::{
    ContextFlags, ContextInput, SkillSelection, build_agent_state, generate_context_envelope,
    sources_summary,
};
use sil_core::paths::ProjectPaths;
use sil_core::{AgentContextEnvelope, AgentState, Config, SilError, Structure};
use sil_db::SilDb;
use sil_git::log_entries;

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

/// Retrieve the canonical, deterministic [`AgentState`] for a project.
pub fn get_agent_state(
    root: &Utf8Path,
    flags: &ContextFlags,
    task: Option<&str>,
) -> Result<AgentState, SilError> {
    let paths = ProjectPaths::new(root);
    let config_yaml = std::fs::read_to_string(paths.config().as_str()).unwrap_or_default();
    let structure_yaml = std::fs::read_to_string(paths.structure().as_str()).unwrap_or_default();
    let structure = Structure::load(&paths.structure()).ok();
    let summary = if paths.db().is_file() {
        if let Ok(db) = SilDb::open(&paths.db()) {
            sources_summary(&db).unwrap_or_default()
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let log = log_entries(root, 15, true).unwrap_or_default();

    let mut skills = if let Some(t) = task {
        SkillSelection::from_task(t)
    } else {
        SkillSelection::always()
    };
    skills.merge_flags(flags);

    let input = ContextInput {
        root,
        config_yaml: &config_yaml,
        structure_yaml: &structure_yaml,
        structure: structure.as_ref(),
        sources_summary: &summary,
        log_entries: &log,
        flags,
        skills,
    };

    build_agent_state(&input).map_err(SilError::from)
}

/// Retrieve the [`AgentContextEnvelope`] packaging canonical deterministic state with execution metadata.
pub fn get_context_envelope(
    root: &Utf8Path,
    flags: &ContextFlags,
    task: Option<&str>,
) -> Result<AgentContextEnvelope, SilError> {
    let paths = ProjectPaths::new(root);
    let config_yaml = std::fs::read_to_string(paths.config().as_str()).unwrap_or_default();
    let structure_yaml = std::fs::read_to_string(paths.structure().as_str()).unwrap_or_default();
    let structure = Structure::load(&paths.structure()).ok();
    let summary = if paths.db().is_file() {
        if let Ok(db) = SilDb::open(&paths.db()) {
            sources_summary(&db).unwrap_or_default()
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let log = log_entries(root, 15, true).unwrap_or_default();

    let mut skills = if let Some(t) = task {
        SkillSelection::from_task(t)
    } else {
        SkillSelection::always()
    };
    skills.merge_flags(flags);

    let input = ContextInput {
        root,
        config_yaml: &config_yaml,
        structure_yaml: &structure_yaml,
        structure: structure.as_ref(),
        sources_summary: &summary,
        log_entries: &log,
        flags,
        skills,
    };

    generate_context_envelope(&input).map_err(SilError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_agent_state_and_envelope() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        std::fs::create_dir_all(root.join(".sil")).unwrap();
        std::fs::create_dir_all(root.join("agent/skills")).unwrap();
        std::fs::write(root.join("agent/skills/SYSTEM.md"), "# SYSTEM").unwrap();
        std::fs::write(
            root.join(".sil/config.yaml"),
            "project:\n  title: Test App\n  stage: draft\n",
        )
        .unwrap();
        std::fs::write(
            root.join(".sil/structure.yaml"),
            "title: Test App\nsections: []\n",
        )
        .unwrap();

        let flags = ContextFlags::default();
        let state = get_agent_state(root, &flags, Some("edit draft")).unwrap();
        assert_eq!(state.project.title, "Test App");
        assert_eq!(state.schema_version, sil_core::AGENT_STATE_SCHEMA_VERSION);

        let env = get_context_envelope(root, &flags, None).unwrap();
        assert_eq!(env.state.project.title, "Test App");
        assert!(env.execution.is_some());
    }
}
