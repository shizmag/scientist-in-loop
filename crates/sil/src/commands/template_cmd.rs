//! `sil template` — apply conference/journal LaTeX templates.

use std::fs;
use std::str::FromStr;

use anyhow::{Context, Result, bail};
use camino::Utf8PathBuf;
use sil_app::template_packs::TemplateRegistry;
use sil_core::{SciAction, SilUi};
use sil_git::CommitProposal;
use sil_template::{PaperTemplate, apply_template};

use crate::cli::TemplateCmd;
use crate::util::load_project;

/// Dispatch `sil template`.
pub fn run(
    cmd: Option<TemplateCmd>,
    top_target: Option<String>,
    top_input: Option<Utf8PathBuf>,
    top_output: Option<Utf8PathBuf>,
    ui: &dyn SilUi,
) -> Result<()> {
    match cmd {
        Some(TemplateCmd::List) => list_templates(ui),
        Some(TemplateCmd::Show { id }) => show(&id, ui),
        Some(TemplateCmd::Install { source, approve }) => install(&source, approve, false, ui),
        Some(TemplateCmd::Verify { id }) => verify(&id, ui),
        Some(TemplateCmd::Update { source, approve }) => install(&source, approve, true, ui),
        Some(TemplateCmd::Remove { id }) => remove(&id, ui),
        Some(TemplateCmd::Stage { id, output }) => stage(&id, output, ui),
        Some(TemplateCmd::Apply {
            target,
            input,
            output,
        }) => apply(
            target.or(top_target),
            input.or(top_input),
            output.or(top_output),
            ui,
        ),
        None => {
            if top_target.is_some() || top_input.is_some() || top_output.is_some() {
                apply(top_target, top_input, top_output, ui)
            } else {
                list_templates(ui)
            }
        }
    }
}

/// Print available templates.
pub fn list_templates(ui: &dyn SilUi) -> Result<()> {
    let installed = sil_core::project_root_from_cwd()
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .ok()
        .map(TemplateRegistry::new)
        .map(|registry| registry.list().map_err(|e| anyhow::anyhow!(e.to_string())))
        .transpose()?
        .unwrap_or_default();
    ui.println("");
    ui.info("Installed template packs:");
    let has_installed = !installed.is_empty();
    for pack in installed {
        ui.success(&format!("  {} {}", pack.id, pack.version));
    }
    if !has_installed {
        ui.muted("  none (legacy aliases remain available to `apply`)");
    }
    ui.info("Legacy aliases:");
    for name in PaperTemplate::ALL {
        let t = name.parse::<PaperTemplate>().unwrap();
        ui.muted(&format!("  {:<10} - {}", t.as_str(), t.description()));
    }
    ui.println("");
    ui.muted("Usage:");
    ui.muted("  sil template apply --target neurips");
    ui.muted("  sil build release");
    ui.println("");
    Ok(())
}

fn show(id: &str, ui: &dyn SilUi) -> Result<()> {
    let (root, _, _) = load_project()?;
    let manifest = TemplateRegistry::new(root)
        .show(id)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    ui.println(&serde_yaml::to_string(&manifest)?);
    Ok(())
}

fn install(source: &camino::Utf8Path, approve: bool, update: bool, ui: &dyn SilUi) -> Result<()> {
    let (root, _, _) = load_project()?;
    let registry = TemplateRegistry::new(root);
    let pack = if update {
        registry.update(source, approve)
    } else {
        registry.install(source, approve)
    }
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    ui.success(&format!(
        "{} template pack {} {}",
        if update { "Updated" } else { "Installed" },
        pack.id,
        pack.version
    ));
    ui.muted("  license and redistribution evidence verified");
    Ok(())
}

fn verify(id: &str, ui: &dyn SilUi) -> Result<()> {
    let (root, _, _) = load_project()?;
    TemplateRegistry::new(root)
        .verify(id)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    ui.success(&format!("Verified template pack {id}"));
    Ok(())
}

fn remove(id: &str, ui: &dyn SilUi) -> Result<()> {
    let (root, _, _) = load_project()?;
    TemplateRegistry::new(root)
        .remove(id)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    ui.success(&format!("Removed template pack {id}"));
    Ok(())
}

fn stage(id: &str, output: Option<camino::Utf8PathBuf>, ui: &dyn SilUi) -> Result<()> {
    let (root, _, paths) = load_project()?;
    let destination = TemplateRegistry::new(root)
        .stage(id, &paths.paper_draft(), output.as_deref())
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    ui.success(&format!("Staged template pack {id} at {destination}"));
    Ok(())
}

/// Apply template to manuscript.
pub fn apply(
    target_name: Option<String>,
    input_path: Option<Utf8PathBuf>,
    output_path: Option<Utf8PathBuf>,
    ui: &dyn SilUi,
) -> Result<()> {
    let (root, config, paths) = load_project()?;

    let t_str = target_name
        .as_deref()
        .unwrap_or(config.latex.template.as_str());

    let template = PaperTemplate::from_str(t_str)
        .map_err(|e| anyhow::anyhow!("invalid target template: {e}"))?;

    let in_file = input_path.unwrap_or_else(|| paths.paper_draft());

    if !in_file.is_file() {
        bail!("input manuscript not found: {in_file}");
    }

    let out_file = output_path.unwrap_or_else(|| {
        let name = format!("paper_{}.tex", template.as_str());
        root.join(name)
    });

    let tex_source = fs::read_to_string(in_file.as_str())
        .with_context(|| format!("read input manuscript {in_file}"))?;

    let rendered = apply_template(template, &tex_source);

    sil_core::write_atomic_str(&out_file, &rendered)
        .with_context(|| format!("write output manuscript {out_file}"))?;

    ui.success(&format!(
        "Formatted manuscript using template '{}'",
        template.as_str()
    ));
    ui.muted(&format!("  input:  {in_file}"));
    ui.muted(&format!("  output: {out_file}"));

    let proposal = CommitProposal::new(
        format!("Apply {} article template", template.as_str()),
        SciAction::PromoteToFinal,
    )
    .with_body(format!(
        "Collected prose from {in_file} into {out_file} using target template '{template}'.",
    ));

    ui.println("");
    ui.info("Commit proposal (not applied — never auto-committed):");
    ui.muted("---");
    for line in proposal.message().lines() {
        ui.muted(line);
    }
    ui.muted("---");

    Ok(())
}
