//! `sil paper assets` — list and validate paper figures, graphics, and data assets.

use anyhow::Result;
use serde::Serialize;
use sil_core::SilUi;

use crate::util::load_project;

#[derive(Debug, Serialize)]
struct AssetReport {
    graphics: Vec<AssetInfo>,
    inputs: Vec<AssetInfo>,
    total_count: usize,
    all_found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    check_fingerprint: Option<String>,
}

#[derive(Debug, Serialize)]
struct AssetInfo {
    path: String,
    exists: bool,
    kind: String,
}

/// Run assets inspection command.
pub fn run(json: bool, ui: &dyn SilUi) -> Result<()> {
    let (root, _config, paths) = load_project()?;
    let check_fingerprint =
        sil_app::load_cached_report(&root)?.map(|report| report.r#static.input_fingerprint);
    let draft_path = paths.paper_draft();

    let content = if draft_path.exists() {
        std::fs::read_to_string(draft_path.as_std_path())?
    } else {
        String::new()
    };

    let mut graphics = Vec::new();
    let mut inputs = Vec::new();

    let img_re = regex::Regex::new(r"\\includegraphics(?:\[.*?\])?\{([^}]+)\}")?;
    for cap in img_re.captures_iter(&content) {
        if let Some(m) = cap.get(1) {
            let p_str = m.as_str().trim();
            let abs_p = root.join(p_str);
            let exists = abs_p.exists()
                || root.join(format!("{p_str}.pdf")).exists()
                || root.join(format!("{p_str}.png")).exists()
                || root.join(format!("{p_str}.jpg")).exists();
            graphics.push(AssetInfo {
                path: p_str.to_string(),
                exists,
                kind: "includegraphics".to_string(),
            });
        }
    }

    let input_re = regex::Regex::new(r"\\(?:input|include)\{([^}]+)\}")?;
    for cap in input_re.captures_iter(&content) {
        if let Some(m) = cap.get(1) {
            let p_str = m.as_str().trim();
            let abs_p = root.join(p_str);
            let exists = abs_p.exists() || root.join(format!("{p_str}.tex")).exists();
            inputs.push(AssetInfo {
                path: p_str.to_string(),
                exists,
                kind: "input".to_string(),
            });
        }
    }

    let total_count = graphics.len() + inputs.len();
    let all_found = graphics.iter().chain(inputs.iter()).all(|a| a.exists);

    let report = AssetReport {
        graphics,
        inputs,
        total_count,
        all_found,
        check_fingerprint,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    ui.info(&format!("Paper assets in {}:", paths.paper_draft()));
    if report.total_count == 0 {
        ui.muted("No \\includegraphics or \\input assets detected in paper_draft.tex.");
        return Ok(());
    }

    for g in &report.graphics {
        if g.exists {
            ui.success(&format!("  ✓ [graphic] {}", g.path));
        } else {
            ui.warn(&format!("  ✖ [graphic] {} (not found)", g.path));
        }
    }
    for i in &report.inputs {
        if i.exists {
            ui.success(&format!("  ✓ [input]   {}", i.path));
        } else {
            ui.warn(&format!("  ✖ [input]   {} (not found)", i.path));
        }
    }

    if all_found {
        ui.success("All referenced paper assets are present.");
    } else {
        ui.warn("Some referenced paper assets are missing.");
    }

    Ok(())
}
