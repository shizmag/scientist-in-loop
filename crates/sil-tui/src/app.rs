//! Application state and logic for `sil-tui`.

use camino::Utf8PathBuf;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use sil_core::{
    AuthorDetails, Config, GlobalSettings, GrantDetails, LocalSettings, ProjectPaths,
    SettingsCache,
};

/// Navigation tabs in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Dashboard = 0,
    PaperDraft = 1,
    GlobalSettings = 2,
    LocalSettings = 3,
    CoAuthorCache = 4,
    GrantCache = 5,
    RagSettings = 6,
}

impl ActiveTab {
    pub const ALL: [ActiveTab; 7] = [
        ActiveTab::Dashboard,
        ActiveTab::PaperDraft,
        ActiveTab::GlobalSettings,
        ActiveTab::LocalSettings,
        ActiveTab::CoAuthorCache,
        ActiveTab::GrantCache,
        ActiveTab::RagSettings,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            ActiveTab::Dashboard => "1. Dashboard",
            ActiveTab::PaperDraft => "2. Paper Draft",
            ActiveTab::GlobalSettings => "3. Global Settings",
            ActiveTab::LocalSettings => "4. Local Settings",
            ActiveTab::CoAuthorCache => "5. Co-Authors Cache",
            ActiveTab::GrantCache => "6. Grants Cache",
            ActiveTab::RagSettings => "7. RAG Settings",
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
        };
        app.reload_paper_draft();
        app
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
            KeyCode::Char('3') => self.active_tab = ActiveTab::GlobalSettings,
            KeyCode::Char('4') => self.active_tab = ActiveTab::LocalSettings,
            KeyCode::Char('5') => self.active_tab = ActiveTab::CoAuthorCache,
            KeyCode::Char('6') => self.active_tab = ActiveTab::GrantCache,
            KeyCode::Char('7') => self.active_tab = ActiveTab::RagSettings,

            KeyCode::Char('s') => self.save_all(),
            KeyCode::Char('v') => {
                if self.active_tab == ActiveTab::PaperDraft || self.project_root.is_some() {
                    self.pending_external_editor = true;
                    self.status_message = "Launching external editor ($EDITOR / nvim / helix)...".to_string();
                }
            }

            KeyCode::PageUp => {
                if self.active_tab == ActiveTab::PaperDraft {
                    self.paper_scroll_offset = self.paper_scroll_offset.saturating_sub(5);
                }
            }
            KeyCode::PageDown => {
                if self.active_tab == ActiveTab::PaperDraft {
                    self.paper_scroll_offset += 5;
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
                ActiveTab::GlobalSettings => {
                    if self.selected_global_field > 0 {
                        self.selected_global_field -= 1;
                    }
                }
                ActiveTab::LocalSettings => {
                    if self.selected_local_field > 0 {
                        self.selected_local_field -= 1;
                    }
                }
                ActiveTab::CoAuthorCache => {
                    if self.cache_coauthor_index > 0 {
                        self.cache_coauthor_index -= 1;
                    }
                }
                ActiveTab::GrantCache => {
                    if self.cache_grant_index > 0 {
                        self.cache_grant_index -= 1;
                    }
                }
                ActiveTab::RagSettings => {
                    if self.selected_rag_field > 0 {
                        self.selected_rag_field -= 1;
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
                ActiveTab::GlobalSettings => {
                    if self.selected_global_field + 1 < GlobalField::ALL.len() {
                        self.selected_global_field += 1;
                    }
                }
                ActiveTab::LocalSettings => {
                    if self.selected_local_field + 1 < LocalField::ALL.len() {
                        self.selected_local_field += 1;
                    }
                }

                ActiveTab::CoAuthorCache => {
                    if !self.cache.co_authors.is_empty()
                        && self.cache_coauthor_index + 1 < self.cache.co_authors.len()
                    {
                        self.cache_coauthor_index += 1;
                    }
                }
                ActiveTab::GrantCache => {
                    if !self.cache.grants.is_empty()
                        && self.cache_grant_index + 1 < self.cache.grants.len()
                    {
                        self.cache_grant_index += 1;
                    }
                }
                ActiveTab::RagSettings => {
                    if self.selected_rag_field + 1 < RagField::ALL.len() {
                        self.selected_rag_field += 1;
                    }
                }
            },
            KeyCode::Enter | KeyCode::Char('e') => self.start_editing_selected_field(),

            // Actions for Co-Authors / Grants
            KeyCode::Char('a') => match self.active_tab {
                ActiveTab::LocalSettings => {
                    if self.selected_local_field == LocalField::CoAuthorsList as usize {
                        if !self.cache.co_authors.is_empty() {
                            self.input_mode = InputMode::ModalPicker;
                            self.status_message = "Select co-author from cache (↑/↓ to navigate, Enter to select, Esc to cancel)".to_string();
                        } else {
                            self.new_author = AuthorDetails::default();
                            self.modal_field_index = 0;
                            self.input_mode = InputMode::ModalAddAuthor;
                        }
                    } else if self.selected_local_field == LocalField::GrantsList as usize {
                        if !self.cache.grants.is_empty() {
                            self.input_mode = InputMode::ModalPicker;
                            self.status_message = "Select grant from cache (↑/↓ to navigate, Enter to select, Esc to cancel)".to_string();
                        } else {
                            self.new_grant = GrantDetails::default();
                            self.modal_field_index = 0;
                            self.input_mode = InputMode::ModalAddGrant;
                        }
                    }
                }
                ActiveTab::CoAuthorCache => {
                    self.new_author = AuthorDetails::default();
                    self.modal_field_index = 0;
                    self.input_mode = InputMode::ModalAddAuthor;
                }
                ActiveTab::GrantCache => {
                    self.new_grant = GrantDetails::default();
                    self.modal_field_index = 0;
                    self.input_mode = InputMode::ModalAddGrant;
                }
                _ => {}
            },
            KeyCode::Char('d') | KeyCode::Delete => match self.active_tab {
                ActiveTab::LocalSettings => {
                    if self.selected_local_field == LocalField::CoAuthorsList as usize
                        && !self.local_settings.co_authors.is_empty()
                        && self.local_coauthor_index < self.local_settings.co_authors.len()
                    {
                        self.local_settings.co_authors.remove(self.local_coauthor_index);
                        if self.local_coauthor_index > 0 {
                            self.local_coauthor_index -= 1;
                        }
                        self.dirty = true;
                        self.status_message = "Removed co-author from project local settings.".to_string();
                    } else if self.selected_local_field == LocalField::GrantsList as usize
                        && !self.local_settings.grants.is_empty()
                        && self.local_grant_index < self.local_settings.grants.len()
                    {
                        self.local_settings.grants.remove(self.local_grant_index);
                        if self.local_grant_index > 0 {
                            self.local_grant_index -= 1;
                        }
                        self.dirty = true;
                        self.status_message = "Removed grant from project local settings.".to_string();
                    }
                }
                ActiveTab::CoAuthorCache => {
                    if !self.cache.co_authors.is_empty()
                        && self.cache_coauthor_index < self.cache.co_authors.len()
                    {
                        self.cache.co_authors.remove(self.cache_coauthor_index);
                        if self.cache_coauthor_index > 0 {
                            self.cache_coauthor_index -= 1;
                        }
                        self.dirty = true;
                        self.status_message = "Removed co-author from cache.".to_string();
                    }
                }
                ActiveTab::GrantCache => {
                    if !self.cache.grants.is_empty()
                        && self.cache_grant_index < self.cache.grants.len()
                    {
                        self.cache.grants.remove(self.cache_grant_index);
                        if self.cache_grant_index > 0 {
                            self.cache_grant_index -= 1;
                        }
                        self.dirty = true;
                        self.status_message = "Removed grant from cache.".to_string();
                    }
                }
                _ => {}
            },
            KeyCode::Char('u') => {
                // Use selected cache item in local settings
                if self.active_tab == ActiveTab::CoAuthorCache && !self.cache.co_authors.is_empty() {
                    let author = self.cache.co_authors[self.cache_coauthor_index].clone();
                    if !self.local_settings.co_authors.contains(&author) {
                        self.local_settings.co_authors.push(author);
                        self.dirty = true;
                        self.status_message = "Added cached co-author to local project!".to_string();
                    }
                } else if self.active_tab == ActiveTab::GrantCache && !self.cache.grants.is_empty() {
                    let grant = self.cache.grants[self.cache_grant_index].clone();
                    if !self.local_settings.grants.contains(&grant) {
                        self.local_settings.grants.push(grant);
                        self.dirty = true;
                        self.status_message = "Added cached grant to local project!".to_string();
                    }
                }
            }
            _ => {}
        }
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
            ActiveTab::GlobalSettings => {
                self.input_buffer = match GlobalField::ALL[self.selected_global_field] {
                    GlobalField::AuthorName => self.global_settings.author.name.clone(),
                    GlobalField::AuthorEmail => self.global_settings.author.email.clone(),
                    GlobalField::AuthorAffiliation => self.global_settings.author.affiliation.clone(),
                    GlobalField::AuthorOrcid => self.global_settings.author.orcid.clone().unwrap_or_default(),
                    GlobalField::GrantFunder => self.global_settings.default_grant.funder.clone(),
                    GlobalField::GrantNumber => self.global_settings.default_grant.grant_number.clone(),
                    GlobalField::GrantAck => self.global_settings.default_grant.acknowledgment.clone(),
                    GlobalField::Engine => self.global_settings.default_latex_engine.clone(),
                    GlobalField::Template => self.global_settings.default_template.clone(),
                };
                self.input_mode = InputMode::Editing;
                self.status_message = "Editing field. Press Enter to confirm, Esc to cancel.".to_string();
            }
            ActiveTab::LocalSettings => {
                if self.selected_local_field == LocalField::Title as usize {
                    self.input_buffer = self.local_settings.title.clone();
                    self.input_mode = InputMode::Editing;
                    self.status_message = "Editing project title. Press Enter to confirm, Esc to cancel.".to_string();
                } else if self.selected_local_field == LocalField::Notes as usize {
                    self.input_buffer = self.local_settings.notes.clone();
                    self.input_mode = InputMode::Editing;
                    self.status_message = "Editing project notes. Press Enter to confirm, Esc to cancel.".to_string();
                }
            }
            ActiveTab::RagSettings => {
                self.input_buffer = match RagField::ALL[self.selected_rag_field] {
                    RagField::EmbedderPath => self.global_settings.rag.onnx_embedder_path.as_ref().map(|p| p.to_string()).unwrap_or_default(),
                    RagField::RerankerPath => self.global_settings.rag.onnx_reranker_path.as_ref().map(|p| p.to_string()).unwrap_or_default(),
                    RagField::ModelsDir => self.global_settings.rag.onnx_models_dir.as_ref().map(|p| p.to_string()).unwrap_or_default(),
                    RagField::CacheDir => self.global_settings.rag.model_cache_dir.to_string(),
                    RagField::ExecutionProvider => self.global_settings.rag.execution_provider.clone(),
                    RagField::NumThreads => self.global_settings.rag.num_threads.to_string(),
                    RagField::ParentChunkSize => self.global_settings.rag.parent_chunk_size.to_string(),
                    RagField::ChildChunkSize => self.global_settings.rag.child_chunk_size.to_string(),
                };
                self.input_mode = InputMode::Editing;
                self.status_message = "Editing RAG setting. Press Enter to confirm, Esc to cancel.".to_string();
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
        if !self.paper_sections.is_empty()
            && self.paper_section_index < self.paper_sections.len()
        {
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
        match self.active_tab {
            ActiveTab::GlobalSettings => {
                match GlobalField::ALL[self.selected_global_field] {
                    GlobalField::AuthorName => self.global_settings.author.name = val,
                    GlobalField::AuthorEmail => self.global_settings.author.email = val,
                    GlobalField::AuthorAffiliation => self.global_settings.author.affiliation = val,
                    GlobalField::AuthorOrcid => {
                        self.global_settings.author.orcid = if val.is_empty() { None } else { Some(val) }
                    }
                    GlobalField::GrantFunder => self.global_settings.default_grant.funder = val,
                    GlobalField::GrantNumber => self.global_settings.default_grant.grant_number = val,
                    GlobalField::GrantAck => self.global_settings.default_grant.acknowledgment = val,
                    GlobalField::Engine => self.global_settings.default_latex_engine = val,
                    GlobalField::Template => self.global_settings.default_template = val,
                }
            }
            ActiveTab::LocalSettings => {
                if self.selected_local_field == LocalField::Title as usize {
                    self.local_settings.title = val;
                } else if self.selected_local_field == LocalField::Notes as usize {
                    self.local_settings.notes = val;
                }
            }
            ActiveTab::RagSettings => {
                match RagField::ALL[self.selected_rag_field] {
                    RagField::EmbedderPath => {
                        let resolved = resolve_onnx_from_dir(&val);
                        self.global_settings.rag.onnx_embedder_path = if resolved.is_empty() { None } else { Some(camino::Utf8PathBuf::from(resolved)) };
                    }
                    RagField::RerankerPath => {
                        let resolved = resolve_onnx_from_dir(&val);
                        self.global_settings.rag.onnx_reranker_path = if resolved.is_empty() { None } else { Some(camino::Utf8PathBuf::from(resolved)) };
                    }
                    RagField::CacheDir => self.global_settings.rag.model_cache_dir = camino::Utf8PathBuf::from(val),
                    RagField::ModelsDir => {
                        self.global_settings.rag.onnx_models_dir = if val.is_empty() { None } else { Some(camino::Utf8PathBuf::from(val)) };
                    }
                    RagField::ExecutionProvider => self.global_settings.rag.execution_provider = val,
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
                }
            }
            _ => {}
        }
    }

    fn handle_modal_picker_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.status_message = "Picker closed.".to_string();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_local_field == LocalField::CoAuthorsList as usize && self.cache_coauthor_index > 0 {
                    self.cache_coauthor_index -= 1;
                } else if self.selected_local_field == LocalField::GrantsList as usize && self.cache_grant_index > 0 {
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
                if self.selected_local_field == LocalField::CoAuthorsList as usize && !self.cache.co_authors.is_empty() {
                    let author = self.cache.co_authors[self.cache_coauthor_index].clone();
                    if !self.local_settings.co_authors.contains(&author) {
                        self.local_settings.co_authors.push(author);
                    }
                } else if self.selected_local_field == LocalField::GrantsList as usize && !self.cache.grants.is_empty() {
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
                // Switch to manually adding new item
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
                self.modal_field_index = if self.modal_field_index == 0 { 3 } else { self.modal_field_index - 1 };
            }
            KeyCode::Enter => {
                if !self.new_author.name.trim().is_empty() {
                    let author = self.new_author.clone();
                    self.cache.remember_co_author(author.clone());
                    if (self.active_tab == ActiveTab::LocalSettings
                        || self.selected_local_field == LocalField::CoAuthorsList as usize)
                        && !self.local_settings.co_authors.contains(&author)
                    {
                        self.local_settings.co_authors.push(author);
                    }
                    self.dirty = true;
                    self.input_mode = InputMode::Normal;
                    self.status_message = "Co-author saved to cache and local settings!".to_string();
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
                self.modal_field_index = if self.modal_field_index == 0 { 2 } else { self.modal_field_index - 1 };
            }
            KeyCode::Enter => {
                if !self.new_grant.funder.trim().is_empty() || !self.new_grant.grant_number.trim().is_empty() {
                    let grant = self.new_grant.clone();
                    self.cache.remember_grant(grant.clone());
                    if (self.active_tab == ActiveTab::LocalSettings
                        || self.selected_local_field == LocalField::GrantsList as usize)
                        && !self.local_settings.grants.contains(&grant)
                    {
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
        assert_eq!(app.active_tab, ActiveTab::GlobalSettings);
        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.active_tab, ActiveTab::LocalSettings);
        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.active_tab, ActiveTab::CoAuthorCache);
        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.active_tab, ActiveTab::GrantCache);
        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.active_tab, ActiveTab::RagSettings);
        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.active_tab, ActiveTab::Dashboard);
    }

    #[test]
    fn add_and_use_coauthor_flow() {
        let mut app = App::new(None);
        app.active_tab = ActiveTab::CoAuthorCache;
        app.handle_key(KeyEvent::from(KeyCode::Char('a')));
        assert_eq!(app.input_mode, InputMode::ModalAddAuthor);

        // Type author details
        for c in "Dr. Smith".chars() {
            app.handle_key(KeyEvent::from(KeyCode::Char(c)));
        }
        app.handle_key(KeyEvent::from(KeyCode::Enter));

        assert_eq!(app.cache.co_authors.len(), 1);
        assert_eq!(app.cache.co_authors[0].name, "Dr. Smith");

        // Use in local project
        app.handle_key(KeyEvent::from(KeyCode::Char('u')));
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
        assert_eq!(app.active_tab, ActiveTab::GlobalSettings);
        app.handle_key(KeyEvent::from(KeyCode::Char('4')));
        assert_eq!(app.active_tab, ActiveTab::LocalSettings);
        app.handle_key(KeyEvent::from(KeyCode::Char('5')));
        assert_eq!(app.active_tab, ActiveTab::CoAuthorCache);
        app.handle_key(KeyEvent::from(KeyCode::Char('6')));
        assert_eq!(app.active_tab, ActiveTab::GrantCache);
        app.handle_key(KeyEvent::from(KeyCode::Char('7')));
        assert_eq!(app.active_tab, ActiveTab::RagSettings);
    }

    #[test]
    fn test_backtab_reverse_navigation() {
        let mut app = App::new(None);
        assert_eq!(app.active_tab, ActiveTab::Dashboard);
        app.handle_key(KeyEvent::from(KeyCode::BackTab));
        assert_eq!(app.active_tab, ActiveTab::RagSettings);
        app.handle_key(KeyEvent::from(KeyCode::BackTab));
        assert_eq!(app.active_tab, ActiveTab::GrantCache);
        app.handle_key(KeyEvent::from(KeyCode::BackTab));
        assert_eq!(app.active_tab, ActiveTab::CoAuthorCache);
    }

    #[test]
    fn test_rag_settings_tui_editing() {
        let mut app = App::new(None);
        app.handle_key(KeyEvent::from(KeyCode::Char('7')));
        assert_eq!(app.active_tab, ActiveTab::RagSettings);
        assert_eq!(app.selected_rag_field, 0);

        // Edit embedder path (field 0)
        app.handle_key(KeyEvent::from(KeyCode::Char('e')));
        assert_eq!(app.input_mode, InputMode::Editing);
        app.input_buffer = "/tmp/my_models/embedder.onnx".to_string();
        app.handle_key(KeyEvent::from(KeyCode::Enter));

        assert_eq!(
            app.global_settings.rag.onnx_embedder_path.as_ref().map(|p| p.as_str()),
            Some("/tmp/my_models/embedder.onnx")
        );
        assert!(app.dirty);

        // Move to num_threads (field 5 in 8-element list)
        for _ in 0..5 {
            app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        }
        assert_eq!(app.selected_rag_field, RagField::NumThreads as usize);

        app.handle_key(KeyEvent::from(KeyCode::Enter));
        app.input_buffer = "12".to_string();
        app.handle_key(KeyEvent::from(KeyCode::Enter));

        assert_eq!(app.global_settings.rag.num_threads, 12);
    }

    #[test]
    fn test_rag_settings_directory_onnx_resolution() {
        let temp_dir = tempfile::tempdir().unwrap();
        let model_file = temp_dir.path().join("my_custom_reranker.onnx");
        std::fs::write(&model_file, b"dummy onnx content").unwrap();

        let mut app = App::new(None);
        app.active_tab = ActiveTab::RagSettings;
        app.selected_rag_field = RagField::RerankerPath as usize;

        app.input_mode = InputMode::Editing;
        app.input_buffer = temp_dir.path().to_string_lossy().to_string();
        app.handle_key(KeyEvent::from(KeyCode::Enter));

        assert_eq!(
            app.global_settings.rag.onnx_reranker_path.as_ref().map(|p| p.as_path()),
            Some(camino::Utf8Path::new(model_file.to_str().unwrap()))
        );
    }

    #[test]
    fn test_dashboard_quit_signal() {
        let mut app = App::new(None);
        app.active_tab = ActiveTab::Dashboard;
        app.handle_key(KeyEvent::from(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn test_paper_draft_reading_and_editing() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let draft_path = root.join("paper_draft.tex");

        let initial_tex = r#"\documentclass{article}
\begin{document}
\section{Introduction}
Intro text here.

% # -- X -- #
% TODO: add ablation table
% # -- X -- #

\section{Methods}
Original methods text.
\end{document}
"#;
        std::fs::write(draft_path.as_std_path(), initial_tex).unwrap();

        let mut app = App::new(Some(root.clone()));
        assert_eq!(app.paper_sections.len(), 2);
        assert_eq!(app.paper_sections[0].title, "Introduction");
        assert_eq!(app.paper_sections[1].title, "Methods");

        // Switch to paper draft tab
        app.handle_key(KeyEvent::from(KeyCode::Char('2')));
        assert_eq!(app.active_tab, ActiveTab::PaperDraft);

        // Move to Methods section
        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.paper_section_index, 1);

        // Enter edit mode
        app.handle_key(KeyEvent::from(KeyCode::Char('e')));
        assert_eq!(app.input_mode, InputMode::EditingPaper);
        assert!(app.paper_edit_buffer.contains("Original methods text."));

        // Modify section text
        app.paper_edit_buffer = "Updated methods text with new algorithm.\n".to_string();
        app.handle_key(KeyEvent::from(KeyCode::Enter));

        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.paper_draft_content.contains("Updated methods text with new algorithm."));

        // Save app state
        app.save_all();
        assert!(!app.dirty);

        let saved = std::fs::read_to_string(draft_path.as_std_path()).unwrap();
        assert!(saved.contains("Updated methods text with new algorithm."));
    }
}

