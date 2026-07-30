//! `sil doctor` — dependency and project health checks.

use std::process::Command;

use anyhow::Result;
use serde::Serialize;
use sil_core::SilUi;

use crate::util::load_project;

#[derive(Debug, Serialize)]
struct DoctorReport {
    project: Option<String>,
    ok: bool,
    checks: Vec<Check>,
}

#[derive(Debug, Serialize)]
struct Check {
    name: String,
    ok: bool,
    detail: String,
}

/// Run environment + project diagnostics.
pub fn run(json: bool, ui: &dyn SilUi) -> Result<()> {
    let mut checks = Vec::new();
    let mut project_root = None;

    // Always: git, python, cargo (optional), latex engines
    checks.push(check_cmd("git", &["git", "--version"]));
    checks.push(check_cmd("python3", &["python3", "--version"]));
    checks.push(check_which("tectonic"));
    checks.push(check_which("pdflatex"));
    checks.push(check_which("latexmk"));

    match load_project() {
        Ok((root, config, paths)) => {
            project_root = Some(root.to_string());
            checks.push(Check {
                name: "sil project".into(),
                ok: true,
                detail: format!("root={root}"),
            });
            checks.push(Check {
                name: "config.yaml".into(),
                ok: paths.config().is_file(),
                detail: paths.config().to_string(),
            });
            checks.push(Check {
                name: "structure.yaml".into(),
                ok: paths.structure().is_file(),
                detail: paths.structure().to_string(),
            });
            checks.push(Check {
                name: "paper_draft.tex".into(),
                ok: paths.paper_draft().is_file(),
                detail: paths.paper_draft().to_string(),
            });
            checks.push(Check {
                name: "draft_sections".into(),
                ok: paths.draft_sections_dir().is_dir(),
                detail: paths.draft_sections_dir().to_string(),
            });
            checks.push(Check {
                name: "improvement".into(),
                ok: paths.improvement_dir().is_dir(),
                detail: paths.improvement_dir().to_string(),
            });
            checks.push(Check {
                name: "sqlite db openable".into(),
                ok: sil_db::SilDb::open(&paths.db()).is_ok(),
                detail: paths.db().to_string(),
            });
            checks.push(Check {
                name: "configured latex engine".into(),
                ok: true,
                detail: format!("{}", config.latex.engine),
            });
            let engine = config.latex.engine.to_string();
            if engine != "tectonic" && engine != "pdflatex" && engine != "xelatex" && engine != "lualatex" && engine != "latexmk" {
                // still ok — just report
            }
            let eng_ok = which_ok(&engine);
            checks.push(Check {
                name: format!("engine '{engine}' on PATH"),
                ok: eng_ok,
                detail: if eng_ok {
                    "found".into()
                } else {
                    "not found (build may fail)".into()
                },
            });
        }
        Err(e) => {
            checks.push(Check {
                name: "sil project".into(),
                ok: false,
                detail: format!("not inside a project: {e}"),
            });
        }
    }

    let ok = checks.iter().all(|c| c.ok || is_soft(&c.name));
    // Soft checks: optional latex engines other than configured one
    let report = DoctorReport {
        project: project_root,
        ok: checks.iter().filter(|c| !is_soft(&c.name)).all(|c| c.ok),
        checks,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    ui.println("");
    ui.info("sil doctor");
    for c in &report.checks {
        if c.ok {
            ui.success(&format!("  ✓ {}: {}", c.name, c.detail));
        } else if is_soft(&c.name) {
            ui.muted(&format!("  · {}: {}", c.name, c.detail));
        } else {
            ui.warn(&format!("  ✖ {}: {}", c.name, c.detail));
        }
    }
    ui.println("");
    if report.ok {
        ui.success("Core checks passed");
    } else {
        ui.warn("Some core checks failed — see above");
    }
    let _ = ok;
    Ok(())
}

fn is_soft(name: &str) -> bool {
    name == "tectonic" || name == "pdflatex" || name == "latexmk" || name.starts_with("engine ")
}

fn check_cmd(name: &str, args: &[&str]) -> Check {
    match Command::new(args[0]).args(&args[1..]).output() {
        Ok(o) if o.status.success() => Check {
            name: name.into(),
            ok: true,
            detail: String::from_utf8_lossy(&o.stdout).trim().to_string(),
        },
        Ok(o) => Check {
            name: name.into(),
            ok: false,
            detail: format!(
                "exit {:?}: {}",
                o.status.code(),
                String::from_utf8_lossy(&o.stderr).trim()
            ),
        },
        Err(e) => Check {
            name: name.into(),
            ok: false,
            detail: format!("not available: {e}"),
        },
    }
}

fn check_which(bin: &str) -> Check {
    let ok = which_ok(bin);
    Check {
        name: bin.into(),
        ok,
        detail: if ok {
            "on PATH".into()
        } else {
            "not on PATH (optional unless selected in config)".into()
        },
    }
}

fn which_ok(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
