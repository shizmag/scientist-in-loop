//! Central command dispatcher implementation for `App`.

use super::*;

impl App {
    /// Dispatch and execute a registered command by its [`CommandId`].
    pub fn dispatch(&mut self, cmd: CommandId) {
        match cmd {
            CommandId::OpenPalette => {
                self.palette_previous_mode = self.input_mode;
                self.palette_filter.clear();
                self.palette_selected_index = 0;
                self.input_mode = InputMode::CommandPalette;
                self.status_message =
                    "Command palette — type to filter, Enter to run, Esc to close".to_string();
            }
            CommandId::SaveAll => {
                if !self.check_mutation_lock("save_all") {
                    return;
                }
                self.confirm_lock_override = false;
                self.save_all();
            }
            CommandId::Quit => {
                self.cleanup_lock();
                self.should_quit = true;
            }
            CommandId::OpenHelp => {
                self.toggle_help_overlay();
            }
            CommandId::Reload => {
                self.reload_sources_and_bib_sync();
                self.status_message = if self.active_tab == ActiveTab::Sources {
                    "✓ Reloaded sources".to_string()
                } else if self.active_tab == ActiveTab::Dashboard {
                    "✓ Reloaded dashboard".to_string()
                } else {
                    "✓ Reloaded project state from disk".to_string()
                };
            }
            CommandId::OpenJobHistory => {
                self.open_job_history();
            }
            CommandId::ParseSelected => {
                if self.sources.is_empty() || self.selected_source_index >= self.sources.len() {
                    self.status_message = "No active source document selected.".to_string();
                    return;
                }
                let doc = self.sources[self.selected_source_index].clone();
                self.queue_source_parse(doc, false);
            }
            CommandId::ParseAll => {
                if self.sources.is_empty() {
                    self.status_message = "No source documents in project to parse.".to_string();
                    return;
                }
                let unparsed: Vec<_> = self.sources.iter().filter(|s| !s.parsed).cloned().collect();
                let (docs_to_parse, force) = if unparsed.is_empty() {
                    (self.sources.clone(), true)
                } else {
                    (unparsed, false)
                };
                let count = docs_to_parse.len();
                for doc in docs_to_parse {
                    self.queue_source_parse(doc, force);
                }
                if count > 1 {
                    self.status_message = format!("Queued background parsing for {count} sources.");
                }
            }
            CommandId::AddSourceLink | CommandId::FetchParse => {
                if !self.check_mutation_lock("add_source") {
                    return;
                }
                self.confirm_lock_override = false;
                self.active_tab = ActiveTab::Sources;
                self.new_source_link_buffer.clear();
                self.input_mode = InputMode::ModalAddSourceLink;
                self.status_message =
                    "Fetch/download source — enter URL / DOI / arXiv (Enter to start fetch, Esc to cancel)"
                        .to_string();
            }
            CommandId::OpenSource => {
                if self.sources.is_empty() || self.selected_source_index >= self.sources.len() {
                    self.status_message = "No active source document selected.".to_string();
                    return;
                }
                let doc = &self.sources[self.selected_source_index];
                let content = self.fetch_source_markdown_content(doc);
                self.reading_md_content = Some(content);
                self.input_mode = InputMode::ReadingSourceMd;
                self.source_scroll_offset = 0;
                self.status_message = format!("Reading {}. Press Esc to exit.", doc.filename);
            }
            CommandId::CiteSource => {
                if !self.check_mutation_lock("cite_source") {
                    return;
                }
                self.confirm_lock_override = false;
                self.append_selected_source_to_bib();
            }
            CommandId::CiteIntoSection => {
                if !self.check_mutation_lock("cite_section") {
                    return;
                }
                self.confirm_lock_override = false;
                if self.sources.is_empty() || self.selected_source_index >= self.sources.len() {
                    self.status_message = "No active source document selected.".to_string();
                    return;
                }
                let doc = self.sources[self.selected_source_index].clone();
                let root = match self.project_root.as_ref() {
                    Some(r) => r.clone(),
                    None => {
                        self.status_message = "Error: paper_draft.tex not found.".to_string();
                        return;
                    }
                };
                let draft_path = root.join("paper_draft.tex");
                let bib_path = root.join("references.bib");
                if !draft_path.is_file() {
                    self.status_message = "Error: paper_draft.tex not found.".to_string();
                    return;
                }
                let _ = sil_core::undo::snapshot(
                    &root,
                    "Cite source in draft section",
                    &[bib_path.clone(), draft_path.clone()],
                );
                let local_bib = sil_core::suggest_from_source(&doc).bibtex;
                let ctx = match sil_app::AppContext::from_root(&root) {
                    Ok(c) => c,
                    Err(e) => {
                        self.status_message = format!("Error writing references.bib: {e}");
                        return;
                    }
                };
                let upsert_res = match sil_app::upsert_bib(
                    &ctx,
                    sil_app::UpsertBib {
                        entry: local_bib,
                        draft: true,
                    },
                ) {
                    Ok(r) => r,
                    Err(e) => {
                        self.status_message = format!("Error writing references.bib: {e}");
                        return;
                    }
                };
                self.load_project_references_bib();
                if doc.should_attempt_metadata_fetch() {
                    self.queue_source_hydration(doc.clone());
                }
                let cite_key = upsert_res.cite_key;
                let mut sections: Vec<String> = self
                    .paper_sections
                    .iter()
                    .filter(|s| s.kind != "document")
                    .map(|s| s.title.clone())
                    .collect();
                if sections.is_empty() {
                    if let Some(first) = self.paper_sections.first() {
                        sections.push(first.title.clone());
                    } else {
                        sections.push("(preamble / body)".to_string());
                    }
                }
                self.pending_cite_key = cite_key;
                self.cite_picker_sections = sections;
                self.cite_picker_selected = 0;
                self.cite_picker_previous_mode = self.input_mode;
                self.input_mode = InputMode::CiteSectionPicker;
                self.status_message = format!(
                    "Select draft section to cite {} (Enter to confirm, Esc to cancel)",
                    self.pending_cite_key
                );
            }
            CommandId::CaptureNote => {
                if self.sources.is_empty() || self.selected_source_index >= self.sources.len() {
                    self.status_message = "No active source document selected.".to_string();
                    return;
                }
                if !self.check_mutation_lock("capture_note") {
                    return;
                }
                self.confirm_lock_override = false;
                self.capture_note_buffer.clear();
                self.input_mode = InputMode::ModalCaptureNote;
                self.status_message =
                    "Capture note for draft (Enter to save, Esc to cancel)".to_string();
            }
            CommandId::GroundSection => {
                let query = self
                    .paper_sections
                    .get(self.paper_section_index)
                    .map(|section| section.body.chars().take(4_000).collect::<String>())
                    .unwrap_or_else(|| self.paper_draft_content.chars().take(4_000).collect());
                self.grounding_hits.clear();
                self.grounding_selected_index = 0;
                if let Some(root) = self.project_root.as_ref() {
                    let paths = ProjectPaths::new(root);
                    if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
                        let embedder = sil_db::OnnxEmbedder::new(None::<&std::path::Path>);
                        if let Ok(hits) = db.search_hybrid(&embedder, &query, 20, true) {
                            self.grounding_hits = hits
                                .into_iter()
                                .map(|hit| GroundingHit {
                                    title: hit
                                        .chunk
                                        .heading_title
                                        .unwrap_or_else(|| "Untitled source".to_string()),
                                    score: hit.score,
                                    source_id: hit.chunk.source_id.to_string(),
                                })
                                .collect();
                        }
                    }
                }
                self.input_mode = InputMode::GroundingModal;
                self.status_message = if self.grounding_hits.is_empty() {
                    "No grounding sources found.".to_string()
                } else {
                    format!(
                        "Showing {} ranked grounding sources.",
                        self.grounding_hits.len()
                    )
                };
            }
            CommandId::RefreshDigest => {
                let effective_query = sil_core::effective_digest_query(
                    &self.global_settings.digest_query,
                    &self.local_settings.digest_query,
                );
                if effective_query.is_none() {
                    self.status_message =
                        "No digest query configured. Set query in Settings tab (Tab 5)."
                            .to_string();
                    return;
                }
                self.queue_digest_refresh();
            }
            CommandId::OpenExternalEditor => {
                self.pending_external_editor = true;
                self.status_message =
                    "Launching external editor ($EDITOR / nvim / helix)...".to_string();
            }
            CommandId::Undo => {
                if let Some(ref root) = self.project_root.clone() {
                    match sil_core::undo(root) {
                        Ok(Some(generation)) => {
                            let paths = ProjectPaths::new(root);
                            let draft_path = root.join("paper_draft.tex");
                            if draft_path.is_file() {
                                if let Ok(content) =
                                    std::fs::read_to_string(draft_path.as_std_path())
                                {
                                    let _ = sil_latex::write_draft_sections_from_file(
                                        &draft_path,
                                        &paths.draft_sections_dir(),
                                    );
                                    if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
                                        let ideas = sil_latex::parse_idea_blocks(&content);
                                        let _ = db.replace_todo_ideas(&ideas);
                                    }
                                }
                            }
                            self.reload_sources_and_bib_sync();
                            self.status_message = format!("Undone: {}", generation.op);
                        }
                        Ok(None) => {
                            self.status_message = "Nothing to undo".to_string();
                        }
                        Err(e) => {
                            self.status_message = format!("Undo failed: {e}");
                        }
                    }
                } else {
                    self.status_message = "No active project loaded".to_string();
                }
            }
            CommandId::RepairDb => {
                if self.project_root.is_none() {
                    self.status_message = "No active project loaded".to_string();
                    return;
                }
                self.input_mode = InputMode::ConfirmRepairDb;
                self.status_message =
                    "Repair SQLite database from sources/? (y/Enter to confirm, Esc to cancel)"
                        .to_string();
            }
            CommandId::RunEstimate => self.run_estimate_job(),
            CommandId::OpenLastReview => self.open_last_review(),
            CommandId::BuildDraft => self.run_build_job(),
            CommandId::ReviewChanges => self.open_proposal_diff(),
        }
    }

    fn open_proposal_diff(&mut self) {
        let Some(root) = self.project_root.clone() else {
            self.status_message = "No active project loaded".to_string();
            return;
        };
        let status = match sil_git::status(&root) {
            Ok(status) => status,
            Err(e) => {
                self.status_message = format!("Git status failed: {e}");
                return;
            }
        };
        let diff = sil_git::diff_for_paths(&root, &["paper_draft.tex", "references.bib"])
            .unwrap_or_else(|e| format!("Unable to read diff: {e}"));
        let proposal = sil_git::propose_from_status(&status, None, None, None)
            .map(|p| p.display())
            .unwrap_or_else(|e| format!("No proposal available: {e}"));
        let diff_text = if diff.is_empty() {
            "(no tracked diff)"
        } else {
            diff.as_str()
        };
        self.proposal_diff_content = Some(format!(
            "Status:\n{}\n\nDiff:\n{}",
            if status.clean {
                "clean".to_string()
            } else {
                status.entries.join("\n")
            },
            diff_text
        ));
        self.proposal_text = Some(proposal);
        self.input_mode = InputMode::ProposalDiff;
        self.status_message =
            "Review changes: y writes proposal, u undoes latest journal entry".to_string();
    }
}
