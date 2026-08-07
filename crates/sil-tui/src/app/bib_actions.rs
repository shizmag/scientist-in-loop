//! Bibliography, reference loading, sorting, filtering, and entry manipulation for `sil-tui`.

use super::*;
use sil_core::{ProjectPaths, ReferenceEntry, SourceDocument};

impl App {
    pub fn reload_sources(&mut self) {
        let mut sources = Vec::new();
        if let Some(ref root) = self.project_root {
            let paths = ProjectPaths::new(root);
            if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
                if let Ok(docs) = db.list_sources() {
                    sources = docs;
                }
            }
            let sources_dir = root.join("sources");
            if sources_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(sources_dir.as_std_path()) {
                    let existing_ids: std::collections::HashSet<_> =
                        sources.iter().map(|d| d.filename.clone()).collect();
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.to_ascii_lowercase().starts_with("readme") {
                            continue;
                        }
                        let path_buf = Utf8PathBuf::from_path_buf(entry.path()).unwrap_or_default();
                        let ext = path_buf.extension().unwrap_or("");
                        if matches!(
                            ext.to_ascii_lowercase().as_str(),
                            "pdf" | "md" | "markdown" | "txt" | "html" | "htm"
                        ) && !existing_ids.contains(&name)
                        {
                            sources.push(SourceDocument::new(path_buf));
                        }
                    }
                }
            }
        }
        self.sources = sources;
        if self.selected_source_index >= self.sources.len() && !self.sources.is_empty() {
            self.selected_source_index = self.sources.len() - 1;
        }
    }

    pub fn setting_items(&self) -> Vec<SettingItem> {
        let mut items = Vec::new();
        // 1. Global Settings
        for f in GlobalField::ALL {
            items.push(SettingItem::Global(f));
        }
        // 2. RAG Settings
        for f in RagField::ALL {
            items.push(SettingItem::Rag(f));
        }
        // 3. Cache section
        if self.cache.co_authors.is_empty() {
            items.push(SettingItem::CacheCoAuthorEmpty);
        } else {
            for i in 0..self.cache.co_authors.len() {
                items.push(SettingItem::CacheCoAuthor(i));
            }
        }
        if self.cache.grants.is_empty() {
            items.push(SettingItem::CacheGrantEmpty);
        } else {
            for i in 0..self.cache.grants.len() {
                items.push(SettingItem::CacheGrant(i));
            }
        }
        // 4. Local Settings
        items.push(SettingItem::LocalTitle);
        items.push(SettingItem::LocalNotes);
        if self.local_settings.co_authors.is_empty() {
            items.push(SettingItem::LocalCoAuthorEmpty);
        } else {
            for i in 0..self.local_settings.co_authors.len() {
                items.push(SettingItem::LocalCoAuthor(i));
            }
        }
        if self.local_settings.grants.is_empty() {
            items.push(SettingItem::LocalGrantEmpty);
        } else {
            for i in 0..self.local_settings.grants.len() {
                items.push(SettingItem::LocalGrant(i));
            }
        }
        items
    }

    pub fn reload_paper_draft(&mut self) {
        if let Some(ref root) = self.project_root {
            let draft_path = root.join("paper_draft.tex");
            if draft_path.is_file() {
                if let Ok(content) = std::fs::read_to_string(draft_path.as_std_path()) {
                    self.paper_draft_content = content;
                    self.paper_sections = sil_latex::split_tex_sections(&self.paper_draft_content);
                }
            }
        }
    }

    pub fn load_project_references_bib(&mut self) {
        self.bib_file_entries.clear();
        if let Some(ref root) = self.project_root {
            let bib_path = root.join("references.bib");
            if bib_path.is_file() {
                if let Ok(content) = std::fs::read_to_string(bib_path.as_std_path()) {
                    self.bib_file_entries = sil_core::parse_bib_blocks(&content);
                }
            }
        }
    }

    pub fn load_all_source_references(&mut self) {
        if let Some(ref root) = self.project_root {
            let paths = ProjectPaths::new(root);
            if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
                if let Ok(refs) = db.get_all_references() {
                    self.source_references = refs;
                }
                if let Ok(sims) = db.get_draft_ref_similarities() {
                    self.draft_ref_similarities = sims;
                }
                if let Ok(hash) = db.get_draft_similarity_hash() {
                    self.draft_similarity_hash = hash;
                }
            }
            self.check_draft_staleness();
            self.sort_source_references();
        }
    }

    pub fn check_draft_staleness(&mut self) {
        if let Some(ref root) = self.project_root {
            let paths = ProjectPaths::new(root);
            let draft_path = paths.paper_draft();
            if draft_path.exists()
                && let Ok(text) = std::fs::read_to_string(draft_path.as_std_path())
            {
                let clean = sil_core::strip_latex_for_embed(&text);
                let current_hash = sil_core::compute_draft_hash(&clean);
                if let Some(ref db_hash) = self.draft_similarity_hash {
                    if db_hash != &current_hash {
                        self.status_message =
                            "⚠ Draft updated — press 'm' / 'X' to recompute similarity".to_string();
                    }
                }
            }
        }
    }

    pub fn current_help_mode(&self) -> HelpMode {
        let mode = if self.input_mode == InputMode::HelpOverlay {
            self.saved_input_mode
        } else {
            self.input_mode
        };

        match mode {
            InputMode::HelpOverlay => HelpMode::Dashboard,
            InputMode::ReadingSourceMd => HelpMode::ReadingSourceMd,
            InputMode::ViewingSourceRefs => HelpMode::ViewingSourceRefs,
            InputMode::SearchingViewingRefs => HelpMode::SearchingViewingRefs,
            InputMode::SearchingRefs => HelpMode::SearchingRefs,
            InputMode::SearchingBib => HelpMode::SearchingBib,
            InputMode::ModalPicker => HelpMode::ModalPicker,
            InputMode::ModalAddAuthor => HelpMode::ModalAddAuthor,
            InputMode::ModalAddGrant => HelpMode::ModalAddGrant,
            InputMode::ModalAddSourceLink => HelpMode::ModalAddSourceLink,
            InputMode::ModalRenameSource => HelpMode::ModalRenameSource,
            InputMode::ConfirmDeleteSource => HelpMode::ConfirmDeleteSource,
            InputMode::JobHistory => HelpMode::JobHistory,
            InputMode::Editing => HelpMode::Editing,
            InputMode::EditingPaper => HelpMode::EditingPaper,
            InputMode::Normal => match self.active_tab {
                ActiveTab::Dashboard => HelpMode::Dashboard,
                ActiveTab::Sources => HelpMode::SourcesList,
                ActiveTab::References => match self.active_ref_pane {
                    RefPane::LeftBib => HelpMode::ReferencesLeft,
                    RefPane::RightSources => HelpMode::ReferencesRight,
                },
                ActiveTab::PaperDraft => HelpMode::PaperDraft,
                ActiveTab::Settings => HelpMode::Settings,
            },
        }
    }

    pub fn toggle_help_overlay(&mut self) {
        if self.input_mode == InputMode::HelpOverlay {
            self.input_mode = self.saved_input_mode;
        } else {
            self.saved_input_mode = self.input_mode;
            self.input_mode = InputMode::HelpOverlay;
        }
    }

    pub fn sort_source_references(&mut self) {
        match self.ref_sort_key {
            RefSortKey::Index => self.source_references.sort_by_key(|a| a.ref_index),
            RefSortKey::Year => self
                .source_references
                .sort_by_key(|b| std::cmp::Reverse(b.year.unwrap_or(0))),
            RefSortKey::Venue => self.source_references.sort_by(|a, b| {
                a.venue
                    .as_deref()
                    .unwrap_or("")
                    .cmp(b.venue.as_deref().unwrap_or(""))
            }),
            RefSortKey::Source => self
                .source_references
                .sort_by(|a, b| a.source_id.as_str().cmp(b.source_id.as_str())),
            RefSortKey::Title => self.source_references.sort_by(|a, b| {
                a.title
                    .as_deref()
                    .unwrap_or(&a.raw_text)
                    .cmp(b.title.as_deref().unwrap_or(&b.raw_text))
            }),
            RefSortKey::Similarity => {
                let sims = &self.draft_ref_similarities;
                self.source_references.sort_by(|a, b| {
                    let score_a = sims.get(&a.id).copied().unwrap_or(0.0);
                    let score_b = sims.get(&b.id).copied().unwrap_or(0.0);
                    score_b
                        .partial_cmp(&score_a)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }
    }

    pub fn filtered_bib_entries(&self) -> Vec<&String> {
        if self.bib_search_query.is_empty() {
            self.bib_file_entries.iter().collect()
        } else {
            let q = self.bib_search_query.to_lowercase();
            self.bib_file_entries
                .iter()
                .filter(|e| e.to_lowercase().contains(&q))
                .collect()
        }
    }

    pub fn filtered_source_references(&self) -> Vec<&ReferenceEntry> {
        let mut refs: Vec<&ReferenceEntry> = self
            .source_references
            .iter()
            .filter(|r| {
                if let Some(min) = self.min_similarity_filter {
                    let score = self
                        .draft_ref_similarities
                        .get(&r.id)
                        .copied()
                        .unwrap_or(0.0);
                    if score < min {
                        return false;
                    }
                }
                if self.ref_search_query.is_empty() {
                    true
                } else {
                    let q = self.ref_search_query.to_lowercase();
                    r.raw_text.to_lowercase().contains(&q)
                        || r.title
                            .as_deref()
                            .is_some_and(|t| t.to_lowercase().contains(&q))
                        || r.authors
                            .as_deref()
                            .is_some_and(|a| a.to_lowercase().contains(&q))
                        || r.venue
                            .as_deref()
                            .is_some_and(|v| v.to_lowercase().contains(&q))
                }
            })
            .collect();

        if self.ref_sort_key == RefSortKey::Similarity {
            let sims = &self.draft_ref_similarities;
            refs.sort_by(|a, b| {
                let score_a = sims.get(&a.id).copied().unwrap_or(0.0);
                let score_b = sims.get(&b.id).copied().unwrap_or(0.0);
                score_b
                    .partial_cmp(&score_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        refs
    }

    pub fn append_selected_source_to_bib(&mut self) {
        if self.sources.is_empty() || self.selected_source_index >= self.sources.len() {
            self.status_message = "No source document selected to append".to_string();
            return;
        }

        let doc = self.sources[self.selected_source_index].clone();
        let doc_name = doc.title.as_deref().unwrap_or(&doc.filename).to_string();

        let local_bib = sil_core::suggest_from_source(&doc).bibtex;
        let marked = sil_core::mark_tui_added_bib_entry(&local_bib);

        if let Some(ref root) = self.project_root {
            let bib_path = root.join(sil_core::paths::rel::REFERENCES);
            let current = std::fs::read_to_string(bib_path.as_std_path()).unwrap_or_default();
            let (updated, _replaced) = sil_core::bib::upsert_bib_entry(&current, &marked);
            if let Err(e) = std::fs::write(bib_path.as_std_path(), updated) {
                self.status_message = format!("Error writing references.bib: {e}");
                return;
            }
            self.load_project_references_bib();
        }

        if doc.should_attempt_metadata_fetch() {
            self.queue_source_hydration(doc);
            self.status_message =
                format!("✓ Added '{doc_name}' to references.bib; fetching official metadata…");
        } else {
            self.status_message = format!(
                "✓ Added '{doc_name}' to references.bib (⚠ No DOI/arXiv/title — cannot hydrate)"
            );
        }
    }

    pub fn recompute_draft_ref_similarities(&mut self) {
        let root = match self.project_root.as_ref() {
            Some(r) => r,
            None => {
                self.status_message = "No active project loaded to compute similarity".to_string();
                return;
            }
        };

        let paths = ProjectPaths::new(root);
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

        let db = match sil_db::SilDb::open(&paths.db()) {
            Ok(d) => d,
            Err(e) => {
                self.status_message = format!("⚠ Database error: {e}");
                return;
            }
        };

        let embedder = sil_db::OnnxEmbedder::from_rag_settings(&self.effective_rag_settings());
        let backend = embedder.backend().summary();
        match db.recompute_draft_ref_similarities(&draft_text, &embedder) {
            Ok(count) => {
                if let Ok(sims) = db.get_draft_ref_similarities() {
                    self.draft_ref_similarities = sims;
                }
                if let Ok(hash) = db.get_draft_similarity_hash() {
                    self.draft_similarity_hash = hash;
                }
                self.sort_source_references();
                self.status_message = format!(
                    "✓ Recomputed draft similarity scores for {count} reference(s) [{backend}]"
                );
            }
            Err(e) => {
                self.status_message = format!("⚠ Failed computing similarity scores: {e}");
            }
        }
    }

    pub fn clamp_bib_selection(&mut self) {
        let count = self.filtered_bib_entries().len();
        if count == 0 {
            self.selected_bib_index = 0;
        } else if self.selected_bib_index >= count {
            self.selected_bib_index = count - 1;
        }
    }

    pub fn clamp_source_ref_selection(&mut self) {
        let count = self.filtered_source_references().len();
        if count == 0 {
            self.selected_source_ref_index = 0;
        } else if self.selected_source_ref_index >= count {
            self.selected_source_ref_index = count - 1;
        }
    }

    pub fn filtered_viewing_source_references(&self) -> Vec<&ReferenceEntry> {
        if self.viewing_ref_search_query.is_empty() {
            self.selected_source_references.iter().collect()
        } else {
            let q = self.viewing_ref_search_query.to_lowercase();
            self.selected_source_references
                .iter()
                .filter(|r| {
                    r.raw_text.to_lowercase().contains(&q)
                        || r.title
                            .as_deref()
                            .is_some_and(|t| t.to_lowercase().contains(&q))
                        || r.authors
                            .as_deref()
                            .is_some_and(|a| a.to_lowercase().contains(&q))
                        || r.venue
                            .as_deref()
                            .is_some_and(|v| v.to_lowercase().contains(&q))
                        || r.year
                            .map(|y| y.to_string())
                            .is_some_and(|y| y.contains(&q))
                        || r.doi
                            .as_deref()
                            .is_some_and(|d| d.to_lowercase().contains(&q))
                })
                .collect()
        }
    }

    pub fn clamp_viewing_ref_selection(&mut self) {
        let count = self.filtered_viewing_source_references().len();
        if count == 0 {
            self.selected_viewing_ref_index = 0;
        } else if self.selected_viewing_ref_index >= count {
            self.selected_viewing_ref_index = count - 1;
        }
    }

    pub fn append_selected_viewing_ref_to_bib(&mut self) {
        if let Some(ref root) = self.project_root {
            let bib_path = root.join("references.bib");
            let mut entries_to_add = Vec::new();
            {
                let filtered = self.filtered_viewing_source_references();
                if self.marked_ref_ids.is_empty() {
                    if self.selected_viewing_ref_index < filtered.len() {
                        entries_to_add.push(filtered[self.selected_viewing_ref_index].clone());
                    }
                } else {
                    for r in &self.selected_source_references {
                        if self.marked_ref_ids.contains(&r.id) {
                            entries_to_add.push(r.clone());
                        }
                    }
                }
            }

            if !entries_to_add.is_empty() {
                let mut current =
                    std::fs::read_to_string(bib_path.as_std_path()).unwrap_or_default();
                let mut fetch_count = 0;
                for e in &entries_to_add {
                    let local_bib = e.to_bibtex();
                    let marked = sil_core::mark_tui_added_bib_entry(&local_bib);
                    let (updated, _) = sil_core::bib::upsert_bib_entry(&current, &marked);
                    current = updated;
                    if e.should_attempt_metadata_fetch() {
                        fetch_count += 1;
                        self.queue_ref_hydration(e.clone());
                    }
                }
                let _ = std::fs::write(bib_path.as_std_path(), current);
                let count = entries_to_add.len();
                self.marked_ref_ids.clear();
                self.load_project_references_bib();
                if fetch_count > 0 {
                    self.status_message =
                        format!("✓ Added {count} ref(s); fetching official metadata…");
                } else {
                    self.status_message =
                        format!("✓ Added {count} ref(s) (⚠ No DOI/arXiv/title — cannot hydrate)");
                }
            }
        }
    }

    pub fn append_all_viewing_refs_to_bib(&mut self) {
        if let Some(ref root) = self.project_root {
            let bib_path = root.join("references.bib");
            let entries_to_add: Vec<ReferenceEntry> = self
                .filtered_viewing_source_references()
                .into_iter()
                .cloned()
                .collect();
            if !entries_to_add.is_empty() {
                let mut current =
                    std::fs::read_to_string(bib_path.as_std_path()).unwrap_or_default();
                let mut fetch_count = 0;
                for e in &entries_to_add {
                    let local_bib = e.to_bibtex();
                    let marked = sil_core::mark_tui_added_bib_entry(&local_bib);
                    let (updated, _) = sil_core::bib::upsert_bib_entry(&current, &marked);
                    current = updated;
                    if e.should_attempt_metadata_fetch() {
                        fetch_count += 1;
                        self.queue_ref_hydration(e.clone());
                    }
                }
                let _ = std::fs::write(bib_path.as_std_path(), current);
                let count = entries_to_add.len();
                self.load_project_references_bib();
                if fetch_count > 0 {
                    self.status_message =
                        format!("✓ Added ALL {count} ref(s); fetching official metadata…");
                } else {
                    self.status_message = format!(
                        "✓ Added ALL {count} ref(s) (⚠ No DOI/arXiv/title — cannot hydrate)"
                    );
                }
            }
        }
    }

    pub fn promote_selected_bib_entry(&mut self) {
        let filtered = self.filtered_bib_entries();
        if filtered.is_empty() || self.selected_bib_index >= filtered.len() {
            self.status_message = "No bibliography entry selected to promote".to_string();
            return;
        }

        let selected_block = filtered[self.selected_bib_index].clone();
        let info = sil_core::extract_bib_entry_info(&selected_block);
        let cite_key = info.cite_key.as_deref().unwrap_or("entry").to_string();

        if !sil_core::is_tui_added_bib_block(&selected_block) {
            self.status_message = format!("Entry '{cite_key}' is already promoted / not TUI-added");
            return;
        }

        let unmarked = sil_core::unmark_tui_added_bib_entry(&selected_block);
        if let Some(ref root) = self.project_root {
            let bib_path = root.join("references.bib");
            let current = std::fs::read_to_string(bib_path.as_std_path()).unwrap_or_default();
            let mut blocks = sil_core::parse_bib_blocks(&current);
            for block in &mut blocks {
                let block_info = sil_core::extract_bib_entry_info(block);
                if sil_core::is_same_paper(&block_info, &info) {
                    *block = unmarked.clone();
                    break;
                }
            }
            let updated = if blocks.is_empty() {
                String::new()
            } else {
                blocks.join("\n\n") + "\n"
            };
            if let Err(e) = std::fs::write(bib_path.as_std_path(), updated) {
                self.status_message = format!("Error writing references.bib: {e}");
                return;
            }
            self.load_project_references_bib();
            self.status_message =
                format!("✓ Promoted '{cite_key}' (removed % [sil: tui-added] marker)");
        } else {
            self.status_message =
                format!("✓ Promoted '{cite_key}' (no project root loaded to save)");
        }
    }

    pub fn delete_selected_bib_entry(&mut self) {
        if self.active_tab == ActiveTab::References && self.active_ref_pane == RefPane::LeftBib {
            let filtered = self.filtered_bib_entries();
            if self.selected_bib_index < filtered.len() {
                let target = filtered[self.selected_bib_index].clone();
                if let Some(pos) = self.bib_file_entries.iter().position(|e| e == &target) {
                    self.bib_file_entries.remove(pos);
                    if let Some(ref root) = self.project_root {
                        let bib_path = root.join("references.bib");
                        let content = self.bib_file_entries.join("\n\n");
                        let _ = std::fs::write(bib_path.as_std_path(), content);
                    }
                    self.clamp_bib_selection();
                    self.status_message =
                        "✓ Deleted reference entry from references.bib".to_string();
                }
            }
        }
    }
}
