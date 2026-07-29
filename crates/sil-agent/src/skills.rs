//! Dynamic skill loading rules.

use std::fs;

use camino::Utf8Path;
use sil_core::paths::rel;

use crate::error::ContextError;

/// Which optional context sections to include.
#[derive(Debug, Clone, Default)]
pub struct ContextFlags {
    /// Include paper_draft.tex split into subsections.
    pub paper: bool,
    /// Include agent/ listing + README.
    pub agent: bool,
    /// Include paper.md skill.
    pub skill_paper: bool,
    /// Include agent-code.md skill.
    pub skill_agent_code: bool,
    /// Explicit skill names to load (basename without path).
    pub skills: Vec<String>,
}

/// Skill loading intent derived from a task description or flags.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillSelection {
    /// Always true when generating context.
    pub system: bool,
    /// Load paper.md.
    pub paper: bool,
    /// Load agent-code.md.
    pub agent_code: bool,
}

impl SkillSelection {
    /// Always-loaded baseline.
    pub fn always() -> Self {
        Self {
            system: true,
            paper: false,
            agent_code: false,
        }
    }

    /// Apply conditional loading rules from a free-text task description.
    pub fn from_task(task: &str) -> Self {
        let t = task.to_ascii_lowercase();
        let mut s = Self::always();
        if t.contains("structure.yaml")
            || t.contains("paper_draft.tex")
            || t.contains("paper.tex")
            || t.contains("section")
            || t.contains("completion")
            || t.contains("manuscript")
            || t.contains("write")
            || t.contains("draft")
        {
            s.paper = true;
        }
        if t.contains("agent/")
            || t.contains("agent\\")
            || t.contains("script")
            || t.contains("reproducib")
            || t.contains("agent-code")
        {
            s.agent_code = true;
        }
        s
    }

    /// Merge explicit CLI flags.
    pub fn merge_flags(&mut self, flags: &ContextFlags) {
        if flags.skill_paper || flags.paper {
            self.paper = true;
        }
        if flags.skill_agent_code || flags.agent {
            self.agent_code = true;
        }
        for name in &flags.skills {
            let n = name.to_ascii_lowercase();
            if n.contains("paper") {
                self.paper = true;
            }
            if n.contains("agent") {
                self.agent_code = true;
            }
        }
    }
}

/// Load skill file contents from the project.
pub fn load_skill(root: &Utf8Path, name: &str) -> Result<String, ContextError> {
    let path = root.join(rel::SKILLS).join(name);
    fs::read_to_string(path.as_str()).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ContextError::MissingSkill(path.to_string())
        } else {
            ContextError::Io(format!("{path}: {e}"))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_selection_from_task() {
        let s = SkillSelection::from_task("edit paper_draft.tex introduction");
        assert!(s.system);
        assert!(s.paper);
        let s2 = SkillSelection::from_task("add a parser under agent/");
        assert!(s2.agent_code);
        let s3 = SkillSelection::from_task("list sources");
        assert!(!s3.paper);
        assert!(!s3.agent_code);
    }

    #[test]
    fn from_task_structure_and_completion() {
        assert!(SkillSelection::from_task("update structure.yaml").paper);
        assert!(SkillSelection::from_task("mark section completion").paper);
        assert!(SkillSelection::from_task("edit paper.tex").paper);
        assert!(SkillSelection::from_task("reproducibility script").agent_code);
    }

    #[test]
    fn merge_flags_enables_skills() {
        let mut s = SkillSelection::always();
        s.merge_flags(&ContextFlags {
            paper: true,
            agent: false,
            skill_paper: false,
            skill_agent_code: true,
            skills: vec!["paper.md".into()],
        });
        assert!(s.paper);
        assert!(s.agent_code);
    }

    #[test]
    fn merge_flags_skills_list() {
        let mut s = SkillSelection::always();
        s.merge_flags(&ContextFlags {
            paper: false,
            agent: false,
            skill_paper: false,
            skill_agent_code: false,
            skills: vec!["agent-code.md".into()],
        });
        assert!(s.agent_code);
        assert!(!s.paper);
    }

    #[test]
    fn always_has_system_only() {
        let s = SkillSelection::always();
        assert!(s.system);
        assert!(!s.paper);
        assert!(!s.agent_code);
    }

    #[test]
    fn load_skill_missing() {
        let dir = tempfile::tempdir().unwrap();
        let root = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let err = load_skill(&root, "SYSTEM.md").unwrap_err();
        assert!(matches!(err, ContextError::MissingSkill(_)));
    }

    #[test]
    fn load_skill_ok() {
        let dir = tempfile::tempdir().unwrap();
        let root = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        std::fs::create_dir_all(root.join(".sil/skills")).unwrap();
        std::fs::write(root.join(".sil/skills/SYSTEM.md"), "hello skill").unwrap();
        let text = load_skill(&root, "SYSTEM.md").unwrap();
        assert_eq!(text, "hello skill");
    }
}
