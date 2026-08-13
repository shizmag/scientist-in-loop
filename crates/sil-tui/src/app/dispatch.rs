//! Central command dispatcher implementation for `App`.

use super::*;

impl App {
    /// Dispatch and execute a registered command by its [`CommandId`].
    pub fn dispatch(&mut self, cmd: CommandId) {
        match cmd {
            CommandId::OpenPalette => {
                self.palette_previous_mode = self.input_mode;
                self.palette_filter.clear();
                self.palette_selected_index = 0;
                self.input_mode = InputMode::CommandPalette;
                self.status_message =
                    "Command palette — type to filter, Enter to run, Esc to close".to_string();
            }
            CommandId::SaveAll => {
                if !self.check_mutation_lock("save_all") {
                    return;
                }
                self.confirm_lock_override = false;
                self.save_all();
            }
            CommandId::Quit => {
                self.cleanup_lock();
                self.should_quit = true;
            }
            CommandId::OpenHelp => {
                self.toggle_help_overlay();
            }
            CommandId::Reload => {
                self.reload_paper_draft();
                self.reload_sources();
                self.load_project_references_bib();
                self.load_all_source_references();
                self.refresh_dashboard();
                self.status_message = if self.active_tab == ActiveTab::Sources {
                    "✓ Reloaded sources".to_string()
                } else if self.active_tab == ActiveTab::Dashboard {
                    "✓ Reloaded dashboard".to_string()
                } else {
                    "✓ Reloaded project state from disk".to_string()
                };
            }
            CommandId::OpenJobHistory => {
                self.open_job_history();
            }
            CommandId::ParseSelected => {
                if self.sources.is_empty() || self.selected_source_index >= self.sources.len() {
                    self.status_message = "No active source document selected.".to_string();
                    return;
                }
                let doc = self.sources[self.selected_source_index].clone();
                self.queue_source_parse(doc, false);
            }
            CommandId::ParseAll => {
                if self.sources.is_empty() {
                    self.status_message = "No source documents in project to parse.".to_string();
                    return;
                }
                let count = self.sources.len();
                for doc in self.sources.clone() {
                    self.queue_source_parse(doc, false);
                }
                self.status_message = format!("Queued background parsing for {count} sources.");
            }
            CommandId::AddSourceLink => {
                if !self.check_mutation_lock("add_source") {
                    return;
                }
                self.confirm_lock_override = false;
                self.active_tab = ActiveTab::Sources;
                self.new_source_link_buffer.clear();
                self.input_mode = InputMode::ModalAddSourceLink;
                self.status_message =
                    "Fetch/download source — enter URL / DOI / arXiv (Enter to start fetch, Esc to cancel)"
                        .to_string();
            }
            CommandId::OpenSource => {
                if self.sources.is_empty() || self.selected_source_index >= self.sources.len() {
                    self.status_message = "No active source document selected.".to_string();
                    return;
                }
                let doc = &self.sources[self.selected_source_index];
                let content = self.fetch_source_markdown_content(doc);
                self.reading_md_content = Some(content);
                self.input_mode = InputMode::ReadingSourceMd;
                self.source_scroll_offset = 0;
                self.status_message = format!("Reading {}. Press Esc to exit.", doc.filename);
            }
            CommandId::CiteSource => {
                if !self.check_mutation_lock("cite_source") {
                    return;
                }
                self.confirm_lock_override = false;
                self.append_selected_source_to_bib();
            }
            CommandId::CaptureNote => {
                if self.sources.is_empty() || self.selected_source_index >= self.sources.len() {
                    self.status_message = "No active source document selected.".to_string();
                    return;
                }
                if !self.check_mutation_lock("capture_note") {
                    return;
                }
                self.confirm_lock_override = false;
                self.capture_note_buffer.clear();
                self.input_mode = InputMode::ModalCaptureNote;
                self.status_message =
                    "Capture note for draft (Enter to save, Esc to cancel)".to_string();
            }
        }
    }
}
