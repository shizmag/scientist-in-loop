//! Application state and logic for `sil-tui`.

pub(crate) mod bib_actions;
pub(crate) mod handlers;
pub(crate) mod jobs;
pub(crate) mod types;

#[cfg(test)]
mod tests;

pub use types::*;

use camino::Utf8PathBuf;
use sil_core::{
    AuthorDetails, Config, GlobalSettings, GrantDetails, LocalSettings, ProjectPaths,
    ReferenceEntry, SettingsCache, SourceDocument,
};

/// Application state struct for TUI.
pub struct App {
    pub active_tab: ActiveTab,
    pub input_mode: InputMode,
    pub saved_input_mode: InputMode,

    pub hydration_tx: std::sync::mpsc::Sender<HydrationResult>,
    pub hydration_rx: std::sync::mpsc::Receiver<HydrationResult>,
    pub in_flight_hydration_keys: std::collections::HashSet<String>,
    pub hydrate_retry_payloads: std::collections::HashMap<String, RetryPayload>,
    pub parse_tx: std::sync::mpsc::Sender<ParseJobResult>,
    pub parse_rx: std::sync::mpsc::Receiver<ParseJobResult>,
    pub in_flight_parse_ids: std::collections::HashSet<sil_core::SourceId>,
    pub parse_retry_payloads: std::collections::HashMap<sil_core::SourceId, RetryPayload>,
    pub fetch_tx: std::sync::mpsc::Sender<FetchJobResult>,
    pub fetch_rx: std::sync::mpsc::Receiver<FetchJobResult>,
    pub in_flight_fetch_targets: std::collections::HashSet<String>,
    pub similarity_tx: std::sync::mpsc::Sender<SimilarityJobResult>,
    pub similarity_rx: std::sync::mpsc::Receiver<SimilarityJobResult>,
    pub in_flight_similarity: bool,
    pub estimate_tx: std::sync::mpsc::Sender<EstimateJobResult>,
    pub estimate_rx: std::sync::mpsc::Receiver<EstimateJobResult>,
    pub in_flight_estimate: bool,
    pub digest_tx: std::sync::mpsc::Sender<DigestJobResult>,
    pub digest_rx: std::sync::mpsc::Receiver<DigestJobResult>,
    pub in_flight_digest: bool,
    pub hydration_batch_succeeded: usize,
    pub hydration_batch_failed: usize,
    /// Unified job history ring (hydrate | fetch | parse | similarity). Cap [`JOB_HISTORY_CAP`].
    pub recent_job_outcomes: std::collections::VecDeque<JobOutcome>,
    pub next_job_id: u64,
    pub selected_job_history_index: usize,

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
    pub capture_note_buffer: String,

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

    // Live dashboard state
    pub dashboard: crate::ui::dashboard::DashboardModel,
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
        let (fetch_tx, fetch_rx) = std::sync::mpsc::channel();
        let (similarity_tx, similarity_rx) = std::sync::mpsc::channel();
        let (estimate_tx, estimate_rx) = std::sync::mpsc::channel();
        let (digest_tx, digest_rx) = std::sync::mpsc::channel();

        let mut app = Self {
            hydration_tx,
            hydration_rx,
            in_flight_hydration_keys: std::collections::HashSet::new(),
            hydrate_retry_payloads: std::collections::HashMap::new(),
            parse_tx,
            parse_rx,
            in_flight_parse_ids: std::collections::HashSet::new(),
            parse_retry_payloads: std::collections::HashMap::new(),
            fetch_tx,
            fetch_rx,
            in_flight_fetch_targets: std::collections::HashSet::new(),
            similarity_tx,
            similarity_rx,
            in_flight_similarity: false,
            estimate_tx,
            estimate_rx,
            in_flight_estimate: false,
            digest_tx,
            digest_rx,
            in_flight_digest: false,
            hydration_batch_succeeded: 0,
            hydration_batch_failed: 0,
            recent_job_outcomes: std::collections::VecDeque::with_capacity(JOB_HISTORY_CAP),
            next_job_id: 1,
            selected_job_history_index: 0,
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
            capture_note_buffer: String::new(),

            draft_ref_similarities: std::collections::HashMap::new(),
            draft_similarity_hash: None,
            min_similarity_filter: None,

            bib_scroll_offset: 0,
            ref_scroll_offset: 0,
            settings_scroll_offset: 0,
            dashboard_scroll_offset: 0,

            selected_setting_index: 0,
            dashboard: crate::ui::dashboard::DashboardModel::default(),
        };
        app.reload_paper_draft();
        app.reload_sources();
        app.load_project_references_bib();
        app.load_all_source_references();
        app.refresh_dashboard();
        app
    }

    pub fn refresh_dashboard(&mut self) {
        self.dashboard = crate::ui::dashboard::DashboardModel::from_app(self);
    }
}
