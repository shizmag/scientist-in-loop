//! Engine command construction.

use std::process::Command;

use camino::Utf8Path;
use sil_core::LatexEngine;

use crate::error::LatexError;

/// Build a `Command` for the configured engine (does not execute).
pub fn build_command(
    engine: LatexEngine,
    main: &Utf8Path,
    workdir: &Utf8Path,
) -> Result<Command, LatexError> {
    if !main.exists() && !workdir.join(main.as_str()).exists() {
        let candidate = if main.is_absolute() {
            main.to_path_buf()
        } else {
            workdir.join(main)
        };
        if !candidate.exists() {
            return Err(LatexError::MainNotFound(main.to_string()));
        }
    }
    let mut cmd = Command::new(engine.command());
    cmd.current_dir(workdir.as_str());
    match engine {
        LatexEngine::Tectonic => {
            cmd.arg(main.file_name().unwrap_or(main.as_str()));
        }
        LatexEngine::Latexmk => {
            cmd.args(["-pdf", "-interaction=nonstopmode"]);
            cmd.arg(main.file_name().unwrap_or(main.as_str()));
        }
        LatexEngine::Pdflatex | LatexEngine::Xelatex | LatexEngine::Lualatex => {
            cmd.arg("-interaction=nonstopmode");
            cmd.arg(main.file_name().unwrap_or(main.as_str()));
        }
    }
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;

    fn with_main() -> (tempfile::TempDir, Utf8PathBuf, Utf8PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let main = root.join("paper_draft.tex");
        std::fs::write(
            main.as_str(),
            "\\documentclass{article}\\begin{document}x\\end{document}",
        )
        .unwrap();
        (dir, root, main)
    }

    #[test]
    fn build_command_missing_main() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let err =
            build_command(LatexEngine::Tectonic, Utf8Path::new("missing.tex"), &root).unwrap_err();
        assert!(matches!(err, LatexError::MainNotFound(_)));
    }

    #[test]
    fn build_command_tectonic() {
        let (_d, root, main) = with_main();
        let cmd = build_command(LatexEngine::Tectonic, &main, &root).unwrap();
        let prog = cmd.get_program().to_string_lossy();
        assert_eq!(prog, "tectonic");
    }

    #[test]
    fn build_command_latexmk_args() {
        let (_d, root, main) = with_main();
        let cmd = build_command(LatexEngine::Latexmk, &main, &root).unwrap();
        let args: Vec<_> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.iter().any(|a| a == "-pdf"));
        assert!(args.iter().any(|a| a == "-interaction=nonstopmode"));
    }

    #[test]
    fn build_command_pdflatex_family() {
        let (_d, root, main) = with_main();
        for eng in [
            LatexEngine::Pdflatex,
            LatexEngine::Xelatex,
            LatexEngine::Lualatex,
        ] {
            let cmd = build_command(eng, &main, &root).unwrap();
            assert_eq!(cmd.get_program().to_string_lossy(), eng.command());
        }
    }
}
