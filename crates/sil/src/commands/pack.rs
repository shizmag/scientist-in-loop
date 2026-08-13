//! `sil paper pack` — build reproducible manuscript package bundle.

use std::fs::File;
use std::io::Write;

use anyhow::{Context, Result};
use sil_core::SilUi;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::util::load_project;

/// Run paper pack repro bundle generator.
pub fn run(output: Option<camino::Utf8PathBuf>, ui: &dyn SilUi) -> Result<()> {
    let (root, config, paths) = load_project()?;

    let zip_name = output.unwrap_or_else(|| root.join("paper_pack.zip"));
    ui.info(&format!(
        "Generating reproducible paper pack bundle: {zip_name}"
    ));

    let file = File::create(zip_name.as_std_path())
        .with_context(|| format!("failed to create zip file at {zip_name}"))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // Include paper_draft.tex
    let draft_p = paths.paper_draft();
    if draft_p.exists()
        && let Ok(c) = std::fs::read(&draft_p)
    {
        zip.start_file("paper_draft.tex", options)?;
        zip.write_all(&c)?;
    }

    // Include config.yaml & structure.yaml
    if paths.config().exists()
        && let Ok(c) = std::fs::read(paths.config())
    {
        zip.start_file("config.yaml", options)?;
        zip.write_all(&c)?;
    }
    if paths.structure().exists()
        && let Ok(c) = std::fs::read(paths.structure())
    {
        zip.start_file("structure.yaml", options)?;
        zip.write_all(&c)?;
    }

    // Include references.bib
    let bib_p = root.join("references.bib");
    if bib_p.exists()
        && let Ok(c) = std::fs::read(&bib_p)
    {
        zip.start_file("references.bib", options)?;
        zip.write_all(&c)?;
    }

    // Include .sil/reviews/ reports
    let reviews_dir = paths.sil_dir().join("reviews");
    if reviews_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(reviews_dir.as_std_path())
    {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file()
                && let Some(fname) = p.file_name().and_then(|n| n.to_str())
                && let Ok(c) = std::fs::read(&p)
            {
                zip.start_file(format!("reviews/{fname}"), options)?;
                zip.write_all(&c)?;
            }
        }
    }

    // Include REPRO.md manifesto
    let repro_manifest = format!(
        "# Reproducibility Manifest — {}\n\n\
         - **Project**: {}\n\
         - **Template**: {}\n\
         - **Engine**: {}\n\
         - **sil Version**: v1.0.0\n",
        config.latex.main,
        root.file_name().unwrap_or("project"),
        config.latex.template,
        config.latex.engine,
    );
    zip.start_file("REPRO.md", options)?;
    zip.write_all(repro_manifest.as_bytes())?;

    zip.finish()?;
    ui.success(&format!("Created reproducible paper pack: {zip_name}"));

    Ok(())
}
