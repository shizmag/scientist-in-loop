//! `sil source` — fetch / list / remove sources.

use std::fs;

use anyhow::{Context, Result};
use camino::Utf8Path;
use serde::Serialize;
use sil_core::{SciAction, SilUi, SourceId};
use sil_db::SilDb;
use sil_git::CommitProposal;
use sil_parse::{fetch_source_target, parse_one};

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

    let mut spinner = ui.spinner(&format!("Fetching {target}"));
    let saved_path = fetch_source_target(target, &sources_dir).map_err(|e| {
        spinner.finish_error("fetch failed");
        anyhow::anyhow!("{e}")
    })?;
    let saved = saved_path.as_str();
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

    // Attempt to resolve official BibTeX for the fetched target and update references.bib
    let bib_path = root.join("references.bib");
    let official_bib = if let Some(doi) = sil_regex::extract_doi(target) {
        sil_parse::journal_digest::fetch_bibtex_by_doi(&doi)
            .ok()
            .flatten()
    } else if let Some(arxiv) = sil_regex::extract_arxiv_id(target) {
        sil_parse::journal_digest::fetch_bibtex_by_arxiv_id(&arxiv)
            .ok()
            .flatten()
    } else {
        None
    };

    if let Some(official_bib) = official_bib {
        let current = std::fs::read_to_string(bib_path.as_std_path()).unwrap_or_default();
        let (updated, replaced) = sil_core::bib::upsert_bib_entry(&current, &official_bib);
        if sil_core::write_atomic_str(&bib_path, &updated).is_ok() {
            if replaced {
                ui.success(&format!(
                    "✓ Replaced incomplete entry in references.bib with official metadata for {target}"
                ));
            } else {
                ui.success(&format!(
                    "✓ Added official metadata for {target} to references.bib"
                ));
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
    let removed = db.remove_source(&id).map_err(|e| anyhow::anyhow!("{e}"))?;

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
        ui.success(&format!(
            "Removed '{id_or_filename}' from database (reparse with sil parse)"
        ));
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
            fs::remove_file(candidate.as_str()).with_context(|| format!("delete {candidate}"))?;
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
        let on_disk =
            sources_dir.join(&doc.filename).is_file() || Utf8Path::new(doc.path.as_str()).is_file();
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
        for entry in
            fs::read_dir(sources_dir.as_str()).with_context(|| format!("read {sources_dir}"))?
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

/// Interactive TUI reader for a parsed or raw markdown source document using termimad.
pub fn read(id_or_filename: &str, ui: &dyn SilUi) -> Result<()> {
    use crossterm::{
        cursor,
        event::{self, Event, KeyCode, KeyModifiers},
        queue, terminal,
    };
    use std::io::Write;
    use termimad::{Area, MadSkin, MadView};

    let (_root, config, paths) = load_project()?;
    let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;

    let (title, content) = if let Some((doc, text)) = db.get_source_content(id_or_filename)? {
        let t = doc.title.unwrap_or_else(|| doc.filename.clone());
        (t, text)
    } else {
        let sources_dir = paths.sources(&config);
        let file_path = sources_dir.join(id_or_filename);
        let path = if file_path.exists() {
            file_path
        } else {
            camino::Utf8PathBuf::from(id_or_filename)
        };
        if path.exists() {
            let text =
                fs::read_to_string(&path).with_context(|| format!("read source file at {path}"))?;
            (path.file_name().unwrap_or(id_or_filename).to_string(), text)
        } else {
            anyhow::bail!("Source '{id_or_filename}' not found in database or on disk");
        }
    };

    if content.trim().is_empty() {
        ui.warn(&format!("Source '{title}' is empty"));
        return Ok(());
    }

    let mut area = Area::full_screen();
    area.pad(1, 1);
    let skin = MadSkin::default();
    let mut view = MadView::from(content, area, skin);
    let mut w = std::io::stdout();

    terminal::enable_raw_mode()?;
    queue!(w, terminal::EnterAlternateScreen, cursor::Hide)?;
    w.flush()?;

    view.write_on(&mut w)?;
    w.flush()?;

    loop {
        match event::read()? {
            Event::Key(key_event) => {
                if key_event.kind == event::KeyEventKind::Press {
                    match key_event.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Up | KeyCode::Char('k') => {
                            view.try_scroll_lines(-1);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            view.try_scroll_lines(1);
                        }
                        KeyCode::PageUp | KeyCode::Char('b') => {
                            view.try_scroll_pages(-1);
                        }
                        KeyCode::PageDown | KeyCode::Char(' ') | KeyCode::Char('f') => {
                            view.try_scroll_pages(1);
                        }
                        KeyCode::Char('c')
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Event::Resize(width, height) => {
                let mut area = Area::new(0, 0, width, height);
                area.pad(1, 1);
                view.resize(&area);
            }
            _ => {}
        }
        view.write_on(&mut w)?;
        w.flush()?;
    }

    queue!(w, cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    w.flush()?;

    Ok(())
}

/// Heal parsed sources: re-extract references and fetch missing metadata via DOI.
pub fn doctor(id: Option<String>, ui: &dyn SilUi) -> Result<()> {
    let (_root, config, paths) = load_project()?;
    let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;

    let docs = if let Some(target) = id {
        if let Some((doc, content)) = db.get_source_content(&target)? {
            vec![(doc, content)]
        } else {
            anyhow::bail!("Source '{target}' not found or unparsed in database");
        }
    } else {
        let mut all = Vec::new();
        for doc in db.list_sources().map_err(|e| anyhow::anyhow!("{e}"))? {
            if doc.parsed
                && let Some((d, c)) = db.get_source_content(doc.id.as_str())?
            {
                all.push((d, c));
            }
        }
        all
    };

    if docs.is_empty() {
        ui.warn("No parsed sources found to heal.");
        return Ok(());
    }

    let mut pb = ui.progress(docs.len() as u64, "Doctoring source documents");
    let mut healed_count = 0;
    let mut warnings = Vec::new();

    for (i, (mut doc, content)) in docs.into_iter().enumerate() {
        pb.set_message(&doc.filename);

        // 1. Reset metadata fields and re-hydrate using header-scoped lookup & frontmatter parsing
        doc.title = None;
        doc.authors = None;
        doc.year = None;
        doc.venue = None;
        doc.doi = None;
        doc.abstract_text = None;

        let path_clone = doc.path.clone();
        sil_parse::hydrate_source_document_metadata(&mut doc, &content, &path_clone);

        if doc.kind == sil_core::SourceKind::Pdf {
            let sources_dir = paths.sources(&config);
            let full_path = if camino::Utf8Path::new(&path_clone).is_absolute() {
                camino::Utf8PathBuf::from(&path_clone)
            } else {
                sources_dir.join(&doc.filename)
            };

            if full_path.exists() {
                pb.set_message(&format!("{} (xberg metadata)", doc.filename));
                if let Ok(rt) = tokio::runtime::Runtime::new() {
                    match rt.block_on(sil_parse::xberg_metadata::extract_metadata_utf8(&full_path))
                    {
                        Ok(meta) => {
                            if !meta.title.is_empty() {
                                doc.title = Some(meta.title);
                            }
                            if !meta.authors.is_empty() {
                                doc.authors = Some(meta.authors.join(", "));
                            }
                        }
                        Err(e) => {
                            let msg = e.to_string();
                            let first_line = msg.lines().next().unwrap_or(&msg);
                            warnings.push(format!(
                                "{}: xberg metadata skipped: {first_line}",
                                doc.filename
                            ));
                        }
                    }
                }
            }
        }

        // 2. Re-extract references block and entries
        let refs_block = sil_parse::references::extract_references_block(&content);
        doc.references_text = refs_block.clone();

        if let Err(e) = db.upsert_parsed(&doc, &content) {
            warnings.push(format!(
                "Failed to update database for {}: {e}",
                doc.filename
            ));
            continue;
        }

        if let Err(e) = db.delete_references_for_source(&doc.id) {
            warnings.push(format!(
                "Failed to clear old references for {}: {e}",
                doc.filename
            ));
        }

        if let Some(ref raw_block) = refs_block {
            let entries = sil_parse::references::parse_reference_entries(&doc.id, raw_block);
            if !entries.is_empty()
                && let Err(e) = db.save_source_references(&entries)
            {
                warnings.push(format!(
                    "Failed to save references for {}: {e}",
                    doc.filename
                ));
            }
        }

        healed_count += 1;
        pb.set_position((i as u64) + 1);
    }

    pb.finish_success(&format!("Healed {healed_count} source document(s)"));

    for w in &warnings {
        ui.warn(w);
    }

    Ok(())
}

/// Structure for JSON output of similarity rankings.
#[derive(Debug, Serialize)]
pub struct SimilarityRankHit {
    pub ref_id: String,
    pub title: Option<String>,
    pub authors: Option<String>,
    pub year: Option<i32>,
    pub score: f32,
    pub raw_text: String,
}

/// Recompute and rank extracted references by cosine similarity against paper_draft.tex.
pub fn rank_draft(min_score: Option<f32>, json: bool, ui: &dyn SilUi) -> Result<()> {
    let (_root, _config, paths) = load_project()?;
    let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;

    let draft_path = paths.paper_draft();
    if !draft_path.exists() {
        anyhow::bail!("paper_draft.tex not found at {draft_path}");
    }
    let draft_text = std::fs::read_to_string(draft_path.as_std_path())?;

    let embedder = sil_db::OnnxEmbedder::default();
    if !json {
        let mut spinner = ui.spinner("Computing cosine similarity against paper_draft.tex...");
        let count = db
            .recompute_draft_ref_similarities(&draft_text, &embedder)
            .map_err(|e| {
                spinner.finish_error("recomputation failed");
                anyhow::anyhow!("{e}")
            })?;
        spinner.finish_success(&format!("Computed similarity for {count} reference(s)"));
    } else {
        db.recompute_draft_ref_similarities(&draft_text, &embedder)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }


    let scores = db
        .get_draft_ref_similarities()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let all_refs = db
        .get_all_references()
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let threshold = min_score.unwrap_or(0.0);
    let mut hits: Vec<SimilarityRankHit> = all_refs
        .into_iter()
        .filter_map(|r| {
            let score = *scores.get(&r.id).unwrap_or(&0.0);
            if score >= threshold {
                Some(SimilarityRankHit {
                    ref_id: r.id,
                    title: r.title,
                    authors: r.authors,
                    year: r.year,
                    score,
                    raw_text: r.raw_text,
                })
            } else {
                None
            }
        })
        .collect();

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&hits)?);
    } else {
        ui.info(&format!(
            "Draft Cosine Similarity Rankings (total: {})",
            hits.len()
        ));
        for hit in &hits {
            let title = hit.title.as_deref().unwrap_or(&hit.raw_text);
            let year = hit.year.map(|y| format!(" ({y})")).unwrap_or_default();
            ui.println(&format!("  [{:.3}] {title}{year}", hit.score));
        }
    }
    Ok(())
}
