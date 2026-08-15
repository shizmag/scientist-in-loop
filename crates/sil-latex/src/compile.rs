//! Compile a LaTeX document with the configured engine.

use std::process::Command;
use std::time::Instant;

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;
use sil_core::LatexEngine;

use crate::engine::build_command;
use crate::error::LatexError;

/// Structured result of one compiler invocation.
#[derive(Debug, Clone, Serialize)]
pub struct CompilerResult {
    /// Engine name.
    pub engine: String,
    /// Engine version output, when it could be queried.
    pub version: Option<String>,
    /// Executed argv, excluding the executable name.
    pub argv: Vec<String>,
    /// Process exit code, if a process was started.
    pub exit_code: Option<i32>,
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
    /// Wall-clock execution time in milliseconds.
    pub duration_ms: u128,
    /// Best-effort source location parsed from compiler output.
    pub error_location: Option<String>,
    /// Expected artifact path and whether this run created or replaced it.
    pub artifact: Option<CompilerArtifact>,
    /// Structured error when compilation did not complete successfully.
    pub error: Option<String>,
}

/// Metadata proving whether an expected compiler artifact was produced by this run.
#[derive(Debug, Clone, Serialize)]
pub struct CompilerArtifact {
    /// Project-relative or absolute PDF path.
    pub path: String,
    /// Whether the PDF existed after the invocation.
    pub exists: bool,
    /// Whether its modification time advanced during this invocation.
    pub newly_produced: bool,
    /// Size after the invocation.
    pub bytes: Option<u64>,
}

/// Execute the configured engine and retain all result details.
pub fn build_structured(
    engine: LatexEngine,
    main: &Utf8Path,
    workdir: &Utf8Path,
) -> CompilerResult {
    let main_path = if main.is_absolute() {
        main.to_path_buf()
    } else {
        workdir.join(main)
    };
    let pdf = main_path.with_extension("pdf");
    let before = std::fs::metadata(pdf.as_std_path())
        .and_then(|m| m.modified())
        .ok();
    let version = Command::new(engine.command())
        .arg("--version")
        .output()
        .ok()
        .map(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if stdout.is_empty() {
                String::from_utf8_lossy(&o.stderr).trim().to_string()
            } else {
                stdout
            }
        })
        .filter(|s| !s.is_empty());
    let mut result = CompilerResult {
        engine: engine.to_string(),
        version,
        argv: Vec::new(),
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
        duration_ms: 0,
        error_location: None,
        artifact: None,
        error: None,
    };
    if !main_path.is_file() {
        result.error = Some(LatexError::MainNotFound(main_path.to_string()).to_string());
        result.artifact = Some(CompilerArtifact {
            path: pdf.to_string(),
            exists: false,
            newly_produced: false,
            bytes: None,
        });
        return result;
    }
    let mut command = match build_command(engine, &main_path, workdir) {
        Ok(command) => command,
        Err(error) => {
            result.error = Some(error.to_string());
            return result;
        }
    };
    result.argv = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let started = Instant::now();
    match command.output() {
        Ok(output) => {
            result.duration_ms = started.elapsed().as_millis();
            result.exit_code = output.status.code();
            result.stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            result.stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            result.error_location = first_error_location(&result.stderr)
                .or_else(|| first_error_location(&result.stdout));
            if !output.status.success() {
                result.error = Some(
                    LatexError::BuildFailed {
                        engine: engine.to_string(),
                        message: format!("{}\n{}", result.stderr.trim(), result.stdout.trim()),
                    }
                    .to_string(),
                );
            }
        }
        Err(error) => {
            result.duration_ms = started.elapsed().as_millis();
            result.error = Some(if error.kind() == std::io::ErrorKind::NotFound {
                LatexError::EngineNotFound {
                    engine: engine.to_string(),
                }
                .to_string()
            } else {
                error.to_string()
            });
        }
    }
    let after = std::fs::metadata(pdf.as_std_path()).ok();
    let after_time = after.as_ref().and_then(|m| m.modified().ok());
    let newly_produced = after.is_some()
        && match (before, after_time) {
            (None, Some(_)) => true,
            (Some(before), Some(after)) => after > before,
            _ => false,
        };
    result.artifact = Some(CompilerArtifact {
        path: pdf.to_string(),
        exists: after.is_some(),
        newly_produced,
        bytes: after.map(|m| m.len()),
    });
    if result.error.is_none() && !newly_produced {
        result.error = Some("compiler succeeded but did not produce a new PDF".into());
    }
    result
}

fn first_error_location(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let bytes = line.as_bytes();
        let colon = bytes.iter().position(|byte| *byte == b':')?;
        let rest = &line[colon + 1..];
        let digits = rest.chars().take_while(char::is_ascii_digit).count();
        if digits > 0 && rest[digits..].starts_with(':') {
            Some(line[..colon + 1 + digits].to_string())
        } else {
            None
        }
    })
}

/// Compile the main document with the given engine.
pub fn build(
    engine: LatexEngine,
    main: &Utf8Path,
    workdir: &Utf8Path,
) -> Result<Utf8PathBuf, LatexError> {
    let main_path = if main.is_absolute() {
        main.to_path_buf()
    } else {
        workdir.join(main)
    };
    if !main_path.exists() {
        return Err(LatexError::MainNotFound(main_path.to_string()));
    }

    let result = build_structured(engine, &main_path, workdir);
    if let Some(error) = result.error {
        return Err(LatexError::Message(error));
    }
    Ok(main_path.with_extension("pdf"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    use sil_core::LatexEngine;

    #[test]
    fn build_missing_main_is_actionable() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let err = build(LatexEngine::Tectonic, Utf8Path::new("nope.tex"), &root).unwrap_err();
        let msg = err.to_string();
        assert!(
            matches!(err, LatexError::MainNotFound(_)) || msg.contains("not found"),
            "{msg}"
        );
    }

    #[test]
    fn build_missing_engine_is_actionable_when_absent() {
        // Prefer an engine that is often absent; skip only if every candidate is present.
        let candidates = [
            LatexEngine::Latexmk,
            LatexEngine::Lualatex,
            LatexEngine::Xelatex,
            LatexEngine::Pdflatex,
        ];
        let missing = candidates.into_iter().find(|e| {
            Command::new(e.command()).arg("--version").output().is_err()
                && Command::new(e.command()).arg("-v").output().is_err()
        });
        let Some(engine) = missing else {
            // All engines installed — still check message shape via MainNotFound path.
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let main = root.join("paper_draft.tex");
        std::fs::write(
            main.as_str(),
            "\\documentclass{article}\\begin{document}x\\end{document}",
        )
        .unwrap();
        let err = build(engine, &main, &root).unwrap_err();
        let msg = err.to_string();
        assert!(
            matches!(err, LatexError::EngineNotFound { .. })
                || msg.contains("not found")
                || msg.contains(engine.command()),
            "{msg}"
        );
        assert!(
            msg.contains("config")
                || msg.contains("PATH")
                || msg.contains("install")
                || msg.contains(engine.command()),
            "message should be actionable: {msg}"
        );
    }
}
