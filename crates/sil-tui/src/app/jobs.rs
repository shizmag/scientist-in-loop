use super::*;
use sil_core::{ProjectPaths, ReferenceEntry, SourceDocument};
use std::panic::{AssertUnwindSafe, catch_unwind};

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
    pub(crate) fn alloc_job_id(&mut self) -> u64 {
        let id = self.next_job_id;
        self.next_job_id = self.next_job_id.saturating_add(1);
        id
    }

    pub(crate) fn push_job_outcome(&mut self, outcome: JobOutcome) {
        if self.recent_job_outcomes.len() >= JOB_HISTORY_CAP {
            self.recent_job_outcomes.pop_front();
        }
        self.recent_job_outcomes.push_back(outcome);
        if self.selected_job_history_index >= self.recent_job_outcomes.len()
            && !self.recent_job_outcomes.is_empty()
        {
            self.selected_job_history_index = self.recent_job_outcomes.len() - 1;
        }
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
                            parse: false,
                        },
                    )
                    .map_err(|e| e.to_string())?;

                    Ok(FetchJobSuccess {
                        downloaded_path: fetch_res.downloaded_path,
                        bib: fetch_res.bib.map(|b| FetchBibSummary {
                            cite_key: b.cite_key,
                            replaced: b.replaced,
                        }),
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
                    self.status_message =
                        format!("⚠ Failed parsing source '{}': {}", res.label, err_msg);
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
                    // Best-effort DB upsert so list_sources picks up metadata quickly.
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
                            sources_dir.join(saved_path.file_name().unwrap_or(saved_path.as_str()))
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
                    self.reload_sources();
                    if succ.bib.is_some() {
                        self.load_project_references_bib();
                    }

                    let bib_msg = match &succ.bib {
                        Some(b) if b.replaced => {
                            format!(" (updated bibliography entry '{}')", b.cite_key)
                        }
                        Some(b) => format!(" (added bibliography entry '{}')", b.cite_key),
                        None => String::new(),
                    };
                    self.status_message = format!("✓ Fetched source '{}'{bib_msg}", res.label);

                    let detail_bib = match &succ.bib {
                        Some(b) => format!(" + bib '{}'", b.cite_key),
                        None => String::new(),
                    };
                    let id = self.alloc_job_id();
                    self.push_job_outcome(JobOutcome {
                        id,
                        kind: JobKind::Fetch,
                        label: res.label.clone(),
                        ok: true,
                        detail: format!("Downloaded → {saved_path}{detail_bib}"),
                        duration_ms: res.duration_ms,
                        retry_payload: None,
                    });
                }
                Err(err_msg) => {
                    self.status_message = format!("⚠ Fetch failed for '{}': {err_msg}", res.label);
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
                    self.status_message =
                        format!("⚠ Failed computing similarity scores: {err_msg}");
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
                self.status_message =
                    "Selected job cannot be retried (success or no payload)".to_string();
            }
        }
    }

    pub fn poll_background_hydration(&mut self) {
        self.poll_background_parse();
        self.poll_background_fetch();
        self.poll_background_similarity();
        self.poll_background_estimate();
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
                    self.status_message =
                        format!("⚠ Metadata fetch failed for '{last_label}': {reason}");
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
            self.status_message = "Estimate error: not inside a sil project root.".to_string();
            return;
        };

        if self.in_flight_estimate {
            self.status_message = "already running manuscript estimate...".to_string();
            return;
        }

        self.in_flight_estimate = true;
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
                sil_agent::run_heuristic_estimate(&input).map_err(|e| e.to_string())
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
                    self.status_message = format!("⚠ Estimate failed: {err_msg}");
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
}
