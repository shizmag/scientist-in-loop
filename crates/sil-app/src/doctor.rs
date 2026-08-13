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

/// Outcome of reparsing a single source document during database repair.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceRepairOutcome {
    /// Source filename.
    pub filename: String,
    /// Whether reparsing succeeded.
    pub ok: bool,
    /// Error message if reparsing failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Summary report for SQLite database repair operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DatabaseRepairReport {
    /// Total source candidate files scanned under `sources/`.
    pub sources_scanned: usize,
    /// Number of source files successfully re-parsed and stored into the new database.
    pub sources_reparsed: usize,
    /// Number of source files that failed re-parsing.
    pub sources_failed: usize,
    /// Backup file path if an existing database was copied aside.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
    /// Detailed outcomes for each scanned source file.
    pub outcomes: Vec<SourceRepairOutcome>,
}

struct MissingMarkerRunner(String);

impl sil_parse::MarkerRunner for MissingMarkerRunner {
    fn parse_pdf(&self, _pdf: &camino::Utf8Path) -> Result<String, sil_parse::ParseError> {
        Err(sil_parse::ParseError::Message(format!(
            "marker PDF parser not found: {}",
            self.0
        )))
    }
}

/// Repair or rebuild SQLite database from on-disk sources when corrupt.
///
/// Steps:
/// 1. Verifies `sources/` directory exists (refuses and errors if missing).
/// 2. If existing `db.sqlite` exists, copies it aside to `db.sqlite.corrupt-<timestamp>`.
/// 3. Removes old live database and WAL/SHM files only after backup succeeds.
/// 4. Opens a fresh `SilDb`.
/// 5. Best-effort re-parses all valid source files in `sources/` and records per-file status.
///
/// Invariant: Never deletes or mutates `sources/` directory.
pub fn repair_sqlite_database(
    project_root: &std::path::Path,
    _ui: &dyn sil_core::SilUi,
) -> Result<DatabaseRepairReport, sil_core::SilError> {
    let root_utf8 = camino::Utf8Path::from_path(project_root)
        .ok_or_else(|| sil_core::SilError::Message("project root path is not valid UTF-8".into()))?;
    let paths = sil_core::ProjectPaths::new(root_utf8);
    let sources_dir = paths.sources_dir();

    if !sources_dir.is_dir() {
        return Err(sil_core::SilError::Message(
            "sources/ directory not found".to_string(),
        ));
    }

    let db_path = paths.db();
    let backup_path_str = if db_path.exists() {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let backup_filename = format!("db.sqlite.corrupt-{ts}");
        let backup_path = db_path.with_file_name(&backup_filename);

        std::fs::copy(&db_path, &backup_path).map_err(sil_core::SilError::Io)?;

        // Remove old db.sqlite and WAL/SHM only after backup succeeds
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(db_path.with_file_name("db.sqlite-wal"));
        let _ = std::fs::remove_file(db_path.with_file_name("db.sqlite.wal"));
        let _ = std::fs::remove_file(db_path.with_file_name("db.sqlite-shm"));
        let _ = std::fs::remove_file(db_path.with_file_name("db.sqlite.shm"));

        Some(backup_path.to_string())
    } else {
        None
    };

    if let Some(parent) = db_path.parent() {
        let _ = std::fs::create_dir_all(parent.as_std_path());
    }

    let db = sil_db::SilDb::open(&db_path).map_err(|e| sil_core::SilError::Database(e.to_string()))?;

    // Collect candidate source files under sources/
    let mut candidate_paths = Vec::new();
    let read_dir = std::fs::read_dir(sources_dir.as_std_path())
        .map_err(sil_core::SilError::Io)?;

    for entry in read_dir {
        let entry = entry.map_err(sil_core::SilError::Io)?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let filename = entry.file_name().to_string_lossy().to_string();
        let lower = filename.to_ascii_lowercase();
        if lower.starts_with("readme") {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        let is_supported = matches!(
            ext.as_str(),
            "pdf" | "md" | "markdown" | "txt" | "html" | "htm" | "tex" | "latex" | "rst"
        );
        if !is_supported {
            continue;
        }

        if let Ok(utf) = camino::Utf8PathBuf::from_path_buf(path) {
            candidate_paths.push((filename, utf));
        }
    }

    candidate_paths.sort_by(|a, b| a.0.cmp(&b.0));

    let runner: Box<dyn sil_parse::MarkerRunner> = match sil_parse::discover_marker_runner() {
        Ok(r) => r,
        Err(err) => Box::new(MissingMarkerRunner(err.to_string())),
    };

    let mut outcomes = Vec::new();
    let mut sources_reparsed = 0;
    let mut sources_failed = 0;
    let null_ui = sil_core::NullUi::new();

    for (filename, path_utf8) in &candidate_paths {
        match sil_parse::parse_one_with_options(
            path_utf8,
            &db,
            runner.as_ref(),
            &null_ui,
            sil_parse::ParseOptions {
                allow_reparse: true,
            },
        ) {
            Ok(_) => {
                sources_reparsed += 1;
                outcomes.push(SourceRepairOutcome {
                    filename: filename.clone(),
                    ok: true,
                    error: None,
                });
            }
            Err(e) => {
                sources_failed += 1;
                outcomes.push(SourceRepairOutcome {
                    filename: filename.clone(),
                    ok: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(DatabaseRepairReport {
        sources_scanned: candidate_paths.len(),
        sources_reparsed,
        sources_failed,
        backup_path: backup_path_str,
        outcomes,
    })
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

    #[test]
    fn test_repair_sqlite_database_missing_sources_dir_errors() {
        let temp = tempfile::tempdir().unwrap();
        let null_ui = sil_core::NullUi::new();
        let err = repair_sqlite_database(temp.path(), &null_ui).unwrap_err();
        assert!(err.to_string().contains("sources/ directory not found"));
    }

    #[test]
    fn test_repair_sqlite_database_corrupt_rebuild() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let sil_dir = root.join(".sil");
        let sources_dir = root.join("sources");
        std::fs::create_dir_all(&sil_dir).unwrap();
        std::fs::create_dir_all(&sources_dir).unwrap();

        // Write a markdown source document
        let doc_content = "# Test Document\n\nContent paragraph.\n\nReferences\n1. Example Citation 2024.";
        std::fs::write(sources_dir.join("paper.md"), doc_content).unwrap();

        // Write corrupted db.sqlite
        let corrupt_bytes = b"CORRUPTED_SQLITE_GARBAGE_BYTES_123456789";
        let db_file = sil_dir.join("db.sqlite");
        std::fs::write(&db_file, corrupt_bytes).unwrap();

        let null_ui = sil_core::NullUi::new();
        let report = repair_sqlite_database(root, &null_ui).unwrap();

        assert_eq!(report.sources_scanned, 1);
        assert_eq!(report.sources_reparsed, 1);
        assert_eq!(report.sources_failed, 0);
        assert!(report.backup_path.is_some());

        let backup = std::path::PathBuf::from(report.backup_path.unwrap());
        assert!(backup.is_file());
        let backup_content = std::fs::read(&backup).unwrap();
        assert_eq!(backup_content, corrupt_bytes);

        // Verify source file was NOT deleted or modified
        assert!(sources_dir.join("paper.md").is_file());
        assert_eq!(std::fs::read_to_string(sources_dir.join("paper.md")).unwrap(), doc_content);

        // Verify new database is openable and passes integrity check
        let db = sil_db::SilDb::open(&camino::Utf8PathBuf::from_path_buf(db_file).unwrap()).unwrap();
        assert_eq!(db.integrity_check().unwrap(), "ok");
        assert_eq!(db.source_count().unwrap(), 1);
    }
}
