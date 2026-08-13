//! Host dependency and environment doctor diagnostics.

use std::process::Command;
use serde::{Deserialize, Serialize};

/// Summary report for doctor checks.
#[derive(Debug, Serialize, Deserialize)]
pub struct DoctorReport {
    /// Active project root path if inside a project.
    pub project: Option<String>,
    /// Overall success status (all non-soft checks pass).
    pub ok: bool,
    /// Detailed results of all checks.
    pub checks: Vec<Check>,
}

/// A single doctor diagnostic check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Check {
    /// Name of the check (e.g. "git", "python3", "tectonic").
    pub name: String,
    /// Whether the check passed.
    pub ok: bool,
    /// Human-readable detail (version or error message).
    pub detail: String,
    /// Optional actionable guidance to resolve failure / missing dependency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// Optional machine-readable payload (e.g. dense RAG mode/reason).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

impl Check {
    /// Construct a simple Check without hint or extra payload.
    pub fn simple(name: impl Into<String>, ok: bool, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ok,
            detail: detail.into(),
            hint: None,
            extra: None,
        }
    }

    /// Construct a Check with optional actionable hint.
    pub fn with_hint(
        name: impl Into<String>,
        ok: bool,
        detail: impl Into<String>,
        hint: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            ok,
            detail: detail.into(),
            hint,
            extra: None,
        }
    }
}

/// Run host environment checks (git, python3, uv, latex engines, marker).
pub fn run_host_checks() -> Vec<Check> {
    vec![
        check_cmd("git", &["git", "--version"]),
        check_cmd("python3", &["python3", "--version"]),
        check_cmd("uv", &["uv", "--version"]),
        check_which("tectonic"),
        check_which("pdflatex"),
        check_which("latexmk"),
        check_marker(),
    ]
}

/// Run a command and report its status as a [`Check`].
pub fn check_cmd(name: &str, args: &[&str]) -> Check {
    match Command::new(args[0]).args(&args[1..]).output() {
        Ok(o) if o.status.success() => Check::simple(
            name,
            true,
            String::from_utf8_lossy(&o.stdout).trim().to_string(),
        ),
        Ok(o) => Check::with_hint(
            name,
            false,
            format!(
                "exit {:?}: {}",
                o.status.code(),
                String::from_utf8_lossy(&o.stderr).trim()
            ),
            tool_hint(name),
        ),
        Err(e) => Check::with_hint(
            name,
            false,
            format!("not available: {e}"),
            tool_hint(name),
        ),
    }
}

/// Check if a binary exists on PATH using `which`.
pub fn check_which(bin: &str) -> Check {
    let ok = which_ok(bin);
    Check::with_hint(
        bin,
        ok,
        if ok {
            "on PATH"
        } else {
            "not on PATH (optional unless selected in config)"
        },
        if ok { None } else { tool_hint(bin) },
    )
}

/// Check if Marker PDF runner is discoverable on the host.
pub fn check_marker() -> Check {
    let runner = sil_parse::discover_marker_runner();
    let ok = runner.is_ok();
    Check::with_hint(
        "marker",
        ok,
        if ok {
            "available"
        } else {
            "not found (optional; needed for PDF parsing)"
        },
        if ok { None } else { tool_hint("marker") },
    )
}

/// Test if a binary is executable via `which`.
pub fn which_ok(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Return remediation advice for a tool/dependency.
pub fn tool_hint(name: &str) -> Option<String> {
    match name {
        "git" => Some(
            "Install git via your system package manager (e.g. `brew install git` or `apt install git`)"
                .to_string(),
        ),
        "python3" | "python" => Some(
            "Install Python 3.10+ (e.g. `brew install python` or `apt install python3`)".to_string(),
        ),
        "uv" => Some(
            "Install uv (`curl -LsSf https://astral.sh/uv/install.sh | sh`) or Python 3.10+"
                .to_string(),
        ),
        "tectonic" => Some(
            "Install tectonic (`brew install tectonic`) or configure latexmk/pdflatex in .sil/config.yaml"
                .to_string(),
        ),
        "pdflatex" => Some(
            "Install TeX Live / MacTeX (e.g. `brew install --cask mactex` or `apt install texlive-latex-base`)"
                .to_string(),
        ),
        "latexmk" => Some(
            "Install latexmk (e.g. `brew install latexmk` or `apt install latexmk`)".to_string(),
        ),
        "marker" => Some(
            "Install marker-pdf (`uv pip install marker-pdf`) for PDF parsing, or use text/markdown sources"
                .to_string(),
        ),
        _ => None,
    }
}

/// Return remediation guidance for a specific LaTeX engine.
pub fn latex_engine_hint(engine: &str) -> String {
    match engine {
        "tectonic" => "Install tectonic (`brew install tectonic`) or configure latexmk/pdflatex in .sil/config.yaml".to_string(),
        "latexmk" => "Install latexmk (`brew install latexmk`) or configure tectonic in .sil/config.yaml".to_string(),
        "pdflatex" | "xelatex" | "lualatex" => "Install TeX Live / MacTeX (e.g. `brew install --cask mactex`) or configure tectonic in .sil/config.yaml".to_string(),
        other => format!("Install '{other}' or configure a supported engine (tectonic, pdflatex, latexmk) in .sil/config.yaml"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_hints_contain_actionable_guidance() {
        assert!(tool_hint("git").unwrap().contains("brew install git"));
        assert!(tool_hint("python3").unwrap().contains("Python 3.10+"));
        assert!(tool_hint("uv").unwrap().contains("curl -LsSf https://astral.sh/uv/install.sh"));
        assert!(tool_hint("tectonic").unwrap().contains("brew install tectonic"));
        assert!(tool_hint("pdflatex").unwrap().contains("TeX Live"));
        assert!(tool_hint("latexmk").unwrap().contains("brew install latexmk"));
        assert!(tool_hint("marker").unwrap().contains("uv pip install marker-pdf"));
        assert_eq!(tool_hint("unknown_tool"), None);
    }

    #[test]
    fn test_latex_engine_hint() {
        let tectonic_hint = latex_engine_hint("tectonic");
        assert!(tectonic_hint.contains("brew install tectonic"));
        assert!(tectonic_hint.contains("latexmk/pdflatex"));

        let latexmk_hint = latex_engine_hint("latexmk");
        assert!(latexmk_hint.contains("brew install latexmk"));

        let pdflatex_hint = latex_engine_hint("pdflatex");
        assert!(pdflatex_hint.contains("TeX Live"));

        let custom_hint = latex_engine_hint("custom-engine");
        assert!(custom_hint.contains("custom-engine"));
    }

    #[test]
    fn test_check_serialization_with_and_without_hint() {
        let check_no_hint = Check::simple("git", true, "git version 2.40.0");
        let json_no_hint = serde_json::to_string(&check_no_hint).unwrap();
        assert!(!json_no_hint.contains("\"hint\""));

        let check_with_hint = Check::with_hint(
            "tectonic",
            false,
            "not on PATH",
            Some("Install tectonic (`brew install tectonic`)".to_string()),
        );
        let json_with_hint = serde_json::to_string(&check_with_hint).unwrap();
        assert!(json_with_hint.contains("\"hint\":"));
        assert!(json_with_hint.contains("brew install tectonic"));
    }

    #[test]
    fn test_run_host_checks_contains_core_tools() {
        let checks = run_host_checks();
        let names: Vec<&str> = checks.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"git"));
        assert!(names.contains(&"python3"));
        assert!(names.contains(&"uv"));
        assert!(names.contains(&"tectonic"));
        assert!(names.contains(&"pdflatex"));
        assert!(names.contains(&"latexmk"));
        assert!(names.contains(&"marker"));
    }
}
