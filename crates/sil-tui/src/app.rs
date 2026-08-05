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
    HelpOverlay,
}

/// Sorting key for references display in TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefSortKey {
    Index,
    Year,
    Source,
    Venue,
    Similarity,
    Title,
}

/// Classification of user input in the Add Source Link modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceInputKind {
    Doi,
    Arxiv,
    Url,
    Filename,
}

impl SourceInputKind {
    pub fn label(&self) -> &'static str {
        match self {
            SourceInputKind::Doi => "DOI",
            SourceInputKind::Arxiv => "arXiv ID",
            SourceInputKind::Url => "URL",
            SourceInputKind::Filename => "Filename",
        }
    }
}

/// Classify an input string as a DOI, arXiv ID, URL, or plain Filename.
pub fn classify_source_input(input: &str) -> SourceInputKind {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return SourceInputKind::Filename;
    }
    let lower = trimmed.to_lowercase();
    if lower.starts_with("arxiv:")
        || lower.contains("arxiv.org/")
        || sil_regex::extract_arxiv_id(trimmed).is_some()
    {
        SourceInputKind::Arxiv
    } else if lower.starts_with("doi:")
        || trimmed.starts_with("10.")
        || lower.contains("doi.org/")
        || sil_regex::extract_doi(trimmed).is_some()
    {
        SourceInputKind::Doi
    } else if lower.starts_with("http://")
        || lower.starts_with("https://")
        || sil_regex::extract_url(trimmed).is_some()
    {
        SourceInputKind::Url
    } else {
        SourceInputKind::Filename
    }
}

/// Helper context mode for keyboard help overlays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpMode {
    Dashboard,
    SourcesList,
    ReadingSourceMd,
    ViewingSourceRefs,
    ReferencesLeft,
    ReferencesRight,
    PaperDraft,
    Settings,
    ModalPicker,
    ModalAddAuthor,
    ModalAddGrant,
    ModalAddSourceLink,
    ModalRenameSource,
    ConfirmDeleteSource,
    Editing,
    EditingPaper,
    SearchingRefs,
    SearchingBib,
    SearchingViewingRefs,
}

impl HelpMode {
    pub fn title(&self) -> &'static str {
        match self {
            HelpMode::Dashboard => "Dashboard",
            HelpMode::SourcesList => "Sources List",
            HelpMode::ReadingSourceMd => "Reading Source Markdown",
            HelpMode::ViewingSourceRefs => "Viewing Source References",
            HelpMode::ReferencesLeft => "References (references.bib)",
            HelpMode::ReferencesRight => "References (Extracted References)",
            HelpMode::PaperDraft => "Paper Draft Viewer & Editor",
            HelpMode::Settings => "Unified Settings",
            HelpMode::ModalPicker => "Co-Author / Grant Picker Modal",
            HelpMode::ModalAddAuthor => "Add Co-Author Modal",
            HelpMode::ModalAddGrant => "Add Grant Modal",
            HelpMode::ModalAddSourceLink => "Add Source Link Modal",
            HelpMode::ModalRenameSource => "Rename Source Title Modal",
            HelpMode::ConfirmDeleteSource => "Confirm Delete Source Modal",
            HelpMode::Editing => "Field Text Editing Modal",
            HelpMode::EditingPaper => "Paper Section Editing Modal",
            HelpMode::SearchingRefs => "Searching Extracted References",
            HelpMode::SearchingBib => "Searching references.bib",
            HelpMode::SearchingViewingRefs => "Searching Source References",
        }
    }
}

/// Pure function returning the (key, action) mapping for a given help context mode.
pub fn keymap_for(mode: HelpMode) -> Vec<(&'static str, &'static str)> {
    match mode {
        HelpMode::Dashboard => vec![
            ("1 - 5", "Switch directly to tab (Dashboard, Sources, References, Draft, Settings)"),
            ("Tab / Shift+Tab", "Cycle forward / backward through tabs"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("q / Esc", "Quit application"),
            ("Ctrl+S / s", "Save all settings and project state"),
        ],
        HelpMode::SourcesList => vec![
            ("j / Down", "Select next source document"),
            ("k / Up", "Select previous source document"),
            ("PageUp / PageDown", "Scroll source list by 5 items"),
            ("Enter", "Read full source document in Markdown viewer"),
            ("e / E", "Parse/extract text and references for selected source ('E' / Shift+E for force re-parse)"),
            ("v", "View extracted reference citations for selected source"),
            ("a", "Add new source document via link / URL / DOI / arXiv"),
            ("b", "Append selected source to references.bib (hydrates metadata if DOI/arXiv)"),
            ("r", "Rename selected source document title"),
            ("R", "Reload sources from disk and database"),
            ("d / Delete", "Delete selected source document (requires confirmation)"),
            ("1 - 5", "Switch to tab"),
            ("Tab / Shift+Tab", "Cycle tabs"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("q / Esc", "Quit application"),
            ("Ctrl+S / s", "Save all settings and project state"),
        ],
        HelpMode::ReadingSourceMd => vec![
            ("j / Down", "Scroll down 1 line"),
            ("k / Up", "Scroll up 1 line"),
            ("PageUp / PageDown", "Scroll up / down 10 lines"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("q / Esc", "Exit Markdown reader mode"),
        ],
        HelpMode::ViewingSourceRefs => vec![
            ("j / Down", "Navigate down to next reference"),
            ("k / Up", "Navigate up to previous reference"),
            ("PageUp / PageDown", "Jump 5 references up / down"),
            ("g / Home", "Jump to first reference"),
            ("G / End", "Jump to last reference"),
            ("Space", "Toggle selection mark on highlighted reference"),
            ("c / b / p", "Append marked (or highlighted) reference to references.bib"),
            ("a", "Append ALL filtered references to references.bib"),
            ("d / e", "Toggle reference inspector card & BibTeX preview"),
            ("/ / f", "Search / filter references by text query"),
            ("y", "Sort references by Year (descending)"),
            ("v", "Sort references by Venue / Journal"),
            ("s", "Sort references by Source document ID"),
            ("i / n", "Sort references by original Index"),
            ("t", "Sort references by Title"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("q / Esc", "Close references viewer (or clear search filter)"),
        ],
        HelpMode::ReferencesLeft => vec![
            ("j / Down", "Select next entry in references.bib"),
            ("k / Up", "Select previous entry in references.bib"),
            ("PageUp / PageDown", "Jump 5 entries up / down"),
            ("Tab", "Switch focus to Right Pane (Extracted References)"),
            ("/ / f", "Search references.bib entries"),
            ("P", "Promote TUI-added entry (strip % [sil: tui-added] marker)"),
            ("Delete", "Delete selected entry from references.bib"),
            ("1 - 5", "Switch to tab"),
            ("Shift+Tab", "Previous tab"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("q / Esc", "Clear search filter (or quit if search empty)"),
            ("Ctrl+S / s", "Save all settings and project state"),
        ],
        HelpMode::ReferencesRight => vec![
            ("j / Down", "Select next extracted reference"),
            ("k / Up", "Select previous extracted reference"),
            ("PageUp / PageDown", "Jump 5 references up / down"),
            ("Tab", "Switch focus to Left Pane (references.bib)"),
            ("Space", "Toggle selection mark on highlighted reference"),
            ("p", "Add marked (or highlighted) reference to references.bib"),
            ("P", "Promote highlighted entry in references.bib"),
            ("/ / f", "Search extracted references"),
            ("m / c", "Sort references by Draft Cosine Similarity (highest score first)"),
            ("X", "Recompute draft similarity scores against ONNX embeddings"),
            ("y", "Sort references by Year (descending)"),
            ("v", "Sort references by Venue / Journal"),
            ("s", "Sort references by Source document"),
            ("i", "Sort references by Index"),
            ("t", "Sort references by Title"),
            ("1 - 5", "Switch to tab"),
            ("Shift+Tab", "Previous tab"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("q / Esc", "Clear search filter (or quit if search empty)"),
            ("Ctrl+S", "Save all settings and project state"),
        ],
        HelpMode::PaperDraft => vec![
            ("j / Down", "Select next manuscript section"),
            ("k / Up", "Select previous manuscript section"),
            ("PageUp / PageDown", "Scroll section content preview"),
            ("e / Enter", "Edit section body in TUI popup editor"),
            ("v", "Launch external $EDITOR (nvim / helix / vim) on paper_draft.tex"),
            ("1 - 5", "Switch to tab"),
            ("Tab / Shift+Tab", "Cycle tabs"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("q / Esc", "Quit application"),
            ("Ctrl+S / s", "Save manuscript and re-index draft sections"),
        ],
        HelpMode::Settings => vec![
            ("j / Down", "Move cursor to next setting field"),
            ("k / Up", "Move cursor to previous setting field"),
            ("e / Enter", "Edit highlighted setting field value"),
            ("a", "Add item (modal author/grant or select from cache)"),
            ("d / Delete", "Remove highlighted cached or local co-author / grant"),
            ("u", "Import selected cached author/grant into local project settings"),
            ("1 - 5", "Switch to tab"),
            ("Tab / Shift+Tab", "Cycle tabs"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("q / Esc", "Quit application"),
            ("Ctrl+S / s", "Save global settings, local config, and cache"),
        ],
        HelpMode::ModalPicker => vec![
            ("j / Down", "Navigate down cache list"),
            ("k / Up", "Navigate up cache list"),
            ("Enter", "Select highlighted item into local project settings"),
            ("n", "Create new item manually"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("Esc", "Close picker modal"),
        ],
        HelpMode::ModalAddAuthor => vec![
            ("Tab / Down", "Focus next field (Name, Email, Affiliation, ORCID)"),
            ("Shift+Tab / Up", "Focus previous field"),
            ("Backspace", "Delete character in active field"),
            ("Char", "Type text into active field"),
            ("Enter", "Save co-author to cache and local project settings"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("Esc", "Cancel and close modal"),
        ],
        HelpMode::ModalAddGrant => vec![
            ("Tab / Down", "Focus next field (Funder, Number, Acknowledgment)"),
            ("Shift+Tab / Up", "Focus previous field"),
            ("Backspace", "Delete character in active field"),
            ("Char", "Type text into active field"),
            ("Enter", "Save grant to cache and local project settings"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("Esc", "Cancel and close modal"),
        ],
        HelpMode::ModalAddSourceLink => vec![
            ("Char", "Type URL / DOI / arXiv link"),
            ("Backspace", "Delete character"),
            ("Enter", "Submit link stub (no download)"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("Esc", "Cancel and close modal"),
        ],
        HelpMode::ModalRenameSource => vec![
            ("Char", "Type new source title"),
            ("Backspace", "Delete character"),
            ("Enter", "Confirm and update source title"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("Esc", "Cancel and close modal"),
        ],
        HelpMode::ConfirmDeleteSource => vec![
            ("y / Enter", "Confirm deletion of source document"),
            ("n / Esc", "Cancel deletion"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
        ],
        HelpMode::Editing => vec![
            ("Char", "Type value into input buffer"),
            ("Backspace", "Delete character"),
            ("Enter", "Confirm edit and update field"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("Esc", "Cancel edit"),
        ],
        HelpMode::EditingPaper => vec![
            ("Char", "Type text into section body buffer"),
            ("Backspace", "Delete character"),
            ("Enter", "Confirm section edit"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("Esc", "Cancel section edit"),
        ],
        HelpMode::SearchingRefs => vec![
            ("Char", "Type search query for extracted references"),
            ("Backspace", "Delete character from search query"),
            ("Enter / Esc", "Finish search mode"),
            ("F1", "Toggle mode-aware keyboard help overlay"),
        ],
        HelpMode::SearchingBib => vec![
            ("Char", "Type search query for references.bib entries"),
            ("Backspace", "Delete character from search query"),
            ("Enter / Esc", "Finish search mode"),
            ("F1", "Toggle mode-aware keyboard help overlay"),
        ],
        HelpMode::SearchingViewingRefs => vec![
            ("Char", "Type search query for source references"),
            ("Backspace", "Delete character from search query"),
            ("Enter / Esc", "Finish search mode"),
            ("F1", "Toggle mode-aware keyboard help overlay"),
        ],
    }
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
    XbergCacheDir = 4,
    ExecutionProvider = 5,
    NumThreads = 6,
    ParentChunkSize = 7,
    ChildChunkSize = 8,
}

impl RagField {
    pub const ALL: [RagField; 9] = [
        RagField::EmbedderPath,
        RagField::RerankerPath,
        RagField::ModelsDir,
        RagField::CacheDir,
        RagField::XbergCacheDir,
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

/// Result message from background metadata hydration thread.
#[derive(Debug, Clone)]
pub struct HydrationResult {
    pub dedup_key: String,
    pub label: String,
    pub outcome: HydrationOutcome,
}

/// Outcome of background metadata fetch.
#[derive(Debug, Clone)]
pub enum HydrationOutcome {
    Success { official_bib: String },
    Failure { reason: String },
}

/// Recent outcome of background metadata hydration.
#[derive(Debug, Clone)]
pub struct HydrationHistoryEntry {
    pub label: String,
    pub success: bool,
    pub detail: String,
}

/// Result of a background parse job.
#[derive(Debug)]
pub struct ParseJobResult {
    pub source_id: sil_core::SourceId,
    pub label: String,
    pub result: Result<sil_parse::batch::ParseResult, String>,
}

/// Application state struct for TUI.
pub struct App {
    pub active_tab: ActiveTab,
    pub input_mode: InputMode,
    pub saved_input_mode: InputMode,

    pub hydration_tx: std::sync::mpsc::Sender<HydrationResult>,
    pub hydration_rx: std::sync::mpsc::Receiver<HydrationResult>,
    pub in_flight_hydration_keys: std::collections::HashSet<String>,
    pub parse_tx: std::sync::mpsc::Sender<ParseJobResult>,
    pub parse_rx: std::sync::mpsc::Receiver<ParseJobResult>,
    pub in_flight_parse_ids: std::collections::HashSet<sil_core::SourceId>,
    pub hydration_batch_succeeded: usize,
    pub hydration_batch_failed: usize,
    pub recent_hydration_outcomes: std::collections::VecDeque<HydrationHistoryEntry>,

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

    // Draft similarity state
    pub draft_ref_similarities: std::collections::HashMap<String, f32>,
    pub draft_similarity_hash: Option<String>,
    pub min_similarity_filter: Option<f32>,

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

        let (hydration_tx, hydration_rx) = std::sync::mpsc::channel();
        let (parse_tx, parse_rx) = std::sync::mpsc::channel();

        let mut app = Self {
            hydration_tx,
            hydration_rx,
            in_flight_hydration_keys: std::collections::HashSet::new(),
            parse_tx,
            parse_rx,
            in_flight_parse_ids: std::collections::HashSet::new(),
            hydration_batch_succeeded: 0,
            hydration_batch_failed: 0,
            recent_hydration_outcomes: std::collections::VecDeque::with_capacity(20),
            project_root,
            loaded_config,
            global_settings,
            cache,
            local_settings,
            active_tab: ActiveTab::Dashboard,
            active_ref_pane: RefPane::RightSources,
            input_mode: InputMode::Normal,
            saved_input_mode: InputMode::Normal,

            source_references: Vec::new(),
            marked_ref_ids: std::collections::HashSet::new(),
            bib_file_entries: Vec::new(),
            selected_bib_index: 0,
            selected_source_ref_index: 0,

            ref_search_query: String::new(),
            bib_search_query: String::new(),

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

            draft_ref_similarities: std::collections::HashMap::new(),
            draft_similarity_hash: None,
            min_similarity_filter: None,

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

    pub fn queue_ref_hydration(&mut self, entry: ReferenceEntry) {
        let label = entry.title.as_deref().unwrap_or(&entry.raw_text).to_string();
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
        self.status_message = format!(
            "⏳ Hydrating ({} in flight)...",
            self.in_flight_hydration_keys.len()
        );
        let tx = self.hydration_tx.clone();

        std::thread::spawn(move || {
            let res = sil_parse::journal_digest::resolve_official_bibtex_entry(&entry);
            let outcome = match res {
                sil_parse::journal_digest::ReferenceBibResolution::Resolved(official_bib) => {
                    HydrationOutcome::Success { official_bib }
                }
                sil_parse::journal_digest::ReferenceBibResolution::Failed(reason) => {
                    HydrationOutcome::Failure { reason }
                }
            };
            let _ = tx.send(HydrationResult {
                dedup_key,
                label,
                outcome,
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
        self.status_message = format!(
            "⏳ Hydrating ({} in flight)...",
            self.in_flight_hydration_keys.len()
        );
        let tx = self.hydration_tx.clone();

        std::thread::spawn(move || {
            let res = sil_parse::journal_digest::resolve_official_bibtex_for_source(&doc);
            let outcome = match res {
                sil_parse::SourceBibResolution::Resolved(official_bib) => {
                    HydrationOutcome::Success { official_bib }
                }
                sil_parse::SourceBibResolution::Failed(reason) => {
                    HydrationOutcome::Failure { reason }
                }
            };
            let _ = tx.send(HydrationResult {
                dedup_key,
                label,
                outcome,
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
        self.status_message = format!("⏳ Parsing source '{label}'...");

        let tx = self.parse_tx.clone();
        let project_root = self.project_root.clone();
        let doc_id = doc.id.clone();
        let path = doc.path.clone();

        std::thread::spawn(move || {
            let result = (|| -> Result<sil_parse::batch::ParseResult, String> {
                let Some(root) = project_root else {
                    return Err("No project root directory available".to_string());
                };
                let paths = ProjectPaths::new(&root);
                let db = sil_db::SilDb::open(&paths.db())
                    .map_err(|e| format!("Database error: {e}"))?;

                if force {
                    let _ = db.remove_source(&doc_id);
                }

                let runner = sil_parse::discover_marker_runner().unwrap_or_else(|_| {
                    Box::new(sil_parse::StubMarkerRunner {
                        content: String::new(),
                    })
                });
                let null_ui = sil_core::NullUi::new();

                sil_parse::batch::parse_one(&path, &db, runner.as_ref(), &null_ui)
                    .map_err(|e| e.to_string())
            })();

            let _ = tx.send(ParseJobResult {
                source_id: doc_id,
                label,
                result,
            });
        });
    }

    pub fn poll_background_parse(&mut self) {
        let mut polled_any = false;
        while let Ok(res) = self.parse_rx.try_recv() {
            polled_any = true;
            self.in_flight_parse_ids.remove(&res.source_id);
            match res.result {
                Ok(_parse_res) => {
                    self.reload_sources();
                    self.load_all_source_references();
                    self.status_message = format!("✓ Parsed source '{}'", res.label);
                }
                Err(err_msg) => {
                    self.status_message =
                        format!("⚠ Failed parsing source '{}': {}", res.label, err_msg);
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

    pub fn poll_background_hydration(&mut self) {
        self.poll_background_parse();
        let mut polled_any = false;
        while let Ok(res) = self.hydration_rx.try_recv() {
            polled_any = true;
            self.in_flight_hydration_keys.remove(&res.dedup_key);
            match res.outcome {
                HydrationOutcome::Success { official_bib } => {
                    self.hydration_batch_succeeded += 1;
                    if self.recent_hydration_outcomes.len() >= 20 {
                        self.recent_hydration_outcomes.pop_front();
                    }

                    if let Some(ref root) = self.project_root {
                        let bib_path = root.join(sil_core::paths::rel::REFERENCES);
                        let current = match std::fs::read_to_string(bib_path.as_std_path()) {
                            Ok(c) => c,
                            Err(e) => {
                                let err_msg = format!("Error reading references.bib: {e}");
                                self.recent_hydration_outcomes.push_back(HydrationHistoryEntry {
                                    label: res.label.clone(),
                                    success: false,
                                    detail: err_msg,
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
                            let is_tui_added = sil_core::is_tui_added_bib_block(matching_block);
                            let entry_to_upsert = if is_tui_added {
                                sil_core::mark_tui_added_bib_entry(&official_bib)
                            } else {
                                sil_core::unmark_tui_added_bib_entry(&official_bib)
                            };

                            let (updated, _) = sil_core::bib::upsert_bib_entry_with_options(
                                &current,
                                &entry_to_upsert,
                                sil_core::bib::UpsertOptions {
                                    preserve_cite_key: true,
                                },
                            );

                            if let Err(e) = std::fs::write(bib_path.as_std_path(), updated) {
                                let err_msg = format!("Error writing references.bib: {e}");
                                self.recent_hydration_outcomes.push_back(HydrationHistoryEntry {
                                    label: res.label.clone(),
                                    success: false,
                                    detail: err_msg,
                                });
                            } else {
                                self.load_project_references_bib();
                                self.recent_hydration_outcomes.push_back(HydrationHistoryEntry {
                                    label: res.label.clone(),
                                    success: true,
                                    detail: format!("Official metadata for '{}'", res.label),
                                });
                            }
                        } else {
                            self.recent_hydration_outcomes.push_back(HydrationHistoryEntry {
                                label: res.label.clone(),
                                success: false,
                                detail: format!(
                                    "Skipped hydration for '{}': entry was deleted from references.bib",
                                    res.label
                                ),
                            });
                        }
                    } else {
                        self.recent_hydration_outcomes.push_back(HydrationHistoryEntry {
                            label: res.label.clone(),
                            success: true,
                            detail: format!("Official metadata for '{}'", res.label),
                        });
                    }
                }
                HydrationOutcome::Failure { reason } => {
                    self.hydration_batch_failed += 1;
                    if self.recent_hydration_outcomes.len() >= 20 {
                        self.recent_hydration_outcomes.pop_front();
                    }
                    self.recent_hydration_outcomes.push_back(HydrationHistoryEntry {
                        label: res.label.clone(),
                        success: false,
                        detail: reason,
                    });
                }
            }
        }

        if polled_any {
            if self.in_flight_hydration_keys.is_empty() {
                if self.hydration_batch_succeeded == 1 && self.hydration_batch_failed == 0 {
                    let last = self.recent_hydration_outcomes.back();
                    if let Some(h) = last && h.success {
                        self.status_message = format!("✓ Official metadata for '{}'", h.label);
                    } else {
                        self.status_message = format!(
                            "✓ Hydration complete: {} succeeded, {} failed",
                            self.hydration_batch_succeeded, self.hydration_batch_failed
                        );
                    }
                } else if self.hydration_batch_failed == 1 && self.hydration_batch_succeeded == 0 {
                    let (last_label, reason) = self
                        .recent_hydration_outcomes
                        .back()
                        .map(|h| (h.label.as_str(), h.detail.as_str()))
                        .unwrap_or(("source", "unknown error"));
                    self.status_message = format!("⚠ Metadata fetch failed for '{last_label}': {reason}");
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
                        self.status_message = "⚠ Draft updated — press 'm' / 'X' to recompute similarity".to_string();
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
                    score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
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
                    let score = self.draft_ref_similarities.get(&r.id).copied().unwrap_or(0.0);
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
                score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
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

        let embedder = sil_db::OnnxEmbedder::default();
        match db.recompute_draft_ref_similarities(&draft_text, &embedder) {
            Ok(count) => {
                if let Ok(sims) = db.get_draft_ref_similarities() {
                    self.draft_ref_similarities = sims;
                }
                if let Ok(hash) = db.get_draft_similarity_hash() {
                    self.draft_similarity_hash = hash;
                }
                self.sort_source_references();
                self.status_message =
                    format!("✓ Recomputed draft similarity scores for {count} reference(s)");
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
                    self.status_message =
                        format!("✓ Added ALL {count} ref(s) (⚠ No DOI/arXiv/title — cannot hydrate)");
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
            self.status_message = format!("✓ Promoted '{cite_key}' (removed % [sil: tui-added] marker)");
        } else {
            self.status_message = format!("✓ Promoted '{cite_key}' (no project root loaded to save)");
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
            InputMode::HelpOverlay => self.handle_help_overlay_mode(key),
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
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if self.active_tab == ActiveTab::Sources {
                    if !self.sources.is_empty()
                        && self.selected_source_index < self.sources.len()
                    {
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
                        "Register link stub (no download) — Enter URL / DOI / arXiv / filename (Enter to submit, Esc to cancel)"
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
                            self.load_project_references_bib();
                            let count = entries_to_add.len();
                            self.marked_ref_ids.clear();
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
                    self.recompute_draft_ref_similarities();
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
                    let kind = classify_source_input(&link);
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
                    self.status_message =
                        format!("✓ Registered {} link stub (no download): {link}", kind.label());
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
                arxiv_id: None,
                url: None,
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
                arxiv_id: None,
                url: None,
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
        assert_eq!(RagField::ALL.len(), 9);
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
                    RagField::XbergCacheDir => "/path/to/xberg_cache".to_string(),
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
                arxiv_id: None,
                url: None,
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
                arxiv_id: None,
                url: None,
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
                arxiv_id: None,
                url: None,
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
                arxiv_id: None,
                url: None,
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
                year: Some(2010 + idx as i32),
                venue: Some("Conf".to_string()),
                doi: None,
                arxiv_id: None,
                url: None,
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
                arxiv_id: None,
                url: None,
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
                arxiv_id: None,
                url: None,
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
            app.filtered_viewing_source_references()[0]
                .authors
                .as_deref(),
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
            arxiv_id: None,
            url: None,
        }];
        app.selected_viewing_ref_index = 0;

        // Append selected ref to bib via 'c'
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()));
        let bib_content =
            std::fs::read_to_string(project_path.join("references.bib").as_std_path()).unwrap();
        assert!(
            bib_content.to_lowercase().contains("attention") || bib_content.contains("Vaswani")
        );
        assert_eq!(app.bib_file_entries.len(), 1);

        // Delete bib entry via delete_selected_bib_entry
        app.active_tab = ActiveTab::References;
        app.active_ref_pane = RefPane::LeftBib;
        app.selected_bib_index = 0;
        app.delete_selected_bib_entry();

        let updated_bib =
            std::fs::read_to_string(project_path.join("references.bib").as_std_path()).unwrap();
        assert!(updated_bib.is_empty());
        assert_eq!(app.bib_file_entries.len(), 0);
    }

    #[test]
    fn test_load_bib_entries_with_comments_and_indentation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_path = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();
        let bib_text = r#"
# Top level comment
@article{entry1,
  title={Paper 1}
}

  @inproceedings{entry2,
  title={Paper 2}
}
"#;
        std::fs::write(project_path.join("references.bib").as_std_path(), bib_text).unwrap();

        let mut app = App::new(Some(project_path));
        app.load_project_references_bib();
        assert_eq!(app.bib_file_entries.len(), 2);
        assert!(app.bib_file_entries[0].contains("entry1"));
        assert!(app.bib_file_entries[1].contains("entry2"));
    }

    #[test]
    fn test_sources_tab_append_selected_to_bib() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_path = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();
        std::fs::write(project_path.join("references.bib").as_std_path(), "").unwrap();

        let mut app = App::new(Some(project_path.clone()));
        app.active_tab = ActiveTab::Sources;
        let mut doc = SourceDocument::new("test_paper.pdf".into());
        doc.title = Some("Deep Learning Advances".into());
        doc.authors = Some("Alice Smith".into());
        app.sources = vec![doc];
        app.selected_source_index = 0;

        // Press 'b' to append selected source to references.bib
        app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty()));

        // Since test_paper.pdf has no DOI or Crossref hit, it warns
        assert!(app.status_message.contains("⚠") || app.status_message.contains("✓"));
    }

    #[test]
    fn test_references_similarity_sorting_and_filtering() {
        let mut app = App::new(None);
        let ref1 = ReferenceEntry {
            id: "ref_1".to_string(),
            source_id: "src".into(),
            ref_index: 1,
            raw_text: "Low similarity ref".to_string(),
            title: Some("Low similarity ref".to_string()),
            authors: None,
            year: Some(2020),
            venue: None,
            doi: None,
            arxiv_id: None,
            url: None,
        };
        let ref2 = ReferenceEntry {
            id: "ref_2".to_string(),
            source_id: "src".into(),
            ref_index: 2,
            raw_text: "High similarity ref".to_string(),
            title: Some("High similarity ref".to_string()),
            authors: None,
            year: Some(2021),
            venue: None,
            doi: None,
            arxiv_id: None,
            url: None,
        };

        app.source_references = vec![ref1, ref2];
        app.draft_ref_similarities.insert("ref_1".to_string(), 0.25);
        app.draft_ref_similarities.insert("ref_2".to_string(), 0.95);

        // Sort by similarity via 'm'
        app.active_tab = ActiveTab::References;
        app.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::empty()));
        assert_eq!(app.ref_sort_key, RefSortKey::Similarity);

        let filtered = app.filtered_source_references();
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].id, "ref_2"); // highest score first
        assert_eq!(filtered[1].id, "ref_1");

        // Filter by min similarity score threshold = 0.5
        app.min_similarity_filter = Some(0.5);
        let filtered_threshold = app.filtered_source_references();
        assert_eq!(filtered_threshold.len(), 1);
        assert_eq!(filtered_threshold[0].id, "ref_2");
    }

    #[test]
    fn test_tui_added_bib_entry_marking_and_promote() {
        use camino::Utf8Path;
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let bib_path = root.join("references.bib");

        let mut app = App::new(Some(root.to_path_buf()));
        let ref_entry = ReferenceEntry {
            id: "ref_test".to_string(),
            source_id: "src_test".into(),
            ref_index: 1,
            raw_text: "Sample Raw Reference Text".to_string(),
            title: Some("Sample Reference Title".to_string()),
            authors: Some("Author A".to_string()),
            year: Some(2024),
            venue: None,
            doi: None,
            arxiv_id: None,
            url: None,
        };
        app.source_references = vec![ref_entry];
        app.active_tab = ActiveTab::References;
        app.active_ref_pane = RefPane::RightSources;
        app.selected_source_ref_index = 0;

        // Paste/add reference to references.bib using 'p'
        app.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::empty()));

        let bib_content = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
        assert!(bib_content.contains("% [sil: tui-added]"));
        assert!(bib_content.contains("@"));

        // Switch to LeftBib pane and promote using 'P'
        app.active_ref_pane = RefPane::LeftBib;
        app.selected_bib_index = 0;
        app.handle_key(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT));

        let promoted_content = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
        assert!(!promoted_content.contains("tui-added"));
        assert!(promoted_content.contains("@"));
    }

    #[test]
    fn test_background_hydration_success_upserts_and_preserves_tui_added() {
        use camino::Utf8Path;
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let bib_path = root.join("references.bib");
        std::fs::write(bib_path.as_std_path(), "% [sil: tui-added]\n@article{stub, title={Stub}}\n").unwrap();

        let mut app = App::new(Some(root.to_path_buf()));
        app.in_flight_hydration_keys.insert("doi:10.1000/182".to_string());

        let official_bib = "@article{stub,\n  title={Official Title},\n  doi={10.1000/182}\n}";
        app.hydration_tx
            .send(HydrationResult {
                dedup_key: "doi:10.1000/182".to_string(),
                label: "Official Title".to_string(),
                outcome: HydrationOutcome::Success {
                    official_bib: official_bib.to_string(),
                },
            })
            .unwrap();

        app.poll_background_hydration();

        assert!(!app.in_flight_hydration_keys.contains("doi:10.1000/182"));
        let updated_content = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
        assert!(updated_content.contains("% [sil: tui-added]"));
        assert!(updated_content.contains("Official Title"));
        assert!(app.status_message.contains("✓ Official metadata for 'Official Title'"));
    }

    #[test]
    fn test_background_hydration_preserves_stub_cite_key() {
        use camino::Utf8Path;
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let bib_path = root.join("references.bib");
        std::fs::write(
            bib_path.as_std_path(),
            "% [sil: tui-added]
@article{stub_key, title={Attention Is All You Need}, doi={10.1000/182}}
",
        )
        .unwrap();

        let mut app = App::new(Some(root.to_path_buf()));
        app.in_flight_hydration_keys
            .insert("doi:10.1000/182".to_string());

        let official_bib = "@article{Vaswani2017,
  title={Attention Is All You Need},
  author={Vaswani, Ashish},
  doi={10.1000/182}
}";
        app.hydration_tx
            .send(HydrationResult {
                dedup_key: "doi:10.1000/182".to_string(),
                label: "Attention Is All You Need".to_string(),
                outcome: HydrationOutcome::Success {
                    official_bib: official_bib.to_string(),
                },
            })
            .unwrap();

        app.poll_background_hydration();

        assert!(!app.in_flight_hydration_keys.contains("doi:10.1000/182"));
        let updated_content = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
        assert!(updated_content.contains("@article{stub_key,"));
        assert!(updated_content.contains("author = {Vaswani, Ashish}"));
        assert!(!updated_content.contains("Vaswani2017"));
    }

    #[test]
    fn test_background_hydration_failure_warns_and_retains_local() {
        use camino::Utf8Path;
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let bib_path = root.join("references.bib");
        let initial_bib = "% [sil: tui-added]\n@article{stub, title={Stub Title}}\n";
        std::fs::write(bib_path.as_std_path(), initial_bib).unwrap();

        let mut app = App::new(Some(root.to_path_buf()));
        app.in_flight_hydration_keys.insert("doi:10.1000/invalid".to_string());

        app.hydration_tx
            .send(HydrationResult {
                dedup_key: "doi:10.1000/invalid".to_string(),
                label: "Stub Title".to_string(),
                outcome: HydrationOutcome::Failure {
                    reason: "HTTP 404 Not Found".to_string(),
                },
            })
            .unwrap();

        app.poll_background_hydration();

        assert!(!app.in_flight_hydration_keys.contains("doi:10.1000/invalid"));
        let content_after = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
        assert_eq!(content_after, initial_bib);
        assert!(app.status_message.contains("⚠ Metadata fetch failed for 'Stub Title': HTTP 404 Not Found"));
    }

    #[test]
    fn test_hydration_deduplication() {
        let mut app = App::new(None);
        let entry = ReferenceEntry {
            id: "ref_dedup".to_string(),
            source_id: "src_1".into(),
            ref_index: 1,
            raw_text: "Ref text".to_string(),
            title: Some("Dedup Title".to_string()),
            authors: None,
            year: None,
            venue: None,
            doi: Some("10.1000/dedup".to_string()),
            arxiv_id: None,
            url: None,
        };

        app.queue_ref_hydration(entry.clone());
        assert!(app.in_flight_hydration_keys.contains("doi:10.1000/dedup"));

        // Attempting second queue for same key should be a no-op
        app.queue_ref_hydration(entry);
        assert_eq!(app.in_flight_hydration_keys.len(), 1);
    }

    #[test]
    fn test_no_fetch_when_no_identifiers() {
        use camino::Utf8Path;
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let bib_path = root.join("references.bib");
        std::fs::write(bib_path.as_std_path(), "").unwrap();

        let mut app = App::new(Some(root.to_path_buf()));
        let empty_entry = ReferenceEntry {
            id: "ref_empty".to_string(),
            source_id: "src_1".into(),
            ref_index: 1,
            raw_text: "Unparseable citation".to_string(),
            title: None,
            authors: None,
            year: None,
            venue: None,
            doi: None,
            arxiv_id: None,
            url: None,
        };

        app.selected_source_references = vec![empty_entry];
        app.selected_viewing_ref_index = 0;
        app.append_selected_viewing_ref_to_bib();

        assert!(app.in_flight_hydration_keys.is_empty());
        assert!(app.status_message.contains("⚠ No DOI/arXiv/title — cannot hydrate"));
        let content = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
        assert!(content.contains("% [sil: tui-added]"));
    }
    #[test]
    fn test_keymap_for_all_modes() {
        let modes = [
            HelpMode::Dashboard,
            HelpMode::SourcesList,
            HelpMode::ReadingSourceMd,
            HelpMode::ViewingSourceRefs,
            HelpMode::ReferencesLeft,
            HelpMode::ReferencesRight,
            HelpMode::PaperDraft,
            HelpMode::Settings,
            HelpMode::ModalPicker,
            HelpMode::ModalAddAuthor,
            HelpMode::ModalAddGrant,
            HelpMode::ModalAddSourceLink,
            HelpMode::ModalRenameSource,
            HelpMode::ConfirmDeleteSource,
            HelpMode::Editing,
            HelpMode::EditingPaper,
            HelpMode::SearchingRefs,
            HelpMode::SearchingBib,
            HelpMode::SearchingViewingRefs,
        ];

        for mode in modes {
            let keymap = keymap_for(mode);
            assert!(!keymap.is_empty(), "Keymap for {:?} should not be empty", mode);
            assert!(!mode.title().is_empty());
            for (key, action) in keymap {
                assert!(!key.is_empty());
                assert!(!action.is_empty());
            }
        }
    }

    #[test]
    fn test_toggle_help_overlay_and_current_help_mode() {
        let mut app = App::new(None);
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.current_help_mode(), HelpMode::Dashboard);

        // Toggle on ?
        app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::HelpOverlay);
        assert_eq!(app.current_help_mode(), HelpMode::Dashboard);

        // Toggle off on any key
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::Normal);

        // Test F1 toggle in Sources view
        app.active_tab = ActiveTab::Sources;
        assert_eq!(app.current_help_mode(), HelpMode::SourcesList);
        app.handle_key(KeyEvent::new(KeyCode::F(1), KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::HelpOverlay);
        assert_eq!(app.current_help_mode(), HelpMode::SourcesList);

        app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::empty()));
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_references_title_sort_binding() {
        let mut app = App::new(None);
        app.active_tab = ActiveTab::References;
        app.active_ref_pane = RefPane::RightSources;
        app.source_references = vec![
            ReferenceEntry {
                id: "r1".to_string(),
                source_id: "s1".into(),
                ref_index: 1,
                raw_text: "Raw Z".to_string(),
                title: Some("Zebra Paper".to_string()),
                authors: None,
                year: None,
                venue: None,
                doi: None,
                arxiv_id: None,
                url: None,
            },
            ReferenceEntry {
                id: "r2".to_string(),
                source_id: "s1".into(),
                ref_index: 2,
                raw_text: "Raw A".to_string(),
                title: Some("Alpha Paper".to_string()),
                authors: None,
                year: None,
                venue: None,
                doi: None,
                arxiv_id: None,
                url: None,
            },
        ];

        app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
        assert_eq!(app.ref_sort_key, RefSortKey::Title);
        assert_eq!(app.source_references[0].title.as_deref(), Some("Alpha Paper"));
        assert_eq!(app.source_references[1].title.as_deref(), Some("Zebra Paper"));
    }

    #[test]
    fn test_hydration_promote_during_flight() {
        use camino::Utf8Path;
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let bib_path = root.join("references.bib");
        std::fs::write(
            bib_path.as_std_path(),
            "% [sil: tui-added]\n@article{stub_key, title={Paper Title}, doi={10.1000/race}}\n",
        )
        .unwrap();

        let mut app = App::new(Some(root.to_path_buf()));
        app.in_flight_hydration_keys.insert("doi:10.1000/race".to_string());

        app.active_tab = ActiveTab::References;
        app.active_ref_pane = RefPane::LeftBib;
        app.selected_bib_index = 0;
        app.promote_selected_bib_entry();

        let promoted_before = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
        assert!(!promoted_before.contains("tui-added"));

        let official_bib = "@article{OfficialKey,\n  title={Paper Title},\n  author={Smith, John},\n  doi={10.1000/race}\n}";
        app.hydration_tx
            .send(HydrationResult {
                dedup_key: "doi:10.1000/race".to_string(),
                label: "Paper Title".to_string(),
                outcome: HydrationOutcome::Success {
                    official_bib: official_bib.to_string(),
                },
            })
            .unwrap();

        app.poll_background_hydration();

        let updated = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
        assert!(updated.contains("@article{stub_key,"));
        assert!(updated.contains("author = {Smith, John}"));
        assert!(!updated.contains("tui-added"));
    }

    #[test]
    fn test_hydration_deleted_during_flight() {
        use camino::Utf8Path;
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let bib_path = root.join("references.bib");
        std::fs::write(
            bib_path.as_std_path(),
            "% [sil: tui-added]\n@article{stub_key, title={Paper Title}, doi={10.1000/deleted}}\n",
        )
        .unwrap();

        let mut app = App::new(Some(root.to_path_buf()));
        app.in_flight_hydration_keys.insert("doi:10.1000/deleted".to_string());

        std::fs::write(bib_path.as_std_path(), "").unwrap();

        let official_bib = "@article{OfficialKey, title={Paper Title}, doi={10.1000/deleted}}";
        app.hydration_tx
            .send(HydrationResult {
                dedup_key: "doi:10.1000/deleted".to_string(),
                label: "Paper Title".to_string(),
                outcome: HydrationOutcome::Success {
                    official_bib: official_bib.to_string(),
                },
            })
            .unwrap();

        app.poll_background_hydration();

        let content_after = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
        assert!(content_after.is_empty());
        assert_eq!(app.status_message, "✓ Hydration complete: 1 succeeded, 0 failed");
        assert!(app.recent_hydration_outcomes.back().unwrap().detail.contains("Skipped hydration for 'Paper Title': entry was deleted"));
    }

    #[test]
    fn test_arxiv_only_source_dedup_key() {
        let mut app = App::new(None);
        let doc = SourceDocument {
            id: "src_arxiv".into(),
            path: "2103.12345.pdf".into(),
            filename: "2103.12345.pdf".to_string(),
            kind: sil_core::SourceKind::Pdf,
            parsed: true,
            status: None,
            title: Some("Attention Is All You Need".to_string()),
            authors: None,
            abstract_text: None,
            doi: None,
            year: None,
            venue: None,
            references_text: None,
        };

        app.queue_source_hydration(doc);
        assert!(app.in_flight_hydration_keys.contains("arxiv:2103.12345"));
    }

    #[test]
    #[allow(clippy::permissions_set_readonly_false)]
    fn test_hydration_write_failure_status_message() {
        use camino::Utf8Path;
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let bib_path = root.join("references.bib");
        std::fs::write(
            bib_path.as_std_path(),
            "% [sil: tui-added]\n@article{stub, title={Stub}, doi={10.1000/writeerr}}\n",
        )
        .unwrap();

        let mut app = App::new(Some(root.to_path_buf()));
        app.in_flight_hydration_keys.insert("doi:10.1000/writeerr".to_string());

        let mut perms = std::fs::metadata(bib_path.as_std_path()).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(bib_path.as_std_path(), perms.clone()).unwrap();

        let official_bib = "@article{stub, title={Stub}, author={Tester}, doi={10.1000/writeerr}}";
        app.hydration_tx
            .send(HydrationResult {
                dedup_key: "doi:10.1000/writeerr".to_string(),
                label: "Stub".to_string(),
                outcome: HydrationOutcome::Success {
                    official_bib: official_bib.to_string(),
                },
            })
            .unwrap();

        app.poll_background_hydration();

        perms.set_readonly(false);
        let _ = std::fs::set_permissions(bib_path.as_std_path(), perms);

        assert_eq!(app.status_message, "✓ Hydration complete: 1 succeeded, 0 failed");
        assert!(app.recent_hydration_outcomes.back().unwrap().detail.contains("Error writing references.bib:"));
    }

    #[test]
    fn test_poll_multiple_results_in_one_tick_and_batch_drain() {
        let mut app = App::new(None);
        app.in_flight_hydration_keys.insert("doi:10.1000/a".to_string());
        app.in_flight_hydration_keys.insert("doi:10.1000/b".to_string());
        app.in_flight_hydration_keys.insert("doi:10.1000/c".to_string());

        app.hydration_tx
            .send(HydrationResult {
                dedup_key: "doi:10.1000/a".to_string(),
                label: "Paper A".to_string(),
                outcome: HydrationOutcome::Success {
                    official_bib: "@article{a, title={Paper A}}".to_string(),
                },
            })
            .unwrap();

        app.hydration_tx
            .send(HydrationResult {
                dedup_key: "doi:10.1000/b".to_string(),
                label: "Paper B".to_string(),
                outcome: HydrationOutcome::Failure {
                    reason: "HTTP 404".to_string(),
                },
            })
            .unwrap();

        app.hydration_tx
            .send(HydrationResult {
                dedup_key: "doi:10.1000/c".to_string(),
                label: "Paper C".to_string(),
                outcome: HydrationOutcome::Success {
                    official_bib: "@article{c, title={Paper C}}".to_string(),
                },
            })
            .unwrap();

        app.poll_background_hydration();

        assert!(app.in_flight_hydration_keys.is_empty());
        assert_eq!(app.hydration_batch_succeeded, 2);
        assert_eq!(app.hydration_batch_failed, 1);
        assert_eq!(app.recent_hydration_outcomes.len(), 3);
        assert_eq!(
            app.status_message,
            "✓ Hydration complete: 2 succeeded, 1 failed"
        );
    }

    #[test]
    fn test_already_hydrating_dedup_and_status() {
        let mut app = App::new(None);
        let entry = sil_core::ReferenceEntry {
            id: "ref_1".to_string(),
            source_id: "src_1".into(),
            ref_index: 1,
            raw_text: "Test Reference".to_string(),
            title: Some("Duplicate Test Paper".to_string()),
            authors: None,
            year: None,
            venue: None,
            doi: Some("10.1000/dup".to_string()),
            arxiv_id: None,
            url: None,
        };

        app.queue_ref_hydration(entry.clone());
        assert!(app.in_flight_hydration_keys.contains("doi:10.1000/dup"));
        assert_eq!(app.status_message, "⏳ Hydrating (1 in flight)...");

        // Request again while in flight
        app.queue_ref_hydration(entry);
        assert_eq!(app.status_message, "already hydrating 'Duplicate Test Paper'...");
        assert_eq!(app.in_flight_hydration_keys.len(), 1);
    }

    #[test]
    fn test_recent_hydration_outcomes_bounded_to_20() {
        let mut app = App::new(None);
        for i in 0..25 {
            app.in_flight_hydration_keys.insert(format!("doi:10.1000/{i}"));
            app.hydration_tx
                .send(HydrationResult {
                    dedup_key: format!("doi:10.1000/{i}"),
                    label: format!("Paper {i}"),
                    outcome: HydrationOutcome::Success {
                        official_bib: format!("@article{{p{i}, title={{Paper {i}}}}}"),
                    },
                })
                .unwrap();
        }

        app.poll_background_hydration();

        assert_eq!(app.recent_hydration_outcomes.len(), 20);
        assert_eq!(app.recent_hydration_outcomes.front().unwrap().label, "Paper 5");
        assert_eq!(app.recent_hydration_outcomes.back().unwrap().label, "Paper 24");
    }

    #[test]
    fn test_classify_source_input() {
        assert_eq!(
            classify_source_input("10.1038/s41586-020-2649-2"),
            SourceInputKind::Doi
        );
        assert_eq!(
            classify_source_input("doi:10.1145/1234567"),
            SourceInputKind::Doi
        );
        assert_eq!(
            classify_source_input("https://doi.org/10.1145/1234567"),
            SourceInputKind::Doi
        );

        assert_eq!(
            classify_source_input("2103.12345"),
            SourceInputKind::Arxiv
        );
        assert_eq!(
            classify_source_input("arXiv:2103.12345v1"),
            SourceInputKind::Arxiv
        );
        assert_eq!(
            classify_source_input("https://arxiv.org/abs/2103.12345"),
            SourceInputKind::Arxiv
        );

        assert_eq!(
            classify_source_input("https://example.com/paper.pdf"),
            SourceInputKind::Url
        );
        assert_eq!(
            classify_source_input("http://site.org/resource"),
            SourceInputKind::Url
        );

        assert_eq!(
            classify_source_input("paper_notes.md"),
            SourceInputKind::Filename
        );
        assert_eq!(
            classify_source_input(""),
            SourceInputKind::Filename
        );
    }

    #[test]
    fn test_sources_reload_action_key_r() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let mut app = App::new(Some(root));

        app.active_tab = ActiveTab::Sources;
        app.status_message = "Initial status".to_string();

        // Press 'R' key in Sources tab
        app.handle_key(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::empty()));
        assert_eq!(app.status_message, "✓ Reloaded sources");

        // Press Shift+'r' in Sources tab
        app.status_message = "Initial status".to_string();
        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::SHIFT));
        assert_eq!(app.status_message, "✓ Reloaded sources");
    }

    #[test]
    fn test_sources_parse_keymap() {
        let keymap = keymap_for(HelpMode::SourcesList);
        let parse_entry = keymap.iter().find(|(key, _)| *key == "e / E");
        assert!(parse_entry.is_some(), "Keymap for SourcesList missing 'e / E'");
    }

    #[test]
    fn test_sources_parse_already_parsed_status() {
        let mut app = App::new(None);
        app.active_tab = ActiveTab::Sources;
        let mut doc = SourceDocument::new(camino::Utf8PathBuf::from("test.txt"));
        doc.parsed = true;
        app.sources.push(doc);

        app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()));
        assert_eq!(
            app.status_message,
            "ℹ Source is already parsed (use 'E' / Shift+E to re-parse)"
        );
        assert!(app.in_flight_parse_ids.is_empty());
    }

    #[test]
    fn test_sources_parse_queueing_normal_and_force() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();
        let sources_dir = root.join("sources");
        std::fs::create_dir_all(sources_dir.as_std_path()).unwrap();
        let file_path = sources_dir.join("sample.txt");
        std::fs::write(
            file_path.as_std_path(),
            "Title: Sample Paper\nAbstract: Test abstract\n\nReferences:\n[1] A. Author, Sample Reference, 2024.",
        )
        .unwrap();

        let mut app = App::new(Some(root.clone()));
        app.active_tab = ActiveTab::Sources;
        app.reload_sources();
        assert_eq!(app.sources.len(), 1);
        assert!(!app.sources[0].parsed);

        // Queue normal parse with 'e'
        app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()));
        assert!(app.in_flight_parse_ids.contains(&app.sources[0].id));
        assert!(app.status_message.starts_with("⏳ Parsing source"));

        // Wait for background parse thread to complete
        for _ in 0..50 {
            app.poll_background_hydration();
            if app.in_flight_parse_ids.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        assert!(app.in_flight_parse_ids.is_empty());
        assert!(app.status_message.starts_with("✓ Parsed source"));
        assert!(app.sources[0].parsed);

        // Pressing 'e' now should inform user that it's already parsed
        app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()));
        assert_eq!(
            app.status_message,
            "ℹ Source is already parsed (use 'E' / Shift+E to re-parse)"
        );

        // Pressing Shift+E ('E') should force re-parse
        app.handle_key(KeyEvent::new(KeyCode::Char('E'), KeyModifiers::SHIFT));
        assert!(app.in_flight_parse_ids.contains(&app.sources[0].id));
        assert!(app.status_message.starts_with("⏳ Parsing source"));

        for _ in 0..50 {
            app.poll_background_hydration();
            if app.in_flight_parse_ids.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        assert!(app.in_flight_parse_ids.is_empty());
        assert!(app.status_message.starts_with("✓ Parsed source"));
    }

    #[test]
    fn test_sources_parse_failure_status() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();

        let mut app = App::new(Some(root));
        app.active_tab = ActiveTab::Sources;
        let doc = SourceDocument::new(Utf8PathBuf::from("/nonexistent/file.pdf"));
        app.sources.push(doc);

        // Queue force parse on non-existent file
        app.handle_key(KeyEvent::new(KeyCode::Char('E'), KeyModifiers::SHIFT));
        assert!(!app.in_flight_parse_ids.is_empty());

        for _ in 0..50 {
            app.poll_background_hydration();
            if app.in_flight_parse_ids.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        assert!(app.in_flight_parse_ids.is_empty());
        assert!(app.status_message.starts_with("⚠ Failed parsing source"));
    }
}
