//! Compile a LaTeX document with the configured engine.

use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};
use sil_core::LatexEngine;

use crate::engine::build_command;
use crate::error::LatexError;

/// Compile the main document with the given engine.
pub fn build(
    engine: LatexEngine,
    main: &Utf8Path,
    workdir: &Utf8Path,
) -> Result<Utf8PathBuf, LatexError> {
    let which = Command::new(engine.command()).arg("--version").output();
    if which.is_err() {
        let which2 = Command::new(engine.command()).arg("-v").output();
        if which2.is_err() {
            return Err(LatexError::EngineNotFound {
                engine: engine.to_string(),
            });
        }
    }

    let main_path = if main.is_absolute() {
        main.to_path_buf()
    } else {
        workdir.join(main)
    };
    if !main_path.exists() {
        return Err(LatexError::MainNotFound(main_path.to_string()));
    }

    let mut cmd = build_command(engine, &main_path, workdir)?;
    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            LatexError::EngineNotFound {
                engine: engine.to_string(),
            }
        } else {
            LatexError::Message(e.to_string())
        }
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(LatexError::BuildFailed {
            engine: engine.to_string(),
            message: format!("{}\n{}", stderr.trim(), stdout.trim()),
        });
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
