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
    PaperDraft = 1,
    Sources = 2,
    Settings = 3,
}

impl ActiveTab {
    pub const ALL: [ActiveTab; 4] = [
        ActiveTab::Dashboard,
        ActiveTab::PaperDraft,
        ActiveTab::Sources,
        ActiveTab::Settings,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            ActiveTab::Dashboard => "1. Dashboard",
            ActiveTab::PaperDraft => "2. Paper Draft",
            ActiveTab::Sources => "3. Sources",
            ActiveTab::Settings => "4. Settings",
        }
    }
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
    ReadingSourceMd,
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
    pub ref_sort_key: RefSortKey,
    pub new_source_link_buffer: String,
    pub rename_source_buffer: String,

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
            ref_sort_key: RefSortKey::Index,
            new_source_link_buffer: String::new(),
            rename_source_buffer: String::new(),

            selected_setting_index: 0,
        };
        app.reload_paper_draft();
        app.reload_sources();
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
            InputMode::ReadingSourceMd => self.handle_reading_source_md_mode(key),
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
                self.should_quit = true;
            }
            KeyCode::Tab => {
                let current = self.active_tab as usize;
                let next = (current + 1) % ActiveTab::ALL.len();
                self.active_tab = ActiveTab::ALL[next];
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
            KeyCode::Char('2') => self.active_tab = ActiveTab::PaperDraft,
            KeyCode::Char('3') => self.active_tab = ActiveTab::Sources,
            KeyCode::Char('4') => self.active_tab = ActiveTab::Settings,

            KeyCode::Char('s') => self.save_all(),
            KeyCode::Char('v') => {
                if self.active_tab == ActiveTab::Sources {
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
                }
            }
            KeyCode::PageDown => {
                if self.active_tab == ActiveTab::PaperDraft {
                    self.paper_scroll_offset += 5;
                } else if self.active_tab == ActiveTab::Sources {
                    self.source_scroll_offset += 5;
                }
            }

            KeyCode::Up | KeyCode::Char('k') => match self.active_tab {
                ActiveTab::Dashboard => {}
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
        self.ref_sort_key = RefSortKey::Index;
        self.input_mode = InputMode::ViewingSourceRefs;
        self.status_message = format!(
            "Viewing references for {filename}. Keys: 'y' (Year), 's' (Source), 'v' (Venue), 'i' (Index), Esc to close."
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
        self.ref_sort_key = RefSortKey::Index;
        self.input_mode = InputMode::ViewingSourceRefs;
        self.status_message = format!(
            "Viewing all {} references in project. Keys: 'y' (Year), 's' (Source), 'v' (Venue), 'i' (Index), Esc to close.",
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
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.input_mode = InputMode::Normal;
                self.selected_source_references.clear();
                self.status_message = "Closed references window.".to_string();
            }
            KeyCode::Char('y') => {
                self.ref_sort_key = RefSortKey::Year;
                self.selected_source_references.sort_by(|a, b| {
                    b.year.unwrap_or(0).cmp(&a.year.unwrap_or(0))
                });
                self.status_message = "Sorted references by Year (descending).".to_string();
            }
            KeyCode::Char('s') => {
                self.ref_sort_key = RefSortKey::Source;
                self.selected_source_references.sort_by(|a, b| {
                    a.source_id.as_str().cmp(b.source_id.as_str())
                });
                self.status_message = "Sorted references by Source document.".to_string();
            }
            KeyCode::Char('v') | KeyCode::Char('j') => {
                self.ref_sort_key = RefSortKey::Venue;
                self.selected_source_references.sort_by(|a, b| {
                    a.venue.as_deref().unwrap_or("").cmp(b.venue.as_deref().unwrap_or(""))
                });
                self.status_message = "Sorted references by Journal/Conference (Venue).".to_string();
            }
            KeyCode::Char('i') | KeyCode::Char('n') => {
                self.ref_sort_key = RefSortKey::Index;
                self.selected_source_references.sort_by(|a, b| {
                    a.ref_index.cmp(&b.ref_index)
                });
                self.status_message = "Sorted references by original Index.".to_string();
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
        assert!(!app.should_quit);
    }

    #[test]
    fn tab_navigation() {
        let mut app = App::new(None);
        assert_eq!(app.active_tab, ActiveTab::Dashboard);
        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.active_tab, ActiveTab::PaperDraft);
        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.active_tab, ActiveTab::Sources);
        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.active_tab, ActiveTab::Settings);
        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.active_tab, ActiveTab::Dashboard);
    }

    #[test]
    fn add_and_use_coauthor_flow() {
        let mut app = App::new(None);
        let initial_cache_len = app.cache.co_authors.len();
        app.active_tab = ActiveTab::Settings;
        let items = app.setting_items();
        let cache_coauthor_idx = items
            .iter()
            .position(|it| {
                matches!(
                    it,
                    SettingItem::CacheCoAuthorEmpty | SettingItem::CacheCoAuthor(_)
                )
            })
            .unwrap();
        app.selected_setting_index = cache_coauthor_idx;

        app.handle_key(KeyEvent::from(KeyCode::Char('a')));
        assert_eq!(app.input_mode, InputMode::ModalAddAuthor);

        // Type author details
        for c in "Dr. Smith".chars() {
            app.handle_key(KeyEvent::from(KeyCode::Char(c)));
        }
        app.handle_key(KeyEvent::from(KeyCode::Enter));

        assert_eq!(app.cache.co_authors.len(), initial_cache_len + 1);
        assert!(app.cache.co_authors.iter().any(|a| a.name == "Dr. Smith"));
        assert_eq!(app.local_settings.co_authors.len(), 1);
        assert_eq!(app.local_settings.co_authors[0].name, "Dr. Smith");
    }

    #[test]
    fn test_direct_digit_tab_switching() {
        let mut app = App::new(None);
        app.handle_key(KeyEvent::from(KeyCode::Char('1')));
        assert_eq!(app.active_tab, ActiveTab::Dashboard);
        app.handle_key(KeyEvent::from(KeyCode::Char('2')));
        assert_eq!(app.active_tab, ActiveTab::PaperDraft);
        app.handle_key(KeyEvent::from(KeyCode::Char('3')));
        assert_eq!(app.active_tab, ActiveTab::Sources);
        app.handle_key(KeyEvent::from(KeyCode::Char('4')));
        assert_eq!(app.active_tab, ActiveTab::Settings);
    }

    #[test]
    fn test_backtab_reverse_navigation() {
        let mut app = App::new(None);
        assert_eq!(app.active_tab, ActiveTab::Dashboard);
        app.handle_key(KeyEvent::from(KeyCode::BackTab));
        assert_eq!(app.active_tab, ActiveTab::Settings);
        app.handle_key(KeyEvent::from(KeyCode::BackTab));
        assert_eq!(app.active_tab, ActiveTab::Sources);
        app.handle_key(KeyEvent::from(KeyCode::BackTab));
        assert_eq!(app.active_tab, ActiveTab::PaperDraft);
    }

    #[test]
    fn test_sources_tab_add_rename_delete_read() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let mut app = App::new(Some(root));

        // Switch to Sources tab
        app.handle_key(KeyEvent::from(KeyCode::Char('3')));
        assert_eq!(app.active_tab, ActiveTab::Sources);

        // Add source via link ('a')
        app.handle_key(KeyEvent::from(KeyCode::Char('a')));
        assert_eq!(app.input_mode, InputMode::ModalAddSourceLink);
        for c in "https://arxiv.org/pdf/2401.00001.pdf".chars() {
            app.handle_key(KeyEvent::from(KeyCode::Char(c)));
        }
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(!app.sources.is_empty());

        // Rename title ('r')
        app.handle_key(KeyEvent::from(KeyCode::Char('r')));
        assert_eq!(app.input_mode, InputMode::ModalRenameSource);
        app.rename_source_buffer = "Attention Paper".to_string();
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.sources[0].title.as_deref(), Some("Attention Paper"));

        // Read Markdown (Enter)
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::ReadingSourceMd);
        assert!(app.reading_md_content.is_some());
        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.source_scroll_offset, 1);
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);

        // View References ('v')
        app.handle_key(KeyEvent::from(KeyCode::Char('v')));
        assert_eq!(app.input_mode, InputMode::ViewingSourceRefs);
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);

        // Delete source ('d')
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert_eq!(app.input_mode, InputMode::ConfirmDeleteSource);
        app.handle_key(KeyEvent::from(KeyCode::Char('y')));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.sources.is_empty());
    }

    #[test]
    fn test_unified_settings_navigation_and_editing() {
        let mut app = App::new(None);
        app.handle_key(KeyEvent::from(KeyCode::Char('4')));
        assert_eq!(app.active_tab, ActiveTab::Settings);
        assert_eq!(app.selected_setting_index, 0);

        // Edit first global field (AuthorName)
        app.handle_key(KeyEvent::from(KeyCode::Char('e')));
        assert_eq!(app.input_mode, InputMode::Editing);
        app.input_buffer = "Alice Turing".to_string();
        app.handle_key(KeyEvent::from(KeyCode::Enter));

        assert_eq!(app.global_settings.author.name, "Alice Turing");
        assert!(app.dirty);

        // Move down to RAG field
        let items = app.setting_items();
        let rag_embedder_idx = items
            .iter()
            .position(|it| matches!(it, SettingItem::Rag(RagField::EmbedderPath)))
            .unwrap();
        app.selected_setting_index = rag_embedder_idx;

        app.handle_key(KeyEvent::from(KeyCode::Char('e')));
        assert_eq!(app.input_mode, InputMode::Editing);
        app.input_buffer = "/tmp/models/embedder.onnx".to_string();
        app.handle_key(KeyEvent::from(KeyCode::Enter));

        assert_eq!(
            app.global_settings
                .rag
                .onnx_embedder_path
                .as_ref()
                .map(|p| p.as_str()),
            Some("/tmp/models/embedder.onnx")
        );
    }
}
