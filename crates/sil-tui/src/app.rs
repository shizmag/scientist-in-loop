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
    GlobalSettings = 0,
    LocalSettings = 1,
    CoAuthorCache = 2,
    GrantCache = 3,
}

impl ActiveTab {
    pub const ALL: [ActiveTab; 4] = [
        ActiveTab::GlobalSettings,
        ActiveTab::LocalSettings,
        ActiveTab::CoAuthorCache,
        ActiveTab::GrantCache,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            ActiveTab::GlobalSettings => "1. Global Settings",
            ActiveTab::LocalSettings => "2. Local Settings",
            ActiveTab::CoAuthorCache => "3. Co-Authors Cache",
            ActiveTab::GrantCache => "4. Grants Cache",
        }
    }
}

/// Mode of user interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
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

        Self {
            active_tab: ActiveTab::GlobalSettings,
            input_mode: InputMode::Normal,
            global_settings,
            local_settings,
            cache,
            project_root,
            loaded_config,
            selected_global_field: 0,
            selected_local_field: 0,
            cache_coauthor_index: 0,
            cache_grant_index: 0,
            local_coauthor_index: 0,
            local_grant_index: 0,
            input_buffer: String::new(),
            status_message: "Ready. Press 'Tab' to switch views, 'e'/'Enter' to edit, 's' to save.".to_string(),
            dirty: false,
            should_quit: false,
            new_author: AuthorDetails::default(),
            new_grant: GrantDetails::default(),
            modal_field_index: 0,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Normal => self.handle_normal_mode(key),
            InputMode::Editing => self.handle_editing_mode(key),
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
                self.active_tab = match (current + 1) % ActiveTab::ALL.len() {
                    0 => ActiveTab::GlobalSettings,
                    1 => ActiveTab::LocalSettings,
                    2 => ActiveTab::CoAuthorCache,
                    _ => ActiveTab::GrantCache,
                };
            }
            KeyCode::BackTab => {
                let current = self.active_tab as usize;
                let next = if current == 0 {
                    ActiveTab::ALL.len() - 1
                } else {
                    current - 1
                };
                self.active_tab = match next {
                    0 => ActiveTab::GlobalSettings,
                    1 => ActiveTab::LocalSettings,
                    2 => ActiveTab::CoAuthorCache,
                    _ => ActiveTab::GrantCache,
                };
            }
            KeyCode::Char('1') => self.active_tab = ActiveTab::GlobalSettings,
            KeyCode::Char('2') => self.active_tab = ActiveTab::LocalSettings,
            KeyCode::Char('3') => self.active_tab = ActiveTab::CoAuthorCache,
            KeyCode::Char('4') => self.active_tab = ActiveTab::GrantCache,
            KeyCode::Char('s') => self.save_all(),

            KeyCode::Up | KeyCode::Char('k') => match self.active_tab {
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
            },
            KeyCode::Down | KeyCode::Char('j') => match self.active_tab {
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
            _ => {}
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
                    if self.active_tab == ActiveTab::LocalSettings
                        || self.selected_local_field == LocalField::CoAuthorsList as usize
                    {
                        if !self.local_settings.co_authors.contains(&author) {
                            self.local_settings.co_authors.push(author);
                        }
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
                    if self.active_tab == ActiveTab::LocalSettings
                        || self.selected_local_field == LocalField::GrantsList as usize
                    {
                        if !self.local_settings.grants.contains(&grant) {
                            self.local_settings.grants.push(grant);
                        }
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
        assert_eq!(app.active_tab, ActiveTab::GlobalSettings);
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(!app.should_quit);
    }

    #[test]
    fn tab_navigation() {
        let mut app = App::new(None);
        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.active_tab, ActiveTab::LocalSettings);
        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.active_tab, ActiveTab::CoAuthorCache);
        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.active_tab, ActiveTab::GrantCache);
        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.active_tab, ActiveTab::GlobalSettings);
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
}
