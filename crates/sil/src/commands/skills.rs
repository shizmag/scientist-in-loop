//! `sil project skills` adapters.

use anyhow::Result;
use sil_agent::{HostCapabilities, SkillRegistry};
use sil_core::SilUi;

use crate::cli::SkillCmd;
use crate::util::load_project;

/// Dispatch a skill registry operation.
pub fn run(action: SkillCmd, ui: &dyn SilUi) -> Result<()> {
    let (root, _, _) = load_project()?;
    let registry = SkillRegistry::new(root);
    match action {
        SkillCmd::List => {
            for item in registry.list()? {
                ui.println(&format!(
                    "{}\t{}\t{}\t{}",
                    item.id, item.version, item.entrypoint, item.path
                ));
            }
        }
        SkillCmd::Show { id } => ui.println(&serde_yaml::to_string(&registry.show(&id)?)?),
        SkillCmd::Install { source, approve } => {
            registry.install(&source, approve)?;
            ui.success("Skill pack installed");
        }
        SkillCmd::Verify { id } => {
            registry.verify(&id)?;
            ui.success("Skill pack verified");
        }
        SkillCmd::CheckUpdate { source } | SkillCmd::Diff { source } => {
            for change in registry.diff(&source)? {
                ui.println(&format!(
                    "{}	{:?}	{:?}",
                    change.path, change.old_sha256, change.new_sha256
                ));
            }
        }
        SkillCmd::ApproveUpdate { source } => {
            registry.update(&source, true)?;
            ui.success("Skill pack updated");
        }
        SkillCmd::Remove { id } => {
            registry.remove(&id)?;
            ui.success("Skill pack removed");
        }
        SkillCmd::Rollback { id } => {
            registry.rollback(&id)?;
            ui.success("Skill pack rolled back");
        }
        SkillCmd::Check {
            id,
            host,
            network,
            process,
        } => {
            let status = registry.check(
                &id,
                &HostCapabilities {
                    host,
                    network,
                    process,
                    ..HostCapabilities::default()
                },
            )?;
            ui.println(&format!("{status:?}"));
        }
    }
    Ok(())
}
