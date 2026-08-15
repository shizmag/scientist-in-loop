//! `sil build`

use std::str::FromStr;

use anyhow::{Result, bail};
use sil_core::SilUi;
use sil_latex::{ReleaseOptions, build as latex_build, create_staged_submission_release};
use sil_template::{PaperTemplate, apply_template};

use crate::util::load_project;

pub fn run(
    target: Option<String>,
    legacy_release: bool,
    source_only: bool,
    ui: &dyn SilUi,
) -> Result<()> {
    let (root, config, paths) = load_project()?;

    let is_release = legacy_release
        || target
            .as_deref()
            .map(|s| {
                let lower = s.to_lowercase();
                lower == "release" || lower == "realese" || lower == "rel"
            })
            .unwrap_or(false);

    if source_only && !is_release {
        bail!("--source-only is only valid with `paper build release`");
    }

    if is_release {
        return run_staged_release(&root, &config, &paths, source_only, ui);
    }

    let main = config.latex.main.clone();

    let engine = config.latex.engine;
    ui.info(&format!("Building {main} with {engine} (draft mode)"));
    let mut spinner = ui.spinner("Compiling LaTeX…");
    let report = sil_app::run_manuscript_check(
        &root,
        sil_app::ManuscriptCheckOptions {
            profile: sil_core::CheckProfile::Draft,
            build: true,
            online: false,
        },
    )?;
    let build = report.run.build.as_ref();
    if let Some(error) = build.and_then(|v| v.get("error")).and_then(|v| v.as_str()) {
        spinner.finish_error("build failed");
        bail!("{error}");
    }
    spinner.finish_success("build completed");
    if let Some(path) = build
        .and_then(|v| v.get("artifact"))
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
    {
        ui.success(&format!("PDF: {path}"));
    }

    let bib_path = root.join("references.bib");
    let bg_doi_handle = if bib_path.is_file() {
        let db_path = paths.db().into_std_path_buf();
        let bib_std = bib_path.clone().into_std_path_buf();
        Some(sil_parse::spawn_background_bib_doi_check(
            db_path, bib_std, false,
        ))
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

    if let Some(handle) = bg_doi_handle
        && let Ok(Ok(report)) = handle.join()
        && !report.broken_dois.is_empty()
    {
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

    Ok(())
}

fn run_staged_release(
    root: &camino::Utf8Path,
    config: &sil_core::Config,
    paths: &sil_core::ProjectPaths,
    source_only: bool,
    ui: &dyn SilUi,
) -> Result<()> {
    let temp = tempfile::tempdir()?;
    let staging = camino::Utf8PathBuf::from_path_buf(temp.path().to_path_buf())
        .map_err(|_| anyhow::anyhow!("staging path is not UTF-8"))?;
    let registry = sil_app::template_packs::TemplateRegistry::new(root);
    let (staged, main, manifest) =
        match registry.stage(&config.latex.template, &paths.paper_draft(), Some(&staging)) {
            Ok(staged) => {
                let manifest = registry
                    .show(&config.latex.template)
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                let main = staged.join(&manifest.entrypoint);
                (staged, main, Some(manifest))
            }
            Err(_) => {
                // Legacy template names remain usable until a project explicitly
                // installs a package manifest.
                let template = PaperTemplate::from_str(&config.latex.template)
                    .map_err(|e| anyhow::anyhow!("invalid legacy template: {e}"))?;
                let source = std::fs::read_to_string(paths.paper_draft().as_std_path())?;
                let main = staging.join(format!("paper_{}.tex", template.as_str()));
                let rendered = apply_template(template, &source);
                std::fs::write(&main, &rendered)?;
                std::fs::write(
                    root.join(format!("paper_{}.tex", template.as_str())),
                    &rendered,
                )?;
                if template.as_str() == "neurips" && root.join("neurips_2024.sty").is_file() {
                    std::fs::copy(
                        root.join("neurips_2024.sty").as_std_path(),
                        staging.join("neurips_2024.sty").as_std_path(),
                    )?;
                }
                (staging.clone(), main, None)
            }
        };
    let bib = root.join("references.bib");
    if bib.is_file() {
        std::fs::copy(
            bib.as_std_path(),
            staged.join("references.bib").as_std_path(),
        )?;
    }
    let pdf = if source_only {
        None
    } else {
        ui.info(&format!(
            "Compiling staged release with {}",
            config.latex.engine
        ));
        let expected_pdf = main.with_extension("pdf");
        let _ = std::fs::remove_file(expected_pdf.as_std_path());
        Some(
            latex_build(config.latex.engine, &main, &staged)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?,
        )
    };
    let output = root.join(format!("submission_{}.zip", config.latex.template));
    let created = create_staged_submission_release(&ReleaseOptions {
        staging_root: staged,
        main_tex: main.strip_prefix(&staging).unwrap_or(&main).to_path_buf(),
        pdf: pdf
            .as_ref()
            .map(|p| p.strip_prefix(&staging).unwrap_or(p).to_path_buf()),
        output,
        source_only,
        engine: config.latex.engine.to_string(),
        engine_version: None,
        template_digest: manifest
            .as_ref()
            .map(serde_yaml::to_string)
            .transpose()?
            .map(|s| digest_bytes(s.as_bytes())),
        package_lock_digest: {
            let lock = root.join(".sil/templates/lock.json");
            lock.is_file()
                .then(|| std::fs::read(lock.as_std_path()).ok())
                .flatten()
                .map(|bytes| digest_bytes(&bytes))
        },
        revision: None,
    })
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    ui.success(&format!("Submission release: {created}"));
    Ok(())
}

fn digest_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
