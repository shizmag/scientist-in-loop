//! `sil build`

use anyhow::{Result, bail};
use sil_core::SilUi;
use sil_latex::build as latex_build;

use crate::util::load_project;

pub fn run(ui: &dyn SilUi) -> Result<()> {
    let (root, config, _paths) = load_project()?;
    let main = &config.latex.main;
    let engine = config.latex.engine;
    ui.info(&format!("Building {main} with {engine}"));
    let mut spinner = ui.spinner("Compiling LaTeX…");
    match latex_build(engine, main, &root) {
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
