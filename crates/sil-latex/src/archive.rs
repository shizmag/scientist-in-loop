//! Autonomous zip-archive generator for journal submission.

use std::fs::{self, File};
use std::io::{Read, Write};

use camino::{Utf8Path, Utf8PathBuf};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::error::LatexError;

/// Create an autonomous zip archive containing all necessary files for journal submission.
///
/// Archive includes:
/// - Main LaTeX source file (`main_tex`)
/// - Compiled PDF (`pdf_file`, if present)
/// - Bibliography databases (`.bib` files)
/// - Style and class files (`.sty`, `.cls`, `.bst`)
/// - Referenced or directory figures/assets (`figures/`, `images/`, `.png`, `.jpg`, `.pdf`, `.eps`, `.svg`)
pub fn create_submission_archive(
    root: &Utf8Path,
    main_tex: &Utf8Path,
    pdf_file: Option<&Utf8Path>,
    zip_output: &Utf8Path,
) -> Result<Utf8PathBuf, LatexError> {
    let zip_file = File::create(zip_output.as_std_path()).map_err(|e| LatexError::BuildFailed {
        engine: "tectonic".to_string(),
        message: format!("Could not create submission zip file {zip_output}: {e}"),
    })?;

    let mut zip = ZipWriter::new(zip_file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    let mut added_paths = std::collections::HashSet::new();

    let mut add_file_to_zip = |abs_path: &Utf8Path, zip_path: &str| -> Result<(), LatexError> {
        if !abs_path.is_file() || added_paths.contains(zip_path) {
            return Ok(());
        }
        added_paths.insert(zip_path.to_string());

        let mut f = File::open(abs_path.as_std_path()).map_err(|e| LatexError::BuildFailed {
            engine: "tectonic".to_string(),
            message: format!("Could not open {abs_path} for zip creation: {e}"),
        })?;

        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer).map_err(|e| LatexError::BuildFailed {
            engine: "tectonic".to_string(),
            message: format!("Could not read {abs_path}: {e}"),
        })?;

        zip.start_file(zip_path, options).map_err(|e| LatexError::BuildFailed {
            engine: "tectonic".to_string(),
            message: format!("Could not write entry {zip_path} to zip: {e}"),
        })?;

        zip.write_all(&buffer).map_err(|e| LatexError::BuildFailed {
            engine: "tectonic".to_string(),
            message: format!("Could not write buffer for {zip_path} to zip: {e}"),
        })?;

        Ok(())
    };

    // 1. Main .tex file
    let main_abs = if main_tex.is_absolute() {
        main_tex.to_path_buf()
    } else {
        root.join(main_tex)
    };
    if main_abs.is_file() {
        let rel_name = main_abs.strip_prefix(root).unwrap_or(main_tex);
        add_file_to_zip(&main_abs, rel_name.as_str())?;
    }

    // 2. Compiled PDF
    if let Some(pdf) = pdf_file {
        let pdf_abs = if pdf.is_absolute() {
            pdf.to_path_buf()
        } else {
            root.join(pdf)
        };
        if pdf_abs.is_file() {
            let rel_name = pdf_abs.strip_prefix(root).unwrap_or(pdf);
            add_file_to_zip(&pdf_abs, rel_name.as_str())?;
        }
    }

    // 3. Scan root and subdirectories for bib, sty, cls, bst, and figures
    if let Ok(entries) = fs::read_dir(root.as_std_path()) {
        for entry in entries.flatten() {
            let path = Utf8PathBuf::from_path_buf(entry.path()).unwrap();
            let file_name = path.file_name().unwrap_or_default();

            if path.is_file() {
                let ext = path.extension().unwrap_or_default().to_lowercase();
                if matches!(ext.as_str(), "bib" | "sty" | "cls" | "bst") {
                    add_file_to_zip(&path, file_name)?;
                }
            } else if path.is_dir() {
                // Include directories like figures, images, plots, img, media
                if matches!(file_name, "figures" | "images" | "plots" | "img" | "media") {
                    add_dir_to_zip(&path, root, &mut add_file_to_zip)?;
                }
            }
        }
    }

    zip.finish().map_err(|e| LatexError::BuildFailed {
        engine: "tectonic".to_string(),
        message: format!("Could not finalize submission zip {zip_output}: {e}"),
    })?;

    Ok(zip_output.to_path_buf())
}

fn add_dir_to_zip<F>(dir: &Utf8Path, root: &Utf8Path, add_file: &mut F) -> Result<(), LatexError>
where
    F: FnMut(&Utf8Path, &str) -> Result<(), LatexError>,
{
    if let Ok(entries) = fs::read_dir(dir.as_std_path()) {
        for entry in entries.flatten() {
            let path = Utf8PathBuf::from_path_buf(entry.path()).unwrap();
            if path.is_file() {
                if let Ok(rel) = path.strip_prefix(root) {
                    add_file(&path, rel.as_str())?;
                }
            } else if path.is_dir() {
                add_dir_to_zip(&path, root, add_file)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_submission_archive() {
        let dir = tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();

        let main_tex = root.join("paper_neurips.tex");
        fs::write(
            &main_tex,
            "\\documentclass{article}\n\\begin{document}\nHello\n\\end{document}",
        )
        .unwrap();

        let bib_file = root.join("references.bib");
        fs::write(&bib_file, "@article{test, author={Me}}").unwrap();

        let sty_file = root.join("neurips_2024.sty");
        fs::write(&sty_file, "% sty file").unwrap();

        let pdf_file = root.join("paper_neurips.pdf");
        fs::write(&pdf_file, "%PDF-1.4 mock").unwrap();

        let fig_dir = root.join("figures");
        fs::create_dir(&fig_dir).unwrap();
        let fig_file = fig_dir.join("fig1.png");
        fs::write(&fig_file, "fake png").unwrap();

        let zip_out = root.join("submission_neurips.zip");
        let result =
            create_submission_archive(root, &main_tex, Some(&pdf_file), &zip_out).unwrap();

        assert!(result.is_file());

        // Verify zip contents
        let file = File::open(&zip_out).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        names.sort();

        assert!(names.contains(&"paper_neurips.tex".to_string()));
        assert!(names.contains(&"paper_neurips.pdf".to_string()));
        assert!(names.contains(&"references.bib".to_string()));
        assert!(names.contains(&"neurips_2024.sty".to_string()));
        assert!(names.contains(&"figures/fig1.png".to_string()));
    }
}
