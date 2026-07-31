//! `sil init` / `sil init --update`

use anyhow::Result;
use camino::Utf8PathBuf;
use sil_core::SilUi;
use sil_core::paths::{find_project_root, project_root_from_cwd};

use crate::init;

pub fn run(name: Option<String>, update: bool, ui: &dyn SilUi) -> Result<()> {
    if update {
        let target = resolve_update_target(name)?;
        init::update_project(&target, ui)?;
    } else {
        let target = resolve_init_target(name)?;
        init::init_project(&target, ui)?;
    }
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
