//! Application state and logic for `sil-tui`.

pub(crate) mod bib_actions;
pub(crate) mod commands;
pub(crate) mod dispatch;
pub(crate) mod handlers;
pub(crate) mod jobs;
pub(crate) mod types;

#[cfg(test)]
mod tests;

pub use commands::*;
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
    pub last_user_error: Option<sil_core::UserError>,
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
    pub selected_digest_index: usize,
    pub dashboard: crate::ui::dashboard::DashboardModel,

    // Command palette state
    pub palette_filter: String,
    pub palette_selected_index: usize,
    pub palette_previous_mode: InputMode,

    // Advisory workspace lock & conflict state
    pub active_lock_conflict: Option<sil_core::WorkspaceLock>,
    pub lock_holder_banner: Option<String>,
    pub confirm_lock_override: bool,
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
            last_user_error: None,
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
            selected_digest_index: 0,
            dashboard: crate::ui::dashboard::DashboardModel::default(),

            palette_filter: String::new(),
            palette_selected_index: 0,
            palette_previous_mode: InputMode::Normal,

            active_lock_conflict: None,
            lock_holder_banner: None,
            confirm_lock_override: false,
        };

        // Acquire initial advisory session lock if inside a project
        if let Some(ref root) = app.project_root {
            let paths = ProjectPaths::new(root);
            if let Ok(sil_core::TakeLockResult::Held(lock)) =
                sil_core::try_acquire_lock(&paths, "tui", "session")
            {
                let pid_str = lock
                    .pid
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let banner = format!("{} is {} (pid {})", lock.holder, lock.op, pid_str);
                app.lock_holder_banner = Some(banner.clone());
                app.last_user_error = Some(sil_core::UserError::new(
                    "lock.held",
                    "Workspace lock is held",
                    format!("{banner} — confirm to override"),
                    None,
                ));
                app.status_message = format!("Warning: {banner}. Confirm to proceed.");
                app.active_lock_conflict = Some(lock);
            }
        }

        app.reload_paper_draft();
        app.reload_sources();
        app.load_project_references_bib();
        app.load_all_source_references();
        app.refresh_dashboard();
        app
    }

    /// Check whether another live process holds the workspace lock before executing a mutating command.
    ///
    /// If lock is held and `confirm_lock_override` is false, this sets the warning banner,
    /// sets `confirm_lock_override = true`, and returns `false` (blocking the mutation).
    /// If confirmed or acquired, returns `true`.
    pub fn check_mutation_lock(&mut self, op: &str) -> bool {
        let Some(ref root) = self.project_root else {
            return true;
        };

        let paths = ProjectPaths::new(root);

        if self.confirm_lock_override {
            // User confirmed override: claim lock for TUI now
            let _ = sil_core::write_lock(&paths, &sil_core::WorkspaceLock::new("tui", op));
            self.active_lock_conflict = None;
            self.lock_holder_banner = None;
            return true;
        }

        match sil_core::try_acquire_lock(&paths, "tui", op) {
            Ok(sil_core::TakeLockResult::Acquired) => {
                self.active_lock_conflict = None;
                self.lock_holder_banner = None;
                true
            }
            Ok(sil_core::TakeLockResult::Held(lock)) => {
                self.active_lock_conflict = Some(lock.clone());
                let pid_str = lock
                    .pid
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let banner = format!("{} is {} (pid {})", lock.holder, lock.op, pid_str);
                self.lock_holder_banner = Some(banner.clone());
                self.last_user_error = Some(sil_core::UserError::new(
                    "lock.held",
                    "Workspace lock is held",
                    format!("{banner} — confirm to override"),
                    None,
                ));
                self.status_message = format!("Warning: {banner}. Confirm to proceed.");
                self.confirm_lock_override = true;
                false
            }
            Err(_) => true,
        }
    }

    /// Clear the session lock on exit if owned by this TUI process.
    pub fn cleanup_lock(&self) {
        if let Some(ref root) = self.project_root {
            let paths = ProjectPaths::new(root);
            if let Ok(Some(lock)) = sil_core::read_lock(&paths) {
                if lock.holder == "tui" && lock.pid == Some(std::process::id()) {
                    let _ = sil_core::clear_lock(&paths);
                }
            }
        }
    }

    pub fn clamp_digest_selection(&mut self) {
        let count = self.dashboard.digest_publications.len();
        if count == 0 {
            self.selected_digest_index = 0;
        } else if self.selected_digest_index >= count {
            self.selected_digest_index = count - 1;
        }
    }

    pub fn refresh_dashboard(&mut self) {
        self.dashboard = crate::ui::dashboard::DashboardModel::from_app(self);
        self.clamp_digest_selection();
    }

    /// Return commands matching current `palette_filter` string.
    pub fn filtered_commands(&self) -> Vec<&'static CommandSpec> {
        let query = self.palette_filter.trim().to_lowercase();
        all_commands()
            .iter()
            .filter(|spec| {
                if query.is_empty() {
                    return true;
                }
                spec.title.to_lowercase().contains(&query)
                    || spec.id.as_str().to_lowercase().contains(&query)
                    || spec.description.to_lowercase().contains(&query)
                    || spec.aliases.iter().any(|a| a.to_lowercase().contains(&query))
            })
            .collect()
    }

    /// Clamp selected palette index to bounds of filtered commands.
    pub fn clamp_palette_selection(&mut self) {
        let count = self.filtered_commands().len();
        if count == 0 {
            self.palette_selected_index = 0;
        } else if self.palette_selected_index >= count {
            self.palette_selected_index = count - 1;
        }
    }
}
