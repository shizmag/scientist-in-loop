//! Building the agent/human context dump.

use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use sil_core::{
    AGENT_STATE_SCHEMA_VERSION, AgentContextEnvelope, AgentExecutionMetadata, AgentFinding,
    AgentState, AgentStateKind, AvailableAction, CapabilitySummary, Config, FindingClass,
    HealthSummary, InputSnapshot, JobSummary, LiteratureSummary, LockSummary,
    ManuscriptHealthReport, PaperKind, ProjectIdentity, ProjectPaths, SilError, Stage, Structure,
    StructureSummary, WorkItemSummary, input_fingerprint, paths::rel, sanitize_agent_state,
};
use sil_db::SilDb;
use sil_git::LogEntry;

use crate::error::ContextError;
use crate::paper::{format_subsections_markdown, paper_subsections};
use crate::registry::SkillRegistry;
use crate::skills::{ContextFlags, SkillRouter, SkillSelection, load_skill};

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

/// Build a canonical, deterministic `AgentState` snapshot from context inputs.
pub fn build_agent_state(input: &ContextInput<'_>) -> Result<AgentState, ContextError> {
    let (mut config_title, mut config_stage, mut latex_engine, mut latex_template, mut latex_main) = {
        let default_cfg = Config::default();
        (
            default_cfg.project.title,
            default_cfg.project.stage,
            default_cfg.latex.engine,
            default_cfg.latex.template,
            default_cfg.latex.main,
        )
    };

    if let Ok(cfg) = Config::from_yaml(input.config_yaml) {
        config_title = cfg.project.title;
        config_stage = cfg.project.stage;
        latex_engine = cfg.latex.engine;
        latex_template = cfg.latex.template;
        latex_main = cfg.latex.main;
    } else if let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(input.config_yaml) {
        if let Some(t) = val
            .get("project")
            .and_then(|p| p.get("title"))
            .and_then(|t| t.as_str())
        {
            config_title = t.to_string();
        }
        if let Some(s) = val
            .get("project")
            .and_then(|p| p.get("stage"))
            .and_then(|s| s.as_str())
            && let Ok(st) = serde_yaml::from_str(s)
        {
            config_stage = st;
        }
        if let Some(e) = val
            .get("latex")
            .and_then(|l| l.get("engine"))
            .and_then(|e| e.as_str())
            .and_then(|e| serde_yaml::from_str(e).ok())
        {
            latex_engine = e;
        }
        if let Some(tpl) = val
            .get("latex")
            .and_then(|l| l.get("template"))
            .and_then(|t| t.as_str())
        {
            latex_template = tpl.to_string();
        }
        if let Some(m) = val
            .get("latex")
            .and_then(|l| l.get("main"))
            .and_then(|m| m.as_str())
        {
            latex_main = Utf8PathBuf::from(m);
        }
    }

    let paths = ProjectPaths::new(input.root);

    // 1. Project identity
    let title = if !config_title.is_empty() {
        config_title
    } else if let Some(st) = input.structure {
        st.title.clone()
    } else if let Ok(st) = Structure::from_yaml(input.structure_yaml)
        && !st.title.is_empty()
    {
        st.title.clone()
    } else {
        String::new()
    };

    let paper_kind = if config_stage == Stage::Final {
        PaperKind::Final
    } else {
        PaperKind::Draft
    };

    let project = ProjectIdentity {
        title,
        stage: config_stage,
        paper_kind,
        latex_engine,
        template: if latex_template.is_empty() {
            None
        } else {
            Some(latex_template)
        },
        relative_root: ".".to_string(),
    };

    // 2. Input snapshot & fingerprints
    let config_fingerprint = if !input.config_yaml.is_empty() {
        Some(input_fingerprint(input.config_yaml.as_bytes()))
    } else {
        None
    };

    let structure_fingerprint = if !input.structure_yaml.is_empty() {
        Some(input_fingerprint(input.structure_yaml.as_bytes()))
    } else {
        None
    };

    let draft_path = input.root.join(&latex_main);
    let draft_fingerprint = if draft_path.is_file() {
        fs::read(draft_path.as_str())
            .ok()
            .map(|b| input_fingerprint(&b))
    } else {
        None
    };

    let bib_path = input.root.join(rel::REFERENCES);
    let bib_fingerprint = if bib_path.is_file() {
        fs::read(bib_path.as_str())
            .ok()
            .map(|b| input_fingerprint(&b))
    } else {
        None
    };

    let mut files_present = Vec::new();
    for candidate in [
        ".sil/config.yaml",
        "config.yaml",
        ".sil/structure.yaml",
        "structure.yaml",
        "paper_draft.tex",
        "paper.tex",
        "references.bib",
        ".sil/skills.lock",
        ".sil/template.lock",
        ".sil/workspace.lock",
    ] {
        if input.root.join(candidate).is_file() {
            files_present.push(candidate.to_string());
        }
    }
    let main_str = latex_main.as_str();
    if !files_present.iter().any(|f| f == main_str) && draft_path.is_file() {
        files_present.push(main_str.to_string());
    }
    files_present.sort();
    files_present.dedup();

    let db_path = paths.db();
    let (sources_count, parsed_sources_count) = if db_path.is_file() {
        if let Ok(db) = SilDb::open(&db_path) {
            let sc = db.source_count().unwrap_or(0);
            let pc = db.parsed_count().unwrap_or(0);
            (sc, pc)
        } else {
            (0, 0)
        }
    } else {
        (0, 0)
    };

    let skill_lock_present =
        input.root.join(".sil/skills.lock").is_file() || input.root.join("skill.lock").is_file();
    let template_lock_present = input.root.join(".sil/template.lock").is_file()
        || input.root.join("template.lock").is_file();

    let inputs = InputSnapshot {
        config_fingerprint,
        structure_fingerprint,
        draft_fingerprint,
        bib_fingerprint,
        files_present,
        sources_count,
        parsed_sources_count,
        skill_lock_present,
        template_lock_present,
    };

    // 3. Health summary and warnings
    let bib_opt = if bib_path.is_file() {
        Some(bib_path.as_path())
    } else {
        None
    };

    let health_report = if draft_path.is_file() {
        sil_latex::audit_manuscript(&draft_path, bib_opt).unwrap_or_default()
    } else {
        ManuscriptHealthReport::default()
    };

    let health = HealthSummary::from(&health_report);

    let mut warnings = Vec::new();
    for diag in &health_report.diagnostics {
        let class = match diag.level {
            sil_core::DiagnosticLevel::Error => FindingClass::InvariantError,
            sil_core::DiagnosticLevel::Warning => FindingClass::ActionableWarning,
            sil_core::DiagnosticLevel::Info => FindingClass::Observation,
        };
        warnings.push(AgentFinding {
            code: format!("latex.{}", diag.category),
            class,
            message: diag.message.clone(),
            path: Some(latex_main.to_string()),
            line: diag.line,
            hint: None,
        });
    }

    // 4. Structure summary
    let structure = if let Some(st) = input.structure {
        StructureSummary::from(st)
    } else if let Ok(st) = Structure::from_yaml(input.structure_yaml) {
        StructureSummary::from(&st)
    } else if let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(input.structure_yaml) {
        let title = val
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
        StructureSummary {
            title,
            ..Default::default()
        }
    } else {
        StructureSummary::default()
    };

    // 5. Work items
    let mut work_items = Vec::new();
    if draft_path.is_file()
        && let Ok(tex) = fs::read_to_string(draft_path.as_str())
    {
        let idea_blocks = sil_latex::parse_idea_blocks(&tex);
        for (idx, idea) in idea_blocks.into_iter().enumerate() {
            let kind = if idea
                .content
                .trim_start()
                .to_ascii_lowercase()
                .starts_with("idea")
            {
                "idea".to_string()
            } else {
                "todo".to_string()
            };
            work_items.push(WorkItemSummary {
                id: format!("todo.{}", idx + 1),
                kind,
                section_id: idea.section_id,
                line_start: Some(idea.line_start),
                line_end: Some(idea.line_end),
                content: idea.content,
                resolved: false,
            });
        }
    }

    // 6. Literature summary
    let unparsed_sources = sources_count.saturating_sub(parsed_sources_count);
    let total_bib_keys = health.total_bib_keys_count;
    let cited_bib_keys = health.cited_bib_keys_count;
    let unmentioned_bib_keys = total_bib_keys.saturating_sub(cited_bib_keys);

    let literature = LiteratureSummary {
        total_sources: sources_count,
        parsed_sources: parsed_sources_count,
        unparsed_sources,
        total_bib_keys,
        cited_bib_keys,
        unmentioned_bib_keys,
        recent_candidates_count: 0,
    };

    // 7. Skills selection summary
    let mut flags = input.flags.clone();
    if input.skills.paper {
        flags.paper = true;
    }
    if input.skills.agent_code {
        flags.agent = true;
    }
    if input.skills.review {
        flags.skills.push("review.md".into());
    }
    if input.skills.visualize_article {
        flags.skills.push("visualize-article".into());
    }
    for dynamic in &input.skills.dynamic_skills {
        flags.skills.push(dynamic.clone());
    }
    let mut router = SkillRouter::new();
    let registry = SkillRegistry::new(input.root);
    let _ = router.load_from_registry(&registry);
    let (_, skills) = router.route(None, Some(&flags), None, Some(input.root));

    // 8. Capability summary
    let capabilities = CapabilitySummary {
        latex_available: true,
        parser_available: true,
        git_available: input.root.join(".git").exists(),
        online_search_available: false,
        llm_provider_available: true,
        supported_actions: vec![
            "check".to_string(),
            "compile".to_string(),
            "upsert_bib".to_string(),
            "parse_source".to_string(),
            "edit_draft".to_string(),
            "promote".to_string(),
        ],
    };

    // 9. Job summary & workspace lock
    let workspace_lock = match sil_core::workspace_lock::read_lock(&paths) {
        Ok(Some(lock)) => {
            let is_alive = lock
                .pid
                .map(sil_core::workspace_lock::pid_is_alive)
                .unwrap_or(true);
            Some(LockSummary {
                locked: is_alive,
                holder: Some(lock.holder),
                pid: lock.pid,
                reason: Some(lock.op),
                stale: !is_alive,
            })
        }
        Ok(None) => Some(LockSummary {
            locked: false,
            holder: None,
            pid: None,
            reason: None,
            stale: false,
        }),
        Err(_) => None,
    };

    let jobs = JobSummary {
        pending_jobs_count: 0,
        running_jobs_count: if workspace_lock.as_ref().map(|l| l.locked).unwrap_or(false) {
            1
        } else {
            0
        },
        failed_jobs_count: 0,
        active_job_id: None,
        workspace_lock,
    };

    // 10. Available actions
    let actions = vec![
        AvailableAction {
            id: "check".to_string(),
            description: "Run deterministic manuscript checks without building".to_string(),
            reason: "Validate syntax, citations, and structure invariants".to_string(),
            safe: true,
            mutating: false,
            required_inputs: Vec::new(),
        },
        AvailableAction {
            id: "compile".to_string(),
            description: "Compile manuscript using configured LaTeX engine".to_string(),
            reason: "Generate PDF artifact from LaTeX source".to_string(),
            safe: true,
            mutating: false,
            required_inputs: Vec::new(),
        },
        AvailableAction {
            id: "upsert_bib".to_string(),
            description: "Add or update BibTeX entries in references.bib".to_string(),
            reason: "Incorporate missing citations or literature references".to_string(),
            safe: false,
            mutating: true,
            required_inputs: vec!["bibtex".to_string()],
        },
        AvailableAction {
            id: "parse_source".to_string(),
            description: "Parse PDF literature sources into SQLite and full text".to_string(),
            reason: "Extract markdown and references from downloaded sources".to_string(),
            safe: false,
            mutating: true,
            required_inputs: vec!["source_id".to_string()],
        },
        AvailableAction {
            id: "edit_draft".to_string(),
            description: "Modify manuscript draft sections".to_string(),
            reason: "Update manuscript prose, resolve TODOs, and implement sections".to_string(),
            safe: false,
            mutating: true,
            required_inputs: vec!["section_id".to_string(), "content".to_string()],
        },
        AvailableAction {
            id: "promote".to_string(),
            description: "Promote draft manuscript to final publication version".to_string(),
            reason: "Transition paper from draft stage to final artifact".to_string(),
            safe: false,
            mutating: true,
            required_inputs: Vec::new(),
        },
    ];

    // 11. State classification
    let state_kind = if jobs
        .workspace_lock
        .as_ref()
        .map(|l| l.locked && !l.stale)
        .unwrap_or(false)
        || health.has_errors
    {
        AgentStateKind::Blocked
    } else if !draft_path.is_file() {
        AgentStateKind::NeedsInput
    } else {
        AgentStateKind::Ready
    };

    let mut state = AgentState {
        schema_version: AGENT_STATE_SCHEMA_VERSION.to_string(),
        state: state_kind,
        project,
        inputs,
        health,
        structure,
        work_items,
        literature,
        skills,
        capabilities,
        jobs,
        actions,
        warnings,
    };

    sanitize_agent_state(&mut state, Some(input.root.as_str()));

    Ok(state)
}

/// Generate the full context as deterministic JSON string.
pub fn generate_context_json(
    input: &ContextInput<'_>,
    compact: bool,
) -> Result<String, ContextError> {
    let state = build_agent_state(input)?;
    if compact {
        serde_json::to_string(&state).map_err(|e| ContextError::Message(e.to_string()))
    } else {
        serde_json::to_string_pretty(&state).map_err(|e| ContextError::Message(e.to_string()))
    }
}

/// Generate the full context envelope packaging deterministic state with execution metadata.
pub fn generate_context_envelope(
    input: &ContextInput<'_>,
) -> Result<AgentContextEnvelope, ContextError> {
    let start = std::time::Instant::now();
    let state = build_agent_state(input)?;
    let duration_ms = start.elapsed().as_millis() as u64;
    let execution = AgentExecutionMetadata {
        checked_at: now_iso8601(),
        duration_ms,
        job_id: state.jobs.active_job_id.clone(),
        host_info: Some(std::env::consts::OS.to_string()),
    };
    Ok(AgentContextEnvelope {
        state,
        execution: Some(execution),
    })
}

fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_epoch_iso8601(secs)
}

fn format_epoch_iso8601(secs: u64) -> String {
    let sec = secs % 60;
    let mins = (secs / 60) % 60;
    let hours = (secs / 3600) % 24;
    let mut days = (secs / 86400) as i64;

    let mut year = 1970;
    loop {
        let leap = is_leap_year(year);
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    let leap = is_leap_year(year);
    let days_in_months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];

    let mut month = 1;
    for &dim in &days_in_months {
        if days < dim {
            break;
        }
        days -= dim;
        month += 1;
    }
    let day = days + 1;

    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{mins:02}:{sec:02}Z")
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
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
    if input.skills.review {
        match load_skill(input.root, "review.md") {
            Ok(s) => {
                out.push_str("## Skill: review.md\n\n");
                out.push_str(&s);
                out.push_str("\n\n");
            }
            Err(ContextError::MissingSkill(_)) => {}
            Err(e) => return Err(e),
        }
    }
    if input.skills.visualize_article {
        match load_skill(input.root, "SKILL.md") {
            Ok(s) => {
                out.push_str("## Skill: visualize-article/SKILL.md\n\n");
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

                let ideas = sil_latex::parse_idea_blocks(&tex);
                if !ideas.is_empty() {
                    out.push_str("### Active Ideas & TODO blocks (# -- X -- #)\n\n");
                    for idea in ideas {
                        let sec = idea.section_id.as_deref().unwrap_or("General");
                        out.push_str(&format!(
                            "- **Lines {}-{} [{sec}]**: {}\n",
                            idea.line_start, idea.line_end, idea.content
                        ));
                    }
                    out.push('\n');
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
    use sil_core::LatexEngine;

    #[test]
    fn generate_minimal_context() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        fs::create_dir_all(root.join("agent/skills")).unwrap();
        fs::write(
            root.join("agent/skills/SYSTEM.md"),
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
        fs::create_dir_all(root.join("agent/skills")).unwrap();
        fs::create_dir_all(root.join("agent")).unwrap();
        fs::write(root.join("agent/skills/SYSTEM.md"), "SYS").unwrap();
        fs::write(root.join("agent/skills/paper.md"), "PAPER SKILL").unwrap();
        fs::write(root.join("agent/skills/agent-code.md"), "AGENT SKILL").unwrap();
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
        fs::create_dir_all(root.join("agent/skills")).unwrap();
        fs::write(root.join("agent/skills/SYSTEM.md"), "SYS").unwrap();
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
        fs::create_dir_all(root.join("agent/skills")).unwrap();
        fs::write(root.join("agent/skills/SYSTEM.md"), "SYS").unwrap();
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
        fs::write(root.join("agent/skills/paper.md"), "PAPER").unwrap();
        fs::write(root.join("agent/skills/agent-code.md"), "AGENTCODE").unwrap();
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

    #[test]
    fn context_includes_idea_blocks_when_paper_flag_set() {
        let (_d, root) = fixture_root_with_system();
        let draft_path = root.join("paper_draft.tex");
        fs::write(
            &draft_path,
            r#"
\section{Methods}
Some method text.

% # -- X -- #
% Idea: Compare model A vs model B.
% # -- X -- #
"#,
        )
        .unwrap();

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
        assert!(ctx.contains("Active Ideas & TODO blocks"));
        assert!(ctx.contains("Compare model A vs model B"));
        assert!(ctx.contains("Methods"));
    }

    #[test]
    fn test_build_agent_state_complete() {
        let (_d, root) = fixture_root_with_system();
        let draft_path = root.join("paper_draft.tex");
        fs::write(
            &draft_path,
            r#"\documentclass{article}
\begin{document}
\section{Introduction}
We cite \cite{Vaswani2017} and refer to Figure~\ref{fig:arch}.
\label{fig:arch}

% # -- X -- #
% Idea: Add ablation studies
% # -- X -- #
\end{document}
"#,
        )
        .unwrap();

        let bib_path = root.join("references.bib");
        fs::write(
            &bib_path,
            "@article{Vaswani2017, title={Attention is all you need}}\n",
        )
        .unwrap();

        let flags = ContextFlags {
            paper: true,
            agent: false,
            skill_paper: true,
            ..Default::default()
        };
        let mut skills = SkillSelection::always();
        skills.merge_flags(&flags);

        let input = ContextInput {
            root: &root,
            config_yaml: "project:\n  title: Test Manuscript\n  stage: draft\nlatex:\n  engine: tectonic\n  main: paper_draft.tex\n  template: standard\n",
            structure_yaml: "title: Test Manuscript\nsections:\n  - id: sec.intro\n    title: Introduction\n    level: 1\n    completion: draft\n",
            structure: None,
            sources_summary: "Sources in database: 0 (parsed: 0)\n",
            log_entries: &[],
            flags: &flags,
            skills,
        };

        let state = build_agent_state(&input).unwrap();
        assert_eq!(state.schema_version, AGENT_STATE_SCHEMA_VERSION);
        assert_eq!(state.state, AgentStateKind::Ready);
        assert_eq!(state.project.title, "Test Manuscript");
        assert_eq!(state.project.stage, Stage::Draft);
        assert_eq!(state.project.paper_kind, PaperKind::Draft);
        assert_eq!(state.project.latex_engine, LatexEngine::Tectonic);
        assert_eq!(state.project.template.as_deref(), Some("standard"));
        assert_eq!(state.project.relative_root, ".");

        assert!(state.inputs.config_fingerprint.is_some());
        assert!(state.inputs.structure_fingerprint.is_some());
        assert!(state.inputs.draft_fingerprint.is_some());
        assert!(state.inputs.bib_fingerprint.is_some());
        assert!(
            state
                .inputs
                .files_present
                .contains(&"paper_draft.tex".to_string())
        );
        assert!(
            state
                .inputs
                .files_present
                .contains(&"references.bib".to_string())
        );

        assert_eq!(state.health.total_bib_keys_count, 1);
        assert_eq!(state.health.cited_bib_keys_count, 1);
        assert_eq!(state.health.missing_citations_count, 0);
        assert_eq!(state.health.todo_ideas_count, 1);

        assert_eq!(state.structure.total_sections, 1);
        assert_eq!(state.structure.in_progress_sections, 1);

        assert_eq!(state.work_items.len(), 1);
        assert_eq!(state.work_items[0].kind, "idea");
        assert!(state.work_items[0].content.contains("ablation studies"));

        assert_eq!(state.literature.total_bib_keys, 1);
        assert_eq!(state.literature.cited_bib_keys, 1);

        assert!(
            state
                .skills
                .active_skill_ids
                .contains(&"SYSTEM".to_string())
        );
        assert!(state.skills.active_skill_ids.contains(&"paper".to_string()));

        assert!(state.capabilities.latex_available);
        assert!(!state.actions.is_empty());
        assert!(state.actions.iter().any(|a| a.id == "check"));
        assert!(state.actions.iter().any(|a| a.id == "compile"));
    }

    #[test]
    fn test_generate_context_json_roundtrip() {
        let (_d, root) = fixture_root_with_system();
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

        let pretty_json = generate_context_json(&input, false).unwrap();
        let compact_json = generate_context_json(&input, true).unwrap();

        assert!(pretty_json.contains('\n'));
        assert!(!compact_json.contains('\n'));

        let de_pretty: AgentState = serde_json::from_str(&pretty_json).unwrap();
        let de_compact: AgentState = serde_json::from_str(&compact_json).unwrap();
        assert_eq!(de_pretty, de_compact);
        assert_eq!(de_pretty.schema_version, AGENT_STATE_SCHEMA_VERSION);
    }

    #[test]
    fn test_generate_context_envelope() {
        let (_d, root) = fixture_root_with_system();
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

        let envelope = generate_context_envelope(&input).unwrap();
        assert_eq!(envelope.state.schema_version, AGENT_STATE_SCHEMA_VERSION);
        let exec = envelope.execution.unwrap();
        assert!(!exec.checked_at.is_empty());
        assert!(exec.checked_at.contains('T') && exec.checked_at.ends_with('Z'));
        assert!(exec.host_info.is_some());
    }

    #[test]
    fn test_deterministic_fingerprint_parity() {
        let (_d, root) = fixture_root_with_system();
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

        let state1 = build_agent_state(&input).unwrap();
        let state2 = build_agent_state(&input).unwrap();
        assert_eq!(state1.stable_fingerprint(), state2.stable_fingerprint());
    }

    #[test]
    fn test_secret_scrubbing_in_agent_state() {
        let (_d, root) = fixture_root_with_system();
        let draft_path = root.join("paper_draft.tex");
        fs::write(
            &draft_path,
            r#"\documentclass{article}
\begin{document}
\section{Intro with sk-proj-1234567890abcdef1234567890}
% # -- X -- #
% Idea: password=supersecretpass123
% # -- X -- #
\end{document}
"#,
        )
        .unwrap();

        let flags = ContextFlags {
            paper: true,
            ..Default::default()
        };
        let input = ContextInput {
            root: &root,
            config_yaml: "project:\n  title: Paper with Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9\n  stage: draft\n",
            structure_yaml: "title: \"Struct with api_key: sk-123456789012345678901234\"\nsections: []\n",
            structure: None,
            sources_summary: "Sources in database: 0 (parsed: 0)\n",
            log_entries: &[],
            flags: &flags,
            skills: SkillSelection::always(),
        };

        let state = build_agent_state(&input).unwrap();
        assert!(
            !state
                .project
                .title
                .contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9")
        );
        assert!(state.project.title.contains("[REDACTED]"));
        assert!(
            !state
                .structure
                .title
                .contains("sk-123456789012345678901234")
        );
        assert!(state.structure.title.contains("[REDACTED]"));
        assert!(!state.work_items[0].content.contains("supersecretpass123"));
        assert!(state.work_items[0].content.contains("[REDACTED]"));
    }
}
