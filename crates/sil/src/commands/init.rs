//! `sil init` / `sil init --update`

use anyhow::{Result, bail};
use camino::Utf8PathBuf;
use sil_core::SilUi;
use sil_core::paths::{find_project_root, project_root_from_cwd};
use sil_db::SilDb;
use sil_parse::{StubMarkerRunner, parse_one};

use crate::init;

pub fn run(name: Option<String>, update: bool, demo: bool, ui: &dyn SilUi) -> Result<()> {
    if update && demo {
        bail!("--demo is only supported when creating a new project");
    }
    if update {
        let target = resolve_update_target(name)?;
        init::update_project(&target, ui)?;
    } else {
        let target = resolve_init_target(name)?;
        init::init_project(&target, ui)?;
        if demo {
            create_demo(&target, ui)?;
        }
    }
    Ok(())
}

fn create_demo(target: &Utf8PathBuf, ui: &dyn SilUi) -> Result<()> {
    let source = target.join("sources/demo-attention.md");
    std::fs::write(
        &source,
        "# Demo Attention\n\nThese synthetic notes describe Demo Attention, a fictional study of toy attention patterns.\n\nThe example is deliberately small and contains no real paper or copyrighted material.\n",
    )?;

    let db = SilDb::open(&target.join(".sil/db.sqlite"))
        .map_err(|e| anyhow::anyhow!("open demo database: {e}"))?;
    parse_one(
        &source,
        &db,
        &StubMarkerRunner {
            content: String::new(),
        },
        ui,
    )
    .map_err(|e| anyhow::anyhow!("parse demo source: {e}"))?;

    std::fs::write(
        target.join("paper_draft.tex"),
        r#"\documentclass{article}
\begin{document}
\section{Introduction}
Demo Attention is a synthetic fixture for exploring the scientist-in-loop workflow.

% # -- X -- #
% TODO: compare the toy attention pattern with a real experiment.
% # -- X -- #

\section{Discussion}
The fixture is intentionally offline and contains no claims about a real publication.
We refer to the synthetic source here \cite{demo2024}.
\bibliographystyle{plain}
\bibliography{references}
\end{document}
"#,
    )?;
    std::fs::write(
        target.join("references.bib"),
        r#"@article{demo2024,
  title = {Demo Attention},
  author = {Scientist, Example},
  year = {2024},
  note = {Synthetic offline fixture; not a real publication}
}
"#,
    )?;
    ui.success("Created offline demo fixture");
    Ok(())
}

fn resolve_init_target(name: Option<String>) -> Result<Utf8PathBuf> {
    match name {
        Some(n) => resolve_path(&n),
        None => {
            let cwd = std::env::current_dir()?;
            Utf8PathBuf::from_path_buf(cwd).map_err(|_| anyhow::anyhow!("cwd not utf-8"))
        }
    }
}

/// For `--update`, prefer an explicit path; otherwise walk up from cwd for `.sil/config.yaml`.
fn resolve_update_target(name: Option<String>) -> Result<Utf8PathBuf> {
    match name {
        Some(n) => {
            let p = resolve_path(&n)?;
            // If the path itself is not a project, allow walking up from it.
            if p.join(".sil/config.yaml").is_file() {
                Ok(p)
            } else if let Some(root) = find_project_root(&p) {
                Ok(root)
            } else {
                Ok(p) // update_project will report a clear error
            }
        }
        None => project_root_from_cwd().map_err(|e| anyhow::anyhow!("{e}")),
    }
}

fn resolve_path(n: &str) -> Result<Utf8PathBuf> {
    let p = Utf8PathBuf::from(n);
    if p.is_absolute() {
        Ok(p)
    } else {
        let cwd = std::env::current_dir()?;
        let cwd = Utf8PathBuf::from_path_buf(cwd).map_err(|_| anyhow::anyhow!("cwd not utf-8"))?;
        Ok(cwd.join(n))
    }
}
