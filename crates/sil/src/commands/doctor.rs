use anyhow::Result;
#[allow(unused_imports)]
pub use sil_app::doctor::{
    Check, DatabaseRepairReport, DoctorReport, SourceRepairOutcome, latex_engine_hint,
    repair_sqlite_database, run_host_checks, tool_hint, which_ok,
};
use sil_core::SilUi;

use crate::util::load_project;

/// Arguments for the doctor command.
#[derive(Debug, Clone, Default)]
pub struct DoctorArgs {
    /// Machine-readable JSON output
    pub json: bool,
    /// Provide ONNX model cache bootstrap directories and export recipe
    pub fix_rag: bool,
    /// Automatically repair mismatched or corrupted references.bib entries with official BibTeX
    pub fix: bool,
    /// Rebuild SQLite database from on-disk sources when corrupt
    pub repair_db: bool,
}

/// Run environment + project diagnostics.
pub fn run(args: DoctorArgs, ui: &dyn SilUi) -> Result<()> {
    if args.repair_db {
        let (root, _config, _paths) = load_project()?;
        let report = sil_app::repair_sqlite_database(root.as_std_path(), ui)?;
        if args.json {
            println!("{}", serde_json::to_string_pretty(&report)?);
            return Ok(());
        }

        ui.println("");
        ui.info("Database Repair Report");
        if let Some(ref backup) = report.backup_path {
            ui.success(&format!("  ✓ Backed up corrupt database to: {backup}"));
        }
        ui.success("  ✓ Fresh database initialized");
        ui.info(&format!(
            "  • Sources scanned: {}\n  • Sources re-parsed: {}\n  • Sources failed: {}",
            report.sources_scanned, report.sources_reparsed, report.sources_failed
        ));
        for outcome in &report.outcomes {
            if outcome.ok {
                ui.success(&format!("    ✓ {}", outcome.filename));
            } else {
                let err = outcome.error.as_deref().unwrap_or("unknown error");
                ui.warn(&format!("    ✖ {} ({err})", outcome.filename));
            }
        }
        ui.println("");
        if report.sources_failed == 0 {
            ui.success("Database repair completed successfully.");
        } else {
            ui.warn("Database repair completed with some source parsing failures.");
        }
        return Ok(());
    }

    let json = args.json;
    let fix = args.fix;

    if args.fix_rag
        && let Some(cache_dir) = dirs::cache_dir()
    {
        let base = cache_dir.join("sil/models");
        let embed_dir = base.join("bge-small-en-v1.5");
        let rerank_dir = base.join("ms-marco-MiniLM-L-6-v2");

        let _ = std::fs::create_dir_all(&embed_dir);
        let _ = std::fs::create_dir_all(&rerank_dir);

        ui.success(&format!(
            "🔧 ONNX Model Cache Directories Initialized:\n  • Embedder: {}\n  • Reranker: {}\n",
            embed_dir.display(),
            rerank_dir.display()
        ));

        ui.info(
            "Export ONNX models + tokenizer.json using optimum-cli:\n\
             $ pip install optimum[onnxruntime]\n\
             $ optimum-cli export onnx --model BAAI/bge-small-en-v1.5 ~/.cache/sil/models/bge-small-en-v1.5/\n\
             $ optimum-cli export onnx --model cross-encoder/ms-marco-MiniLM-L-6-v2 ~/.cache/sil/models/ms-marco-MiniLM-L-6-v2/\n\
             \n\
             Build sil with cargo features: cargo build -p sil --features onnx\n",
        );
    }

    let mut checks = Vec::new();
    let mut project_root = None;
    let mut manuscript_report = None;

    // Always: git, python, uv (optional), cargo (optional), latex engines, marker (optional)
    checks.extend(run_host_checks());

    match load_project() {
        Ok((root, config, paths)) => {
            project_root = Some(root.to_string());
            manuscript_report = sil_app::run_manuscript_check(
                &root,
                sil_app::ManuscriptCheckOptions {
                    profile: sil_core::CheckProfile::Draft,
                    build: false,
                    online: false,
                },
            )
            .ok();
            checks.push(Check::simple("sil project", true, format!("root={root}")));
            let cfg_ok = paths.config().is_file();
            checks.push(Check::with_hint(
                "config.yaml",
                cfg_ok,
                paths.config().to_string(),
                if cfg_ok {
                    None
                } else {
                    Some("Run `sil init` or restore .sil/config.yaml".to_string())
                },
            ));
            let struct_ok = paths.structure().is_file();
            checks.push(Check::with_hint(
                "structure.yaml",
                struct_ok,
                paths.structure().to_string(),
                if struct_ok {
                    None
                } else {
                    Some("Run `sil init` or restore .sil/structure.yaml".to_string())
                },
            ));
            let draft_ok = paths.paper_draft().is_file();
            checks.push(Check::with_hint(
                "paper_draft.tex",
                draft_ok,
                paths.paper_draft().to_string(),
                if draft_ok {
                    None
                } else {
                    Some("Create paper_draft.tex or run `sil init`".to_string())
                },
            ));
            let sec_ok = paths.draft_sections_dir().is_dir();
            checks.push(Check::with_hint(
                "draft_sections",
                sec_ok,
                paths.draft_sections_dir().to_string(),
                if sec_ok {
                    None
                } else {
                    Some("Run `sil split` or `sil init` to scaffold draft sections".to_string())
                },
            ));
            let imp_ok = paths.improvement_dir().is_dir();
            checks.push(Check::with_hint(
                "improvement",
                imp_ok,
                paths.improvement_dir().to_string(),
                if imp_ok {
                    None
                } else {
                    Some("Create .sil/improvement directory or run `sil init`".to_string())
                },
            ));
            let sources_path = paths.sources(&config);
            let sources_ok = sources_path.is_dir();
            checks.push(Check::with_hint(
                "sources",
                sources_ok,
                sources_path.to_string(),
                if sources_ok {
                    None
                } else {
                    Some("Create `sources/` directory or run `sil init`".to_string())
                },
            ));
            let (db_open_ok, integrity_res) = match sil_db::SilDb::open(&paths.db()) {
                Ok(db) => match db.integrity_check() {
                    Ok(res) => (true, Ok(res)),
                    Err(e) => (true, Err(e.to_string())),
                },
                Err(e) => (false, Err(e.to_string())),
            };
            checks.push(Check::with_hint(
                "sqlite db openable",
                db_open_ok,
                paths.db().to_string(),
                if db_open_ok {
                    None
                } else {
                    Some(
                        "Ensure .sil/db.sqlite is readable and not locked by another process"
                            .to_string(),
                    )
                },
            ));
            let (integrity_ok, integrity_detail) = match integrity_res {
                Ok(ref res) if res == "ok" => (true, "ok".to_string()),
                Ok(ref res) => (false, format!("integrity check: {res}")),
                Err(ref err) => (false, format!("integrity check failed: {err}")),
            };
            checks.push(Check::with_hint(
                "sqlite integrity",
                integrity_ok,
                integrity_detail,
                if integrity_ok {
                    None
                } else {
                    Some("Database integrity failed. Backup db.sqlite before repair. Do not delete sources/".to_string())
                },
            ));
            checks.push(Check::simple(
                "configured latex engine",
                true,
                format!("{}", config.latex.engine),
            ));
            let engine = config.latex.engine.to_string();
            let eng_ok = which_ok(&engine);
            checks.push(Check::with_hint(
                format!("engine '{engine}' on PATH"),
                eng_ok,
                if eng_ok {
                    "found".to_string()
                } else {
                    "not found (build may fail)".to_string()
                },
                if eng_ok {
                    None
                } else {
                    Some(latex_engine_hint(&engine))
                },
            ));

            // Dense RAG honesty (feature onnx + models under ~/.cache/sil/models)
            checks.push(dense_rag_check(&config));

            // Manuscript Health & Quality Audit
            let bib_path = root.join("references.bib");
            let bib_opt = if bib_path.exists() {
                Some(bib_path.as_path())
            } else {
                None
            };
            match sil_latex::audit_manuscript(&paths.paper_draft(), bib_opt) {
                Ok(report) => {
                    let missing = report.missing_citations_count;
                    let missing_ok = missing == 0;
                    checks.push(Check::with_hint(
                        "manuscript health: citations",
                        missing_ok,
                        if missing_ok {
                            "all cite keys resolved".to_string()
                        } else {
                            format!("{missing} missing citation key(s) in references.bib")
                        },
                        if missing_ok {
                            None
                        } else {
                            Some("Add missing citation keys to references.bib or fetch with `sil paper fetch`".to_string())
                        },
                    ));

                    let (cited, total) = report.bib_citation_ratio();
                    let bib_cov_ok = total == 0 || cited == total;
                    checks.push(Check::with_hint(
                        "manuscript health: bib coverage",
                        bib_cov_ok,
                        if total == 0 {
                            "0 references in references.bib".to_string()
                        } else if cited == total {
                            format!("{cited}/{total} references mentioned in paper_*.tex")
                        } else {
                            format!(
                                "{cited}/{total} references mentioned in paper_*.tex ({} unmentioned)",
                                total - cited
                            )
                        },
                        if bib_cov_ok {
                            None
                        } else {
                            Some("Reference uncited bibliography items in manuscript or clean up references.bib".to_string())
                        },
                    ));

                    let unref = report.unreferenced_labels_count;
                    checks.push(Check::simple(
                        "manuscript health: labels",
                        true, // warning soft
                        if unref == 0 {
                            "all labels referenced".to_string()
                        } else {
                            format!("{unref} unreferenced label(s)")
                        },
                    ));

                    checks.push(Check::simple(
                        "manuscript health: word count",
                        true,
                        format!("{} words in paper_draft.tex", report.word_count),
                    ));

                    checks.push(Check::simple(
                        "manuscript health: # -- X -- # ideas",
                        true,
                        format!("{} active idea/TODO block(s)", report.todo_ideas_count),
                    ));

                    if bib_path.exists()
                        && let Ok(db) = sil_db::SilDb::open(&paths.db())
                        && let Ok(bib_content) = std::fs::read_to_string(bib_path.as_path())
                    {
                        match sil_parse::checkers::run_all_checkers_incremental(
                            &db,
                            &bib_content,
                            fix,
                        ) {
                            Ok(rep) => {
                                if fix
                                    && rep.autofixed_count > 0
                                    && let Some(ref updated) = rep.updated_bib_content
                                {
                                    let _ = sil_core::write_atomic_str(&bib_path, updated);
                                    ui.success(&format!(
                                        "🔧 Autofixed {} reference entry(ies) in references.bib",
                                        rep.autofixed_count
                                    ));
                                }

                                let broken = rep.broken_identifiers.len();
                                let mismatched = rep.mismatched_identifiers.len();
                                let ok = broken == 0 && mismatched == 0;
                                let detail = if ok {
                                    format!(
                                        "all {} verified identifier(s) valid ({} checked online, {} cached)",
                                        rep.entries_with_identifier,
                                        rep.checked_online,
                                        rep.skipped_cached
                                    )
                                } else {
                                    let mut parts = Vec::new();
                                    if mismatched > 0 {
                                        let m_list: Vec<String> = rep
                                            .mismatched_identifiers
                                            .iter()
                                            .map(|(k, id_type, loc, off, sim)| {
                                                format!("{k} ({id_type} title mismatch: '{loc}' vs '{off}', sim {sim:.2})")
                                            })
                                            .collect();
                                        parts.push(format!(
                                            "{mismatched} title mismatch(es) [{}]",
                                            m_list.join("; ")
                                        ));
                                    }
                                    if broken > 0 {
                                        let b_list: Vec<String> = rep
                                            .broken_identifiers
                                            .iter()
                                            .map(|(k, id_type, id)| format!("{k} ({id_type} {id})"))
                                            .collect();
                                        parts.push(format!(
                                            "{broken} broken identifier(s) [{}]",
                                            b_list.join("; ")
                                        ));
                                    }
                                    format!("references.bib issues: {}", parts.join("; "))
                                };
                                checks.push(Check::with_hint(
                                    "manuscript health: bib identifiers",
                                    ok,
                                    detail,
                                    if ok {
                                        None
                                    } else {
                                        Some("Fix broken or mismatched identifiers in references.bib or run `sil project doctor --fix`".to_string())
                                    },
                                ));
                            }
                            Err(e) => {
                                checks.push(Check::simple(
                                    "manuscript health: bib identifiers",
                                    true,
                                    format!("Reference check skipped: {e}"),
                                ));
                            }
                        }
                    }
                }
                Err(e) => {
                    checks.push(Check::with_hint(
                        "manuscript health audit",
                        false,
                        format!("audit failed: {e}"),
                        Some(
                            "Check paper_draft.tex and references.bib for syntax errors"
                                .to_string(),
                        ),
                    ));
                }
            }
        }
        Err(e) => {
            checks.push(Check::with_hint(
                "sil project",
                false,
                format!("not inside a project: {e}"),
                Some(
                    "Run `sil init` to initialize a project or open an existing project"
                        .to_string(),
                ),
            ));
            // Still report dense RAG even outside a project (uses global defaults).
            checks.push(dense_rag_check_from_settings(
                &sil_core::GlobalSettings::load_or_default(None).rag,
            ));
        }
    }

    let ok = checks.iter().all(|c| c.ok || is_soft(&c.name));
    // Soft checks: optional latex engines other than configured one
    let report = DoctorReport {
        project: project_root,
        ok: checks.iter().filter(|c| !is_soft(&c.name)).all(|c| c.ok),
        checks,
        check: manuscript_report,
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
            if let Some(ref hint) = c.hint {
                ui.muted(&format!("    ↳ Hint: {hint}"));
            }
        } else {
            ui.warn(&format!("  ✖ {}: {}", c.name, c.detail));
            if let Some(ref hint) = c.hint {
                ui.warn(&format!("    ↳ Hint: {hint}"));
            }
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
    name == "tectonic"
        || name == "pdflatex"
        || name == "latexmk"
        || name == "uv"
        || name == "marker"
        || name == "dense_rag"
        || name.starts_with("engine ")
        || name == "manuscript health: bib coverage"
        || name == "manuscript health: labels"
        || name == "manuscript health: word count"
        || name == "manuscript health: # -- X -- # ideas"
}

fn dense_rag_check(config: &sil_core::Config) -> Check {
    let rag = config
        .rag
        .clone()
        .unwrap_or_else(|| sil_core::GlobalSettings::load_or_default(None).rag);
    dense_rag_check_from_settings(&rag)
}

fn dense_rag_check_from_settings(rag: &sil_core::RagSettings) -> Check {
    use sil_db::{OnnxEmbedder, OnnxReranker, RagBackend};

    let embedder = OnnxEmbedder::from_rag_settings(rag);
    let reranker = OnnxReranker::from_rag_settings(rag);
    let emb_backend = embedder.backend();
    let rerank_backend = reranker.backend();

    let embedder_path = rag.resolve_embedder_path().map(|p| p.to_string());
    let reranker_path = rag.resolve_reranker_path().map(|p| p.to_string());

    let (mode, reason, dim, tokenizer_ok) = match &emb_backend {
        RagBackend::Onnx { dim } => ("onnx".to_string(), None::<String>, *dim, true),
        RagBackend::Fallback { reason } => (
            "fallback".to_string(),
            Some(reason.as_str().to_string()),
            embedder.dimension(),
            false,
        ),
    };

    // Intentional fallback (feature off / models optional) is not a hard failure.
    // Soft-fail when paths were configured but session/tokenizer load failed.
    let configured_but_broken = embedder_path.is_some()
        && matches!(
            emb_backend,
            RagBackend::Fallback {
                reason: sil_db::RagFallbackReason::SessionLoadFailed
                    | sil_db::RagFallbackReason::MissingTokenizer
            }
        );

    let ok = !configured_but_broken;
    let detail = match &emb_backend {
        RagBackend::Onnx { dim } => {
            format!(
                "onnx (dim={dim}); embedder={}; reranker={}",
                emb_backend.summary(),
                rerank_backend.summary()
            )
        }
        RagBackend::Fallback { reason } => {
            format!(
                "fallback (hash); reason={}; dim={}; models under ~/.cache/sil/models (export HF ONNX + tokenizer.json); build with --features onnx for real dense RAG",
                reason.as_str(),
                embedder.dimension()
            )
        }
    };

    let hint = if configured_but_broken {
        Some("Ensure ONNX model files (model.onnx and tokenizer.json) exist at configured paths or run `sil project doctor --fix-rag`".to_string())
    } else {
        None
    };

    Check {
        name: "dense_rag".into(),
        ok,
        detail,
        hint,
        extra: Some(serde_json::json!({
            "mode": mode,
            "reason": reason,
            "dim": dim,
            "embedder_path": embedder_path,
            "reranker_path": reranker_path,
            "tokenizer_ok": tokenizer_ok,
            "embedder": emb_backend.summary(),
            "reranker": rerank_backend.summary(),
            "model_cache_hint": "~/.cache/sil/models/<name>/model.onnx + tokenizer.json",
        })),
    }
}
