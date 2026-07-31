//! `sil source` — fetch / list / remove sources.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};
use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;
use sil_core::{SciAction, SilUi, SourceId};
use sil_db::SilDb;
use sil_git::CommitProposal;
use sil_parse::parse_one;

use crate::util::{load_project, marker_runner, print_proposal};

/// One row in `sil source list` (parsed DB + on-disk sources).
#[derive(Debug, Clone, Serialize)]
pub struct SourceListEntry {
    /// Stable source id (usually filename).
    pub id: String,
    /// Filename.
    pub filename: String,
    /// Path relative to project or absolute.
    pub path: String,
    /// Kind of source document (pdf, markdown, html, text, etc.).
    pub kind: String,
    /// Whether content is in the FTS database.
    pub parsed: bool,
    /// Whether a source file exists on disk under sources/.
    pub on_disk: bool,
    /// Optional title from parse metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

pub fn fetch(target: &str, no_parse: bool, ui: &dyn SilUi) -> Result<()> {
    let (root, config, paths) = load_project()?;
    let sources_dir = paths.sources(&config);
    fs::create_dir_all(sources_dir.as_str())?;

    let script = discover_download_script()?;
    let python = std::env::var("SIL_PYTHON").unwrap_or_else(|_| "python3".into());

    let mut spinner = ui.spinner(&format!("Fetching {target}"));
    let output = Command::new(&python)
        .arg(script.as_str())
        .arg(target)
        .arg(sources_dir.as_str())
        .output()
        .with_context(|| format!("failed to spawn {python} {script}"))?;
    if !output.status.success() {
        spinner.finish_error("fetch failed");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!("download failed: {}\n{}", stderr.trim(), stdout.trim());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let saved = stdout
        .lines()
        .rev()
        .find(|l| l.trim().ends_with(".pdf") || l.contains("sources/"))
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| stdout.trim().to_string());
    spinner.finish_success(&format!("Downloaded → {saved}"));

    let proposal = CommitProposal::new(format!("Fetch source: {target}"), SciAction::FetchSource)
        .with_body(format!("Saved to {saved}"));
    print_proposal(ui, &proposal);

    if !no_parse {
        let pdf_path = {
            let p = Utf8Path::new(saved.trim());
            if p.is_absolute() {
                p.to_path_buf()
            } else if sources_dir
                .join(p.file_name().unwrap_or(p.as_str()))
                .exists()
            {
                sources_dir.join(p.file_name().unwrap_or(p.as_str()))
            } else {
                root.join(p)
            }
        };
        if pdf_path.exists() {
            ui.info("Parsing downloaded PDF…");
            let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;
            let runner = marker_runner()?;
            match parse_one(&pdf_path, &db, runner.as_ref(), ui) {
                Ok(r) => {
                    ui.success(&format!("Parsed {}", r.document.filename));
                    let p2 = CommitProposal::new(
                        format!("Parse PDF: {}", r.document.filename),
                        SciAction::ParsePdf,
                    );
                    print_proposal(ui, &p2);
                }
                Err(e) => ui.warn(&format!("Parse skipped/failed: {e}")),
            }
        }
    }
    Ok(())
}

fn kind_tag(kind: &str) -> &'static str {
    match kind.to_ascii_lowercase().as_str() {
        "pdf" => "pdf",
        "markdown" | "md" => "md",
        "html" | "htm" => "html",
        "text" | "txt" => "txt",
        "code" => "code",
        "dataset" => "data",
        _ => "unk",
    }
}

/// List sources with parsed vs unparsed (and on-disk) visibility.
pub fn list(json: bool, ui: &dyn SilUi) -> Result<()> {
    let entries = collect_source_entries()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }
    if entries.is_empty() {
        ui.warn("No sources in database or sources/ directory");
        ui.muted("  tip: sil source fetch <doi|arxiv|url>");
        return Ok(());
    }
    let parsed = entries.iter().filter(|e| e.parsed).count();
    let unparsed = entries.len() - parsed;
    ui.info(&format!(
        "{} source(s): {parsed} parsed, {unparsed} unparsed",
        entries.len()
    ));
    ui.println("");
    for e in &entries {
        let tag = kind_tag(&e.kind);
        let flag = if e.parsed { "parsed" } else { "unparsed" };
        let disk = if e.on_disk { "on-disk" } else { "missing-file" };
        let title = e.title.as_deref().unwrap_or("");
        ui.println(&format!(
            "  [{tag}/{flag}/{disk}] {} {}",
            e.filename,
            if title.is_empty() {
                String::new()
            } else {
                format!("— {title}")
            }
        ));
    }
    ui.println("");
    Ok(())
}

/// Remove a source from the database (optionally delete the PDF file).
pub fn remove(id_or_filename: &str, delete_file: bool, ui: &dyn SilUi) -> Result<()> {
    let (_root, config, paths) = load_project()?;
    let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let id = SourceId::new(id_or_filename);
    let removed = db
        .remove_source(&id)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Also try by matching filename among listed sources
    let mut did_remove = removed;
    if !did_remove {
        for doc in db.list_sources().map_err(|e| anyhow::anyhow!("{e}"))? {
            if doc.filename == id_or_filename || doc.id.as_str() == id_or_filename {
                did_remove = db
                    .remove_source(&doc.id)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                break;
            }
        }
    }

    if !did_remove {
        // Allow clearing unparsed on-disk only? still report
        ui.warn(&format!(
            "no database row for '{id_or_filename}' (may already be unparsed)"
        ));
    } else {
        ui.success(&format!("Removed '{id_or_filename}' from database (reparse with sil parse)"));
    }

    if delete_file {
        let sources_dir = paths.sources(&config);
        let candidate = sources_dir.join(id_or_filename);
        let candidate = if candidate.extension().is_none() {
            sources_dir.join(format!("{id_or_filename}.pdf"))
        } else {
            candidate
        };
        if candidate.is_file() {
            fs::remove_file(candidate.as_str())
                .with_context(|| format!("delete {candidate}"))?;
            ui.success(&format!("Deleted file {candidate}"));
        } else {
            ui.warn(&format!("no file at {candidate}"));
        }
    }
    Ok(())
}

/// Collect merged view of DB rows + on-disk files under sources/.
pub fn collect_source_entries() -> Result<Vec<SourceListEntry>> {
    let (_root, config, paths) = load_project()?;
    let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let sources_dir = paths.sources(&config);
    let docs = db.list_sources().map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut by_name: std::collections::BTreeMap<String, SourceListEntry> =
        std::collections::BTreeMap::new();

    for doc in docs {
        let on_disk = sources_dir.join(&doc.filename).is_file()
            || Utf8Path::new(doc.path.as_str()).is_file();
        by_name.insert(
            doc.filename.clone(),
            SourceListEntry {
                id: doc.id.as_str().to_string(),
                filename: doc.filename.clone(),
                path: doc.path.to_string(),
                kind: doc.kind.to_string(),
                parsed: doc.parsed,
                on_disk,
                title: doc.title.clone(),
            },
        );
    }

    // Scan sources/ for source files not yet in DB
    if sources_dir.is_dir() {
        for entry in fs::read_dir(sources_dir.as_str())
            .with_context(|| format!("read {sources_dir}"))?
        {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let name_lower = name.to_ascii_lowercase();
            if name_lower.starts_with("readme") {
                continue;
            }
            let path_buf = Utf8Path::new(&name);
            let ext = path_buf.extension().unwrap_or("");
            let is_supported = matches!(
                ext.to_ascii_lowercase().as_str(),
                "pdf" | "md" | "markdown" | "txt" | "html" | "htm"
            );
            if !is_supported {
                continue;
            }
            let kind = sil_core::SourceKind::from_path(path_buf);
            by_name.entry(name.clone()).or_insert(SourceListEntry {
                id: name.clone(),
                filename: name.clone(),
                path: format!("sources/{name}"),
                kind: kind.to_string(),
                parsed: false,
                on_disk: path.is_file(),
                title: None,
            });
        }
    }

    Ok(by_name.into_values().collect())
}

fn discover_download_script() -> Result<Utf8PathBuf> {
    if let Ok(p) = std::env::var("SIL_DOWNLOAD_SCRIPT") {
        return Ok(Utf8PathBuf::from(p));
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let m = PathBuf::from(manifest);
        candidates.push(m.join("../../python/download_pdf.py"));
        candidates.push(m.join("../python/download_pdf.py"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("python/download_pdf.py"));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        candidates.push(dir.join("../../python/download_pdf.py"));
        candidates.push(dir.join("../python/download_pdf.py"));
    }
    for c in candidates {
        if c.is_file() {
            return Utf8PathBuf::from_path_buf(c)
                .map_err(|_| anyhow::anyhow!("download script path not utf-8"));
        }
    }
    bail!("could not locate python/download_pdf.py; set SIL_DOWNLOAD_SCRIPT");
}
