//! `sil build`

use anyhow::{Result, bail};
use sil_core::SilUi;
use sil_latex::{build as latex_build, create_submission_archive};

use crate::commands::template_cmd;
use crate::util::load_project;

struct BibRestorer {
    path: camino::Utf8PathBuf,
    original_content: String,
}

impl Drop for BibRestorer {
    fn drop(&mut self) {
        let _ = std::fs::write(&self.path, &self.original_content);
    }
}

pub fn run(target: Option<String>, legacy_release: bool, ui: &dyn SilUi) -> Result<()> {
    let (root, config, _paths) = load_project()?;

    let is_release = legacy_release
        || target
            .as_deref()
            .map(|s| {
                let lower = s.to_lowercase();
                lower == "release" || lower == "realese" || lower == "rel"
            })
            .unwrap_or(false);

    let _bib_guard = if is_release {
        let bib_path = root.join(sil_core::paths::rel::REFERENCES);
        if bib_path.is_file() {
            if let Ok(orig) = std::fs::read_to_string(bib_path.as_std_path()) {
                let stripped = sil_core::strip_tui_added_bib_entries(&orig);
                if stripped != orig {
                    if std::fs::write(bib_path.as_std_path(), &stripped).is_ok() {
                        Some(BibRestorer {
                            path: bib_path,
                            original_content: orig,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let main = if is_release {
        template_cmd::apply(None, None, None, ui)?;
        let t_name = config.latex.template.clone();
        camino::Utf8PathBuf::from(format!("paper_{t_name}.tex"))
    } else {
        config.latex.main.clone()
    };

    let mode_str = if is_release { "release" } else { "draft" };
    let engine = config.latex.engine;
    ui.info(&format!("Building {main} with {engine} ({mode_str} mode)"));
    let mut spinner = ui.spinner("Compiling LaTeX…");

    let pdf = match latex_build(engine, &main, &root) {
        Ok(pdf) => {
            spinner.finish_success(&format!("Built {pdf}"));
            ui.success(&format!("PDF: {pdf}"));
            Some(pdf)
        }
        Err(e) => {
            spinner.finish_error("build failed");
            if is_release {
                ui.warn(&format!("LaTeX engine warning: {e}"));
                None
            } else {
                bail!("{e}");
            }
        }
    };

    let bib_path = root.join("references.bib");
    let bg_doi_handle = if bib_path.is_file() {
        let db_path = _paths.db().into_std_path_buf();
        let bib_std = bib_path.clone().into_std_path_buf();
        Some(sil_parse::spawn_background_bib_doi_check(db_path, bib_std))
    } else {
        None
    };

    let bib_opt = if bib_path.is_file() {
        Some(bib_path.as_path())
    } else {
        None
    };
    if let Ok(report) = sil_latex::audit_manuscript(&main, bib_opt) {
        let (cited, total) = report.bib_citation_ratio();
        if total > 0 {
            if cited == total {
                ui.success(&format!(
                    "Reference coverage: {cited}/{total} mentioned in {main}"
                ));
            } else {
                ui.warn(&format!(
                    "Reference coverage: {cited}/{total} mentioned in {main} ({} unmentioned in references.bib)",
                    total - cited
                ));
            }
        }
    }

    if let Some(handle) = bg_doi_handle {
        if let Ok(Ok(report)) = handle.join() {
            if !report.broken_dois.is_empty() {
                let broken_list: Vec<String> = report
                    .broken_dois
                    .iter()
                    .map(|(k, d)| format!("{k} ({d})"))
                    .collect();
                ui.warn(&format!(
                    "⚠ Background DOI check: {} broken DOI(s) in references.bib: [{}]",
                    report.broken_dois.len(),
                    broken_list.join(", ")
                ));
            }
        }
    }

    if is_release {
        let t_name = config.latex.template.clone();
        let zip_name = format!("submission_{t_name}.zip");
        let zip_path = root.join(&zip_name);

        ui.info(&format!(
            "Packaging autonomous journal submission archive: {zip_name}"
        ));
        match create_submission_archive(&root, &main, pdf.as_deref(), &zip_path) {
            Ok(created_zip) => {
                ui.success(&format!("Journal Submission Archive: {created_zip}"));
            }
            Err(e) => {
                ui.warn(&format!("Could not create submission archive: {e}"));
            }
        }
    }

    Ok(())
}
