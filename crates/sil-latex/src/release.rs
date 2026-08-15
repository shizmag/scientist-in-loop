//! Deterministic submission release creation from an isolated staging tree.

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Write;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::LatexError;
use crate::graph::{DependencyKind, GraphOptions, build_dependency_graph};

/// Inputs and provenance for a staged release.
#[derive(Debug, Clone)]
pub struct ReleaseOptions {
    /// Isolated staging directory.
    pub staging_root: Utf8PathBuf,
    /// Main TeX file relative to the staging directory.
    pub main_tex: Utf8PathBuf,
    /// Compiled PDF relative to the staging directory, when compilation succeeded.
    pub pdf: Option<Utf8PathBuf>,
    /// Destination ZIP, published atomically after validation.
    pub output: Utf8PathBuf,
    /// Whether this is an explicitly requested source-only release.
    pub source_only: bool,
    /// Selected LaTeX engine.
    pub engine: String,
    /// Engine version, if available.
    pub engine_version: Option<String>,
    /// Template manifest digest, if available.
    pub template_digest: Option<String>,
    /// Package lock digest, if available.
    pub package_lock_digest: Option<String>,
    /// Project/source revision, if available.
    pub revision: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReleaseManifest {
    schema: String,
    revision: Option<String>,
    input_fingerprint: String,
    template_digest: Option<String>,
    package_lock_digest: Option<String>,
    engine: String,
    engine_version: Option<String>,
    compile: CompileStatus,
    members: Vec<Member>,
    exclusions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CompileStatus {
    requested: bool,
    performed: bool,
    succeeded: bool,
    mode: &'static str,
}

#[derive(Debug, Serialize)]
struct Member {
    path: String,
    sha256: String,
    size: u64,
    mode: u32,
}

/// Build and atomically publish a dependency-complete staged release.
pub fn create_staged_submission_release(
    options: &ReleaseOptions,
) -> Result<Utf8PathBuf, LatexError> {
    let root = &options.staging_root;
    let main = root.join(&options.main_tex);
    if !main.is_file() {
        return Err(LatexError::MainNotFound(main.to_string()));
    }
    let graph = build_dependency_graph(&GraphOptions::new(root.clone(), options.main_tex.clone()))?;
    let missing: Vec<_> = graph
        .dependencies
        .iter()
        .filter(|n| !n.exists && !is_system_texlive_dependency(n))
        .map(|n| n.path.clone())
        .collect();
    if !missing.is_empty() {
        return Err(LatexError::BuildFailed {
            engine: "release".into(),
            message: format!("missing dependencies: {}", missing.join(", ")),
        });
    }
    if graph.dependencies.iter().any(|n| n.external) {
        return Err(LatexError::BuildFailed {
            engine: "release".into(),
            message: "dependency path escapes staging tree".into(),
        });
    }
    if !options.source_only
        && !options
            .pdf
            .as_ref()
            .map(|p| root.join(p).is_file())
            .unwrap_or(false)
    {
        return Err(LatexError::BuildFailed {
            engine: "release".into(),
            message: "successful release requires a newly produced PDF".into(),
        });
    }

    let mut paths = BTreeSet::new();
    for node in &graph.dependencies {
        if !node.exists && is_system_texlive_dependency(node) {
            continue;
        }
        paths.insert(node.path.replace('\\', "/"));
    }
    if let Some(pdf) = &options.pdf
        && !options.source_only
    {
        paths.insert(pdf.as_str().replace('\\', "/"));
    }
    let mut members = Vec::new();
    for path in paths {
        let file = root.join(&path);
        if !file.is_file() {
            return Err(LatexError::BuildFailed {
                engine: "release".into(),
                message: format!("dependency is not a regular file: {path}"),
            });
        }
        let bytes = fs::read(file.as_std_path()).map_err(|e| io_error(&file, e))?;
        members.push(Member {
            path,
            sha256: digest(&bytes),
            size: bytes.len() as u64,
            mode: 0o644,
        });
    }
    let fingerprint = digest(
        serde_json::to_vec(&members)
            .map_err(|e| LatexError::Message(e.to_string()))?
            .as_slice(),
    );
    let manifest = ReleaseManifest {
        schema: "sil.dev/release/v1".into(),
        revision: options.revision.clone(),
        input_fingerprint: fingerprint,
        template_digest: options.template_digest.clone(),
        package_lock_digest: options.package_lock_digest.clone(),
        engine: options.engine.clone(),
        engine_version: options.engine_version.clone(),
        compile: CompileStatus {
            requested: !options.source_only,
            performed: !options.source_only,
            succeeded: !options.source_only,
            mode: if options.source_only {
                "source-only"
            } else {
                "compiled"
            },
        },
        members,
        exclusions: vec!["workspace files outside the staging tree".into()],
    };
    let manifest_bytes =
        serde_json::to_vec_pretty(&manifest).map_err(|e| LatexError::Message(e.to_string()))?;
    let parent = options.output.parent().unwrap_or(Utf8Path::new("."));
    fs::create_dir_all(parent.as_std_path()).map_err(|e| io_error(parent, e))?;
    let temp = options.output.with_extension("zip.part");
    let result = write_zip(root, &temp, &manifest_bytes, &manifest.members);
    if result.is_err() {
        let _ = fs::remove_file(temp.as_std_path());
    }
    result?;
    fs::rename(temp.as_std_path(), options.output.as_std_path())
        .map_err(|e| io_error(&options.output, e))?;
    Ok(options.output.clone())
}

fn is_system_texlive_dependency(node: &crate::graph::DependencyNode) -> bool {
    if node.exists || !matches!(node.kind, DependencyKind::Style | DependencyKind::Class) {
        return false;
    }
    matches!(
        node.path.as_str(),
        "inputenc.sty"
            | "fontenc.sty"
            | "hyperref.sty"
            | "url.sty"
            | "booktabs.sty"
            | "amsfonts.sty"
            | "nicefrac.sty"
            | "microtype.sty"
            | "xcolor.sty"
            | "graphicx.sty"
            | "amsmath.sty"
            | "amssymb.sty"
            | "article.cls"
    )
}

fn write_zip(
    root: &Utf8Path,
    output: &Utf8Path,
    manifest: &[u8],
    members: &[Member],
) -> Result<(), LatexError> {
    let file = File::create(output.as_std_path()).map_err(|e| io_error(output, e))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    for member in members {
        zip.start_file(&member.path, options)
            .map_err(|e| LatexError::Message(e.to_string()))?;
        zip.write_all(
            &fs::read(root.join(&member.path))
                .map_err(|e| io_error(&root.join(&member.path), e))?,
        )
        .map_err(|e| io_error(output, e))?;
    }
    zip.start_file("SIL-RELEASE.json", options)
        .map_err(|e| LatexError::Message(e.to_string()))?;
    zip.write_all(manifest).map_err(|e| io_error(output, e))?;
    zip.finish()
        .map_err(|e| LatexError::Message(e.to_string()))?;
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn io_error(path: &Utf8Path, source: std::io::Error) -> LatexError {
    LatexError::Io {
        path: path.to_string(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::tempdir;

    fn options(root: &Utf8Path, output: &Utf8Path, source_only: bool) -> ReleaseOptions {
        ReleaseOptions {
            staging_root: root.to_path_buf(),
            main_tex: "main.tex".into(),
            pdf: None,
            output: output.to_path_buf(),
            source_only,
            engine: "test-engine".into(),
            engine_version: Some("1".into()),
            template_digest: Some("template".into()),
            package_lock_digest: Some("lock".into()),
            revision: Some("revision".into()),
        }
    }

    #[test]
    fn source_only_manifest_is_labelled_and_workspace_is_untouched() {
        let dir = tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        fs::write(
            root.join("main.tex"),
            "\\documentclass{local}\\begin{document}x\\input{nested}\\end{document}",
        )
        .unwrap();
        fs::write(root.join("local.cls"), "\\NeedsTeXFormat{LaTeX2e}").unwrap();
        fs::write(root.join("nested.tex"), "nested").unwrap();
        let before = fs::read(root.join("main.tex")).unwrap();
        let output = root.join("release.zip");
        create_staged_submission_release(&options(root, &output, true)).unwrap();
        assert_eq!(fs::read(root.join("main.tex")).unwrap(), before);
        let mut zip = zip::ZipArchive::new(File::open(output).unwrap()).unwrap();
        let mut manifest = String::new();
        zip.by_name("SIL-RELEASE.json")
            .unwrap()
            .read_to_string(&mut manifest)
            .unwrap();
        assert!(manifest.contains("source-only"));
        assert!(manifest.contains("\"performed\": false"));
        assert!(manifest.contains("nested.tex"));
    }

    #[test]
    fn identical_runs_are_byte_equal_and_missing_dependency_is_hard_failure() {
        let dir = tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        fs::write(root.join("main.tex"), "\\input{missing}").unwrap();
        let error = create_staged_submission_release(&options(root, &root.join("bad.zip"), true))
            .unwrap_err();
        assert!(error.to_string().contains("missing dependencies"));
        fs::write(root.join("missing.tex"), "ok").unwrap();
        let a = root.join("a.zip");
        let b = root.join("b.zip");
        create_staged_submission_release(&options(root, &a, true)).unwrap();
        create_staged_submission_release(&options(root, &b, true)).unwrap();
        assert_eq!(fs::read(a).unwrap(), fs::read(b).unwrap());
    }

    #[test]
    fn compiled_release_requires_pdf_and_records_member_hashes() {
        let dir = tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        fs::write(
            root.join("main.tex"),
            "\\documentclass{article}\\begin{document}x\\end{document}",
        )
        .unwrap();
        fs::write(root.join("paper.pdf"), b"%PDF-1.7 fixture").unwrap();
        let mut release = options(root, &root.join("compiled.zip"), false);
        release.pdf = Some("paper.pdf".into());
        create_staged_submission_release(&release).unwrap();

        let mut zip = zip::ZipArchive::new(File::open(root.join("compiled.zip")).unwrap()).unwrap();
        let mut manifest = String::new();
        zip.by_name("SIL-RELEASE.json")
            .unwrap()
            .read_to_string(&mut manifest)
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&manifest).unwrap();
        let members = value["members"].as_array().unwrap();
        let pdf = members
            .iter()
            .find(|member| member["path"] == "paper.pdf")
            .unwrap();
        assert_eq!(pdf["size"], 16);
        assert_eq!(pdf["sha256"].as_str().unwrap().len(), 64);
        assert_eq!(value["compile"]["succeeded"], true);
    }
}
