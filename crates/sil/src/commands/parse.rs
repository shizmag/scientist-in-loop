//! `sil parse`

use std::path::PathBuf;

use anyhow::{Result, bail};
use camino::Utf8PathBuf;
use sil_core::{SciAction, SilUi};
use sil_db::SilDb;
use sil_git::CommitProposal;
use sil_parse::{list_unparsed_pdfs, parse_many, parse_one, select_pdfs_interactive};

use crate::util::{load_project, marker_runner, print_proposal};

pub fn run(path: Option<PathBuf>, ui: &dyn SilUi) -> Result<()> {
    let (_root, config, paths) = load_project()?;
    let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let runner = marker_runner()?;
    let sources_dir = paths.sources(&config);

    let to_parse: Vec<Utf8PathBuf> = if let Some(p) = path {
        let utf = Utf8PathBuf::from_path_buf(p).map_err(|_| anyhow::anyhow!("path not utf-8"))?;
        let abs = if utf.is_absolute() {
            utf
        } else {
            let cwd = std::env::current_dir()?;
            Utf8PathBuf::from_path_buf(cwd.join(utf))
                .map_err(|_| anyhow::anyhow!("path not utf-8"))?
        };
        vec![abs]
    } else {
        let unparsed =
            list_unparsed_pdfs(&sources_dir, &db).map_err(|e| anyhow::anyhow!("{e}"))?;
        let selected =
            select_pdfs_interactive(&unparsed, ui).map_err(|e| anyhow::anyhow!("{e}"))?;
        selected.into_iter().map(|i| unparsed[i].clone()).collect()
    };

    if to_parse.is_empty() {
        ui.warn("Nothing to parse.");
        return Ok(());
    }

    if to_parse.len() == 1 {
        match parse_one(&to_parse[0], &db, runner.as_ref(), ui) {
            Ok(r) => {
                ui.success(&format!("Parsed {}", r.document.filename));
                let proposal = CommitProposal::new(
                    format!("Parse PDF: {}", r.document.filename),
                    SciAction::ParsePdf,
                )
                .with_body(format!(
                    "Ingested {} into SQLite + FTS5.",
                    r.document.filename
                ));
                print_proposal(ui, &proposal);
            }
            Err(e) => bail!("{e}"),
        }
    } else {
        let (ok, failed, errors) = parse_many(&to_parse, &db, runner.as_ref(), ui);
        for (p, err) in &errors {
            ui.error(&format!("{}: {err}", p.file_name().unwrap_or(p.as_str())));
        }
        if ok > 0 {
            let proposal = CommitProposal::new(format!("Parse {ok} PDF(s)"), SciAction::ParsePdf)
                .with_body(format!("Parsed {ok} file(s), {failed} failed."));
            print_proposal(ui, &proposal);
        }
        if failed > 0 {
            bail!("Parsed {ok} PDF(s), {failed} failed");
        }
        ui.success(&format!("Parsed {ok} PDF(s)"));
    }
    Ok(())
}
