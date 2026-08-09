//! `sil status`

use anyhow::Result;
use serde::Serialize;
use sil_core::{SilUi, Structure, paths::rel};
use sil_db::SilDb;
use sil_git::{path_has_changes, status as git_status};

use crate::util::load_project;

#[derive(Debug, Serialize)]
struct StatusJson {
    project: String,
    title: String,
    stage: String,
    latex_engine: String,
    latex_main: String,
    sources: SourcesJson,
    structure: StructureJson,
    git: GitJson,
    draft_dirty: bool,
}

#[derive(Debug, Serialize)]
struct SourcesJson {
    total: usize,
    parsed: usize,
}

#[derive(Debug, Serialize)]
struct StructureJson {
    summary: String,
    total: usize,
    empty: usize,
    outline: usize,
    draft: usize,
    polished: usize,
    sections: Vec<SectionJson>,
}

#[derive(Debug, Serialize)]
struct SectionJson {
    id: String,
    title: String,
    completion: String,
}

#[derive(Debug, Serialize)]
struct GitJson {
    is_repo: bool,
    clean: bool,
    uncommitted: usize,
}

pub fn run(json: bool, ui: &dyn SilUi) -> Result<()> {
    let (root, config, paths) = load_project()?;
    let structure = Structure::load(&paths.structure()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let source_count = db.source_count().map_err(|e| anyhow::anyhow!("{e}"))?;
    let parsed_count = db.parsed_count().map_err(|e| anyhow::anyhow!("{e}"))?;
    let git = git_status(&root).map_err(|e| anyhow::anyhow!("{e}"))?;
    let draft_dirty = path_has_changes(&root, rel::PAPER_DRAFT).unwrap_or(false);
    let summary = structure.completion_summary();

    if json {
        let payload = StatusJson {
            project: root.to_string(),
            title: config.project.title.clone(),
            stage: config.project.stage.to_string(),
            latex_engine: config.latex.engine.to_string(),
            latex_main: config.latex.main.to_string(),
            sources: SourcesJson {
                total: source_count,
                parsed: parsed_count,
            },
            structure: StructureJson {
                summary: summary.to_string(),
                total: summary.total,
                empty: summary.empty,
                outline: summary.outline,
                draft: summary.draft,
                polished: summary.polished,
                sections: structure
                    .sections
                    .iter()
                    .map(|s| SectionJson {
                        id: s.id.clone(),
                        title: s.title.clone(),
                        completion: s.completion.to_string(),
                    })
                    .collect(),
            },
            git: GitJson {
                is_repo: git.is_repo,
                clean: git.clean,
                uncommitted: git.entries.len(),
            },
            draft_dirty,
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    ui.println("");
    ui.info(&format!("Project: {root}"));
    ui.println(&format!("  title:  {}", config.project.title));
    ui.println(&format!("  stage:  {}", config.project.stage));
    ui.println(&format!(
        "  latex:  {} → {}",
        config.latex.engine, config.latex.main
    ));
    let bib_path = root.join("references.bib");
    let bib_opt = if bib_path.is_file() {
        Some(bib_path.as_path())
    } else {
        None
    };
    if let Ok(report) = sil_latex::audit_manuscript(&paths.paper_draft(), bib_opt) {
        let (cited, total) = report.bib_citation_ratio();
        if total > 0 {
            ui.println(&format!(
                "  bib coverage: {cited}/{total} references mentioned in {}",
                rel::PAPER_DRAFT
            ));
        }
    }
    ui.println("");
    ui.info("Sources");
    ui.println(&format!(
        "  database: {source_count} source(s), {parsed_count} parsed"
    ));
    ui.println("");
    ui.info("Structure");
    ui.println(&format!("  {summary}"));
    for sec in &structure.sections {
        ui.muted(&format!(
            "  - [{}] {} ({})",
            sec.completion, sec.id, sec.title
        ));
    }
    ui.println("");
    ui.info("Git");
    if !git.is_repo {
        ui.warn("  not a git repository");
    } else if git.clean {
        ui.success("  working tree clean");
    } else {
        ui.warn(&format!("  {} uncommitted change(s)", git.entries.len()));
        for e in git.entries.iter().take(12) {
            ui.muted(&format!("    {e}"));
        }
        if git.entries.len() > 12 {
            ui.muted(&format!("    … {} more", git.entries.len() - 12));
        }
    }
    if draft_dirty {
        ui.warn("  paper_draft.tex has uncommitted changes");
    } else {
        ui.muted("  paper_draft.tex: no uncommitted changes");
    }
    ui.println("");
    Ok(())
}
