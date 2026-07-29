//! `sil source fetch`

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};
use camino::{Utf8Path, Utf8PathBuf};
use sil_core::{SciAction, SilUi};
use sil_db::SilDb;
use sil_git::CommitProposal;
use sil_parse::parse_one;

use crate::util::{load_project, marker_runner, print_proposal};

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
