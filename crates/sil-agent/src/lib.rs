//! Dynamic skills loading and context generation for agents/humans.
//!
//! Stage 0: skill loading rules + context builder skeleton.
//! Stage 5: full `sil context` integration.

#![deny(missing_docs)]

use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use sil_core::{Config, ProjectPaths, SilError, Structure, paths::rel};
use sil_db::SilDb;
use sil_git::LogEntry;
use sil_latex::split_tex_sections;
use thiserror::Error;

/// Context generation errors.
#[derive(Debug, Error)]
pub enum ContextError {
    /// I/O failure.
    #[error("I/O: {0}")]
    Io(String),
    /// Missing required skill.
    #[error("missing skill file: {0}")]
    MissingSkill(String),
    /// Other.
    #[error("{0}")]
    Message(String),
}

impl From<ContextError> for SilError {
    fn from(value: ContextError) -> Self {
        SilError::Message(value.to_string())
    }
}

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

/// Inputs for building a full context document.
pub struct ContextInput<'a> {
    /// Project root.
    pub root: &'a Utf8Path,
    /// Loaded config YAML text (or serialized).
    pub config_yaml: &'a str,
    /// Loaded structure YAML text.
    pub structure_yaml: &'a str,
    /// Optional parsed structure for summaries.
    pub structure: Option<&'a Structure>,
    /// Source summary lines.
    pub sources_summary: &'a str,
    /// Recent git log entries with Sci-Action.
    pub log_entries: &'a [LogEntry],
    /// Flags controlling optional sections.
    pub flags: &'a ContextFlags,
    /// Optional skill selection override.
    pub skills: SkillSelection,
}

/// Generate the full context markdown string.
pub fn generate_context(input: &ContextInput<'_>) -> Result<String, ContextError> {
    let mut out = String::new();
    out.push_str("# sil project context\n\n");

    // SYSTEM.md always
    if input.skills.system {
        let system = load_skill(input.root, "SYSTEM.md")?;
        out.push_str("## Skill: SYSTEM.md\n\n");
        out.push_str(&system);
        out.push_str("\n\n");
    }

    // Conditional skills
    if input.skills.paper {
        match load_skill(input.root, "paper.md") {
            Ok(s) => {
                out.push_str("## Skill: paper.md\n\n");
                out.push_str(&s);
                out.push_str("\n\n");
            }
            Err(ContextError::MissingSkill(_)) => {}
            Err(e) => return Err(e),
        }
    }
    if input.skills.agent_code {
        match load_skill(input.root, "agent-code.md") {
            Ok(s) => {
                out.push_str("## Skill: agent-code.md\n\n");
                out.push_str(&s);
                out.push_str("\n\n");
            }
            Err(ContextError::MissingSkill(_)) => {}
            Err(e) => return Err(e),
        }
    }

    out.push_str("## structure.yaml\n\n```yaml\n");
    out.push_str(input.structure_yaml.trim_end());
    out.push_str("\n```\n\n");

    out.push_str("## config.yaml\n\n```yaml\n");
    out.push_str(input.config_yaml.trim_end());
    out.push_str("\n```\n\n");

    out.push_str("## Recent Sci-Action history\n\n");
    if input.log_entries.is_empty() {
        out.push_str("_No commits with Sci-Action trailers yet._\n\n");
    } else {
        for e in input.log_entries {
            let act = e
                .action
                .map(|a| a.as_str().to_string())
                .unwrap_or_else(|| "-".into());
            out.push_str(&format!("- `{}` **{}** — {}\n", e.hash, act, e.subject));
        }
        out.push('\n');
    }

    out.push_str("## Sources summary\n\n");
    out.push_str(input.sources_summary.trim_end());
    out.push_str("\n\n");

    if let Some(st) = input.structure {
        out.push_str("## Structure completion\n\n");
        out.push_str(&format!("{}\n\n", st.completion_summary()));
    }

    if input.flags.paper {
        out.push_str("## Paper content (subsections)\n\n");
        let draft = input.root.join(rel::PAPER_DRAFT);
        match fs::read_to_string(draft.as_str()) {
            Ok(tex) => {
                let sections = split_tex_sections(&tex);
                for sec in sections {
                    out.push_str(&format!(
                        "### \\{}{{{}}}  (line {})\n\n",
                        sec.kind, sec.title, sec.line_start
                    ));
                    let body = sec.body.trim();
                    if body.is_empty() {
                        out.push_str("_empty_\n\n");
                    } else {
                        out.push_str("```tex\n");
                        out.push_str(body);
                        out.push_str("\n```\n\n");
                    }
                }
            }
            Err(e) => {
                out.push_str(&format!("_Could not read paper_draft.tex: {e}_\n\n"));
            }
        }
    }

    if input.flags.agent {
        out.push_str("## Agent directory\n\n");
        let agent_dir = input.root.join(rel::AGENT);
        let readme = agent_dir.join("README.md");
        if readme.is_file() {
            match fs::read_to_string(readme.as_str()) {
                Ok(r) => {
                    out.push_str("### agent/README.md\n\n");
                    out.push_str(&r);
                    out.push_str("\n\n");
                }
                Err(e) => out.push_str(&format!("_README read error: {e}_\n\n")),
            }
        }
        out.push_str("### Listing\n\n");
        match list_dir_recursive(&agent_dir, &agent_dir) {
            Ok(entries) if entries.is_empty() => {
                out.push_str("_No files yet (only README)._\n\n");
            }
            Ok(entries) => {
                for e in entries {
                    out.push_str(&format!("- `{e}`\n"));
                }
                out.push('\n');
            }
            Err(e) => out.push_str(&format!("_list error: {e}_\n\n")),
        }
    }

    Ok(out)
}

fn list_dir_recursive(base: &Utf8Path, dir: &Utf8Path) -> Result<Vec<String>, ContextError> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    let rd = fs::read_dir(dir.as_str()).map_err(|e| ContextError::Io(e.to_string()))?;
    let mut paths: Vec<Utf8PathBuf> = Vec::new();
    for ent in rd {
        let ent = ent.map_err(|e| ContextError::Io(e.to_string()))?;
        let p = Utf8PathBuf::from_path_buf(ent.path())
            .map_err(|_| ContextError::Io("non-utf8 path".into()))?;
        paths.push(p);
    }
    paths.sort();
    for p in paths {
        if p.is_dir() {
            out.extend(list_dir_recursive(base, &p)?);
        } else {
            let rel = p.strip_prefix(base).unwrap_or(&p);
            out.push(rel.as_str().replace('\\', "/"));
        }
    }
    Ok(out)
}

/// Build a short sources summary from the database.
pub fn sources_summary(db: &SilDb) -> Result<String, ContextError> {
    let count = db
        .source_count()
        .map_err(|e| ContextError::Message(e.to_string()))?;
    let parsed = db
        .parsed_count()
        .map_err(|e| ContextError::Message(e.to_string()))?;
    let mut s = format!("Sources in database: {count} (parsed: {parsed})\n");
    if count > 0 {
        let list = db
            .list_sources()
            .map_err(|e| ContextError::Message(e.to_string()))?;
        for doc in list {
            let flag = if doc.parsed { "parsed" } else { "unparsed" };
            s.push_str(&format!("- {} [{flag}]\n", doc.filename));
        }
    }
    Ok(s)
}

/// Convenience: load config + structure paths for a project.
pub fn load_project_texts(paths: &ProjectPaths) -> Result<(Config, String, Structure, String), SilError> {
    let config = Config::load(&paths.config())?;
    let config_yaml = fs::read_to_string(paths.config().as_str())?;
    let structure = Structure::load(&paths.structure())?;
    let structure_yaml = fs::read_to_string(paths.structure().as_str())?;
    Ok((config, config_yaml, structure, structure_yaml))
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
    fn generate_minimal_context() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        fs::create_dir_all(root.join(".sil/skills")).unwrap();
        fs::write(
            root.join(".sil/skills/SYSTEM.md"),
            "# SYSTEM RULES\nAlways read this.\n",
        )
        .unwrap();
        let flags = ContextFlags::default();
        let input = ContextInput {
            root: &root,
            config_yaml: "project:\n  stage: draft\n",
            structure_yaml: "title: T\nsections: []\n",
            structure: None,
            sources_summary: "Sources in database: 0 (parsed: 0)\n",
            log_entries: &[],
            flags: &flags,
            skills: SkillSelection::always(),
        };
        let ctx = generate_context(&input).unwrap();
        assert!(ctx.contains("SYSTEM RULES"));
        assert!(ctx.contains("structure.yaml"));
        assert!(ctx.contains("config.yaml"));
        assert!(ctx.contains("Sources summary"));
    }
}
