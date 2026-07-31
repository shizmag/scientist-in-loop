//! Application state and logic for `sil-tui`.

use camino::Utf8PathBuf;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use sil_core::{
    AuthorDetails, Config, GlobalSettings, GrantDetails, LocalSettings, ProjectPaths,
    ReferenceEntry, SettingsCache, SourceDocument,
};

/// Navigation tabs in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Dashboard = 0,
    Sources = 1,
    References = 2,
    PaperDraft = 3,
    Settings = 4,
}

impl ActiveTab {
    pub const ALL: [ActiveTab; 5] = [
        ActiveTab::Dashboard,
        ActiveTab::Sources,
        ActiveTab::References,
        ActiveTab::PaperDraft,
        ActiveTab::Settings,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            ActiveTab::Dashboard => "1. Dashboard",
            ActiveTab::Sources => "2. Sources",
            ActiveTab::References => "3. References",
            ActiveTab::PaperDraft => "4. Paper Draft",
            ActiveTab::Settings => "5. Settings",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefPane {
    LeftBib,
    RightSources,
}

/// Mode of user interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
    EditingPaper,
    ModalPicker,
    ModalAddAuthor,
    ModalAddGrant,
    ModalAddSourceLink,
    ModalRenameSource,
    ConfirmDeleteSource,
    ViewingSourceRefs,
    SearchingRefs,
    SearchingBib,
    ReadingSourceMd,
    SearchingViewingRefs,
}

/// Sorting key for references display in TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefSortKey {
    Index,
    Year,
    Source,
    Venue,
}

/// Items present in the unified Settings tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingItem {
    Global(GlobalField),
    Rag(RagField),
    CacheCoAuthor(usize),
    CacheCoAuthorEmpty,
    CacheGrant(usize),
    CacheGrantEmpty,
    LocalTitle,
    LocalNotes,
    LocalCoAuthor(usize),
    LocalCoAuthorEmpty,
    LocalGrant(usize),
    LocalGrantEmpty,
}

/// Currently active input field in forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalField {
    AuthorName = 0,
    AuthorEmail = 1,
    AuthorAffiliation = 2,
    AuthorOrcid = 3,
    GrantFunder = 4,
    GrantNumber = 5,
    GrantAck = 6,
    Engine = 7,
    Template = 8,
}

impl GlobalField {
    pub const ALL: [GlobalField; 9] = [
        GlobalField::AuthorName,
        GlobalField::AuthorEmail,
        GlobalField::AuthorAffiliation,
        GlobalField::AuthorOrcid,
        GlobalField::GrantFunder,
        GlobalField::GrantNumber,
        GlobalField::GrantAck,
        GlobalField::Engine,
        GlobalField::Template,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalField {
    Title = 0,
    CoAuthorsList = 1,
    GrantsList = 2,
    Notes = 3,
}

impl LocalField {
    pub const ALL: [LocalField; 4] = [
        LocalField::Title,
        LocalField::CoAuthorsList,
        LocalField::GrantsList,
        LocalField::Notes,
    ];
}

/// Currently active input field in RAG settings form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RagField {
    EmbedderPath = 0,
    RerankerPath = 1,
    ModelsDir = 2,
    CacheDir = 3,
    ExecutionProvider = 4,
    NumThreads = 5,
    ParentChunkSize = 6,
    ChildChunkSize = 7,
}

impl RagField {
    pub const ALL: [RagField; 8] = [
        RagField::EmbedderPath,
        RagField::RerankerPath,
        RagField::ModelsDir,
        RagField::CacheDir,
        RagField::ExecutionProvider,
        RagField::NumThreads,
        RagField::ParentChunkSize,
        RagField::ChildChunkSize,
    ];
}

fn resolve_onnx_from_dir(val: &str) -> String {
    let path = std::path::Path::new(val);
    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("onnx") {
                    return entry.path().to_string_lossy().to_string();
                }
            }
        }
    }
    val.to_string()
}

/// Application state struct for TUI.
pub struct App {
    pub active_tab: ActiveTab,
    pub input_mode: InputMode,

    pub active_ref_pane: RefPane,
    pub bib_file_entries: Vec<String>,
    pub selected_bib_index: usize,
    pub source_references: Vec<ReferenceEntry>,
    pub selected_source_ref_index: usize,
    pub marked_ref_ids: std::collections::HashSet<String>,
    pub ref_search_query: String,
    pub bib_search_query: String,

    pub global_settings: GlobalSettings,
    pub local_settings: LocalSettings,
    pub cache: SettingsCache,
    pub project_root: Option<Utf8PathBuf>,
    pub loaded_config: Option<Config>,

    pub selected_global_field: usize,
    pub selected_local_field: usize,
    pub selected_rag_field: usize,

    pub cache_coauthor_index: usize,
    pub cache_grant_index: usize,

    pub local_coauthor_index: usize,
    pub local_grant_index: usize,

    pub input_buffer: String,
    pub status_message: String,
    pub dirty: bool,
    pub should_quit: bool,

    // Buffers for adding new modal entities
    pub new_author: AuthorDetails,
    pub new_grant: GrantDetails,
    pub modal_field_index: usize,

    // Paper draft state & LaTeX sections
    pub paper_draft_content: String,
    pub paper_sections: Vec<sil_latex::TexSection>,
    pub paper_section_index: usize,
    pub paper_scroll_offset: usize,
    pub paper_edit_buffer: String,
    pub pending_external_editor: bool,

    // Sources state
    pub sources: Vec<SourceDocument>,
    pub selected_source_index: usize,
    pub source_scroll_offset: usize,
    pub reading_md_content: Option<String>,
    pub selected_source_references: Vec<ReferenceEntry>,
    pub selected_viewing_ref_index: usize,
    pub viewing_ref_search_query: String,
    pub viewing_ref_show_detail: bool,
    pub ref_sort_key: RefSortKey,
    pub new_source_link_buffer: String,
    pub rename_source_buffer: String,

    // Scroll offsets for scrollbars & lists
    pub bib_scroll_offset: usize,
    pub ref_scroll_offset: usize,
    pub settings_scroll_offset: usize,
    pub dashboard_scroll_offset: usize,

    // Unified settings navigation
    pub selected_setting_index: usize,
}

impl App {
    pub fn new(project_root: Option<Utf8PathBuf>) -> Self {
        let global_settings = GlobalSettings::load_or_default(None);
        let cache = SettingsCache::load_or_default(None);

        let (local_settings, loaded_config) = if let Some(ref root) = project_root {
            let paths = ProjectPaths::new(root);
            if let Ok(cfg) = Config::load(&paths.config()) {
                (cfg.settings.clone(), Some(cfg))
            } else {
                (LocalSettings::default(), None)
            }
        } else {
            (LocalSettings::default(), None)
        };

        let mut app = Self {
            active_tab: ActiveTab::Dashboard,

            input_mode: InputMode::Normal,
            active_ref_pane: RefPane::RightSources,
            bib_file_entries: Vec::new(),
            selected_bib_index: 0,
            source_references: Vec::new(),
            selected_source_ref_index: 0,
            marked_ref_ids: std::collections::HashSet::new(),
            ref_search_query: String::new(),
            bib_search_query: String::new(),

            global_settings,
            local_settings,
            cache,
            project_root,
            loaded_config,
            selected_global_field: 0,
            selected_local_field: 0,
            selected_rag_field: 0,
            cache_coauthor_index: 0,
            cache_grant_index: 0,
            local_coauthor_index: 0,
            local_grant_index: 0,
            input_buffer: String::new(),
            status_message: "Ready. Press 'Tab' to switch views, 'e' to edit section, 'v' for external $EDITOR, 's' to save.".to_string(),
            dirty: false,
            should_quit: false,
            new_author: AuthorDetails::default(),
            new_grant: GrantDetails::default(),
            modal_field_index: 0,

            paper_draft_content: String::new(),
            paper_sections: Vec::new(),
            paper_section_index: 0,
            paper_scroll_offset: 0,
            paper_edit_buffer: String::new(),
            pending_external_editor: false,

            sources: Vec::new(),
            selected_source_index: 0,
            source_scroll_offset: 0,
            reading_md_content: None,
            selected_source_references: Vec::new(),
            selected_viewing_ref_index: 0,
            viewing_ref_search_query: String::new(),
            viewing_ref_show_detail: true,
            ref_sort_key: RefSortKey::Index,
            new_source_link_buffer: String::new(),
            rename_source_buffer: String::new(),

            bib_scroll_offset: 0,
            ref_scroll_offset: 0,
            settings_scroll_offset: 0,
            dashboard_scroll_offset: 0,

            selected_setting_index: 0,
        };
        app.reload_paper_draft();
        app.reload_sources();
        app.load_project_references_bib();
        app.load_all_source_references();
        app
    }

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
                    let mut current_entry = String::new();
                    for line in content.lines() {
                        if line.starts_with('@') {
                            if !current_entry.is_empty() {
                                self.bib_file_entries.push(current_entry.trim().to_string());
                                current_entry.clear();
                            }
                        }
                        current_entry.push_str(line);
                        current_entry.push('\n');
                    }
                    if !current_entry.is_empty() {
                        self.bib_file_entries.push(current_entry.trim().to_string());
                    }
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
        if self.ref_search_query.is_empty() {
            self.source_references.iter().collect()
        } else {
            let q = self.ref_search_query.to_lowercase();
            self.source_references
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
                })
                .collect()
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
                for e in &entries_to_add {
                    current.push_str(&e.to_bibtex());
                    current.push('\n');
                }
                let _ = std::fs::write(bib_path.as_std_path(), current);
                let count = entries_to_add.len();
                self.marked_ref_ids.clear();
                self.load_project_references_bib();
                self.status_message = format!("✓ Added {count} reference(s) to references.bib");
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
                for e in &entries_to_add {
                    current.push_str(&e.to_bibtex());
                    current.push('\n');
                }
                let _ = std::fs::write(bib_path.as_std_path(), current);
                let count = entries_to_add.len();
                self.load_project_references_bib();
                self.status_message = format!("✓ Added ALL {count} reference(s) to references.bib");
            }
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

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Normal => self.handle_normal_mode(key),
            InputMode::Editing => self.handle_editing_mode(key),
            InputMode::EditingPaper => self.handle_editing_paper_mode(key),
            InputMode::ModalPicker => self.handle_modal_picker_mode(key),
            InputMode::ModalAddAuthor => self.handle_modal_add_author_mode(key),
            InputMode::ModalAddGrant => self.handle_modal_add_grant_mode(key),
            InputMode::ModalAddSourceLink => self.handle_modal_add_source_link_mode(key),
            InputMode::ModalRenameSource => self.handle_modal_rename_source_mode(key),
            InputMode::ConfirmDeleteSource => self.handle_confirm_delete_source_mode(key),
            InputMode::ViewingSourceRefs => self.handle_viewing_source_refs_mode(key),
            InputMode::SearchingRefs => self.handle_searching_refs_mode(key),
            InputMode::SearchingBib => self.handle_searching_bib_mode(key),
            InputMode::ReadingSourceMd => self.handle_reading_source_md_mode(key),
            InputMode::SearchingViewingRefs => self.handle_searching_viewing_refs_mode(key),
        }
    }

    fn handle_normal_mode(&mut self, key: KeyEvent) {
        // Global shortcuts
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            self.save_all();
            return;
        }

        match key.code {
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
                    self.source_references
                        .sort_by_key(|r| r.source_id.to_string());
                } else {
                    self.save_all();
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
            KeyCode::Down | KeyCode::Char('j') => match self.active_tab {
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
                    if !self.sources.is_empty()
                        && self.selected_source_index + 1 < self.sources.len()
                    {
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
            },
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
            KeyCode::Char('e') => self.start_editing_selected_field(),

            // Actions for Sources & Settings
            KeyCode::Char('a') => match self.active_tab {
                ActiveTab::Sources => {
                    self.new_source_link_buffer.clear();
                    self.input_mode = InputMode::ModalAddSourceLink;
                    self.status_message =
                        "Enter URL / DOI / arXiv link to fetch (Enter to submit, Esc to cancel)"
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
            KeyCode::Char('r') => {
                if self.active_tab == ActiveTab::Sources
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
                    if let Some(ref root) = self.project_root {
                        let bib_path = root.join("references.bib");
                        let mut entries_to_add = Vec::new();
                        if self.marked_ref_ids.is_empty() {
                            let filtered = self.filtered_source_references();
                            if self.selected_source_ref_index < filtered.len() {
                                entries_to_add
                                    .push(filtered[self.selected_source_ref_index].clone());
                            }
                        } else {
                            for r in &self.source_references {
                                if self.marked_ref_ids.contains(&r.id) {
                                    entries_to_add.push(r.clone());
                                }
                            }
                        }
                        if !entries_to_add.is_empty() {
                            let mut current =
                                std::fs::read_to_string(bib_path.as_std_path()).unwrap_or_default();
                            for e in &entries_to_add {
                                let mut bibtex = format!(
                                    "@misc{{\n  title = {{{}}},\n",
                                    e.title.as_deref().unwrap_or("")
                                );
                                if let Some(ref a) = e.authors {
                                    bibtex.push_str(&format!("  author = {{{}}},\n", a));
                                }
                                if let Some(ref y) = e.year {
                                    bibtex.push_str(&format!("  year = {{{}}},\n", y));
                                }
                                if let Some(ref v) = e.venue {
                                    bibtex.push_str(&format!("  journal = {{{}}},\n", v));
                                }
                                bibtex.push_str("}\n\n");
                                current.push_str(&bibtex);
                            }
                            let _ = std::fs::write(bib_path.as_std_path(), current);
                            self.load_project_references_bib();
                            let count = entries_to_add.len();
                            self.marked_ref_ids.clear();
                            self.status_message =
                                format!("Pasted {} reference(s) to references.bib", count);
                        }
                    }
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
            KeyCode::Char('y') => {
                if self.active_tab == ActiveTab::References {
                    self.source_references
                        .sort_by_key(|r| std::cmp::Reverse(r.year.unwrap_or_default()));
                }
            }
            KeyCode::Char('i') => {
                if self.active_tab == ActiveTab::References {
                    self.source_references.sort_by_key(|r| r.ref_index);
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

    fn fetch_source_markdown_content(&self, doc: &SourceDocument) -> String {
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

    fn start_editing_selected_field(&mut self) {
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
                    SettingItem::LocalTitle => self.local_settings.title = val,
                    SettingItem::LocalNotes => self.local_settings.notes = val,
                    _ => {}
                }
            }
        }
    }

    fn handle_modal_picker_mode(&mut self, key: KeyEvent) {
        match key.code {
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

    fn handle_modal_add_source_link_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.status_message = "Add source cancelled.".to_string();
            }
            KeyCode::Enter => {
                let link = self.new_source_link_buffer.trim().to_string();
                if !link.is_empty() {
                    if let Some(ref root) = self.project_root {
                        let paths = ProjectPaths::new(root);
                        let sources_dir = root.join("sources");
                        std::fs::create_dir_all(sources_dir.as_std_path()).ok();
                        let filename = Utf8PathBuf::from(&link)
                            .file_name()
                            .unwrap_or("new_source.md")
                            .to_string();
                        let file_path = sources_dir.join(&filename);
                        if !file_path.exists() {
                            let _ = std::fs::write(
                                file_path.as_std_path(),
                                format!("# Source: {filename}\nLink: {link}\n"),
                            );
                        }
                        let doc = SourceDocument::new(file_path);
                        if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
                            let _ = db.upsert_parsed(
                                &doc,
                                &format!("# Source: {filename}\nLink: {link}\n"),
                            );
                        }
                    } else {
                        let doc = SourceDocument::new(Utf8PathBuf::from(&link));
                        self.sources.push(doc);
                    }
                    self.reload_sources();
                    self.status_message = format!("Added source link: {link}");
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
                self.selected_viewing_ref_index =
                    self.selected_viewing_ref_index.saturating_sub(5);
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
            KeyCode::Esc | KeyCode::Char('q') => {
                self.input_mode = InputMode::Normal;
                self.reading_md_content = None;
                self.status_message = "Exited Markdown reader.".to_string();
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
                        if std::fs::write(config_path.as_std_path(), yaml).is_ok() {
                            messages.push("Local config.yaml updated".to_string());
                        }
                    }
                }
            }

            // 4. Save paper_draft.tex if present
            if !self.paper_draft_content.is_empty() {
                let draft_path = root.join("paper_draft.tex");
                if std::fs::write(draft_path.as_std_path(), &self.paper_draft_content).is_ok() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_initialization() {
        let app = App::new(None);
        assert_eq!(app.active_tab, ActiveTab::Dashboard);
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_tab_navigation() {
        let mut app = App::new(None);
        assert_eq!(app.active_tab, ActiveTab::Dashboard);

        app.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::empty()));
        assert_eq!(app.active_tab, ActiveTab::Sources);

        app.handle_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::empty()));
        assert_eq!(app.active_tab, ActiveTab::References);

        app.handle_key(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::empty()));
        assert_eq!(app.active_tab, ActiveTab::PaperDraft);

        app.handle_key(KeyEvent::new(KeyCode::Char('5'), KeyModifiers::empty()));
        assert_eq!(app.active_tab, ActiveTab::Settings);

        app.handle_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::empty()));
        assert_eq!(app.active_tab, ActiveTab::Dashboard);
    }

    #[test]
    fn test_references_tab_navigation() {
        let mut app = App::new(None);

        // Go to References
        app.handle_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::empty()));
        assert_eq!(app.active_tab, ActiveTab::References);

        // Default pane is RightSources
        assert_eq!(app.active_ref_pane, RefPane::RightSources);

        // Tab toggles pane
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
        assert_eq!(app.active_ref_pane, RefPane::LeftBib);

        // Tab again toggles pane back
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
        assert_eq!(app.active_ref_pane, RefPane::RightSources);
    }

    #[test]
    fn test_references_marking_and_searching() {
        let mut app = App::new(None);
        app.active_tab = ActiveTab::References;
        app.source_references = vec![
            sil_core::ReferenceEntry {
                id: "ref1".to_string(),
                source_id: "doc1".into(),
                ref_index: 1,
                raw_text: "Deep learning".to_string(),
                title: None,
                authors: None,
                year: None,
                venue: None,
                doi: None,
            },
            sil_core::ReferenceEntry {
                id: "ref2".to_string(),
                source_id: "doc2".into(),
                ref_index: 2,
                raw_text: "Transformer models".to_string(),
                title: None,
                authors: None,
                year: None,
                venue: None,
                doi: None,
            },
        ];

        app.selected_source_ref_index = 0;
        app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty()));
        assert!(app.marked_ref_ids.contains("ref1"));

        app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::SearchingRefs);

        app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()));

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::Normal);

        app.selected_source_ref_index = 0;
        app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty()));
        assert!(app.marked_ref_ids.contains("ref2"));
    }

    #[test]
    fn test_enums_and_titles() {
        assert_eq!(ActiveTab::ALL.len(), 5);
        assert_eq!(ActiveTab::Dashboard.title(), "1. Dashboard");
        assert_eq!(ActiveTab::Sources.title(), "2. Sources");
        assert_eq!(ActiveTab::References.title(), "3. References");
        assert_eq!(ActiveTab::PaperDraft.title(), "4. Paper Draft");
        assert_eq!(ActiveTab::Settings.title(), "5. Settings");

        assert_eq!(GlobalField::ALL.len(), 9);
        assert_eq!(LocalField::ALL.len(), 4);
        assert_eq!(RagField::ALL.len(), 8);
    }

    #[test]
    fn test_resolve_onnx_from_dir() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();

        // Non-dir string
        assert_eq!(resolve_onnx_from_dir("/no/such/dir"), "/no/such/dir");

        // Dir without onnx
        assert_eq!(resolve_onnx_from_dir(dir_path.as_str()), dir_path.as_str());

        // Dir with onnx file
        let onnx_file = dir_path.join("model.onnx");
        std::fs::write(onnx_file.as_std_path(), b"onnx").unwrap();
        let resolved = resolve_onnx_from_dir(dir_path.as_str());
        assert!(resolved.ends_with("model.onnx"));
    }

    #[test]
    fn test_app_with_project_root_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();

        // Create paper_draft.tex
        let tex_content = "\\section{Intro}\nHello world\n";
        std::fs::write(root.join("paper_draft.tex").as_std_path(), tex_content).unwrap();

        // Create references.bib
        let bib_content =
            "@misc{key1,\n  title = {Paper One},\n}\n@article{key2,\n  title = {Paper Two},\n}\n";
        std::fs::write(root.join("references.bib").as_std_path(), bib_content).unwrap();

        // Create sources dir with md file
        let sources_dir = root.join("sources");
        std::fs::create_dir_all(sources_dir.as_std_path()).unwrap();
        std::fs::write(sources_dir.join("readme.md").as_std_path(), "ignore me").unwrap();
        std::fs::write(
            sources_dir.join("source1.md").as_std_path(),
            "# Source 1 Content",
        )
        .unwrap();

        let app = App::new(Some(root.clone()));
        assert_eq!(app.paper_draft_content, tex_content);
        assert_eq!(app.bib_file_entries.len(), 2);
        assert_eq!(app.sources.len(), 1);
        assert_eq!(app.sources[0].filename, "source1.md");

        // Test fetch_source_markdown_content
        let content = app.fetch_source_markdown_content(&app.sources[0]);
        assert_eq!(content, "# Source 1 Content");
    }

    #[test]
    fn test_normal_mode_navigation_and_shortcuts() {
        let mut app = App::new(None);

        // Ctrl+s saves
        app.dirty = true;
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert!(!app.dirty);

        // esc / q quits
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()));
        assert!(app.should_quit);

        app.should_quit = false;
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert!(app.should_quit);

        // BackTab
        app.should_quit = false;
        app.active_tab = ActiveTab::Dashboard;
        app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::empty()));
        assert_eq!(app.active_tab, ActiveTab::Settings);

        app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::empty()));
        assert_eq!(app.active_tab, ActiveTab::PaperDraft);
    }

    #[test]
    fn test_dashboard_up_down_keys() {
        let mut app = App::new(None);
        app.active_tab = ActiveTab::Dashboard;
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
        assert_eq!(app.active_tab, ActiveTab::Dashboard);
    }

    #[test]
    fn test_sources_tab_actions() {
        let mut app = App::new(None);
        app.active_tab = ActiveTab::Sources;
        app.sources = vec![SourceDocument::new(Utf8PathBuf::from("test.md"))];

        // Read source markdown
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::ReadingSourceMd);
        assert!(app.reading_md_content.is_some());

        // Exit reader with Esc
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.reading_md_content.is_none());

        // Add source link mode
        app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::ModalAddSourceLink);
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));

        // Rename source mode
        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::ModalRenameSource);
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));

        // Confirm delete source mode
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::ConfirmDeleteSource);
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));

        // View source references
        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::ViewingSourceRefs);
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    }

    #[test]
    fn test_editing_all_global_settings_fields() {
        let mut app = App::new(None);
        app.active_tab = ActiveTab::Settings;

        // Iterate over setting items and test editing fields
        let items = app.setting_items();
        for (idx, item) in items.iter().enumerate() {
            if let SettingItem::Global(field) = item {
                app.selected_setting_index = idx;
                app.start_editing_selected_field();
                assert_eq!(app.input_mode, InputMode::Editing);

                app.input_buffer = match field {
                    GlobalField::AuthorName => "New Author".to_string(),
                    GlobalField::AuthorEmail => "author@test.com".to_string(),
                    GlobalField::AuthorAffiliation => "Test Uni".to_string(),
                    GlobalField::AuthorOrcid => "0000-0002".to_string(),
                    GlobalField::GrantFunder => "DOE".to_string(),
                    GlobalField::GrantNumber => "G-100".to_string(),
                    GlobalField::GrantAck => "Thanks DOE".to_string(),
                    GlobalField::Engine => "pdflatex".to_string(),
                    GlobalField::Template => "neurips".to_string(),
                };

                app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
                assert_eq!(app.input_mode, InputMode::Normal);
            }
        }

        assert_eq!(app.global_settings.author.name, "New Author");
        assert_eq!(app.global_settings.author.email, "author@test.com");
        assert_eq!(app.global_settings.author.affiliation, "Test Uni");
        assert_eq!(
            app.global_settings.author.orcid,
            Some("0000-0002".to_string())
        );
        assert_eq!(app.global_settings.default_grant.funder, "DOE");
        assert_eq!(app.global_settings.default_grant.grant_number, "G-100");
        assert_eq!(
            app.global_settings.default_grant.acknowledgment,
            "Thanks DOE"
        );
        assert_eq!(app.global_settings.default_latex_engine, "pdflatex");
        assert_eq!(app.global_settings.default_template, "neurips");
    }

    #[test]
    fn test_editing_all_rag_settings_fields() {
        let mut app = App::new(None);
        app.active_tab = ActiveTab::Settings;

        let items = app.setting_items();
        for (idx, item) in items.iter().enumerate() {
            if let SettingItem::Rag(field) = item {
                app.selected_setting_index = idx;
                app.start_editing_selected_field();
                assert_eq!(app.input_mode, InputMode::Editing);

                app.input_buffer = match field {
                    RagField::EmbedderPath => "/path/to/embedder.onnx".to_string(),
                    RagField::RerankerPath => "/path/to/reranker.onnx".to_string(),
                    RagField::ModelsDir => "/path/to/models".to_string(),
                    RagField::CacheDir => "/path/to/cache".to_string(),
                    RagField::ExecutionProvider => "cuda".to_string(),
                    RagField::NumThreads => "16".to_string(),
                    RagField::ParentChunkSize => "2000".to_string(),
                    RagField::ChildChunkSize => "500".to_string(),
                };

                app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
                assert_eq!(app.input_mode, InputMode::Normal);
            }
        }

        assert_eq!(
            app.global_settings.rag.onnx_embedder_path,
            Some(Utf8PathBuf::from("/path/to/embedder.onnx"))
        );
        assert_eq!(
            app.global_settings.rag.onnx_reranker_path,
            Some(Utf8PathBuf::from("/path/to/reranker.onnx"))
        );
        assert_eq!(
            app.global_settings.rag.onnx_models_dir,
            Some(Utf8PathBuf::from("/path/to/models"))
        );
        assert_eq!(
            app.global_settings.rag.model_cache_dir,
            Utf8PathBuf::from("/path/to/cache")
        );
        assert_eq!(app.global_settings.rag.execution_provider, "cuda");
        assert_eq!(app.global_settings.rag.num_threads, 16);
        assert_eq!(app.global_settings.rag.parent_chunk_size, 2000);
        assert_eq!(app.global_settings.rag.child_chunk_size, 500);
    }

    #[test]
    fn test_editing_local_settings_fields() {
        let mut app = App::new(None);
        app.active_tab = ActiveTab::Settings;

        let items = app.setting_items();
        for (idx, item) in items.iter().enumerate() {
            match item {
                SettingItem::LocalTitle => {
                    app.selected_setting_index = idx;
                    app.start_editing_selected_field();
                    app.input_buffer = "My Great Paper".to_string();
                    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
                }
                SettingItem::LocalNotes => {
                    app.selected_setting_index = idx;
                    app.start_editing_selected_field();
                    app.input_buffer = "Important research notes".to_string();
                    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
                }
                _ => {}
            }
        }

        assert_eq!(app.local_settings.title, "My Great Paper");
        assert_eq!(app.local_settings.notes, "Important research notes");
    }

    #[test]
    fn test_modal_add_author_and_grant_workflows() {
        let mut app = App::new(None);
        app.cache = SettingsCache::default();
        app.local_settings = LocalSettings::default();

        // Add author modal
        app.input_mode = InputMode::ModalAddAuthor;
        app.modal_field_index = 0;

        // Type name
        app.handle_key(KeyEvent::new(KeyCode::Char('B'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty()));
        assert_eq!(app.new_author.name, "Bob");

        // Tab to email
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
        assert_eq!(app.modal_field_index, 1);
        app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('@'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::empty()));
        assert_eq!(app.new_author.email, "b@m");

        // Enter submits author
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.cache.co_authors.len(), 1);
        assert_eq!(app.local_settings.co_authors.len(), 1);

        // Add grant modal
        app.input_mode = InputMode::ModalAddGrant;
        app.modal_field_index = 0;

        app.handle_key(KeyEvent::new(KeyCode::Char('N'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::empty()));
        assert_eq!(app.new_grant.funder, "NIH");

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.cache.grants.len(), 1);
        assert_eq!(app.local_settings.grants.len(), 1);
    }

    #[test]
    fn test_modal_add_source_link_and_rename_and_delete() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let mut app = App::new(Some(root.clone()));

        // Add source link modal
        app.active_tab = ActiveTab::Sources;
        app.input_mode = InputMode::ModalAddSourceLink;
        app.new_source_link_buffer = "https://example.com/paper.pdf".to_string();
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.sources.len(), 1);

        // Rename source modal
        app.selected_source_index = 0;
        app.input_mode = InputMode::ModalRenameSource;
        app.rename_source_buffer = "Renamed Title".to_string();
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert_eq!(app.sources[0].title, Some("Renamed Title".to_string()));

        // Confirm delete source modal
        app.input_mode = InputMode::ConfirmDeleteSource;
        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty()));
        assert!(app.sources.is_empty());
    }

    #[test]
    fn test_paper_draft_editing_and_scrolling() {
        let mut app = App::new(None);
        app.active_tab = ActiveTab::PaperDraft;
        app.paper_draft_content = "\\section{Intro}\nInitial content".to_string();
        app.paper_sections = sil_latex::split_tex_sections(&app.paper_draft_content);
        app.paper_section_index = 0;

        // Edit paper section
        app.start_editing_selected_field();
        assert_eq!(app.input_mode, InputMode::EditingPaper);
        app.paper_edit_buffer = "Updated intro section body".to_string();
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));

        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(
            app.paper_draft_content
                .contains("Updated intro section body")
        );

        // PageUp / PageDown scrolling
        app.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty()));
        assert_eq!(app.paper_scroll_offset, 5);
        app.handle_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty()));
        assert_eq!(app.paper_scroll_offset, 0);
    }

    #[test]
    fn test_viewing_source_refs_sorting() {
        let mut app = App::new(None);
        app.input_mode = InputMode::ViewingSourceRefs;
        app.selected_source_references = vec![
            ReferenceEntry {
                id: "1".to_string(),
                source_id: "s1".into(),
                ref_index: 2,
                raw_text: "Ref 2".to_string(),
                title: None,
                authors: None,
                year: Some(2020),
                venue: Some("NeurIPS".to_string()),
                doi: None,
            },
            ReferenceEntry {
                id: "2".to_string(),
                source_id: "s2".into(),
                ref_index: 1,
                raw_text: "Ref 1".to_string(),
                title: None,
                authors: None,
                year: Some(2024),
                venue: Some("ICML".to_string()),
                doi: None,
            },
        ];

        // Sort by Year
        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty()));
        assert_eq!(app.ref_sort_key, RefSortKey::Year);
        assert_eq!(app.selected_source_references[0].year, Some(2024));

        // Sort by Venue
        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::empty()));
        assert_eq!(app.ref_sort_key, RefSortKey::Venue);
        assert_eq!(
            app.selected_source_references[0].venue,
            Some("ICML".to_string())
        );

        // Sort by Index
        app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty()));
        assert_eq!(app.ref_sort_key, RefSortKey::Index);
        assert_eq!(app.selected_source_references[0].ref_index, 1);

        // Sort by Source
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()));
        assert_eq!(app.ref_sort_key, RefSortKey::Source);
        assert_eq!(app.selected_source_references[0].source_id.as_str(), "s1");

        // Esc closes viewer
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_save_all_in_project() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let sil_dir = root.join(".sil");
        std::fs::create_dir_all(sil_dir.as_std_path()).unwrap();

        let config_path = sil_dir.join("config.yaml");
        let initial_cfg = Config::default();
        std::fs::write(config_path.as_std_path(), initial_cfg.to_yaml().unwrap()).unwrap();

        let mut app = App::new(Some(root.clone()));
        app.local_settings.title = "Saved Paper Title".to_string();
        app.paper_draft_content = "\\section{Main}\nSaved content\n".to_string();
        app.dirty = true;

        app.save_all();
        assert!(!app.dirty);
        assert!(app.status_message.contains("✓"));

        let reloaded_cfg = Config::load(&config_path).unwrap();
        assert_eq!(reloaded_cfg.project.title, "Saved Paper Title");

        let draft_path = root.join("paper_draft.tex");
        let saved_tex = std::fs::read_to_string(draft_path.as_std_path()).unwrap();
        assert_eq!(saved_tex, "\\section{Main}\nSaved content\n");
    }

    #[test]
    fn test_left_bib_search_and_filtering() {
        let mut app = App::new(None);
        app.active_tab = ActiveTab::References;
        app.active_ref_pane = RefPane::LeftBib;

        app.bib_file_entries = vec![
            "@article{attn, title={Attention is All You Need}}".to_string(),
            "@misc{resnet, title={Deep Residual Learning}}".to_string(),
        ];

        // Pressing '/' in LeftBib enters SearchingBib
        app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::SearchingBib);

        // Type query 'attn'
        app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()));

        assert_eq!(app.bib_search_query, "attn");
        assert_eq!(app.filtered_bib_entries().len(), 1);
        assert!(app.filtered_bib_entries()[0].contains("attn"));

        // Enter exits SearchingBib mode
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::Normal);

        // Pressing Esc in Normal mode with active filter clears the query
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert_eq!(app.bib_search_query, "");
        assert_eq!(app.filtered_bib_entries().len(), 2);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_references_tab_right_pane_sorting() {
        let mut app = App::new(None);
        app.active_tab = ActiveTab::References;
        app.active_ref_pane = RefPane::RightSources;

        app.source_references = vec![
            ReferenceEntry {
                id: "ref_a".to_string(),
                source_id: "src_z".into(),
                ref_index: 2,
                raw_text: "Ref A".to_string(),
                title: Some("Paper A".to_string()),
                authors: Some("Author A".to_string()),
                year: Some(2020),
                venue: Some("NeurIPS".to_string()),
                doi: None,
            },
            ReferenceEntry {
                id: "ref_b".to_string(),
                source_id: "src_a".into(),
                ref_index: 1,
                raw_text: "Ref B".to_string(),
                title: Some("Paper B".to_string()),
                authors: Some("Author B".to_string()),
                year: Some(2024),
                venue: Some("ICML".to_string()),
                doi: None,
            },
        ];

        // Sort by year ('y')
        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty()));
        assert_eq!(app.source_references[0].year, Some(2024));

        // Sort by venue ('v')
        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::empty()));
        assert_eq!(app.source_references[0].venue, Some("ICML".to_string()));

        // Sort by source_id ('s')
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()));
        assert_eq!(app.source_references[0].source_id.as_str(), "src_a");

        // Sort by index ('i')
        app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty()));
        assert_eq!(app.source_references[0].ref_index, 1);
    }

    #[test]
    fn test_viewing_source_refs_navigation_and_scrolling() {
        let mut app = App::new(None);
        app.input_mode = InputMode::ViewingSourceRefs;
        app.selected_source_references = (1..=10)
            .map(|idx| ReferenceEntry {
                id: format!("ref_{idx}"),
                source_id: "doc1.pdf".into(),
                ref_index: idx,
                raw_text: format!("Reference item {idx}"),
                title: Some(format!("Title {idx}")),
                authors: Some(format!("Author {idx}")),
                year: Some(2010 + idx as u32),
                venue: Some("Conf".to_string()),
                doi: None,
            })
            .collect();
        app.selected_viewing_ref_index = 0;

        // Down / 'j' navigation
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()));
        assert_eq!(app.selected_viewing_ref_index, 1);

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert_eq!(app.selected_viewing_ref_index, 2);

        // Up / 'k' navigation
        app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty()));
        assert_eq!(app.selected_viewing_ref_index, 1);

        // PageDown & PageUp
        app.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty()));
        assert_eq!(app.selected_viewing_ref_index, 6);

        app.handle_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty()));
        assert_eq!(app.selected_viewing_ref_index, 1);

        // End & Home
        app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::empty()));
        assert_eq!(app.selected_viewing_ref_index, 9);

        app.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::empty()));
        assert_eq!(app.selected_viewing_ref_index, 0);
    }

    #[test]
    fn test_viewing_source_refs_sorting_and_filtering() {
        let mut app = App::new(None);
        app.input_mode = InputMode::ViewingSourceRefs;
        app.selected_source_references = vec![
            ReferenceEntry {
                id: "ref_1".to_string(),
                source_id: "src1".into(),
                ref_index: 1,
                raw_text: "Attention is All You Need".to_string(),
                title: Some("Attention is All You Need".to_string()),
                authors: Some("Vaswani".to_string()),
                year: Some(2017),
                venue: Some("NeurIPS".to_string()),
                doi: Some("10.1000/1".to_string()),
            },
            ReferenceEntry {
                id: "ref_2".to_string(),
                source_id: "src2".into(),
                ref_index: 2,
                raw_text: "Deep Residual Learning".to_string(),
                title: Some("Deep Residual Learning".to_string()),
                authors: Some("He".to_string()),
                year: Some(2016),
                venue: Some("CVPR".to_string()),
                doi: None,
            },
        ];

        // Sort by Year ('y')
        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty()));
        assert_eq!(app.selected_source_references[0].year, Some(2017));

        // Sort by Title ('t')
        app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
        assert_eq!(
            app.selected_source_references[0].title.as_deref(),
            Some("Attention is All You Need")
        );

        // Enter search mode with '/'
        app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::SearchingViewingRefs);

        // Type 'He'
        app.handle_key(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()));
        assert_eq!(app.filtered_viewing_source_references().len(), 1);
        assert_eq!(
            app.filtered_viewing_source_references()[0].authors.as_deref(),
            Some("He")
        );

        // Esc exits search mode
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::ViewingSourceRefs);
    }

    #[test]
    fn test_viewing_source_refs_bibtex_append_and_delete() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_path = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();
        std::fs::write(project_path.join("references.bib").as_std_path(), "").unwrap();

        let mut app = App::new(Some(project_path.clone()));
        app.input_mode = InputMode::ViewingSourceRefs;
        app.selected_source_references = vec![ReferenceEntry {
            id: "ref_1".to_string(),
            source_id: "src1".into(),
            ref_index: 1,
            raw_text: "Attention is All You Need".to_string(),
            title: Some("Attention is All You Need".to_string()),
            authors: Some("Vaswani".to_string()),
            year: Some(2017),
            venue: Some("NeurIPS".to_string()),
            doi: None,
        }];
        app.selected_viewing_ref_index = 0;

        // Append selected ref to bib via 'c'
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()));
        let bib_content = std::fs::read_to_string(project_path.join("references.bib").as_std_path()).unwrap();
        assert!(bib_content.contains("@article{attention_is_all_you_need"));
        assert_eq!(app.bib_file_entries.len(), 1);

        // Delete bib entry via delete_selected_bib_entry
        app.active_tab = ActiveTab::References;
        app.active_ref_pane = RefPane::LeftBib;
        app.selected_bib_index = 0;
        app.delete_selected_bib_entry();

        let updated_bib = std::fs::read_to_string(project_path.join("references.bib").as_std_path()).unwrap();
        assert!(updated_bib.is_empty());
        assert_eq!(app.bib_file_entries.len(), 0);
    }
}
