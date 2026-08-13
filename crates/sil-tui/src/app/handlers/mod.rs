//! Key event handlers and user action dispatchers for `sil-tui`.

use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use sil_core::{AuthorDetails, GrantDetails, ProjectPaths, ReferenceEntry, SourceDocument};

impl App {
    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::HelpOverlay => self.handle_help_overlay_mode(key),
            InputMode::Normal => self.handle_normal_mode(key),
            InputMode::Editing => self.handle_editing_mode(key),
            InputMode::EditingPaper => self.handle_editing_paper_mode(key),
            InputMode::ModalPicker => self.handle_modal_picker_mode(key),
            InputMode::ModalAddAuthor => self.handle_modal_add_author_mode(key),
            InputMode::ModalAddGrant => self.handle_modal_add_grant_mode(key),
            InputMode::ModalAddSourceLink => self.handle_modal_add_source_link_mode(key),
            InputMode::ModalRenameSource => self.handle_modal_rename_source_mode(key),
            InputMode::ModalCaptureNote => self.handle_modal_capture_note_mode(key),
            InputMode::ConfirmDeleteSource => self.handle_confirm_delete_source_mode(key),
            InputMode::JobHistory => self.handle_job_history_mode(key),
            InputMode::ViewingSourceRefs => self.handle_viewing_source_refs_mode(key),
            InputMode::SearchingRefs => self.handle_searching_refs_mode(key),
            InputMode::SearchingBib => self.handle_searching_bib_mode(key),
            InputMode::ReadingSourceMd => self.handle_reading_source_md_mode(key),
            InputMode::SearchingViewingRefs => self.handle_searching_viewing_refs_mode(key),
        }
    }

    fn handle_help_overlay_mode(&mut self, _key: KeyEvent) {
        self.input_mode = self.saved_input_mode;
    }

    fn handle_normal_mode(&mut self, key: KeyEvent) {
        // Global shortcuts
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            self.save_all();
            return;
        }

        match key.code {
            KeyCode::Char('?') | KeyCode::F(1) => {
                self.toggle_help_overlay();
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.active_tab == ActiveTab::References
                    && !self.bib_search_query.is_empty()
                    && self.active_ref_pane == RefPane::LeftBib
                {
                    self.bib_search_query.clear();
                    self.clamp_bib_selection();
                } else if self.active_tab == ActiveTab::References
                    && !self.ref_search_query.is_empty()
                    && self.active_ref_pane == RefPane::RightSources
                {
                    self.ref_search_query.clear();
                    self.clamp_source_ref_selection();
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Tab => {
                if self.active_tab == ActiveTab::References {
                    self.active_ref_pane = match self.active_ref_pane {
                        RefPane::LeftBib => RefPane::RightSources,
                        RefPane::RightSources => RefPane::LeftBib,
                    };
                } else {
                    let current = self.active_tab as usize;
                    let next = (current + 1) % ActiveTab::ALL.len();
                    self.active_tab = ActiveTab::ALL[next];
                }
            }
            KeyCode::BackTab => {
                let current = self.active_tab as usize;
                let next = if current == 0 {
                    ActiveTab::ALL.len() - 1
                } else {
                    current - 1
                };
                self.active_tab = ActiveTab::ALL[next];
            }
            KeyCode::Char('1') => self.active_tab = ActiveTab::Dashboard,
            KeyCode::Char('2') => self.active_tab = ActiveTab::Sources,
            KeyCode::Char('3') => self.active_tab = ActiveTab::References,
            KeyCode::Char('4') => self.active_tab = ActiveTab::PaperDraft,
            KeyCode::Char('5') => self.active_tab = ActiveTab::Settings,

            KeyCode::Char('s') => {
                if self.active_tab == ActiveTab::References {
                    self.ref_sort_key = RefSortKey::Source;
                    self.sort_source_references();
                    self.clamp_source_ref_selection();
                } else {
                    self.save_all();
                }
            }
            KeyCode::Char('t') => {
                if self.active_tab == ActiveTab::References {
                    self.ref_sort_key = RefSortKey::Title;
                    self.sort_source_references();
                    self.clamp_source_ref_selection();
                    self.status_message = "Sorted references by Title.".to_string();
                }
            }
            KeyCode::Char('v') => {
                if self.active_tab == ActiveTab::References {
                    self.source_references
                        .sort_by_key(|r| r.venue.clone().unwrap_or_default());
                } else if self.active_tab == ActiveTab::Sources {
                    if !self.sources.is_empty() && self.selected_source_index < self.sources.len() {
                        let doc_id = self.sources[self.selected_source_index].id.clone();
                        let filename = self.sources[self.selected_source_index].filename.clone();
                        let ref_text = self.sources[self.selected_source_index]
                            .references_text
                            .clone();
                        self.load_and_view_references(&doc_id, &filename, ref_text.as_deref());
                    }
                } else if self.active_tab == ActiveTab::PaperDraft || self.project_root.is_some() {
                    self.pending_external_editor = true;
                    self.status_message =
                        "Launching external editor ($EDITOR / nvim / helix)...".to_string();
                }
            }

            KeyCode::PageUp => {
                if self.active_tab == ActiveTab::PaperDraft {
                    self.paper_scroll_offset = self.paper_scroll_offset.saturating_sub(5);
                } else if self.active_tab == ActiveTab::Sources {
                    self.source_scroll_offset = self.source_scroll_offset.saturating_sub(5);
                } else if self.active_tab == ActiveTab::References {
                    match self.active_ref_pane {
                        RefPane::LeftBib => {
                            self.selected_bib_index = self.selected_bib_index.saturating_sub(5);
                        }
                        RefPane::RightSources => {
                            self.selected_source_ref_index =
                                self.selected_source_ref_index.saturating_sub(5);
                        }
                    }
                }
            }
            KeyCode::PageDown => {
                if self.active_tab == ActiveTab::PaperDraft {
                    self.paper_scroll_offset += 5;
                } else if self.active_tab == ActiveTab::Sources {
                    self.source_scroll_offset += 5;
                } else if self.active_tab == ActiveTab::References {
                    match self.active_ref_pane {
                        RefPane::LeftBib => {
                            let count = self.filtered_bib_entries().len();
                            if count > 0 {
                                self.selected_bib_index =
                                    (self.selected_bib_index + 5).min(count - 1);
                            }
                        }
                        RefPane::RightSources => {
                            let count = self.filtered_source_references().len();
                            if count > 0 {
                                self.selected_source_ref_index =
                                    (self.selected_source_ref_index + 5).min(count - 1);
                            }
                        }
                    }
                }
            }

            KeyCode::Up | KeyCode::Char('k') => match self.active_tab {
                ActiveTab::Dashboard => {}
                ActiveTab::References => match self.active_ref_pane {
                    RefPane::LeftBib => {
                        if self.selected_bib_index > 0 {
                            self.selected_bib_index -= 1;
                        }
                    }
                    RefPane::RightSources => {
                        if self.selected_source_ref_index > 0 {
                            self.selected_source_ref_index -= 1;
                        }
                    }
                },
                ActiveTab::PaperDraft => {
                    if self.paper_section_index > 0 {
                        self.paper_section_index -= 1;
                        self.paper_scroll_offset = 0;
                    }
                }
                ActiveTab::Sources => {
                    if self.selected_source_index > 0 {
                        self.selected_source_index -= 1;
                        self.source_scroll_offset = 0;
                    }
                }
                ActiveTab::Settings => {
                    if self.selected_setting_index > 0 {
                        self.selected_setting_index -= 1;
                    }
                }
            },
            // Uppercase J / Shift+j: job history (before lowercase j navigation).
            KeyCode::Char('J') => {
                self.open_job_history();
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.open_job_history();
            }
            KeyCode::Down | KeyCode::Char('j') => self.navigate_down(),
            KeyCode::Enter => match self.active_tab {
                ActiveTab::PaperDraft => self.start_editing_selected_field(),
                ActiveTab::Sources => {
                    if !self.sources.is_empty() && self.selected_source_index < self.sources.len() {
                        let doc = &self.sources[self.selected_source_index];
                        let content = self.fetch_source_markdown_content(doc);
                        self.reading_md_content = Some(content);
                        self.input_mode = InputMode::ReadingSourceMd;
                        self.source_scroll_offset = 0;
                        self.status_message =
                            format!("Reading {}. Press Esc to exit.", doc.filename);
                    }
                }
                ActiveTab::Settings => self.start_editing_selected_field(),
                _ => {}
            },
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if self.active_tab == ActiveTab::Sources {
                    if !self.sources.is_empty() && self.selected_source_index < self.sources.len() {
                        let force = key.code == KeyCode::Char('E')
                            || key.modifiers.contains(KeyModifiers::SHIFT);
                        let doc = self.sources[self.selected_source_index].clone();
                        self.queue_source_parse(doc, force);
                    }
                } else if key.code == KeyCode::Char('e') {
                    self.start_editing_selected_field();
                }
            }

            // Actions for Sources & Settings
            KeyCode::Char('a') => match self.active_tab {
                ActiveTab::Sources => {
                    self.new_source_link_buffer.clear();
                    self.input_mode = InputMode::ModalAddSourceLink;
                    self.status_message =
                        "Fetch/download source — enter URL / DOI / arXiv (Enter to start fetch, Esc to cancel)"
                            .to_string();
                }
                ActiveTab::Settings => {
                    let items = self.setting_items();
                    if self.selected_setting_index < items.len() {
                        match items[self.selected_setting_index] {
                            SettingItem::CacheCoAuthor(_) | SettingItem::CacheCoAuthorEmpty => {
                                self.new_author = AuthorDetails::default();
                                self.modal_field_index = 0;
                                self.input_mode = InputMode::ModalAddAuthor;
                            }
                            SettingItem::CacheGrant(_) | SettingItem::CacheGrantEmpty => {
                                self.new_grant = GrantDetails::default();
                                self.modal_field_index = 0;
                                self.input_mode = InputMode::ModalAddGrant;
                            }
                            SettingItem::LocalCoAuthor(_) | SettingItem::LocalCoAuthorEmpty => {
                                if !self.cache.co_authors.is_empty() {
                                    self.selected_local_field = LocalField::CoAuthorsList as usize;
                                    self.input_mode = InputMode::ModalPicker;
                                    self.status_message = "Select co-author from cache (↑/↓ to navigate, Enter to select, Esc to cancel)".to_string();
                                } else {
                                    self.new_author = AuthorDetails::default();
                                    self.modal_field_index = 0;
                                    self.input_mode = InputMode::ModalAddAuthor;
                                }
                            }
                            SettingItem::LocalGrant(_) | SettingItem::LocalGrantEmpty => {
                                if !self.cache.grants.is_empty() {
                                    self.selected_local_field = LocalField::GrantsList as usize;
                                    self.input_mode = InputMode::ModalPicker;
                                    self.status_message = "Select grant from cache (↑/↓ to navigate, Enter to select, Esc to cancel)".to_string();
                                } else {
                                    self.new_grant = GrantDetails::default();
                                    self.modal_field_index = 0;
                                    self.input_mode = InputMode::ModalAddGrant;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            KeyCode::Char('R') => {
                if self.active_tab == ActiveTab::Sources {
                    self.reload_sources();
                    self.status_message = "✓ Reloaded sources".to_string();
                } else if self.active_tab == ActiveTab::Dashboard {
                    self.reload_paper_draft();
                    self.reload_sources();
                    self.load_project_references_bib();
                    self.refresh_dashboard();
                    self.status_message = "✓ Reloaded dashboard".to_string();
                }
            }
            KeyCode::Char('r') => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    if self.active_tab == ActiveTab::Sources {
                        self.reload_sources();
                        self.status_message = "✓ Reloaded sources".to_string();
                    }
                } else if self.active_tab == ActiveTab::Sources
                    && !self.sources.is_empty()
                    && self.selected_source_index < self.sources.len()
                {
                    let doc = &self.sources[self.selected_source_index];
                    self.rename_source_buffer =
                        doc.title.clone().unwrap_or_else(|| doc.filename.clone());
                    self.input_mode = InputMode::ModalRenameSource;
                    self.status_message =
                        "Enter new title for source (Enter to confirm, Esc to cancel)".to_string();
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => match self.active_tab {
                ActiveTab::Sources => {
                    if !self.sources.is_empty() && self.selected_source_index < self.sources.len() {
                        self.input_mode = InputMode::ConfirmDeleteSource;
                        self.status_message = format!(
                            "Delete source '{}'? Press 'y' or Enter to confirm, 'n' or Esc to cancel.",
                            self.sources[self.selected_source_index].filename
                        );
                    }
                }
                ActiveTab::Settings => {
                    let items = self.setting_items();
                    if self.selected_setting_index < items.len() {
                        match items[self.selected_setting_index] {
                            SettingItem::CacheCoAuthor(idx) => {
                                if idx < self.cache.co_authors.len() {
                                    self.cache.co_authors.remove(idx);
                                    self.dirty = true;
                                    self.status_message =
                                        "Removed co-author from cache.".to_string();
                                }
                            }
                            SettingItem::CacheGrant(idx) => {
                                if idx < self.cache.grants.len() {
                                    self.cache.grants.remove(idx);
                                    self.dirty = true;
                                    self.status_message = "Removed grant from cache.".to_string();
                                }
                            }
                            SettingItem::LocalCoAuthor(idx) => {
                                if idx < self.local_settings.co_authors.len() {
                                    self.local_settings.co_authors.remove(idx);
                                    self.dirty = true;
                                    self.status_message =
                                        "Removed co-author from local settings.".to_string();
                                }
                            }
                            SettingItem::LocalGrant(idx) => {
                                if idx < self.local_settings.grants.len() {
                                    self.local_settings.grants.remove(idx);
                                    self.dirty = true;
                                    self.status_message =
                                        "Removed grant from local settings.".to_string();
                                }
                            }
                            _ => {}
                        }
                        let total = self.setting_items().len();
                        if self.selected_setting_index >= total && total > 0 {
                            self.selected_setting_index = total - 1;
                        }
                    }
                }
                _ => {}
            },
            KeyCode::Char('u') => {
                if self.active_tab == ActiveTab::Settings {
                    let items = self.setting_items();
                    if self.selected_setting_index < items.len() {
                        match items[self.selected_setting_index] {
                            SettingItem::CacheCoAuthor(idx) => {
                                if idx < self.cache.co_authors.len() {
                                    let author = self.cache.co_authors[idx].clone();
                                    if !self.local_settings.co_authors.contains(&author) {
                                        self.local_settings.co_authors.push(author);
                                        self.dirty = true;
                                        self.status_message =
                                            "Added cached co-author to local project!".to_string();
                                    }
                                }
                            }
                            SettingItem::CacheGrant(idx) => {
                                if idx < self.cache.grants.len() {
                                    let grant = self.cache.grants[idx].clone();
                                    if !self.local_settings.grants.contains(&grant) {
                                        self.local_settings.grants.push(grant);
                                        self.dirty = true;
                                        self.status_message =
                                            "Added cached grant to local project!".to_string();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            KeyCode::Char('p') => {
                if self.active_tab == ActiveTab::References {
                    self.append_selected_extracted_refs_to_bib();
                }
            }
            KeyCode::Char('P') => {
                if self.active_tab == ActiveTab::References {
                    self.promote_selected_bib_entry();
                }
            }
            KeyCode::Char('/') | KeyCode::Char('f') => {
                if self.active_tab == ActiveTab::References {
                    match self.active_ref_pane {
                        RefPane::LeftBib => {
                            self.input_mode = InputMode::SearchingBib;
                            self.status_message =
                                "Search bib entries: type query, Enter/Esc to finish".to_string();
                        }
                        RefPane::RightSources => {
                            self.input_mode = InputMode::SearchingRefs;
                            self.status_message =
                                "Search right pane: type query, Enter/Esc to finish".to_string();
                        }
                    }
                }
            }
            KeyCode::Char('b') => {
                if self.active_tab == ActiveTab::Sources {
                    self.append_selected_source_to_bib();
                }
            }
            KeyCode::Char('m') | KeyCode::Char('c') => {
                if self.active_tab == ActiveTab::References {
                    self.ref_sort_key = RefSortKey::Similarity;
                    self.sort_source_references();
                    self.status_message =
                        "Sorted references by Draft Cosine Similarity (highest first).".to_string();
                }
            }
            KeyCode::Char('X') => {
                if self.active_tab == ActiveTab::References {
                    self.enqueue_similarity_job();
                }
            }
            KeyCode::Char('y') => {
                if self.active_tab == ActiveTab::References {
                    self.ref_sort_key = RefSortKey::Year;
                    self.sort_source_references();
                }
            }
            KeyCode::Char('i') => {
                if self.active_tab == ActiveTab::References {
                    self.ref_sort_key = RefSortKey::Index;
                    self.sort_source_references();
                }
            }
            KeyCode::Char(' ') => {
                if self.active_tab == ActiveTab::References
                    && self.active_ref_pane == RefPane::RightSources
                {
                    let filtered = self.filtered_source_references();
                    if self.selected_source_ref_index < filtered.len() {
                        let id = filtered[self.selected_source_ref_index].id.clone();
                        if self.marked_ref_ids.contains(&id) {
                            self.marked_ref_ids.remove(&id);
                        } else {
                            self.marked_ref_ids.insert(id);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn fetch_source_markdown_content(&self, doc: &SourceDocument) -> String {
        if let Some(ref root) = self.project_root {
            let paths = ProjectPaths::new(root);
            if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
                if let Ok(Some((_, content))) = db.get_source_content(doc.id.as_str()) {
                    if !content.trim().is_empty() {
                        return content;
                    }
                }
                if let Ok(Some((_, content))) = db.get_source_content(&doc.filename) {
                    if !content.trim().is_empty() {
                        return content;
                    }
                }
            }
        }
        if doc.path.is_file() {
            if let Ok(c) = std::fs::read_to_string(doc.path.as_std_path()) {
                return c;
            }
        }
        format!(
            "# {}\n\n{}",
            doc.title.as_deref().unwrap_or(&doc.filename),
            doc.abstract_text
                .as_deref()
                .unwrap_or("No markdown content available for this source.")
        )
    }

    fn load_and_view_references(
        &mut self,
        doc_id: &sil_core::SourceId,
        filename: &str,
        references_text: Option<&str>,
    ) {
        let mut refs = Vec::new();
        if let Some(ref root) = self.project_root {
            let paths = ProjectPaths::new(root);
            if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
                if let Ok(r) = db.get_references_for_source(doc_id) {
                    refs = r;
                }
            }
        }
        if refs.is_empty() {
            if let Some(rt) = references_text {
                for (idx, line) in rt.lines().enumerate() {
                    let clean = line.trim();
                    if !clean.is_empty() {
                        refs.push(ReferenceEntry {
                            id: format!("{}_ref_{}", filename, idx + 1),
                            source_id: doc_id.clone(),
                            ref_index: idx + 1,
                            raw_text: clean.to_string(),
                            title: None,
                            authors: None,
                            year: None,
                            venue: None,
                            doi: None,
                            arxiv_id: None,
                            url: None,
                        });
                    }
                }
            }
        }
        self.selected_source_references = refs;
        self.selected_viewing_ref_index = 0;
        self.viewing_ref_search_query.clear();
        self.viewing_ref_show_detail = true;
        self.ref_sort_key = RefSortKey::Index;
        self.input_mode = InputMode::ViewingSourceRefs;
        self.status_message = format!(
            "Viewing references for {filename}. Keys: 'j'/'k' Nav, 'c' Add to Bib, 'a' Add All, '/' Filter, 'd' Toggle Detail, Esc Close."
        );
    }

    pub fn load_and_view_all_references(&mut self) {
        let mut refs = Vec::new();
        if let Some(ref root) = self.project_root {
            let paths = ProjectPaths::new(root);
            if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
                if let Ok(r) = db.get_all_references() {
                    refs = r;
                }
            }
        }
        self.selected_source_references = refs;
        self.selected_viewing_ref_index = 0;
        self.viewing_ref_search_query.clear();
        self.viewing_ref_show_detail = true;
        self.ref_sort_key = RefSortKey::Index;
        self.input_mode = InputMode::ViewingSourceRefs;
        self.status_message = format!(
            "Viewing all {} references in project. Keys: 'j'/'k' Nav, 'c' Add to Bib, 'a' Add All, '/' Filter, 'd' Toggle Detail, Esc Close.",
            self.selected_source_references.len()
        );
    }

    pub(crate) fn start_editing_selected_field(&mut self) {
        match self.active_tab {
            ActiveTab::PaperDraft => {
                if !self.paper_sections.is_empty()
                    && self.paper_section_index < self.paper_sections.len()
                {
                    self.paper_edit_buffer =
                        self.paper_sections[self.paper_section_index].body.clone();
                } else {
                    self.paper_edit_buffer = self.paper_draft_content.clone();
                }
                self.input_mode = InputMode::EditingPaper;
                self.status_message =
                    "Editing section body. Press Enter to confirm, Esc to cancel.".to_string();
            }
            ActiveTab::Settings => {
                let items = self.setting_items();
                if self.selected_setting_index < items.len() {
                    match items[self.selected_setting_index] {
                        SettingItem::Global(f) => {
                            self.input_buffer = match f {
                                GlobalField::AuthorName => self.global_settings.author.name.clone(),
                                GlobalField::AuthorEmail => {
                                    self.global_settings.author.email.clone()
                                }
                                GlobalField::AuthorAffiliation => {
                                    self.global_settings.author.affiliation.clone()
                                }
                                GlobalField::AuthorOrcid => self
                                    .global_settings
                                    .author
                                    .orcid
                                    .clone()
                                    .unwrap_or_default(),
                                GlobalField::GrantFunder => {
                                    self.global_settings.default_grant.funder.clone()
                                }
                                GlobalField::GrantNumber => {
                                    self.global_settings.default_grant.grant_number.clone()
                                }
                                GlobalField::GrantAck => {
                                    self.global_settings.default_grant.acknowledgment.clone()
                                }
                                GlobalField::Engine => {
                                    self.global_settings.default_latex_engine.clone()
                                }
                                GlobalField::Template => {
                                    self.global_settings.default_template.clone()
                                }
                            };
                            self.input_mode = InputMode::Editing;
                            self.status_message =
                                "Editing global setting. Press Enter to confirm, Esc to cancel."
                                    .to_string();
                        }
                        SettingItem::Rag(f) => {
                            self.input_buffer = match f {
                                RagField::EmbedderPath => self
                                    .global_settings
                                    .rag
                                    .onnx_embedder_path
                                    .as_ref()
                                    .map(|p| p.to_string())
                                    .unwrap_or_default(),
                                RagField::RerankerPath => self
                                    .global_settings
                                    .rag
                                    .onnx_reranker_path
                                    .as_ref()
                                    .map(|p| p.to_string())
                                    .unwrap_or_default(),
                                RagField::ModelsDir => self
                                    .global_settings
                                    .rag
                                    .onnx_models_dir
                                    .as_ref()
                                    .map(|p| p.to_string())
                                    .unwrap_or_default(),
                                RagField::CacheDir => {
                                    self.global_settings.rag.model_cache_dir.to_string()
                                }
                                RagField::XbergCacheDir => {
                                    self.global_settings.rag.xberg_model_cache_dir.to_string()
                                }
                                RagField::ExecutionProvider => {
                                    self.global_settings.rag.execution_provider.clone()
                                }
                                RagField::NumThreads => {
                                    self.global_settings.rag.num_threads.to_string()
                                }
                                RagField::ParentChunkSize => {
                                    self.global_settings.rag.parent_chunk_size.to_string()
                                }
                                RagField::ChildChunkSize => {
                                    self.global_settings.rag.child_chunk_size.to_string()
                                }
                            };
                            self.input_mode = InputMode::Editing;
                            self.status_message =
                                "Editing RAG setting. Press Enter to confirm, Esc to cancel."
                                    .to_string();
                        }
                        SettingItem::Digest(f) => {
                            self.input_buffer = match f {
                                DigestField::GlobalQuery => {
                                    self.global_settings.digest_query.clone()
                                }
                                DigestField::RefreshHours => {
                                    self.global_settings.digest_refresh_hours.to_string()
                                }
                                DigestField::LocalQuery => self.local_settings.digest_query.clone(),
                            };
                            self.input_mode = InputMode::Editing;
                            self.status_message =
                                "Editing digest setting. Press Enter to confirm, Esc to cancel."
                                    .to_string();
                        }
                        SettingItem::LocalTitle => {
                            self.input_buffer = self.local_settings.title.clone();
                            self.input_mode = InputMode::Editing;
                            self.status_message =
                                "Editing project title. Press Enter to confirm, Esc to cancel."
                                    .to_string();
                        }
                        SettingItem::LocalNotes => {
                            self.input_buffer = self.local_settings.notes.clone();
                            self.input_mode = InputMode::Editing;
                            self.status_message =
                                "Editing project notes. Press Enter to confirm, Esc to cancel."
                                    .to_string();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_searching_refs_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::F(1) => self.toggle_help_overlay(),
            KeyCode::Enter | KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.status_message = "Ready. Press 'Tab' to switch views, 'e' to edit section, 'v' for external $EDITOR, 's' to save.".to_string();
            }
            KeyCode::Backspace => {
                self.ref_search_query.pop();
                self.clamp_source_ref_selection();
            }
            KeyCode::Char(c) => {
                self.ref_search_query.push(c);
                self.clamp_source_ref_selection();
            }
            _ => {}
        }
    }

    fn handle_searching_bib_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::F(1) => self.toggle_help_overlay(),
            KeyCode::Enter | KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.status_message = "Ready. Press 'Tab' to switch views, 'e' to edit section, 'v' for external $EDITOR, 's' to save.".to_string();
            }
            KeyCode::Backspace => {
                self.bib_search_query.pop();
                self.clamp_bib_selection();
            }
            KeyCode::Char(c) => {
                self.bib_search_query.push(c);
                self.clamp_bib_selection();
            }
            _ => {}
        }
    }

    fn handle_editing_paper_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::F(1) => self.toggle_help_overlay(),
            KeyCode::Enter => {
                self.commit_edited_paper();
                self.input_mode = InputMode::Normal;
                self.dirty = true;
                self.status_message =
                    "Section body updated (unsaved changes). Press 's' to save.".to_string();
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.status_message = "Section edit cancelled.".to_string();
            }
            KeyCode::Backspace => {
                self.paper_edit_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.paper_edit_buffer.push(c);
            }
            _ => {}
        }
    }

    fn commit_edited_paper(&mut self) {
        if !self.paper_sections.is_empty() && self.paper_section_index < self.paper_sections.len() {
            self.paper_sections[self.paper_section_index].body = self.paper_edit_buffer.clone();
            let mut out = String::new();
            for sec in &self.paper_sections {
                if sec.kind != "document" {
                    out.push_str(&format!("\\{}{{{}}}\n", sec.kind, sec.title));
                }
                out.push_str(&sec.body);
                if !sec.body.ends_with('\n') {
                    out.push('\n');
                }
            }
            self.paper_draft_content = out;
        } else {
            self.paper_draft_content = self.paper_edit_buffer.clone();
            self.paper_sections = sil_latex::split_tex_sections(&self.paper_draft_content);
        }
    }

    fn handle_editing_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::F(1) => self.toggle_help_overlay(),
            KeyCode::Enter => {
                self.commit_edited_field();
                self.input_mode = InputMode::Normal;
                self.dirty = true;
                self.status_message = "Field updated (unsaved changes).".to_string();
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.status_message = "Edit cancelled.".to_string();
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
    }

    fn commit_edited_field(&mut self) {
        let val = self.input_buffer.trim().to_string();
        if self.active_tab == ActiveTab::Settings {
            let items = self.setting_items();
            if self.selected_setting_index < items.len() {
                match items[self.selected_setting_index] {
                    SettingItem::Global(f) => match f {
                        GlobalField::AuthorName => self.global_settings.author.name = val,
                        GlobalField::AuthorEmail => self.global_settings.author.email = val,
                        GlobalField::AuthorAffiliation => {
                            self.global_settings.author.affiliation = val
                        }
                        GlobalField::AuthorOrcid => {
                            self.global_settings.author.orcid =
                                if val.is_empty() { None } else { Some(val) }
                        }
                        GlobalField::GrantFunder => self.global_settings.default_grant.funder = val,
                        GlobalField::GrantNumber => {
                            self.global_settings.default_grant.grant_number = val
                        }
                        GlobalField::GrantAck => {
                            self.global_settings.default_grant.acknowledgment = val
                        }
                        GlobalField::Engine => self.global_settings.default_latex_engine = val,
                        GlobalField::Template => self.global_settings.default_template = val,
                    },
                    SettingItem::Rag(f) => match f {
                        RagField::EmbedderPath => {
                            let resolved = resolve_onnx_from_dir(&val);
                            self.global_settings.rag.onnx_embedder_path = if resolved.is_empty() {
                                None
                            } else {
                                Some(camino::Utf8PathBuf::from(resolved))
                            };
                        }
                        RagField::RerankerPath => {
                            let resolved = resolve_onnx_from_dir(&val);
                            self.global_settings.rag.onnx_reranker_path = if resolved.is_empty() {
                                None
                            } else {
                                Some(camino::Utf8PathBuf::from(resolved))
                            };
                        }
                        RagField::CacheDir => {
                            self.global_settings.rag.model_cache_dir =
                                camino::Utf8PathBuf::from(val)
                        }
                        RagField::XbergCacheDir => {
                            self.global_settings.rag.xberg_model_cache_dir =
                                camino::Utf8PathBuf::from(val)
                        }
                        RagField::ModelsDir => {
                            self.global_settings.rag.onnx_models_dir = if val.is_empty() {
                                None
                            } else {
                                Some(camino::Utf8PathBuf::from(val))
                            };
                        }
                        RagField::ExecutionProvider => {
                            self.global_settings.rag.execution_provider = val
                        }
                        RagField::NumThreads => {
                            if let Ok(n) = val.parse::<usize>() {
                                self.global_settings.rag.num_threads = n;
                            }
                        }
                        RagField::ParentChunkSize => {
                            if let Ok(n) = val.parse::<usize>() {
                                self.global_settings.rag.parent_chunk_size = n;
                            }
                        }
                        RagField::ChildChunkSize => {
                            if let Ok(n) = val.parse::<usize>() {
                                self.global_settings.rag.child_chunk_size = n;
                            }
                        }
                    },
                    SettingItem::Digest(f) => match f {
                        DigestField::GlobalQuery => {
                            self.global_settings.digest_query = val;
                        }
                        DigestField::RefreshHours => {
                            if let Ok(n) = val.parse::<u32>() {
                                self.global_settings.digest_refresh_hours =
                                    sil_core::effective_digest_refresh_hours(n);
                            }
                        }
                        DigestField::LocalQuery => {
                            self.local_settings.digest_query = val;
                        }
                    },
                    SettingItem::LocalTitle => self.local_settings.title = val,
                    SettingItem::LocalNotes => self.local_settings.notes = val,
                    _ => {}
                }
            }
        }
    }

    fn handle_modal_picker_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::F(1) => {
                self.toggle_help_overlay();
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.status_message = "Picker closed.".to_string();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_local_field == LocalField::CoAuthorsList as usize
                    && self.cache_coauthor_index > 0
                {
                    self.cache_coauthor_index -= 1;
                } else if self.selected_local_field == LocalField::GrantsList as usize
                    && self.cache_grant_index > 0
                {
                    self.cache_grant_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_local_field == LocalField::CoAuthorsList as usize
                    && !self.cache.co_authors.is_empty()
                    && self.cache_coauthor_index + 1 < self.cache.co_authors.len()
                {
                    self.cache_coauthor_index += 1;
                } else if self.selected_local_field == LocalField::GrantsList as usize
                    && !self.cache.grants.is_empty()
                    && self.cache_grant_index + 1 < self.cache.grants.len()
                {
                    self.cache_grant_index += 1;
                }
            }
            KeyCode::Enter => {
                if self.selected_local_field == LocalField::CoAuthorsList as usize
                    && !self.cache.co_authors.is_empty()
                {
                    let author = self.cache.co_authors[self.cache_coauthor_index].clone();
                    if !self.local_settings.co_authors.contains(&author) {
                        self.local_settings.co_authors.push(author);
                    }
                } else if self.selected_local_field == LocalField::GrantsList as usize
                    && !self.cache.grants.is_empty()
                {
                    let grant = self.cache.grants[self.cache_grant_index].clone();
                    if !self.local_settings.grants.contains(&grant) {
                        self.local_settings.grants.push(grant);
                    }
                }
                self.input_mode = InputMode::Normal;
                self.dirty = true;
                self.status_message = "Added from cache to local project settings!".to_string();
            }
            KeyCode::Char('n') => {
                if self.selected_local_field == LocalField::CoAuthorsList as usize {
                    self.new_author = AuthorDetails::default();
                    self.modal_field_index = 0;
                    self.input_mode = InputMode::ModalAddAuthor;
                } else {
                    self.new_grant = GrantDetails::default();
                    self.modal_field_index = 0;
                    self.input_mode = InputMode::ModalAddGrant;
                }
            }
            _ => {}
        }
    }

    fn handle_modal_add_author_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::F(1) => self.toggle_help_overlay(),
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.status_message = "Add co-author cancelled.".to_string();
            }
            KeyCode::Tab | KeyCode::Down => {
                self.modal_field_index = (self.modal_field_index + 1) % 4;
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.modal_field_index = if self.modal_field_index == 0 {
                    3
                } else {
                    self.modal_field_index - 1
                };
            }
            KeyCode::Enter => {
                if !self.new_author.name.trim().is_empty() {
                    let author = self.new_author.clone();
                    self.cache.remember_co_author(author.clone());
                    if !self.local_settings.co_authors.contains(&author) {
                        self.local_settings.co_authors.push(author);
                    }
                    self.dirty = true;
                    self.input_mode = InputMode::Normal;
                    self.status_message =
                        "Co-author saved to cache and local settings!".to_string();
                } else {
                    self.status_message = "Author name cannot be empty.".to_string();
                }
            }
            KeyCode::Backspace => {
                let target = match self.modal_field_index {
                    0 => &mut self.new_author.name,
                    1 => &mut self.new_author.email,
                    2 => &mut self.new_author.affiliation,
                    _ => {
                        let mut s = self.new_author.orcid.clone().unwrap_or_default();
                        s.pop();
                        self.new_author.orcid = if s.is_empty() { None } else { Some(s) };
                        return;
                    }
                };
                target.pop();
            }
            KeyCode::Char(c) => {
                let target = match self.modal_field_index {
                    0 => &mut self.new_author.name,
                    1 => &mut self.new_author.email,
                    2 => &mut self.new_author.affiliation,
                    _ => {
                        let mut s = self.new_author.orcid.clone().unwrap_or_default();
                        s.push(c);
                        self.new_author.orcid = Some(s);
                        return;
                    }
                };
                target.push(c);
            }
            _ => {}
        }
    }

    fn handle_modal_add_grant_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::F(1) => self.toggle_help_overlay(),
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.status_message = "Add grant cancelled.".to_string();
            }
            KeyCode::Tab | KeyCode::Down => {
                self.modal_field_index = (self.modal_field_index + 1) % 3;
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.modal_field_index = if self.modal_field_index == 0 {
                    2
                } else {
                    self.modal_field_index - 1
                };
            }
            KeyCode::Enter => {
                if !self.new_grant.funder.trim().is_empty()
                    || !self.new_grant.grant_number.trim().is_empty()
                {
                    let grant = self.new_grant.clone();
                    self.cache.remember_grant(grant.clone());
                    if !self.local_settings.grants.contains(&grant) {
                        self.local_settings.grants.push(grant);
                    }
                    self.dirty = true;
                    self.input_mode = InputMode::Normal;
                    self.status_message = "Grant saved to cache and local settings!".to_string();
                } else {
                    self.status_message = "Grant funder or number required.".to_string();
                }
            }
            KeyCode::Backspace => {
                let target = match self.modal_field_index {
                    0 => &mut self.new_grant.funder,
                    1 => &mut self.new_grant.grant_number,
                    _ => &mut self.new_grant.acknowledgment,
                };
                target.pop();
            }
            KeyCode::Char(c) => {
                let target = match self.modal_field_index {
                    0 => &mut self.new_grant.funder,
                    1 => &mut self.new_grant.grant_number,
                    _ => &mut self.new_grant.acknowledgment,
                };
                target.push(c);
            }
            _ => {}
        }
    }

    fn navigate_down(&mut self) {
        match self.active_tab {
            ActiveTab::Dashboard => {}
            ActiveTab::References => match self.active_ref_pane {
                RefPane::LeftBib => {
                    let count = self.filtered_bib_entries().len();
                    if count > 0 && self.selected_bib_index + 1 < count {
                        self.selected_bib_index += 1;
                    }
                }
                RefPane::RightSources => {
                    let count = self.filtered_source_references().len();
                    if count > 0 && self.selected_source_ref_index + 1 < count {
                        self.selected_source_ref_index += 1;
                    }
                }
            },
            ActiveTab::PaperDraft => {
                if !self.paper_sections.is_empty()
                    && self.paper_section_index + 1 < self.paper_sections.len()
                {
                    self.paper_section_index += 1;
                    self.paper_scroll_offset = 0;
                }
            }
            ActiveTab::Sources => {
                if !self.sources.is_empty() && self.selected_source_index + 1 < self.sources.len() {
                    self.selected_source_index += 1;
                    self.source_scroll_offset = 0;
                }
            }
            ActiveTab::Settings => {
                let total = self.setting_items().len();
                if total > 0 && self.selected_setting_index + 1 < total {
                    self.selected_setting_index += 1;
                }
            }
        }
    }

    fn open_job_history(&mut self) {
        if self.recent_job_outcomes.is_empty() {
            self.selected_job_history_index = 0;
        } else if self.selected_job_history_index >= self.recent_job_outcomes.len() {
            self.selected_job_history_index = self.recent_job_outcomes.len() - 1;
        }
        self.input_mode = InputMode::JobHistory;
        self.status_message =
            "Job history — ↑/↓ navigate, Enter/r retry failed, Esc close".to_string();
    }

    fn handle_job_history_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::F(1) | KeyCode::Char('?') => self.toggle_help_overlay(),
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('J') => {
                self.input_mode = InputMode::Normal;
                self.status_message = "Closed job history.".to_string();
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.input_mode = InputMode::Normal;
                self.status_message = "Closed job history.".to_string();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.recent_job_outcomes.is_empty()
                    && self.selected_job_history_index + 1 < self.recent_job_outcomes.len()
                {
                    self.selected_job_history_index += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_job_history_index > 0 {
                    self.selected_job_history_index -= 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('r') => {
                let idx = self.selected_job_history_index;
                self.retry_job_outcome(idx);
            }
            _ => {}
        }
    }

    fn handle_modal_add_source_link_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::F(1) => self.toggle_help_overlay(),
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.status_message = "Add source cancelled.".to_string();
            }
            KeyCode::Enter => {
                let link = self.new_source_link_buffer.trim().to_string();
                if !link.is_empty() {
                    self.queue_source_fetch(link);
                } else {
                    self.status_message = "Empty URL / DOI / arXiv — nothing to fetch".to_string();
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                self.new_source_link_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.new_source_link_buffer.push(c);
            }
            _ => {}
        }
    }

    fn handle_modal_rename_source_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::F(1) => self.toggle_help_overlay(),
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.status_message = "Rename cancelled.".to_string();
            }
            KeyCode::Enter => {
                let new_title = self.rename_source_buffer.trim().to_string();
                if !new_title.is_empty() && self.selected_source_index < self.sources.len() {
                    let doc = &mut self.sources[self.selected_source_index];
                    doc.title = Some(new_title.clone());
                    if let Some(ref root) = self.project_root {
                        let paths = ProjectPaths::new(root);
                        if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
                            let _ = db.update_source_title(&doc.id, &new_title);
                        }
                    }
                    self.dirty = true;
                    self.status_message = format!("Renamed source title to '{new_title}'.");
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                self.rename_source_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.rename_source_buffer.push(c);
            }
            _ => {}
        }
    }

    fn handle_confirm_delete_source_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::F(1) => {
                self.toggle_help_overlay();
            }
            KeyCode::Char('y') | KeyCode::Enter => {
                if !self.sources.is_empty() && self.selected_source_index < self.sources.len() {
                    let doc = self.sources.remove(self.selected_source_index);
                    if let Some(ref root) = self.project_root {
                        let paths = ProjectPaths::new(root);
                        if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
                            let _ = db.remove_source(&doc.id);
                        }
                        if doc.path.is_file() {
                            let _ = std::fs::remove_file(doc.path.as_std_path());
                        }
                    }
                    if self.selected_source_index >= self.sources.len() && !self.sources.is_empty()
                    {
                        self.selected_source_index = self.sources.len() - 1;
                    }
                    self.dirty = true;
                    self.status_message = format!("Deleted source '{}'.", doc.filename);
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.status_message = "Delete source cancelled.".to_string();
            }
            _ => {}
        }
    }

    fn handle_viewing_source_refs_mode(&mut self, key: KeyEvent) {
        let count = self.filtered_viewing_source_references().len();
        match key.code {
            KeyCode::Char('?') | KeyCode::F(1) => {
                self.toggle_help_overlay();
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                if !self.viewing_ref_search_query.is_empty() {
                    self.viewing_ref_search_query.clear();
                    self.clamp_viewing_ref_selection();
                } else {
                    self.input_mode = InputMode::Normal;
                    self.selected_source_references.clear();
                    self.status_message = "Closed references window.".to_string();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_viewing_ref_index > 0 {
                    self.selected_viewing_ref_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if count > 0 && self.selected_viewing_ref_index + 1 < count {
                    self.selected_viewing_ref_index += 1;
                }
            }
            KeyCode::PageUp => {
                self.selected_viewing_ref_index = self.selected_viewing_ref_index.saturating_sub(5);
            }
            KeyCode::PageDown => {
                if count > 0 {
                    self.selected_viewing_ref_index =
                        (self.selected_viewing_ref_index + 5).min(count - 1);
                }
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.selected_viewing_ref_index = 0;
            }
            KeyCode::End | KeyCode::Char('G') => {
                if count > 0 {
                    self.selected_viewing_ref_index = count - 1;
                }
            }
            KeyCode::Char('y') => {
                self.ref_sort_key = RefSortKey::Year;
                self.selected_source_references
                    .sort_by_key(|b| std::cmp::Reverse(b.year.unwrap_or(0)));
                self.clamp_viewing_ref_selection();
                self.status_message = "Sorted references by Year (descending).".to_string();
            }
            KeyCode::Char('v') => {
                self.ref_sort_key = RefSortKey::Venue;
                self.selected_source_references.sort_by(|a, b| {
                    a.venue
                        .as_deref()
                        .unwrap_or("")
                        .cmp(b.venue.as_deref().unwrap_or(""))
                });
                self.clamp_viewing_ref_selection();
                self.status_message =
                    "Sorted references by Journal/Conference (Venue).".to_string();
            }
            KeyCode::Char('s') => {
                self.ref_sort_key = RefSortKey::Source;
                self.selected_source_references
                    .sort_by(|a, b| a.source_id.as_str().cmp(b.source_id.as_str()));
                self.clamp_viewing_ref_selection();
                self.status_message = "Sorted references by Source document.".to_string();
            }
            KeyCode::Char('i') | KeyCode::Char('n') => {
                self.ref_sort_key = RefSortKey::Index;
                self.selected_source_references.sort_by_key(|a| a.ref_index);
                self.clamp_viewing_ref_selection();
                self.status_message = "Sorted references by original Index.".to_string();
            }
            KeyCode::Char('t') => {
                self.selected_source_references.sort_by(|a, b| {
                    a.title
                        .as_deref()
                        .unwrap_or(&a.raw_text)
                        .cmp(b.title.as_deref().unwrap_or(&b.raw_text))
                });
                self.clamp_viewing_ref_selection();
                self.status_message = "Sorted references by Title.".to_string();
            }
            KeyCode::Char('c') | KeyCode::Char('b') | KeyCode::Char('p') => {
                self.append_selected_viewing_ref_to_bib();
            }
            KeyCode::Char('a') => {
                self.append_all_viewing_refs_to_bib();
            }
            KeyCode::Char(' ') => {
                let filtered = self.filtered_viewing_source_references();
                if self.selected_viewing_ref_index < filtered.len() {
                    let id = filtered[self.selected_viewing_ref_index].id.clone();
                    if self.marked_ref_ids.contains(&id) {
                        self.marked_ref_ids.remove(&id);
                    } else {
                        self.marked_ref_ids.insert(id);
                    }
                }
            }
            KeyCode::Char('/') | KeyCode::Char('f') => {
                self.input_mode = InputMode::SearchingViewingRefs;
            }
            KeyCode::Char('d') | KeyCode::Char('e') => {
                self.viewing_ref_show_detail = !self.viewing_ref_show_detail;
            }
            _ => {}
        }
    }

    fn handle_searching_viewing_refs_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::F(1) => self.toggle_help_overlay(),
            KeyCode::Esc | KeyCode::Enter => {
                self.input_mode = InputMode::ViewingSourceRefs;
            }
            KeyCode::Backspace => {
                self.viewing_ref_search_query.pop();
                self.clamp_viewing_ref_selection();
            }
            KeyCode::Char(c) => {
                self.viewing_ref_search_query.push(c);
                self.clamp_viewing_ref_selection();
            }
            _ => {}
        }
    }

    fn handle_reading_source_md_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('?') | KeyCode::F(1) => {
                self.toggle_help_overlay();
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.input_mode = InputMode::Normal;
                self.reading_md_content = None;
                self.status_message = "Exited Markdown reader.".to_string();
            }
            KeyCode::Char('b') => {
                self.append_selected_source_to_bib();
            }
            KeyCode::Char('n') => {
                self.capture_note_buffer.clear();
                self.input_mode = InputMode::ModalCaptureNote;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.source_scroll_offset = self.source_scroll_offset.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.source_scroll_offset += 1;
            }
            KeyCode::PageUp => {
                self.source_scroll_offset = self.source_scroll_offset.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.source_scroll_offset += 10;
            }
            _ => {}
        }
    }

    fn handle_modal_capture_note_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::F(1) => self.toggle_help_overlay(),
            KeyCode::Esc => {
                self.input_mode = InputMode::ReadingSourceMd;
            }
            KeyCode::Enter => {
                let note = self.capture_note_buffer.trim().to_string();
                if !note.is_empty() {
                    self.save_reader_note(&note);
                }
                self.input_mode = InputMode::ReadingSourceMd;
            }
            KeyCode::Backspace => {
                self.capture_note_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.capture_note_buffer.push(c);
            }
            _ => {}
        }
    }

    fn short_hash(s: &str) -> String {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in s.as_bytes() {
            hash ^= u64::from(*b);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:08x}", (hash ^ (hash >> 32)) as u32)
    }

    pub fn save_reader_note(&mut self, note: &str) {
        if self.sources.is_empty() || self.selected_source_index >= self.sources.len() {
            self.status_message = "No active source document selected.".to_string();
            return;
        }
        let doc = self.sources[self.selected_source_index].clone();
        let root = match self.project_root.as_ref() {
            Some(r) => r,
            None => {
                self.status_message = "Error: paper_draft.tex not found.".to_string();
                return;
            }
        };
        let draft_path = root.join("paper_draft.tex");
        if !draft_path.is_file() {
            self.status_message = "Error: paper_draft.tex not found.".to_string();
            return;
        }

        let existing = match std::fs::read_to_string(draft_path.as_std_path()) {
            Ok(c) => c,
            Err(e) => {
                self.status_message = format!("Error reading paper_draft.tex: {e}");
                return;
            }
        };

        let note_hash = Self::short_hash(note);
        let block_id = format!("from-{}-{}", doc.id.as_str(), note_hash);
        let content = format!("from: {}\n{}", doc.filename, note);

        let mut block = sil_core::IdeaBlock::new(block_id, content, None, 0, 0);
        block.status = "open".to_string();
        block.priority = "medium".to_string();
        block.author_type = "human".to_string();
        block.tags = vec!["from-source".to_string()];

        let updated = sil_latex::update_or_insert_idea_block(&existing, &block);
        if let Err(e) = sil_core::write_atomic_str(&draft_path, &updated) {
            self.status_message = format!("Error writing paper_draft.tex: {e}");
        } else {
            let paths = ProjectPaths::new(root);
            let _ = sil_latex::write_draft_sections_from_file(
                &draft_path,
                &paths.draft_sections_dir(),
            );
            if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
                let ideas = sil_latex::parse_idea_blocks(&updated);
                let _ = db.replace_todo_ideas(&ideas);
            }
            self.reload_paper_draft();
            self.status_message = format!("Parked note from {}", doc.filename);
        }
    }

    pub fn save_all(&mut self) {
        let mut messages = Vec::new();

        // 1. Save global settings
        if let Err(e) = self.global_settings.save(None) {
            messages.push(format!("Global save error: {e}"));
        } else {
            messages.push("Global settings saved".to_string());
        }

        // 2. Save cache
        if let Err(e) = self.cache.save(None) {
            messages.push(format!("Cache save error: {e}"));
        } else {
            messages.push("Cache saved".to_string());
        }

        // 3. Save local project settings if inside a project
        if let Some(ref root) = self.project_root {
            let paths = ProjectPaths::new(root);
            let config_path = paths.config();
            if config_path.exists() {
                if let Ok(mut cfg) = Config::load(&config_path) {
                    cfg.settings = self.local_settings.clone();
                    if !self.local_settings.title.is_empty() {
                        cfg.project.title = self.local_settings.title.clone();
                    }
                    if let Ok(yaml) = cfg.to_yaml() {
                        if sil_core::write_atomic_str(&config_path, &yaml).is_ok() {
                            messages.push("Local config.yaml updated".to_string());
                        }
                    }
                }
            }

            // 4. Save paper_draft.tex if present
            if !self.paper_draft_content.is_empty() {
                let draft_path = root.join("paper_draft.tex");
                if sil_core::write_atomic_str(&draft_path, &self.paper_draft_content).is_ok() {
                    let _ = sil_latex::write_draft_sections_from_file(
                        &draft_path,
                        &paths.draft_sections_dir(),
                    );
                    if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
                        let ideas = sil_latex::parse_idea_blocks(&self.paper_draft_content);
                        let _ = db.replace_todo_ideas(&ideas);
                    }
                    messages.push("paper_draft.tex saved & re-indexed".to_string());
                }
            }
        }

        self.dirty = false;
        self.status_message = format!("✓ {}", messages.join(" | "));
    }
}
