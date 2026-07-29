//! `sil init`

use anyhow::Result;
use camino::Utf8PathBuf;
use sil_core::SilUi;

use crate::init;

pub fn run(name: Option<String>, ui: &dyn SilUi) -> Result<()> {
    let target = match name {
        Some(n) => {
            let p = Utf8PathBuf::from(&n);
            if p.is_absolute() {
                p
            } else {
                let cwd = std::env::current_dir()?;
                let cwd = Utf8PathBuf::from_path_buf(cwd)
                    .map_err(|_| anyhow::anyhow!("cwd not utf-8"))?;
                cwd.join(n)
            }
        }
        None => {
            let cwd = std::env::current_dir()?;
            Utf8PathBuf::from_path_buf(cwd).map_err(|_| anyhow::anyhow!("cwd not utf-8"))?
        }
    };
    init::init_project(&target, ui)?;
    Ok(())
}
