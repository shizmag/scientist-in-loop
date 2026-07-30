//! `sil build`

use anyhow::{Result, bail};
use sil_core::SilUi;
use sil_latex::build as latex_build;

use crate::commands::template_cmd;
use crate::util::load_project;

pub fn run(release: bool, ui: &dyn SilUi) -> Result<()> {
    let (root, config, _paths) = load_project()?;
    let main = if release {
        template_cmd::apply(None, None, None, ui)?;
        let t_name = config.latex.template.clone();
        camino::Utf8PathBuf::from(format!("paper_{t_name}.tex"))
    } else {
        config.latex.main.clone()
    };

    let engine = config.latex.engine;
    ui.info(&format!("Building {main} with {engine}"));
    let mut spinner = ui.spinner("Compiling LaTeX…");
    match latex_build(engine, &main, &root) {
        Ok(pdf) => {
            spinner.finish_success(&format!("Built {pdf}"));
            ui.success(&format!("PDF: {pdf}"));
        }
        Err(e) => {
            spinner.finish_error("build failed");
            bail!("{e}");
        }
    }
    Ok(())
}
