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

    // Disk snapshot mtimes & conflict banner state
    pub file_mtimes: std::collections::HashMap<Utf8PathBuf, std::time::SystemTime>,
    pub disk_conflict_banner: Option<String>,
    pub disk_conflict_pending: bool,
    pub confirm_disk_overwrite: bool,
    pub disk_conflict_dismissed: bool,

    // First-run wizard state
    pub wizard_state: WizardState,
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

        let is_no_project = project_root.is_none();
        let wizard_state = WizardState::new(&global_settings);

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
            input_mode: if is_no_project {
                InputMode::Wizard
            } else {
                InputMode::Normal
            },
            saved_input_mode: if is_no_project {
                InputMode::Wizard
            } else {
                InputMode::Normal
            },

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
            status_message: if is_no_project {
                "Welcome to scientist-in-loop! Select an option below or press 1-4.".to_string()
            } else {
                "Ready. Press 'Tab' to switch views, 'e' to edit section, 'v' for external $EDITOR, 's' to save.".to_string()
            },
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

            file_mtimes: std::collections::HashMap::new(),
            disk_conflict_banner: None,
            disk_conflict_pending: false,
            confirm_disk_overwrite: false,
            disk_conflict_dismissed: false,

            wizard_state,
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
        app.update_file_mtimes();
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

    /// Return the list of watched file paths for conflict detection.
    pub fn watched_files(&self) -> Vec<Utf8PathBuf> {
        let Some(ref root) = self.project_root else {
            return Vec::new();
        };
        let paths = ProjectPaths::new(root);
        vec![
            paths.paper_draft(),
            root.join("references.bib"),
            paths.config(),
        ]
    }

    /// Snapshot modified times for watched files on disk.
    pub fn update_file_mtimes(&mut self) {
        self.file_mtimes.clear();
        for path in self.watched_files() {
            if let Ok(metadata) = std::fs::metadata(path.as_std_path()) {
                if let Ok(mtime) = metadata.modified() {
                    self.file_mtimes.insert(path, mtime);
                }
            }
        }
    }

    /// Check whether any watched files on disk were modified externally after our snapshot.
    ///
    /// If disk is newer AND `self.dirty`:
    /// - Sets `disk_conflict_banner`, `disk_conflict_pending = true`, sets `last_user_error` with `"conflict.disk_newer"`.
    /// - Returns `true` (conflict active, blocking overwrite).
    ///
    /// If disk is newer AND NOT `self.dirty`:
    /// - Sets status message warning without blocking navigation.
    /// - Returns `false`.
    ///
    /// If no files are newer, returns `false`.
    pub fn check_disk_conflicts(&mut self) -> bool {
        let mut conflict = false;
        let mut conflict_file: Option<String> = None;

        for path in self.watched_files() {
            if let Ok(metadata) = std::fs::metadata(path.as_std_path()) {
                if let Ok(mtime) = metadata.modified() {
                    if let Some(&recorded) = self.file_mtimes.get(&path) {
                        if mtime > recorded {
                            conflict = true;
                            conflict_file = Some(path.file_name().unwrap_or("file").to_string());
                            break;
                        }
                    } else if !self.file_mtimes.is_empty() {
                        // File appeared on disk after initial snapshot
                        conflict = true;
                        conflict_file = Some(path.file_name().unwrap_or("file").to_string());
                        break;
                    }
                }
            }
        }

        if conflict {
            if self.dirty {
                self.disk_conflict_pending = true;
                let banner = "Disk changed externally: Reload (R) or Keep TUI (Ctrl+S again to overwrite)".to_string();
                self.disk_conflict_banner = Some(banner);
                self.last_user_error = Some(sil_core::UserError::new(
                    "conflict.disk_newer",
                    "Disk files modified externally",
                    "Press R to reload from disk, or save again to overwrite disk changes",
                    None,
                ));
                true
            } else {
                let file_str = conflict_file.as_deref().unwrap_or("Disk files");
                self.status_message = format!("Disk changed externally ({file_str}) — press R to reload");
                false
            }
        } else {
            false
        }
    }

    /// Dismiss the disk conflict banner ("Keep TUI").
    pub fn dismiss_disk_conflict(&mut self) {
        self.disk_conflict_banner = None;
        self.disk_conflict_pending = false;
        self.disk_conflict_dismissed = true;
    }

    /// Reload all project state (sources, bib, draft, config) from disk and refresh snapshots.
    pub fn reload_sources_and_bib_sync(&mut self) {
        if let Some(ref root) = self.project_root {
            let paths = ProjectPaths::new(root);
            if let Ok(cfg) = Config::load(&paths.config()) {
                self.local_settings = cfg.settings.clone();
                self.loaded_config = Some(cfg);
            }
        }
        self.reload_paper_draft();
        self.reload_sources();
        self.load_project_references_bib();
        self.load_all_source_references();
        self.refresh_dashboard();
        self.dirty = false;
        self.disk_conflict_banner = None;
        self.disk_conflict_pending = false;
        self.confirm_disk_overwrite = false;
        self.disk_conflict_dismissed = false;
        self.update_file_mtimes();
    }

    /// Open a sil project at the given path.
    ///
    /// Validates that the directory exists and contains a sil project (`.sil/config.yaml` or `.sil`).
    /// If invalid, sets `last_user_error` with `project.not_found` and an error status message.
    /// If valid, sets `project_root`, loads config, sources, bib, draft, updates recent projects,
    /// and resets `input_mode` to `InputMode::Normal`.
    pub fn open_project_path(&mut self, path: Utf8PathBuf) -> bool {
        let resolved = if path.is_absolute() {
            path
        } else if let Ok(cwd) = std::env::current_dir() {
            if let Ok(cwd_utf8) = Utf8PathBuf::from_path_buf(cwd) {
                cwd_utf8.join(path)
            } else {
                path
            }
        } else {
            path
        };

        let paths = ProjectPaths::new(&resolved);
        let is_valid = resolved.is_dir() && (paths.config().is_file() || paths.sil_dir().is_dir());

        if !is_valid {
            self.last_user_error = Some(sil_core::UserError::new(
                "project.not_found",
                "Not a valid sil project",
                format!("Directory '{resolved}' does not contain .sil/config.yaml or .sil directory"),
                None,
            ));
            self.status_message = format!("Not a valid sil project: {resolved}");
            return false;
        }

        self.project_root = Some(resolved.clone());

        if let Ok(cfg) = Config::load(&paths.config()) {
            self.local_settings = cfg.settings.clone();
            self.loaded_config = Some(cfg);
        } else {
            self.local_settings = LocalSettings::default();
            self.loaded_config = None;
        }

        // Advisory session lock check
        if let Ok(sil_core::TakeLockResult::Held(lock)) =
            sil_core::try_acquire_lock(&paths, "tui", "session")
        {
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
            self.active_lock_conflict = Some(lock);
        } else {
            self.active_lock_conflict = None;
            self.lock_holder_banner = None;
        }

        self.reload_paper_draft();
        self.reload_sources();
        self.load_project_references_bib();
        self.load_all_source_references();
        self.refresh_dashboard();
        self.update_file_mtimes();

        // Record in recents
        self.global_settings.touch_recent_project(resolved.clone());
        let _ = self.global_settings.save(None);
        self.wizard_state.refresh_recent_projects(&self.global_settings);

        self.input_mode = InputMode::Normal;
        self.dirty = false;
        let proj_name = resolved.file_name().unwrap_or(resolved.as_str());
        self.status_message = format!("✓ Opened project: {proj_name}");
        true
    }

    /// Create a new sil project at `name_or_path` and open it.
    pub fn create_and_open_project(&mut self, name_or_path: &str) -> bool {
        let trimmed = name_or_path.trim();
        if trimmed.is_empty() {
            self.status_message = "Project name cannot be empty.".to_string();
            return false;
        }

        let target_path = if camino::Utf8Path::new(trimmed).is_absolute() {
            Utf8PathBuf::from(trimmed)
        } else if let Ok(cwd) = std::env::current_dir() {
            if let Ok(cwd_utf8) = Utf8PathBuf::from_path_buf(cwd) {
                cwd_utf8.join(trimmed)
            } else {
                Utf8PathBuf::from(trimmed)
            }
        } else {
            Utf8PathBuf::from(trimmed)
        };

        let ui = sil_core::NullUi::new();
        match sil_app::init::init_project(&target_path, &ui) {
            Ok(_) => {
                let ok = self.open_project_path(target_path.clone());
                if ok {
                    let proj_name = target_path.file_name().unwrap_or(target_path.as_str());
                    self.status_message = format!("✓ Created and opened project: {proj_name}");
                }
                ok
            }
            Err(e) => {
                self.last_user_error = Some(sil_core::UserError::classify(&e.to_string()));
                self.status_message = format!("Project creation failed: {e}");
                false
            }
        }
    }

    /// Run host environment doctor checks and switch to WizardDoctorReport mode.
    pub fn run_wizard_doctor(&mut self) {
        self.wizard_state.doctor_checks = sil_app::doctor::run_host_checks();
        self.wizard_state.doctor_scroll_offset = 0;
        self.input_mode = InputMode::WizardDoctorReport;
        let ok_count = self.wizard_state.doctor_checks.iter().filter(|c| c.ok).count();
        let total = self.wizard_state.doctor_checks.len();
        self.status_message =
            format!("Host doctor finished: {ok_count}/{total} checks passed (Esc to back).");
    }

    /// Activate the currently selected option in the Wizard menu.
    pub fn activate_wizard_selection(&mut self) {
        match self.wizard_state.selected_menu_index {
            0 => {
                // Open Recent Project
                if self.wizard_state.recent_projects.is_empty() {
                    self.last_user_error = Some(sil_core::UserError::new(
                        "project.not_found",
                        "No recent projects found",
                        "Choose 'Open Directory / Path' or 'Create New Project' to get started",
                        None,
                    ));
                    self.status_message = "No recent projects found.".to_string();
                } else {
                    let idx = self.wizard_state.selected_recent_index;
                    if idx < self.wizard_state.recent_projects.len() {
                        let path = self.wizard_state.recent_projects[idx].clone();
                        self.open_project_path(path);
                    }
                }
            }
            1 => {
                // Open Directory / Path
                self.wizard_state.open_path_buffer.clear();
                self.input_mode = InputMode::WizardOpenPath;
                self.status_message =
                    "Enter project directory path (Enter to open, Esc to back)".to_string();
            }
            2 => {
                // Create New Project
                self.wizard_state.create_project_buffer.clear();
                self.input_mode = InputMode::WizardCreateProject;
                self.status_message =
                    "Enter project name or path (Enter to create, Esc to back)".to_string();
            }
            3 => {
                // Run System Doctor
                self.run_wizard_doctor();
            }
            _ => {}
        }
    }
}
