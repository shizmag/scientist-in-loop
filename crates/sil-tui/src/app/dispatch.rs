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
                self.reload_sources_and_bib_sync();
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
                let unparsed: Vec<_> = self.sources.iter().filter(|s| !s.parsed).cloned().collect();
                let (docs_to_parse, force) = if unparsed.is_empty() {
                    (self.sources.clone(), true)
                } else {
                    (unparsed, false)
                };
                let count = docs_to_parse.len();
                for doc in docs_to_parse {
                    self.queue_source_parse(doc, force);
                }
                if count > 1 {
                    self.status_message = format!("Queued background parsing for {count} sources.");
                }
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
            CommandId::RefreshDigest => {
                let effective_query = sil_core::effective_digest_query(
                    &self.global_settings.digest_query,
                    &self.local_settings.digest_query,
                );
                if effective_query.is_none() {
                    self.status_message =
                        "No digest query configured. Set query in Settings tab (Tab 5).".to_string();
                    return;
                }
                self.queue_digest_refresh();
            }
            CommandId::OpenExternalEditor => {
                self.pending_external_editor = true;
                self.status_message =
                    "Launching external editor ($EDITOR / nvim / helix)...".to_string();
            }
        }
    }
}
