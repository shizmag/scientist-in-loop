//! Shared manuscript check use case.

use camino::Utf8Path;
use serde::Serialize;
use serde_json::json;
use sil_core::{
    CheckFinding, CheckProfile, CheckReport, CheckRunMetadata, CheckStaticReport, Config,
    FindingClass, ProjectPaths, serialized_input_fingerprint,
};
use sil_latex::{GraphOptions, build_dependency_graph, build_structured};

use crate::error::AppError;

/// Inputs controlling one manuscript check.
#[derive(Debug, Clone, Copy)]
pub struct ManuscriptCheckOptions {
    /// Profile used for exit policy.
    pub profile: CheckProfile,
    /// Request a compiler run.
    pub build: bool,
    /// Request explicitly online checks.
    pub online: bool,
}

/// Run the shared, read-only manuscript check for a project root.
pub fn run_manuscript_check(
    project_root: &Utf8Path,
    options: ManuscriptCheckOptions,
) -> Result<CheckReport, AppError> {
    let paths = ProjectPaths::new(project_root);
    let config = Config::load(&paths.config()).map_err(|e| AppError::Message(e.to_string()))?;
    let main = project_root.join(&config.latex.main);
    let bib = project_root.join(sil_core::paths::rel::REFERENCES);
    let graph = build_dependency_graph(&GraphOptions {
        project_root: project_root.to_path_buf(),
        main: config.latex.main.clone(),
        bibliography: if bib.is_file() {
            vec![bib.clone()]
        } else {
            Vec::new()
        },
        ..GraphOptions::new(project_root.to_path_buf(), config.latex.main.clone())
    })
    .map_err(|e| AppError::Message(e.to_string()))?;

    let mut findings = graph.report.findings.clone();
    let health = sil_latex::audit_manuscript(&main, bib.is_file().then_some(bib.as_path()))
        .map_err(|e| AppError::Message(e.to_string()))?;
    let bib_cited = health.bib_citation_ratio().0;
    for diagnostic in health.diagnostics {
        let class = match diagnostic.level {
            sil_core::DiagnosticLevel::Error => FindingClass::InvariantError,
            sil_core::DiagnosticLevel::Warning => FindingClass::ActionableWarning,
            sil_core::DiagnosticLevel::Info => FindingClass::Observation,
        };
        findings.push(CheckFinding {
            code: format!("latex.{}", diagnostic.category),
            class,
            path: Some(config.latex.main.to_string()),
            line: diagnostic.line,
            message: diagnostic.message,
            hint: None,
            evidence: json!({}),
        });
    }
    findings.push(CheckFinding {
        code: "metrics.words".into(),
        class: FindingClass::Observation,
        path: Some(config.latex.main.to_string()),
        line: None,
        message: format!("{} words", health.word_count),
        hint: None,
        evidence: json!({"words": health.word_count}),
    });

    let snapshot = InputSnapshot::from_project(project_root, &config, &graph);
    let fingerprint = serialized_input_fingerprint(&snapshot)
        .map_err(|e| AppError::Message(format!("failed to fingerprint check inputs: {e}")))?;
    let mut static_report = CheckStaticReport::new(options.profile, fingerprint, findings);
    static_report.dependencies = graph.dependencies.iter().map(|d| d.path.clone()).collect();
    static_report
        .metrics
        .insert("words".into(), json!(health.word_count));
    static_report
        .metrics
        .insert("bib_keys".into(), json!(health.total_bib_keys_count));
    static_report
        .metrics
        .insert("bib_cited".into(), json!(bib_cited));
    static_report.template = Some(json!({"name": config.latex.template}));

    let build = if options.build {
        Some(
            serde_json::to_value(build_structured(
                config.latex.engine,
                &config.latex.main,
                project_root,
            ))
            .map_err(|e| AppError::Message(e.to_string()))?,
        )
    } else {
        None
    };
    let online = options
        .online
        .then(|| json!({"requested": true, "status": "not_configured"}));
    let report = CheckReport {
        r#static: static_report,
        run: CheckRunMetadata {
            checked_at: now_string(),
            build,
            online,
        },
    };
    persist_report(project_root, &report)?;
    Ok(report)
}

/// Load the last canonical report without executing checkers or changing project state.
pub fn load_cached_report(project_root: &Utf8Path) -> Result<Option<CheckReport>, AppError> {
    let path = project_root.join(".sil/checks/latest.json");
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = std::fs::read(path.as_std_path()).map_err(|source| AppError::Io {
        path: path.to_string(),
        source,
    })?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| AppError::Message(format!("failed to read cached check report: {e}")))
}

#[derive(Serialize)]
struct InputSnapshot {
    config: String,
    files: Vec<(String, Vec<u8>)>,
    dependencies: Vec<String>,
}

impl InputSnapshot {
    fn from_project(
        root: &Utf8Path,
        config: &Config,
        graph: &sil_latex::DependencySnapshot,
    ) -> Self {
        let mut dependencies = graph
            .dependencies
            .iter()
            .map(|d| d.path.clone())
            .collect::<Vec<_>>();
        dependencies.sort();
        let mut files = dependencies
            .iter()
            .filter_map(|path| {
                let path = root.join(path);
                std::fs::read(path.as_std_path()).ok().map(|bytes| {
                    (
                        path.strip_prefix(root)
                            .unwrap_or(path.as_path())
                            .to_string(),
                        bytes,
                    )
                })
            })
            .collect::<Vec<_>>();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        Self {
            config: serde_yaml::to_string(config).unwrap_or_default(),
            files,
            dependencies,
        }
    }
}

fn now_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

fn persist_report(root: &Utf8Path, report: &CheckReport) -> Result<(), AppError> {
    let dir = root.join(".sil/checks");
    std::fs::create_dir_all(dir.as_std_path()).map_err(|source| AppError::Io {
        path: dir.to_string(),
        source,
    })?;
    let path = dir.join("latest.json");
    let bytes = serde_json::to_vec_pretty(report).map_err(|e| AppError::Message(e.to_string()))?;
    std::fs::write(path.as_std_path(), bytes).map_err(|source| AppError::Io {
        path: path.to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sil_core::{CheckFinding, FindingClass};
    use std::fs;

    #[test]
    fn cached_report_round_trips_without_running_checkers() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let report = CheckReport {
            r#static: CheckStaticReport::new(
                CheckProfile::Draft,
                "sha256:fixture",
                vec![CheckFinding {
                    code: "fixture.warning".into(),
                    class: FindingClass::ActionableWarning,
                    path: None,
                    line: None,
                    message: "fixture".into(),
                    hint: None,
                    evidence: serde_json::json!({}),
                }],
            ),
            run: CheckRunMetadata::default(),
        };
        persist_report(root, &report).unwrap();
        assert_eq!(load_cached_report(root).unwrap(), Some(report));
    }

    #[test]
    fn offline_fixture_treats_scientific_output_changes_as_nonblocking() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        fs::create_dir_all(root.join(".sil")).unwrap();
        fs::create_dir_all(root.join("sections")).unwrap();
        fs::create_dir_all(root.join("figures")).unwrap();
        fs::write(
            root.join(".sil/config.yaml"),
            include_str!("../../../tests/fixtures/pr-v/config.yaml"),
        )
        .unwrap();
        fs::write(
            root.join("paper_draft.tex"),
            include_str!("../../../tests/fixtures/pr-v/paper_draft.tex"),
        )
        .unwrap();
        fs::write(
            root.join("article.cls"),
            include_str!("../../../tests/fixtures/pr-v/article.cls"),
        )
        .unwrap();
        fs::write(
            root.join("sections/methods.tex"),
            include_str!("../../../tests/fixtures/pr-v/sections/methods.tex"),
        )
        .unwrap();
        fs::write(
            root.join("references.bib"),
            include_str!("../../../tests/fixtures/pr-v/references.bib"),
        )
        .unwrap();
        fs::write(
            root.join("figures/plot.png"),
            include_bytes!("../../../tests/fixtures/pr-v/figures/plot.png"),
        )
        .unwrap();

        let first = run_manuscript_check(
            root,
            ManuscriptCheckOptions {
                profile: CheckProfile::Draft,
                build: false,
                online: false,
            },
        )
        .unwrap();
        assert!(first.passes(&[]));
        fs::write(root.join("figures/plot.png"), b"changed plot bytes").unwrap();
        let second = run_manuscript_check(
            root,
            ManuscriptCheckOptions {
                profile: CheckProfile::Draft,
                build: false,
                online: false,
            },
        )
        .unwrap();
        assert!(second.passes(&[]));
        assert_ne!(
            first.r#static.input_fingerprint,
            second.r#static.input_fingerprint
        );
        assert!(
            second
                .r#static
                .findings
                .iter()
                .all(|finding| finding.code != "baseline.changed")
        );
    }
}
