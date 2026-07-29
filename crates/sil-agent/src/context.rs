//! Building the agent/human context dump.

use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use sil_core::{Config, ProjectPaths, SilError, Structure, paths::rel};
use sil_db::SilDb;
use sil_git::LogEntry;

use crate::error::ContextError;
use crate::paper::{format_subsections_markdown, paper_subsections};
use crate::skills::{ContextFlags, SkillSelection, load_skill};

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

    if input.skills.system {
        let system = load_skill(input.root, "SYSTEM.md")?;
        out.push_str("## Skill: SYSTEM.md\n\n");
        out.push_str(&system);
        out.push_str("\n\n");
    }

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
            out.push_str(&format!("- `{}` **{act}** — {}\n", e.hash, e.subject));
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
                let sections = paper_subsections(&tex);
                out.push_str(&format_subsections_markdown(&sections));
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
pub fn load_project_texts(
    paths: &ProjectPaths,
) -> Result<(Config, String, Structure, String), SilError> {
    let config = Config::load(&paths.config())?;
    let config_yaml = fs::read_to_string(paths.config().as_str())?;
    let structure = Structure::load(&paths.structure())?;
    let structure_yaml = fs::read_to_string(paths.structure().as_str())?;
    Ok((config, config_yaml, structure, structure_yaml))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::SkillSelection;

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

    #[test]
    fn context_includes_paper_and_agent_sections() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        fs::create_dir_all(root.join(".sil/skills")).unwrap();
        fs::create_dir_all(root.join("agent")).unwrap();
        fs::write(root.join(".sil/skills/SYSTEM.md"), "SYS").unwrap();
        fs::write(root.join(".sil/skills/paper.md"), "PAPER SKILL").unwrap();
        fs::write(root.join(".sil/skills/agent-code.md"), "AGENT SKILL").unwrap();
        fs::write(
            root.join("paper_draft.tex"),
            "\\section{Intro}\nHello.\n\\section{Methods}\nWorld.\n",
        )
        .unwrap();
        fs::write(root.join("agent/README.md"), "# Agent\n").unwrap();
        fs::write(root.join("agent/script.py"), "print(1)\n").unwrap();

        let flags = ContextFlags {
            paper: true,
            agent: true,
            skill_paper: true,
            skill_agent_code: true,
            skills: vec![],
        };
        let mut skills = SkillSelection::always();
        skills.merge_flags(&flags);
        let input = ContextInput {
            root: &root,
            config_yaml: "x: 1",
            structure_yaml: "title: t",
            structure: None,
            sources_summary: "none",
            log_entries: &[],
            flags: &flags,
            skills,
        };
        let ctx = generate_context(&input).unwrap();
        assert!(ctx.contains("PAPER SKILL"));
        assert!(ctx.contains("AGENT SKILL"));
        assert!(ctx.contains("Paper content"));
        assert!(ctx.contains("Intro"));
        assert!(ctx.contains("Agent directory"));
        assert!(ctx.contains("script.py"));
    }

    #[test]
    fn sources_summary_lists_docs() {
        let db = SilDb::open_in_memory().unwrap();
        let empty = sources_summary(&db).unwrap();
        assert!(empty.contains("0"));
        let mut doc = sil_core::SourceDocument::new("a.pdf".into());
        doc.parsed = true;
        doc.status = Some(sil_core::DocumentStatus::ValidPdf);
        db.upsert_parsed(&doc, "text").unwrap();
        let s = sources_summary(&db).unwrap();
        assert!(s.contains("a.pdf"));
        assert!(s.contains("parsed"));
    }

    #[test]
    fn context_with_log_entries() {
        use sil_core::SciAction;
        use sil_git::LogEntry;

        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        fs::create_dir_all(root.join(".sil/skills")).unwrap();
        fs::write(root.join(".sil/skills/SYSTEM.md"), "SYS").unwrap();
        let entries = [LogEntry {
            hash: "abc123".into(),
            subject: "Initialize".into(),
            action: Some(SciAction::Init),
        }];
        let flags = ContextFlags::default();
        let input = ContextInput {
            root: &root,
            config_yaml: "c",
            structure_yaml: "s",
            structure: None,
            sources_summary: "sum",
            log_entries: &entries,
            flags: &flags,
            skills: SkillSelection::always(),
        };
        let ctx = generate_context(&input).unwrap();
        assert!(ctx.contains("abc123"));
        assert!(ctx.contains("init"));
        assert!(ctx.contains("Initialize"));
    }

    fn fixture_root_with_system() -> (tempfile::TempDir, Utf8PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        fs::create_dir_all(root.join(".sil/skills")).unwrap();
        fs::write(root.join(".sil/skills/SYSTEM.md"), "SYS").unwrap();
        (dir, root)
    }

    #[test]
    fn context_missing_paper_draft_is_graceful() {
        let (_d, root) = fixture_root_with_system();
        let flags = ContextFlags {
            paper: true,
            ..Default::default()
        };
        let input = ContextInput {
            root: &root,
            config_yaml: "c",
            structure_yaml: "s",
            structure: None,
            sources_summary: "sum",
            log_entries: &[],
            flags: &flags,
            skills: SkillSelection::always(),
        };
        let ctx = generate_context(&input).unwrap();
        assert!(ctx.contains("Paper content"));
        assert!(
            ctx.contains("Could not read") || ctx.contains("paper_draft"),
            "{ctx}"
        );
    }

    #[test]
    fn context_large_draft_no_panic_and_structured() {
        let (_d, root) = fixture_root_with_system();
        let mut tex = String::from("\\documentclass{article}\n\\begin{document}\n");
        for i in 0..200 {
            tex.push_str(&format!("\\section{{Section {i}}}\n"));
            tex.push_str(&"word ".repeat(80));
            tex.push('\n');
        }
        tex.push_str("\\end{document}\n");
        fs::write(root.join("paper_draft.tex"), &tex).unwrap();

        let flags = ContextFlags {
            paper: true,
            ..Default::default()
        };
        let input = ContextInput {
            root: &root,
            config_yaml: "c",
            structure_yaml: "s",
            structure: None,
            sources_summary: "sum",
            log_entries: &[],
            flags: &flags,
            skills: SkillSelection::always(),
        };
        let ctx = generate_context(&input).unwrap();
        assert!(ctx.contains("Section 0"));
        assert!(ctx.contains("Section 199"));
        // Structured: section headers present, not a single raw dump without markers
        assert!(ctx.matches("### \\").count() >= 50);
    }

    #[test]
    fn context_flag_combinations() {
        let (_d, root) = fixture_root_with_system();
        fs::write(root.join(".sil/skills/paper.md"), "PAPER").unwrap();
        fs::write(root.join(".sil/skills/agent-code.md"), "AGENTCODE").unwrap();
        fs::create_dir_all(root.join("agent")).unwrap();
        fs::write(root.join("agent/README.md"), "readme").unwrap();

        // paper only
        let flags = ContextFlags {
            paper: true,
            skill_paper: true,
            ..Default::default()
        };
        let mut skills = SkillSelection::always();
        skills.merge_flags(&flags);
        let input = ContextInput {
            root: &root,
            config_yaml: "c",
            structure_yaml: "s",
            structure: None,
            sources_summary: "sum",
            log_entries: &[],
            flags: &flags,
            skills,
        };
        let ctx = generate_context(&input).unwrap();
        assert!(ctx.contains("PAPER"));
        assert!(!ctx.contains("AGENTCODE"));

        // agent only
        let flags = ContextFlags {
            agent: true,
            skill_agent_code: true,
            ..Default::default()
        };
        let mut skills = SkillSelection::always();
        skills.merge_flags(&flags);
        let input = ContextInput {
            root: &root,
            config_yaml: "c",
            structure_yaml: "s",
            structure: None,
            sources_summary: "sum",
            log_entries: &[],
            flags: &flags,
            skills,
        };
        let ctx = generate_context(&input).unwrap();
        assert!(ctx.contains("AGENTCODE"));
        assert!(ctx.contains("Agent directory"));
        assert!(!ctx.contains("PAPER"));
    }

    #[test]
    fn missing_system_skill_fails_context() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        // no skills at all
        let flags = ContextFlags::default();
        let input = ContextInput {
            root: &root,
            config_yaml: "c",
            structure_yaml: "s",
            structure: None,
            sources_summary: "sum",
            log_entries: &[],
            flags: &flags,
            skills: SkillSelection::always(),
        };
        let err = generate_context(&input).unwrap_err();
        assert!(err.to_string().contains("missing") || err.to_string().contains("SYSTEM"));
    }

    #[test]
    fn optional_paper_skill_missing_is_ok() {
        let (_d, root) = fixture_root_with_system();
        // request paper skill but file absent
        let mut skills = SkillSelection::always();
        skills.paper = true;
        let flags = ContextFlags::default();
        let input = ContextInput {
            root: &root,
            config_yaml: "c",
            structure_yaml: "s",
            structure: None,
            sources_summary: "sum",
            log_entries: &[],
            flags: &flags,
            skills,
        };
        let ctx = generate_context(&input).unwrap();
        assert!(ctx.contains("SYS"));
        assert!(!ctx.contains("## Skill: paper.md"));
    }

    #[test]
    fn agent_listing_nested_files() {
        let (_d, root) = fixture_root_with_system();
        fs::create_dir_all(root.join("agent/sub")).unwrap();
        fs::write(root.join("agent/sub/nested.py"), "print(1)\n").unwrap();
        let flags = ContextFlags {
            agent: true,
            ..Default::default()
        };
        let input = ContextInput {
            root: &root,
            config_yaml: "c",
            structure_yaml: "s",
            structure: None,
            sources_summary: "sum",
            log_entries: &[],
            flags: &flags,
            skills: SkillSelection::always(),
        };
        let ctx = generate_context(&input).unwrap();
        assert!(ctx.contains("nested.py") || ctx.contains("sub/"));
    }

    #[test]
    fn sources_summary_unparsed_flag() {
        let db = SilDb::open_in_memory().unwrap();
        // insert via upsert always marks parsed=1; summary of empty still ok
        let s = sources_summary(&db).unwrap();
        assert!(s.contains("0"));
    }
}
