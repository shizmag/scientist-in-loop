use super::*;
use sil_core::{ProjectPaths, ReferenceEntry, SourceDocument};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum PersistedJobStatus {
    Running,
    Ok,
    Fail,
    Stale,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PersistedJob {
    id: u64,
    kind: String,
    label: String,
    status: PersistedJobStatus,
    started: u64,
    ended: Option<u64>,
    error_code: Option<String>,
}

fn job_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl App {
    fn jobs_path(&self) -> Option<camino::Utf8PathBuf> {
        self.project_root
            .as_ref()
            .map(|root| ProjectPaths::new(root).sil_dir().join("jobs.json"))
    }

    fn save_persisted_jobs(&self, jobs: &[PersistedJob]) {
        let Some(path) = self.jobs_path() else { return };
        if let Ok(text) = serde_json::to_string_pretty(jobs) {
            let _ = sil_core::write_atomic_str(&path, &text);
        }
    }

    fn read_persisted_jobs_raw(&self) -> Vec<PersistedJob> {
        let Some(path) = self.jobs_path() else {
            return Vec::new();
        };
        let text = match std::fs::read_to_string(path.as_std_path()) {
            Ok(text) => text,
            Err(_) => return Vec::new(),
        };
        serde_json::from_str::<Vec<PersistedJob>>(&text).unwrap_or_default()
    }

    fn read_persisted_jobs(&mut self) -> Vec<PersistedJob> {
        let Some(path) = self.jobs_path() else {
            return Vec::new();
        };
        let text = match std::fs::read_to_string(path.as_std_path()) {
            Ok(text) => text,
            Err(_) => return Vec::new(),
        };
        match serde_json::from_str::<Vec<PersistedJob>>(&text) {
            Ok(mut jobs) => {
                if jobs.len() > PERSISTED_JOB_CAP {
                    jobs.drain(..jobs.len() - PERSISTED_JOB_CAP);
                }
                let mut changed = false;
                for job in &mut jobs {
                    if job.status == PersistedJobStatus::Running {
                        job.status = PersistedJobStatus::Stale;
                        job.ended = Some(job_now());
                        changed = true;
                    }
                }
                if changed {
                    self.save_persisted_jobs(&jobs);
                }
                jobs
            }
            Err(error) => {
                self.status_message = format!("Warning: invalid .sil/jobs.json ({error})");
                self.last_user_error = Some(sil_core::UserError::new(
                    "jobs.invalid_json",
                    "Job history could not be loaded",
                    error.to_string(),
                    None,
                ));
                Vec::new()
            }
        }
    }

    pub(crate) fn load_persisted_jobs(&mut self) {
        let jobs = self.read_persisted_jobs();
        self.next_job_id = jobs
            .iter()
            .map(|job| job.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        self.recent_job_outcomes.clear();
        for job in jobs {
            let kind = match job.kind.as_str() {
                "fetch" => JobKind::Fetch,
                "parse" => JobKind::Parse,
                "digest" => JobKind::Digest,
                "estimate" => JobKind::Estimate,
                "build" => JobKind::Build,
                "hydrate" => JobKind::Hydrate,
                "similarity" => JobKind::Similarity,
                _ => continue,
            };
            let stale = job.status == PersistedJobStatus::Stale;
            self.push_job_outcome_inner(
                JobOutcome {
                    id: job.id,
                    kind,
                    label: job.label.clone(),
                    ok: job.status == PersistedJobStatus::Ok,
                    detail: job.error_code.unwrap_or_else(|| {
                        format!(
                            "{} job {}",
                            job.kind,
                            if stale { "stale" } else { "completed" }
                        )
                    }),
                    duration_ms: None,
                    retry_payload: match kind {
                        JobKind::Fetch => Some(RetryPayload::Fetch {
                            target: job.label.clone(),
                        }),
                        JobKind::Similarity => Some(RetryPayload::Similarity),
                        _ => None,
                    },
                },
                false,
            );
        }
    }

    fn start_persisted_job(&mut self, kind: JobKind, label: &str) {
        let mut jobs = self.read_persisted_jobs_raw();
        jobs.push(PersistedJob {
            id: self.alloc_job_id(),
            kind: kind.label().to_string(),
            label: label.to_string(),
            status: PersistedJobStatus::Running,
            started: job_now(),
            ended: None,
            error_code: None,
        });
        if jobs.len() > PERSISTED_JOB_CAP {
            jobs.drain(..jobs.len() - PERSISTED_JOB_CAP);
        }
        self.save_persisted_jobs(&jobs);
    }
}

pub(crate) fn extract_panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        format!("worker panicked: {s}")
    } else if let Some(s) = payload.downcast_ref::<String>() {
        format!("worker panicked: {s}")
    } else {
        "worker panicked: unknown reason".to_string()
    }
}

impl App {
    /// Start the configured draft build outside the UI thread.
    pub fn run_build_job(&mut self) {
        let Some(root) = self.project_root.clone() else {
            self.status_message = "No active project loaded".to_string();
            return;
        };
        if self.in_flight_build {
            self.status_message = "already building draft...".to_string();
            return;
        }
        let Some(config) = self.loaded_config.clone() else {
            self.status_message = "Build error: project configuration is unavailable".to_string();
            return;
        };
        self.in_flight_build = true;
        self.start_persisted_job(JobKind::Build, "draft build");
        self.active_tab = ActiveTab::PaperDraft;
        self.status_message = "⏳ Building draft PDF...".to_string();
        let tx = self.build_tx.clone();
        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let result = catch_unwind(AssertUnwindSafe(|| {
                sil_latex::build(config.latex.engine, &config.latex.main, &root)
                    .map_err(|e| e.to_string())
            }));
            let result = match result {
                Ok(result) => result,
                Err(p) => Err(extract_panic_message(p)),
            };
            let log = result.as_ref().err().cloned().unwrap_or_default();
            let _ = tx.send(BuildJobResult {
                result,
                log,
                duration_ms: Some(started.elapsed().as_millis() as u64),
            });
        });
    }

    pub fn poll_background_build(&mut self) {
        while let Ok(res) = self.build_rx.try_recv() {
            self.in_flight_build = false;
            let id = self.alloc_job_id();
            match res.result {
                Ok(pdf) => {
                    self.status_message = format!("✓ Built draft PDF: {pdf}");
                    self.push_job_outcome(JobOutcome {
                        id,
                        kind: JobKind::Build,
                        label: "draft build".to_string(),
                        ok: true,
                        detail: format!("PDF: {pdf}"),
                        duration_ms: res.duration_ms,
                        retry_payload: None,
                    });
                }
                Err(err) => {
                    let user_err = sil_core::UserError::classify(&err);
                    self.status_message = format!("⚠ Draft build failed: {}", user_err.title);
                    self.last_user_error = Some(user_err);
                    if let Some((file, line)) = parse_latex_error_location(&res.log) {
                        self.jump_to_draft_line(&file, line);
                    }
                    self.push_job_outcome(JobOutcome {
                        id,
                        kind: JobKind::Build,
                        label: "draft build".to_string(),
                        ok: false,
                        detail: err,
                        duration_ms: res.duration_ms,
                        retry_payload: None,
                    });
                }
            }
        }
    }

    pub(crate) fn alloc_job_id(&mut self) -> u64 {
        let id = self.next_job_id;
        self.next_job_id = self.next_job_id.saturating_add(1);
        id
    }

    fn push_job_outcome_inner(&mut self, outcome: JobOutcome, persist: bool) {
        if self.recent_job_outcomes.len() >= JOB_HISTORY_CAP {
            self.recent_job_outcomes.pop_front();
        }
        self.recent_job_outcomes.push_back(outcome);
        if self.selected_job_history_index >= self.recent_job_outcomes.len()
            && !self.recent_job_outcomes.is_empty()
        {
            self.selected_job_history_index = self.recent_job_outcomes.len() - 1;
        }
        if persist {
            self.persist_completion();
        }
    }

    pub(crate) fn push_job_outcome(&mut self, outcome: JobOutcome) {
        self.push_job_outcome_inner(outcome, true);
    }

    fn persist_completion(&self) {
        let Some(path) = self.jobs_path() else { return };
        let Ok(text) = std::fs::read_to_string(path.as_std_path()) else {
            return;
        };
        let Ok(mut jobs) = serde_json::from_str::<Vec<PersistedJob>>(&text) else {
            return;
        };
        let outcome = self.recent_job_outcomes.back().unwrap();
        let kind = outcome.kind.label();
        if let Some(job) = jobs.iter_mut().rev().find(|job| {
            job.status == PersistedJobStatus::Running
                && job.kind == kind
                && job.label == outcome.label
        }) {
            job.status = if outcome.ok {
                PersistedJobStatus::Ok
            } else {
                PersistedJobStatus::Fail
            };
            job.ended = Some(job_now());
            job.error_code = (!outcome.ok).then(|| {
                sil_core::UserError::classify(&outcome.detail)
                    .code
                    .to_string()
            });
        } else {
            jobs.push(PersistedJob {
                id: outcome.id,
                kind: kind.to_string(),
                label: outcome.label.clone(),
                status: if outcome.ok {
                    PersistedJobStatus::Ok
                } else {
                    PersistedJobStatus::Fail
                },
                started: job_now(),
                ended: Some(job_now()),
                error_code: (!outcome.ok).then(|| {
                    sil_core::UserError::classify(&outcome.detail)
                        .code
                        .to_string()
                }),
            });
        }
        if jobs.len() > PERSISTED_JOB_CAP {
            jobs.drain(..jobs.len() - PERSISTED_JOB_CAP);
        }
        self.save_persisted_jobs(&jobs);
    }

    /// Effective RAG settings: local config override, else global.
    pub fn effective_rag_settings(&self) -> sil_core::RagSettings {
        self.loaded_config
            .as_ref()
            .and_then(|c| c.rag.clone())
            .unwrap_or_else(|| self.global_settings.rag.clone())
    }

    pub fn queue_ref_hydration(&mut self, entry: ReferenceEntry) {
        let label = entry
            .title
            .as_deref()
            .unwrap_or(&entry.raw_text)
            .to_string();
        let dedup_key = if let Some(ref doi) = entry.doi {
            format!("doi:{}", doi.trim())
        } else if let Some(ref arxiv_id) = entry.arxiv_id {
            format!("arxiv:{}", arxiv_id.trim())
        } else {
            format!("ref_id:{}", entry.id)
        };

        if self.in_flight_hydration_keys.contains(&dedup_key) {
            self.status_message = format!("already hydrating '{label}'...");
            return;
        }

        if self.in_flight_hydration_keys.is_empty() {
            self.hydration_batch_succeeded = 0;
            self.hydration_batch_failed = 0;
        }

        self.in_flight_hydration_keys.insert(dedup_key.clone());
        self.start_persisted_job(JobKind::Hydrate, &label);
        self.hydrate_retry_payloads.insert(
            dedup_key.clone(),
            RetryPayload::HydrateRef {
                entry: entry.clone(),
            },
        );
        self.status_message = format!(
            "⏳ Hydrating ({} in flight)...",
            self.in_flight_hydration_keys.len()
        );
        let tx = self.hydration_tx.clone();

        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let catch_res = catch_unwind(AssertUnwindSafe(|| {
                sil_parse::journal_digest::resolve_official_bibtex_entry(&entry)
            }));
            let outcome = match catch_res {
                Ok(sil_parse::journal_digest::ReferenceBibResolution::Resolved(official_bib)) => {
                    HydrationOutcome::Success { official_bib }
                }
                Ok(sil_parse::journal_digest::ReferenceBibResolution::Failed(reason)) => {
                    HydrationOutcome::Failure { reason }
                }
                Err(p) => HydrationOutcome::Failure {
                    reason: extract_panic_message(p),
                },
            };
            let _ = tx.send(HydrationResult {
                dedup_key,
                label,
                outcome,
                duration_ms: Some(started.elapsed().as_millis() as u64),
            });
        });
    }

    pub fn queue_source_hydration(&mut self, doc: SourceDocument) {
        let label = doc.title.as_deref().unwrap_or(&doc.filename).to_string();
        let arxiv_candidate = doc
            .doi
            .as_deref()
            .and_then(sil_regex::extract_arxiv_id)
            .or_else(|| sil_regex::extract_arxiv_id(&doc.filename))
            .or_else(|| doc.title.as_deref().and_then(sil_regex::extract_arxiv_id));

        let dedup_key = if let Some(doi) = doc.doi.as_ref().filter(|s| !s.trim().is_empty()) {
            format!("doi:{}", doi.trim())
        } else if let Some(ref arxiv) = arxiv_candidate {
            let clean = arxiv
                .trim_start_matches("arxiv:")
                .trim_start_matches("arXiv:")
                .trim_start_matches("ARXIV:")
                .trim();
            format!("arxiv:{clean}")
        } else {
            format!("source_id:{}", doc.id)
        };

        if self.in_flight_hydration_keys.contains(&dedup_key) {
            self.status_message = format!("already hydrating '{label}'...");
            return;
        }

        if self.in_flight_hydration_keys.is_empty() {
            self.hydration_batch_succeeded = 0;
            self.hydration_batch_failed = 0;
        }

        self.in_flight_hydration_keys.insert(dedup_key.clone());
        self.start_persisted_job(JobKind::Hydrate, &label);
        self.hydrate_retry_payloads.insert(
            dedup_key.clone(),
            RetryPayload::HydrateSource { doc: doc.clone() },
        );
        self.status_message = format!(
            "⏳ Hydrating ({} in flight)...",
            self.in_flight_hydration_keys.len()
        );
        let tx = self.hydration_tx.clone();

        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let catch_res = catch_unwind(AssertUnwindSafe(|| {
                sil_parse::journal_digest::resolve_official_bibtex_for_source(&doc)
            }));
            let outcome = match catch_res {
                Ok(sil_parse::SourceBibResolution::Resolved(official_bib)) => {
                    HydrationOutcome::Success { official_bib }
                }
                Ok(sil_parse::SourceBibResolution::Failed(reason)) => {
                    HydrationOutcome::Failure { reason }
                }
                Err(p) => HydrationOutcome::Failure {
                    reason: extract_panic_message(p),
                },
            };
            let _ = tx.send(HydrationResult {
                dedup_key,
                label,
                outcome,
                duration_ms: Some(started.elapsed().as_millis() as u64),
            });
        });
    }

    pub fn queue_source_parse(&mut self, doc: SourceDocument, force: bool) {
        let label = doc.title.as_deref().unwrap_or(&doc.filename).to_string();

        let is_already_parsed =
            doc.parsed || matches!(doc.status, Some(sil_core::DocumentStatus::AlreadyParsed));
        if is_already_parsed && !force {
            self.status_message =
                "ℹ Source is already parsed (use 'E' / Shift+E to re-parse)".to_string();
            return;
        }

        if self.in_flight_parse_ids.contains(&doc.id) {
            self.status_message = format!("already parsing '{label}'...");
            return;
        }

        self.in_flight_parse_ids.insert(doc.id.clone());
        self.start_persisted_job(JobKind::Parse, &label);
        self.parse_retry_payloads.insert(
            doc.id.clone(),
            RetryPayload::Parse {
                doc: doc.clone(),
                force,
            },
        );
        self.status_message = format!("⏳ Parsing source '{label}'...");

        let tx = self.parse_tx.clone();
        let project_root = self.project_root.clone();
        let doc_id = doc.id.clone();
        let path = doc.path.clone();

        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let catch_res = catch_unwind(AssertUnwindSafe(|| {
                (|| -> Result<sil_parse::batch::ParseResult, String> {
                    let Some(root) = project_root else {
                        return Err("No project root directory available".to_string());
                    };
                    let paths = ProjectPaths::new(&root);
                    let db = sil_db::SilDb::open(&paths.db())
                        .map_err(|e| format!("Database error: {e}"))?;

                    let runner = sil_parse::discover_marker_runner().unwrap_or_else(|_| {
                        Box::new(sil_parse::StubMarkerRunner {
                            content: String::new(),
                        })
                    });
                    let null_ui = sil_core::NullUi::new();
                    let opts = sil_parse::ParseOptions {
                        allow_reparse: force,
                    };

                    sil_parse::parse_one_with_options(&path, &db, runner.as_ref(), &null_ui, opts)
                        .map_err(|e| e.to_string())
                })()
            }));

            let result = match catch_res {
                Ok(res) => res,
                Err(p) => Err(extract_panic_message(p)),
            };

            let _ = tx.send(ParseJobResult {
                source_id: doc_id,
                label,
                result,
                duration_ms: Some(started.elapsed().as_millis() as u64),
                force,
            });
        });
    }

    /// Enqueue a background download via `sil_parse::fetch_source_target` (DOI/arXiv/URL).
    pub fn queue_source_fetch(&mut self, target: String) {
        let target = target.trim().to_string();
        if target.is_empty() {
            self.status_message = "Empty fetch target".to_string();
            return;
        }
        if self.project_root.is_none() {
            self.status_message = "No active project — cannot fetch source".to_string();
            return;
        }
        if self.in_flight_fetch_targets.contains(&target) {
            self.status_message = format!("already fetching '{target}'...");
            return;
        }

        let kind = classify_source_input(&target);
        let label = target.clone();
        self.in_flight_fetch_targets.insert(target.clone());
        self.start_persisted_job(JobKind::Fetch, &label);
        self.status_message = format!("⏳ fetching… ({})", kind.label());

        let tx = self.fetch_tx.clone();
        let project_root = self.project_root.clone();

        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let catch_res = catch_unwind(AssertUnwindSafe(|| {
                (|| -> Result<FetchJobSuccess, String> {
                    let root = project_root.ok_or_else(|| "No project root".to_string())?;
                    let ctx = sil_app::AppContext::from_root(root).map_err(|e| e.to_string())?;
                    let fetch_res = sil_app::fetch_source(
                        &ctx,
                        sil_app::FetchSource {
                            target: target.clone(),
                            parse: true,
                        },
                    )
                    .map_err(|e| e.to_string())?;

                    Ok(FetchJobSuccess {
                        downloaded_path: fetch_res.downloaded_path,
                        bib: fetch_res.bib.map(|b| FetchBibSummary {
                            cite_key: b.cite_key,
                            replaced: b.replaced,
                        }),
                        parsed: fetch_res.parsed,
                        parse_error: fetch_res.parse_error,
                    })
                })()
            }));

            let result = match catch_res {
                Ok(res) => res,
                Err(p) => Err(extract_panic_message(p)),
            };

            let _ = tx.send(FetchJobResult {
                target,
                label,
                result,
                duration_ms: Some(started.elapsed().as_millis() as u64),
            });
        });
    }

    /// Enqueue background draft–ref similarity recompute (used only by `X`).
    pub fn enqueue_similarity_job(&mut self) {
        let root = match self.project_root.as_ref() {
            Some(r) => r.clone(),
            None => {
                self.status_message = "No active project loaded to compute similarity".to_string();
                return;
            }
        };

        if self.in_flight_similarity {
            self.status_message = "already recomputing draft similarity...".to_string();
            return;
        }

        let paths = ProjectPaths::new(&root);
        let draft_path = paths.paper_draft();
        if !draft_path.exists() {
            self.status_message = format!(
                "⚠ Paper draft not found at {}",
                draft_path.file_name().unwrap_or(draft_path.as_str())
            );
            return;
        }

        let draft_text = match std::fs::read_to_string(draft_path.as_std_path()) {
            Ok(t) => t,
            Err(e) => {
                self.status_message = format!("⚠ Failed reading paper draft: {e}");
                return;
            }
        };

        let clean = sil_core::strip_latex_for_embed(&draft_text);
        let draft_hash = sil_core::compute_draft_hash(&clean);
        let rag = self.effective_rag_settings();
        let db_path = paths.db();

        self.in_flight_similarity = true;
        self.start_persisted_job(JobKind::Similarity, "draft–ref similarity");
        self.status_message = "⏳ Recomputing draft similarity…".to_string();
        let tx = self.similarity_tx.clone();

        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let embedder = sil_db::OnnxEmbedder::from_rag_settings(&rag);
            let backend_summary = embedder.backend().summary();
            let catch_res = catch_unwind(AssertUnwindSafe(|| {
                (|| -> Result<usize, String> {
                    let db = sil_db::SilDb::open(&db_path)
                        .map_err(|e| format!("Database error: {e}"))?;
                    db.recompute_draft_ref_similarities(&draft_text, &embedder)
                        .map_err(|e| e.to_string())
                })()
            }));

            let result = match catch_res {
                Ok(res) => res,
                Err(p) => Err(extract_panic_message(p)),
            };

            let _ = tx.send(SimilarityJobResult {
                draft_hash,
                backend_summary,
                result,
                duration_ms: Some(started.elapsed().as_millis() as u64),
            });
        });
    }

    pub fn poll_background_parse(&mut self) {
        let mut polled_any = false;
        while let Ok(res) = self.parse_rx.try_recv() {
            polled_any = true;
            self.in_flight_parse_ids.remove(&res.source_id);
            let retry = self.parse_retry_payloads.remove(&res.source_id);
            match res.result {
                Ok(_parse_res) => {
                    self.reload_sources();
                    self.load_all_source_references();
                    self.status_message = format!("✓ Parsed source '{}'", res.label);
                    let id = self.alloc_job_id();
                    self.push_job_outcome(JobOutcome {
                        id,
                        kind: JobKind::Parse,
                        label: res.label.clone(),
                        ok: true,
                        detail: format!("Parsed source '{}'", res.label),
                        duration_ms: res.duration_ms,
                        retry_payload: None,
                    });
                }
                Err(err_msg) => {
                    let user_err = sil_core::UserError::classify(&err_msg);
                    self.status_message = format!(
                        "⚠ Failed parsing source '{}': {}",
                        res.label, user_err.title
                    );
                    self.last_user_error = Some(user_err);
                    let id = self.alloc_job_id();
                    self.push_job_outcome(JobOutcome {
                        id,
                        kind: JobKind::Parse,
                        label: res.label.clone(),
                        ok: false,
                        detail: err_msg,
                        duration_ms: res.duration_ms,
                        retry_payload: retry,
                    });
                }
            }
        }

        if polled_any && !self.in_flight_parse_ids.is_empty() {
            self.status_message = format!(
                "⏳ Parsing ({} in flight)...",
                self.in_flight_parse_ids.len()
            );
        }
    }

    pub fn poll_background_fetch(&mut self) {
        let mut polled_any = false;
        while let Ok(res) = self.fetch_rx.try_recv() {
            polled_any = true;
            self.in_flight_fetch_targets.remove(&res.target);
            match res.result {
                Ok(succ) => {
                    let saved_path = &succ.downloaded_path;
                    // Best-effort DB upsert so list_sources picks up metadata quickly if not parsed.
                    if succ.parsed.is_none() {
                        if let Some(ref root) = self.project_root {
                            let paths = ProjectPaths::new(root);
                            let config = self.loaded_config.clone().unwrap_or_default();
                            let sources_dir = paths.sources(&config);
                            let pdf_path = if saved_path.is_absolute() {
                                saved_path.clone()
                            } else if sources_dir
                                .join(saved_path.file_name().unwrap_or(saved_path.as_str()))
                                .exists()
                            {
                                sources_dir
                                    .join(saved_path.file_name().unwrap_or(saved_path.as_str()))
                            } else {
                                root.join(saved_path)
                            };
                            if pdf_path.exists()
                                && let Ok(db) = sil_db::SilDb::open(&paths.db())
                            {
                                let doc = SourceDocument::new(pdf_path);
                                let _ = db.upsert_parsed(&doc, "");
                            }
                        }
                    }
                    self.reload_sources();
                    if succ.parsed.is_some() {
                        self.load_all_source_references();
                    }
                    if succ.bib.is_some() {
                        self.load_project_references_bib();
                    }

                    if let Some(ref parse_err) = succ.parse_error {
                        let user_err = sil_core::UserError::classify(parse_err);
                        self.status_message =
                            format!("⚠ Source fetched but parse failed: {}", user_err.title);
                        self.last_user_error = Some(user_err);
                        let id = self.alloc_job_id();
                        self.push_job_outcome(JobOutcome {
                            id,
                            kind: JobKind::Fetch,
                            label: res.label.clone(),
                            ok: false,
                            detail: format!("Downloaded → {saved_path}, parse failed: {parse_err}"),
                            duration_ms: res.duration_ms,
                            retry_payload: Some(RetryPayload::Fetch {
                                target: res.target.clone(),
                            }),
                        });
                    } else {
                        let bib_msg = match &succ.bib {
                            Some(b) if b.replaced => {
                                format!(" (updated bibliography entry '{}')", b.cite_key)
                            }
                            Some(b) => format!(" (added bibliography entry '{}')", b.cite_key),
                            None => String::new(),
                        };
                        self.status_message = format!(
                            "✓ Source fetched & parsed{bib_msg} — Open from Sources or palette"
                        );

                        let detail_bib = match &succ.bib {
                            Some(b) => format!(" + bib '{}'", b.cite_key),
                            None => String::new(),
                        };
                        let detail_parse = match &succ.parsed {
                            Some(p) => format!(", parsed ({} refs)", p.reference_count),
                            None => String::new(),
                        };
                        let id = self.alloc_job_id();
                        self.push_job_outcome(JobOutcome {
                            id,
                            kind: JobKind::Fetch,
                            label: res.label.clone(),
                            ok: true,
                            detail: format!("Downloaded → {saved_path}{detail_parse}{detail_bib}"),
                            duration_ms: res.duration_ms,
                            retry_payload: None,
                        });
                    }
                }
                Err(err_msg) => {
                    let user_err = sil_core::UserError::classify(&err_msg);
                    self.status_message =
                        format!("⚠ Fetch failed for '{}': {}", res.label, user_err.title);
                    self.last_user_error = Some(user_err);
                    let id = self.alloc_job_id();
                    self.push_job_outcome(JobOutcome {
                        id,
                        kind: JobKind::Fetch,
                        label: res.label.clone(),
                        ok: false,
                        detail: err_msg,
                        duration_ms: res.duration_ms,
                        retry_payload: Some(RetryPayload::Fetch {
                            target: res.target.clone(),
                        }),
                    });
                }
            }
        }
        if polled_any && !self.in_flight_fetch_targets.is_empty() {
            self.status_message = format!(
                "⏳ fetching… ({} in flight)",
                self.in_flight_fetch_targets.len()
            );
        }
    }

    pub fn poll_background_similarity(&mut self) {
        while let Ok(res) = self.similarity_rx.try_recv() {
            self.in_flight_similarity = false;

            // Staleness: discard if draft changed while job ran.
            let current_hash = self.current_draft_hash();
            if current_hash.as_ref() != Some(&res.draft_hash) {
                let id = self.alloc_job_id();
                self.push_job_outcome(JobOutcome {
                    id,
                    kind: JobKind::Similarity,
                    label: "draft–ref similarity".to_string(),
                    ok: false,
                    detail: "Discarded stale similarity results (draft changed mid-job)"
                        .to_string(),
                    duration_ms: res.duration_ms,
                    retry_payload: Some(RetryPayload::Similarity),
                });
                self.status_message =
                    "⚠ Draft changed — discarded stale similarity results (press X to recompute)"
                        .to_string();
                continue;
            }

            match res.result {
                Ok(count) => {
                    if let Some(ref root) = self.project_root {
                        let paths = ProjectPaths::new(root);
                        if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
                            if let Ok(sims) = db.get_draft_ref_similarities() {
                                self.draft_ref_similarities = sims;
                            }
                            if let Ok(hash) = db.get_draft_similarity_hash() {
                                self.draft_similarity_hash = hash;
                            }
                        }
                    }
                    self.sort_source_references();
                    self.status_message = format!(
                        "✓ Recomputed draft similarity for {count} reference(s) [{}]",
                        res.backend_summary
                    );
                    let id = self.alloc_job_id();
                    self.push_job_outcome(JobOutcome {
                        id,
                        kind: JobKind::Similarity,
                        label: "draft–ref similarity".to_string(),
                        ok: true,
                        detail: format!(
                            "Recomputed scores for {count} reference(s) [{}]",
                            res.backend_summary
                        ),
                        duration_ms: res.duration_ms,
                        retry_payload: None,
                    });
                }
                Err(err_msg) => {
                    let user_err = sil_core::UserError::classify(&err_msg);
                    self.status_message =
                        format!("⚠ Failed computing similarity scores: {}", user_err.title);
                    self.last_user_error = Some(user_err);
                    let id = self.alloc_job_id();
                    self.push_job_outcome(JobOutcome {
                        id,
                        kind: JobKind::Similarity,
                        label: "draft–ref similarity".to_string(),
                        ok: false,
                        detail: err_msg,
                        duration_ms: res.duration_ms,
                        retry_payload: Some(RetryPayload::Similarity),
                    });
                }
            }
        }
    }

    pub(crate) fn current_draft_hash(&self) -> Option<String> {
        let root = self.project_root.as_ref()?;
        let paths = ProjectPaths::new(root);
        let draft_path = paths.paper_draft();
        if !draft_path.exists() {
            return None;
        }
        let text = std::fs::read_to_string(draft_path.as_std_path()).ok()?;
        let clean = sil_core::strip_latex_for_embed(&text);
        Some(sil_core::compute_draft_hash(&clean))
    }

    pub fn retry_job_outcome(&mut self, index: usize) {
        let payload = match self.recent_job_outcomes.get(index) {
            Some(o) if !o.ok => o.retry_payload.clone(),
            _ => None,
        };
        match payload {
            Some(RetryPayload::HydrateRef { entry }) => {
                self.status_message = "Retrying hydration…".to_string();
                self.queue_ref_hydration(entry);
            }
            Some(RetryPayload::HydrateSource { doc }) => {
                self.status_message = "Retrying source hydration…".to_string();
                self.queue_source_hydration(doc);
            }
            Some(RetryPayload::Fetch { target }) => {
                self.status_message = "Retrying fetch…".to_string();
                self.queue_source_fetch(target);
            }
            Some(RetryPayload::Parse { doc, force }) => {
                self.status_message = "Retrying parse…".to_string();
                self.queue_source_parse(doc, force);
            }
            Some(RetryPayload::Similarity) => {
                self.status_message = "Retrying similarity recompute…".to_string();
                self.enqueue_similarity_job();
            }
            None => {
                let stale = self
                    .recent_job_outcomes
                    .get(index)
                    .is_some_and(|outcome| outcome.detail.ends_with(" job stale"));
                if stale
                    && let Some(outcome) = self.recent_job_outcomes.get(index)
                    && outcome.kind == JobKind::Parse
                    && let Some(doc) = self.sources.iter().find(|doc| {
                        doc.filename == outcome.label
                            || doc.title.as_deref() == Some(outcome.label.as_str())
                    })
                {
                    self.status_message = "Retrying parse…".to_string();
                    self.queue_source_parse(doc.clone(), false);
                } else {
                    self.status_message =
                        "Selected job cannot be retried (success or no payload)".to_string();
                }
            }
        }
    }

    pub fn poll_background_hydration(&mut self) {
        self.poll_background_parse();
        self.poll_background_fetch();
        self.poll_background_similarity();
        self.poll_background_estimate();
        self.poll_background_digest();
        self.poll_background_build();
        self.check_auto_digest_refresh();
        if self.dirty && !self.disk_conflict_pending && !self.disk_conflict_dismissed {
            self.check_disk_conflicts();
        }
        let mut polled_any = false;
        while let Ok(res) = self.hydration_rx.try_recv() {
            polled_any = true;
            self.in_flight_hydration_keys.remove(&res.dedup_key);
            let retry = self.hydrate_retry_payloads.remove(&res.dedup_key);
            match res.outcome {
                HydrationOutcome::Success { official_bib } => {
                    self.hydration_batch_succeeded += 1;

                    if let Some(ref root) = self.project_root {
                        let ctx = match sil_app::AppContext::from_root(root) {
                            Ok(c) => c,
                            Err(e) => {
                                let err_msg = format!("Error writing references.bib: {e}");
                                self.last_user_error =
                                    Some(sil_core::UserError::classify(&err_msg));
                                let id = self.alloc_job_id();
                                self.push_job_outcome(JobOutcome {
                                    id,
                                    kind: JobKind::Hydrate,
                                    label: res.label.clone(),
                                    ok: false,
                                    detail: err_msg,
                                    duration_ms: res.duration_ms,
                                    retry_payload: retry,
                                });
                                continue;
                            }
                        };

                        let bib_path = ctx.paths.join(sil_core::paths::rel::REFERENCES);
                        let current = match std::fs::read_to_string(bib_path.as_str()) {
                            Ok(c) => c,
                            Err(e) => {
                                let err_msg = format!("Error reading references.bib: {e}");
                                self.last_user_error =
                                    Some(sil_core::UserError::classify(&err_msg));
                                let id = self.alloc_job_id();
                                self.push_job_outcome(JobOutcome {
                                    id,
                                    kind: JobKind::Hydrate,
                                    label: res.label.clone(),
                                    ok: false,
                                    detail: err_msg,
                                    duration_ms: res.duration_ms,
                                    retry_payload: retry,
                                });
                                continue;
                            }
                        };

                        let official_info = sil_core::extract_bib_entry_info(&official_bib);
                        let blocks = sil_core::parse_bib_blocks(&current);
                        let existing_block = blocks.iter().find(|block| {
                            let info = sil_core::extract_bib_entry_info(block);
                            sil_core::is_same_paper(&info, &official_info)
                        });

                        if let Some(matching_block) = existing_block {
                            let draft = sil_core::is_tui_added_bib_block(matching_block);

                            match sil_app::upsert_bib(
                                &ctx,
                                sil_app::UpsertBib {
                                    entry: official_bib,
                                    draft,
                                },
                            ) {
                                Ok(_) => {
                                    self.load_project_references_bib();
                                    let id = self.alloc_job_id();
                                    self.push_job_outcome(JobOutcome {
                                        id,
                                        kind: JobKind::Hydrate,
                                        label: res.label.clone(),
                                        ok: true,
                                        detail: format!("Official metadata for '{}'", res.label),
                                        duration_ms: res.duration_ms,
                                        retry_payload: None,
                                    });
                                }
                                Err(e) => {
                                    let err_msg = format!("Error writing references.bib: {e}");
                                    self.last_user_error =
                                        Some(sil_core::UserError::classify(&err_msg));
                                    let id = self.alloc_job_id();
                                    self.push_job_outcome(JobOutcome {
                                        id,
                                        kind: JobKind::Hydrate,
                                        label: res.label.clone(),
                                        ok: false,
                                        detail: err_msg,
                                        duration_ms: res.duration_ms,
                                        retry_payload: retry,
                                    });
                                }
                            }
                        } else {
                            let id = self.alloc_job_id();
                            self.push_job_outcome(JobOutcome {
                                id,
                                kind: JobKind::Hydrate,
                                label: res.label.clone(),
                                ok: false,
                                detail: format!(
                                    "Skipped hydration for '{}': entry was deleted from references.bib",
                                    res.label
                                ),
                                duration_ms: res.duration_ms,
                                retry_payload: None,
                            });
                        }
                    } else {
                        let id = self.alloc_job_id();
                        self.push_job_outcome(JobOutcome {
                            id,
                            kind: JobKind::Hydrate,
                            label: res.label.clone(),
                            ok: true,
                            detail: format!("Official metadata for '{}'", res.label),
                            duration_ms: res.duration_ms,
                            retry_payload: None,
                        });
                    }
                }
                HydrationOutcome::Failure { reason } => {
                    self.hydration_batch_failed += 1;
                    let user_err = sil_core::UserError::classify(&reason);
                    self.last_user_error = Some(user_err);
                    let id = self.alloc_job_id();
                    self.push_job_outcome(JobOutcome {
                        id,
                        kind: JobKind::Hydrate,
                        label: res.label.clone(),
                        ok: false,
                        detail: reason,
                        duration_ms: res.duration_ms,
                        retry_payload: retry,
                    });
                }
            }
        }

        if polled_any {
            if self.in_flight_hydration_keys.is_empty() {
                if self.hydration_batch_succeeded == 1 && self.hydration_batch_failed == 0 {
                    let last = self.recent_job_outcomes.back();
                    if let Some(h) = last
                        && h.ok
                    {
                        self.status_message = format!("✓ Official metadata for '{}'", h.label);
                    } else {
                        self.status_message = format!(
                            "✓ Hydration complete: {} succeeded, {} failed",
                            self.hydration_batch_succeeded, self.hydration_batch_failed
                        );
                    }
                } else if self.hydration_batch_failed == 1 && self.hydration_batch_succeeded == 0 {
                    let (last_label, reason) = self
                        .recent_job_outcomes
                        .back()
                        .map(|h| (h.label.as_str(), h.detail.as_str()))
                        .unwrap_or(("source", "unknown error"));
                    let user_err = sil_core::UserError::classify(reason);
                    self.status_message = format!(
                        "⚠ Metadata fetch failed for '{last_label}': {}",
                        user_err.title
                    );
                    self.last_user_error = Some(user_err);
                } else {
                    self.status_message = format!(
                        "✓ Hydration complete: {} succeeded, {} failed",
                        self.hydration_batch_succeeded, self.hydration_batch_failed
                    );
                }
            } else {
                self.status_message = format!(
                    "⏳ Hydrating ({} in flight)...",
                    self.in_flight_hydration_keys.len()
                );
            }
        }
    }

    /// Trigger background manuscript estimate job.
    pub fn run_estimate_job(&mut self) {
        let Some(root) = self.project_root.clone() else {
            let user_err = sil_core::UserError::classify("not inside a sil project");
            self.status_message = format!("Estimate error: {}", user_err.title);
            self.last_user_error = Some(user_err);
            return;
        };

        if self.in_flight_estimate {
            self.status_message = "already running manuscript estimate...".to_string();
            return;
        }

        self.in_flight_estimate = true;
        self.start_persisted_job(JobKind::Estimate, "manuscript estimate");
        self.status_message = "⏳ Running L0 manuscript estimate...".to_string();
        let tx = self.estimate_tx.clone();

        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let catch_res = catch_unwind(AssertUnwindSafe(|| {
                let input = sil_agent::EstimateInput {
                    root: root.as_path(),
                    mode: sil_agent::EstimateMode::Quick,
                    structure: None,
                };
                let report =
                    sil_agent::run_heuristic_estimate(&input).map_err(|e| e.to_string())?;
                sil_agent::write_estimate_report(&root, &report).map_err(|e| e.to_string())?;
                Ok(report)
            }));

            let result = match catch_res {
                Ok(res) => res,
                Err(p) => Err(extract_panic_message(p)),
            };

            let _ = tx.send(EstimateJobResult {
                result,
                duration_ms: Some(started.elapsed().as_millis() as u64),
            });
        });
    }

    pub fn poll_background_estimate(&mut self) {
        while let Ok(res) = self.estimate_rx.try_recv() {
            self.in_flight_estimate = false;
            let id = self.alloc_job_id();
            match res.result {
                Ok(report) => {
                    self.status_message = format!(
                        "✓ L0 estimate complete: score={}, decision={:?}",
                        report.overall_score, report.decision
                    );
                    self.push_job_outcome(JobOutcome {
                        id,
                        kind: JobKind::Estimate,
                        label: "manuscript estimate".to_string(),
                        ok: true,
                        detail: format!(
                            "score={}, decision={:?}, findings={}",
                            report.overall_score,
                            report.decision,
                            report.findings.len()
                        ),
                        duration_ms: res.duration_ms,
                        retry_payload: None,
                    });
                }
                Err(err_msg) => {
                    let user_err = sil_core::UserError::classify(&err_msg);
                    self.status_message = format!("⚠ Estimate failed: {}", user_err.title);
                    self.last_user_error = Some(user_err);
                    self.push_job_outcome(JobOutcome {
                        id,
                        kind: JobKind::Estimate,
                        label: "manuscript estimate".to_string(),
                        ok: false,
                        detail: format!("failed: {err_msg}"),
                        duration_ms: res.duration_ms,
                        retry_payload: None,
                    });
                }
            }
        }
    }

    pub fn queue_digest_refresh(&mut self) {
        let effective_query = sil_core::effective_digest_query(
            &self.global_settings.digest_query,
            &self.local_settings.digest_query,
        );

        let Some(query) = effective_query.map(|s| s.to_string()) else {
            return;
        };

        if self.in_flight_digest {
            return;
        }

        let Some(root) = self.project_root.clone() else {
            return;
        };

        self.in_flight_digest = true;
        self.start_persisted_job(JobKind::Digest, &format!("digest: {query}"));
        self.status_message = format!("⏳ Refreshing literature digest for '{query}'...");

        let tx = self.digest_tx.clone();
        let query_clone = query.clone();

        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let catch_res = catch_unwind(AssertUnwindSafe(|| {
                (|| -> Result<usize, String> {
                    let items = sil_parse::fetch_journal_publications(&query_clone, 10, None, None)
                        .map_err(|e| e.to_string())?;

                    let paths = sil_core::ProjectPaths::new(&root);
                    let db = sil_db::SilDb::open(&paths.db())
                        .map_err(|e| format!("Database error: {e}"))?;

                    for item in &items {
                        db.save_journal_publication(item)
                            .map_err(|e| format!("Failed to save publication: {e}"))?;
                    }

                    Ok(items.len())
                })()
            }));

            let result = match catch_res {
                Ok(res) => res,
                Err(p) => Err(extract_panic_message(p)),
            };

            let _ = tx.send(DigestJobResult {
                query,
                result,
                duration_ms: Some(started.elapsed().as_millis() as u64),
            });
        });
    }

    pub fn poll_background_digest(&mut self) {
        while let Ok(res) = self.digest_rx.try_recv() {
            self.in_flight_digest = false;
            let id = self.alloc_job_id();
            match res.result {
                Ok(count) => {
                    self.refresh_dashboard();
                    self.status_message = format!(
                        "✓ Refreshed literature digest for '{}' ({count} paper(s))",
                        res.query
                    );
                    self.push_job_outcome(JobOutcome {
                        id,
                        kind: JobKind::Digest,
                        label: format!("digest: {}", res.query),
                        ok: true,
                        detail: format!("Refreshed {count} publication(s) for '{}'", res.query),
                        duration_ms: res.duration_ms,
                        retry_payload: None,
                    });
                }
                Err(err_msg) => {
                    let user_err = sil_core::UserError::classify(&err_msg);
                    self.status_message = format!(
                        "⚠ Digest refresh failed for '{}': {}",
                        res.query, user_err.title
                    );
                    self.last_user_error = Some(user_err);
                    self.push_job_outcome(JobOutcome {
                        id,
                        kind: JobKind::Digest,
                        label: format!("digest: {}", res.query),
                        ok: false,
                        detail: err_msg,
                        duration_ms: res.duration_ms,
                        retry_payload: None,
                    });
                }
            }
        }
    }

    pub fn check_auto_digest_refresh(&mut self) {
        if self.active_tab != ActiveTab::Dashboard || self.in_flight_digest {
            return;
        }

        let effective_query = sil_core::effective_digest_query(
            &self.global_settings.digest_query,
            &self.local_settings.digest_query,
        );

        if effective_query.is_none() {
            return;
        }

        let is_stale = if let Some(ref root) = self.project_root {
            let paths = sil_core::ProjectPaths::new(root);
            if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
                let last_fetched = db.digest_last_fetched_at().ok().flatten();
                let hours = sil_core::effective_digest_refresh_hours(
                    self.global_settings.digest_refresh_hours,
                );
                let now_secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                is_digest_cache_stale(last_fetched.as_deref(), hours, now_secs)
            } else {
                true
            }
        } else {
            false
        };

        if is_stale {
            self.queue_digest_refresh();
        }
    }
}

/// Extract the first TeX source location from compiler output.
pub fn parse_latex_error_location(log: &str) -> Option<(String, usize)> {
    for token in log.split_whitespace() {
        let token = token.trim_matches(|c: char| "([{<\"'".contains(c));
        let parts: Vec<_> = token.split(':').collect();
        for index in 1..parts.len() {
            let file = parts[..index].join(":");
            let line = parts[index]
                .trim_end_matches(|c: char| ")]}>\"',".contains(c))
                .parse()
                .ok();
            if let Some(line) = line
                && !file.is_empty()
                && line > 0
                && (file.ends_with(".tex") || file.contains('/'))
            {
                return Some((file, line));
            }
        }
    }
    None
}

impl App {
    pub(crate) fn jump_to_draft_line(&mut self, file: &str, line: usize) {
        let target = self.project_root.as_ref().map(|root| root.join(file));
        let is_main = target.as_ref().is_some_and(|p| {
            p == &sil_core::ProjectPaths::new(self.project_root.as_ref().unwrap()).paper_draft()
        }) || file.ends_with("paper_draft.tex");
        if !is_main {
            return;
        }
        let max_line = self.paper_draft_content.lines().count().max(1);
        let line = line.clamp(1, max_line);
        self.active_tab = ActiveTab::PaperDraft;
        self.paper_scroll_offset = line - 1;
        if let Some((index, section)) = self
            .paper_sections
            .iter()
            .enumerate()
            .rev()
            .find(|(_, section)| section.line_start <= line)
        {
            self.paper_section_index = index;
            self.paper_scroll_offset =
                (line - section.line_start).min(section.body.lines().count().saturating_sub(1));
        }
    }
}

impl App {
    pub fn open_last_review(&mut self) {
        let Some(root) = self.project_root.as_ref() else {
            self.status_message = "No active project loaded".to_string();
            return;
        };
        let reviews = sil_core::ProjectPaths::new(root).reviews_dir();
        let mut candidates = Vec::new();
        if let Ok(entries) = std::fs::read_dir(reviews.as_std_path()) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let report = [path.join("report.md"), path.join("report.json")]
                    .into_iter()
                    .find(|p| p.is_file());
                if let Some(report) = report {
                    let modified = entry.metadata().and_then(|m| m.modified()).ok();
                    candidates.push((modified, report));
                }
            }
        }
        candidates.sort_by_key(|a| a.0);
        self.estimate_report_content = candidates
            .last()
            .and_then(|(_, path)| std::fs::read_to_string(path).ok())
            .or_else(|| Some("no reviews yet — run Estimate".to_string()));
        self.estimate_report_scroll_offset = 0;
        self.input_mode = InputMode::EstimateReport;
        self.status_message = "Viewing last estimate report. Press Esc to exit.".to_string();
    }
}

/// Helper to parse ISO/SQLite UTC timestamp ("YYYY-MM-DD HH:MM:SS" or "YYYY-MM-DDTHH:MM:SSZ") into Unix epoch seconds.
pub fn parse_utc_timestamp(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.len() < 19 {
        return None;
    }
    let year: i64 = s[0..4].parse().ok()?;
    let month: i64 = s[5..7].parse().ok()?;
    let day: i64 = s[8..10].parse().ok()?;
    let hour: u64 = s[11..13].parse().ok()?;
    let min: u64 = s[14..16].parse().ok()?;
    let sec: u64 = s[17..19].parse().ok()?;

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || min > 59 || sec > 59 {
        return None;
    }

    let days_before_month = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);

    let y = year - 1970;
    let leap_years = (year - 1969) / 4 - (year - 1901) / 100 + (year - 1601) / 400;
    let mut total_days = y * 365 + leap_years + days_before_month[(month - 1) as usize];
    if month > 2 && is_leap {
        total_days += 1;
    }
    total_days += day - 1;

    let total_secs = (total_days as u64).checked_mul(86400)? + hour * 3600 + min * 60 + sec;
    Some(total_secs)
}

/// Determine whether the digest cache is stale based on the last fetched ISO string, configured refresh interval (hours, min 1), and current Unix time.
pub fn is_digest_cache_stale(
    last_fetched: Option<&str>,
    refresh_hours: u32,
    now_epoch_secs: u64,
) -> bool {
    let Some(fetched_str) = last_fetched else {
        return true;
    };

    let Some(fetched_secs) = parse_utc_timestamp(fetched_str) else {
        return true;
    };

    let age_secs = now_epoch_secs.saturating_sub(fetched_secs);
    let refresh_secs = (refresh_hours.max(1) as u64) * 3600;
    age_secs >= refresh_secs
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use camino::Utf8PathBuf;

    fn app_with_jobs_file(contents: &str) -> (App, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        std::fs::create_dir_all(root.join(".sil")).unwrap();
        std::fs::write(root.join(".sil/jobs.json"), contents).unwrap();
        let mut app = App::new(None);
        app.project_root = Some(root);
        (app, dir)
    }

    #[test]
    fn running_rows_become_stale_on_load() {
        let (mut app, _dir) = app_with_jobs_file(
            r#"[{"id":7,"kind":"fetch","label":"10.1000/test","status":"running","started":1,"ended":null,"error_code":null}]"#,
        );
        app.load_persisted_jobs();
        assert_eq!(app.recent_job_outcomes.len(), 1);
        assert!(!app.recent_job_outcomes[0].ok);
        assert!(app.recent_job_outcomes[0].detail.ends_with("job stale"));
        let saved = std::fs::read_to_string(app.jobs_path().unwrap()).unwrap();
        assert!(saved.contains("\"status\": \"stale\""));
    }

    #[test]
    fn successful_job_is_saved_as_ok() {
        let (mut app, _dir) = app_with_jobs_file("[]");
        app.start_persisted_job(JobKind::Fetch, "10.1000/test");
        app.push_job_outcome(JobOutcome {
            id: 1,
            kind: JobKind::Fetch,
            label: "10.1000/test".to_string(),
            ok: true,
            detail: "downloaded".to_string(),
            duration_ms: None,
            retry_payload: None,
        });
        let saved = std::fs::read_to_string(app.jobs_path().unwrap()).unwrap();
        assert!(saved.contains("\"status\": \"ok\""));
    }

    #[test]
    fn persisted_jobs_are_capped_at_fifty() {
        let (mut app, _dir) = app_with_jobs_file("[]");
        for id in 0..(PERSISTED_JOB_CAP + 5) {
            app.start_persisted_job(JobKind::Fetch, &format!("target-{id}"));
        }
        let saved = std::fs::read_to_string(app.jobs_path().unwrap()).unwrap();
        let jobs: Vec<PersistedJob> = serde_json::from_str(&saved).unwrap();
        assert_eq!(jobs.len(), PERSISTED_JOB_CAP);
    }

    #[test]
    fn corrupt_jobs_file_warns_and_stays_empty() {
        let (mut app, _dir) = app_with_jobs_file("not json");
        app.load_persisted_jobs();
        assert!(app.recent_job_outcomes.is_empty());
        assert!(app.status_message.contains("invalid .sil/jobs.json"));
        assert_eq!(
            app.last_user_error.as_ref().unwrap().code,
            "jobs.invalid_json"
        );
    }
}
