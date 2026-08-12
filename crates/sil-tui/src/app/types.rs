//! Data types, enums, and classification functions for `sil-tui` app state.

use sil_core::{ReferenceEntry, SourceDocument};

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
    JobHistory,
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
    JobHistory,
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
            HelpMode::JobHistory => "Background Job History",
        }
    }
}

/// Pure function returning the (key, action) mapping for a given help context mode.
pub fn keymap_for(mode: HelpMode) -> Vec<(&'static str, &'static str)> {
    match mode {
        HelpMode::Dashboard => vec![
            (
                "1 - 5",
                "Switch directly to tab (Dashboard, Sources, References, Draft, Settings)",
            ),
            ("Tab / Shift+Tab", "Cycle forward / backward through tabs"),
            (
                "J",
                "Open background job history (hydrate / fetch / parse / similarity) + retry",
            ),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("q / Esc", "Quit application"),
            ("Ctrl+S / s", "Save all settings and project state"),
        ],
        HelpMode::SourcesList => vec![
            ("j / Down", "Select next source document"),
            ("k / Up", "Select previous source document"),
            ("PageUp / PageDown", "Scroll source list by 5 items"),
            ("Enter", "Read full source document in Markdown viewer"),
            (
                "e / E",
                "Parse/extract text and references for selected source ('E' / Shift+E for force re-parse)",
            ),
            (
                "v",
                "View extracted reference citations for selected source",
            ),
            (
                "a",
                "Fetch/download source via URL / DOI / arXiv (background job)",
            ),
            (
                "b",
                "Append selected source to references.bib (hydrates metadata if DOI/arXiv)",
            ),
            ("r", "Rename selected source document title"),
            ("R", "Reload sources from disk and database"),
            (
                "d / Delete",
                "Delete selected source document (requires confirmation)",
            ),
            ("J", "Open background job history + retry failed jobs"),
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
            (
                "c / b / p",
                "Append marked (or highlighted) reference to references.bib",
            ),
            ("a", "Append ALL filtered references to references.bib"),
            ("d / e", "Toggle reference inspector card & BibTeX preview"),
            ("/ / f", "Search / filter references by text query"),
            ("y", "Sort references by Year (descending)"),
            ("v", "Sort references by Venue / Journal"),
            ("s", "Sort references by Source document ID"),
            ("i / n", "Sort references by original Index"),
            ("t", "Sort references by Title"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            (
                "q / Esc",
                "Close references viewer (or clear search filter)",
            ),
        ],
        HelpMode::ReferencesLeft => vec![
            ("j / Down", "Select next entry in references.bib"),
            ("k / Up", "Select previous entry in references.bib"),
            ("PageUp / PageDown", "Jump 5 entries up / down"),
            ("Tab", "Switch focus to Right Pane (Extracted References)"),
            ("/ / f", "Search references.bib entries"),
            (
                "P",
                "Promote TUI-added entry (strip % [sil: tui-added] marker)",
            ),
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
            (
                "p",
                "Add marked (or highlighted) reference to references.bib",
            ),
            ("P", "Promote highlighted entry in references.bib"),
            ("/ / f", "Search extracted references"),
            (
                "m / c",
                "Sort references by existing Draft Cosine Similarity scores (no recompute)",
            ),
            (
                "X",
                "Enqueue background recompute of draft–ref similarity (settings embedder / hash fallback)",
            ),
            ("y", "Sort references by Year (descending)"),
            ("v", "Sort references by Venue / Journal"),
            ("s", "Sort references by Source document"),
            ("i", "Sort references by Index"),
            ("t", "Sort references by Title"),
            ("J", "Open background job history + retry failed jobs"),
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
            (
                "v",
                "Launch external $EDITOR (nvim / helix / vim) on paper_draft.tex",
            ),
            ("J", "Open background job history + retry failed jobs"),
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
            (
                "d / Delete",
                "Remove highlighted cached or local co-author / grant",
            ),
            (
                "u",
                "Import selected cached author/grant into local project settings",
            ),
            ("J", "Open background job history + retry failed jobs"),
            ("1 - 5", "Switch to tab"),
            ("Tab / Shift+Tab", "Cycle tabs"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("q / Esc", "Quit application"),
            (
                "Ctrl+S / s",
                "Save global settings, local config, and cache",
            ),
        ],
        HelpMode::ModalPicker => vec![
            ("j / Down", "Navigate down cache list"),
            ("k / Up", "Navigate up cache list"),
            (
                "Enter",
                "Select highlighted item into local project settings",
            ),
            ("n", "Create new item manually"),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("Esc", "Close picker modal"),
        ],
        HelpMode::ModalAddAuthor => vec![
            (
                "Tab / Down",
                "Focus next field (Name, Email, Affiliation, ORCID)",
            ),
            ("Shift+Tab / Up", "Focus previous field"),
            ("Backspace", "Delete character in active field"),
            ("Char", "Type text into active field"),
            (
                "Enter",
                "Save co-author to cache and local project settings",
            ),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("Esc", "Cancel and close modal"),
        ],
        HelpMode::ModalAddGrant => vec![
            (
                "Tab / Down",
                "Focus next field (Funder, Number, Acknowledgment)",
            ),
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
            ("Enter", "Start background fetch/download of the source"),
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
        HelpMode::JobHistory => vec![
            ("j / Down", "Select next job outcome"),
            ("k / Up", "Select previous job outcome"),
            (
                "Enter / r",
                "Retry selected failed job (when retry payload available)",
            ),
            ("? / F1", "Toggle mode-aware keyboard help overlay"),
            ("Esc / q / J", "Close job history modal"),
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

pub(crate) fn resolve_onnx_from_dir(val: &str) -> String {
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

/// Cap for the unified background job history ring buffer.
pub const JOB_HISTORY_CAP: usize = 20;

/// Kind of background job recorded in job history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobKind {
    Hydrate,
    Fetch,
    Parse,
    Similarity,
    Estimate,
}

impl JobKind {
    pub fn label(self) -> &'static str {
        match self {
            JobKind::Hydrate => "hydrate",
            JobKind::Fetch => "fetch",
            JobKind::Parse => "parse",
            JobKind::Similarity => "similarity",
            JobKind::Estimate => "estimate",
        }
    }
}

/// Payload needed to re-enqueue a failed background job.
#[derive(Debug, Clone)]
pub enum RetryPayload {
    HydrateRef { entry: ReferenceEntry },
    HydrateSource { doc: SourceDocument },
    Fetch { target: String },
    Parse { doc: SourceDocument, force: bool },
    Similarity,
}

/// Unified recent outcome for hydrate / fetch / parse / similarity jobs.
#[derive(Debug, Clone)]
pub struct JobOutcome {
    pub id: u64,
    pub kind: JobKind,
    pub label: String,
    pub ok: bool,
    pub detail: String,
    pub duration_ms: Option<u64>,
    pub retry_payload: Option<RetryPayload>,
}

/// Result message from background metadata hydration thread.
#[derive(Debug, Clone)]
pub struct HydrationResult {
    pub dedup_key: String,
    pub label: String,
    pub outcome: HydrationOutcome,
    pub duration_ms: Option<u64>,
}

/// Outcome of background metadata fetch.
#[derive(Debug, Clone)]
pub enum HydrationOutcome {
    Success { official_bib: String },
    Failure { reason: String },
}

/// Result of a background parse job.
#[derive(Debug)]
pub struct ParseJobResult {
    pub source_id: sil_core::SourceId,
    pub label: String,
    pub result: Result<sil_parse::batch::ParseResult, String>,
    pub duration_ms: Option<u64>,
    pub force: bool,
}

/// Result of a background source fetch job.
#[derive(Debug)]
pub struct FetchJobResult {
    pub target: String,
    pub label: String,
    pub result: Result<camino::Utf8PathBuf, String>,
    pub duration_ms: Option<u64>,
}

/// Result of a background draft–ref similarity recompute job.
#[derive(Debug)]
pub struct SimilarityJobResult {
    pub draft_hash: String,
    pub backend_summary: String,
    pub result: Result<usize, String>,
    pub duration_ms: Option<u64>,
}

/// Result of a background manuscript L0 estimate job.
#[derive(Debug)]
pub struct EstimateJobResult {
    pub result: Result<sil_agent::EstimateReport, String>,
    pub duration_ms: Option<u64>,
}
