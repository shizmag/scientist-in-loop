//! Command registry, identifiers, specifications, and availability rules for `sil-tui`.

use super::{ActiveTab, App};

/// Unique identifier for every executable command in `sil-tui`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandId {
    /// Save all settings, draft, and project state to disk.
    SaveAll,
    /// Open the command palette to search and run commands.
    OpenPalette,
    /// Exit the application.
    Quit,
    /// Toggle the contextual keyboard help overlay.
    OpenHelp,
    /// Reload project sources, references, draft, and dashboard from disk.
    Reload,
    /// Open the background job history modal.
    OpenJobHistory,
    /// Parse and extract text and references from the selected source document.
    ParseSelected,
    /// Queue background parsing for all unparsed source documents.
    ParseAll,
    /// Open modal to add and download a source via DOI, arXiv, or URL.
    AddSourceLink,
    /// Open and read the selected source document in Markdown viewer.
    OpenSource,
    /// Append the selected source document to references.bib.
    CiteSource,
    /// Capture an idea note from the selected source onto paper_draft.tex.
    CaptureNote,
    /// Refresh the literature digest publications.
    RefreshDigest,
    /// Open paper draft in external $EDITOR.
    OpenExternalEditor,
    /// Undo the last file deletion or insertion mutation.
    Undo,
}

impl CommandId {
    /// Return the canonical string identifier for this command.
    pub fn as_str(&self) -> &'static str {
        match self {
            CommandId::SaveAll => "save_all",
            CommandId::OpenPalette => "open_palette",
            CommandId::Quit => "quit",
            CommandId::OpenHelp => "open_help",
            CommandId::Reload => "reload",
            CommandId::OpenJobHistory => "open_job_history",
            CommandId::ParseSelected => "parse_selected",
            CommandId::ParseAll => "parse_all",
            CommandId::AddSourceLink => "add_source_link",
            CommandId::OpenSource => "open_source",
            CommandId::CiteSource => "cite_source",
            CommandId::CaptureNote => "capture_note",
            CommandId::RefreshDigest => "refresh_digest",
            CommandId::OpenExternalEditor => "open_external_editor",
            CommandId::Undo => "undo",
        }
    }
}

impl std::fmt::Display for CommandId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Metadata and registration specification for a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    /// Unique command identifier.
    pub id: CommandId,
    /// Human-readable title displayed in palette and menus.
    pub title: &'static str,
    /// Search aliases / keywords for palette filtering.
    pub aliases: &'static [&'static str],
    /// Default keyboard shortcut(s) hint.
    pub default_keys: &'static str,
    /// Optional associated tab context.
    pub tab: Option<ActiveTab>,
    /// One-line description of what the command does.
    pub description: &'static str,
}

impl CommandSpec {
    /// Check whether this command is currently executable in the current `App` context.
    /// Returns `Ok(())` if available, or `Err(reason)` if disabled.
    pub fn is_available(&self, app: &App) -> Result<(), &'static str> {
        match self.id {
            CommandId::SaveAll
            | CommandId::OpenPalette
            | CommandId::Quit
            | CommandId::OpenHelp
            | CommandId::OpenJobHistory => Ok(()),
            CommandId::Reload => {
                if app.project_root.is_none() {
                    Err("requires active project")
                } else {
                    Ok(())
                }
            }
            CommandId::ParseSelected => {
                if app.project_root.is_none() {
                    Err("requires active project")
                } else if app.sources.is_empty() || app.selected_source_index >= app.sources.len() {
                    Err("no source selected")
                } else {
                    Ok(())
                }
            }
            CommandId::ParseAll => {
                if app.project_root.is_none() {
                    Err("requires active project")
                } else if app.sources.is_empty() {
                    Err("no sources in project")
                } else {
                    Ok(())
                }
            }
            CommandId::AddSourceLink => {
                if app.project_root.is_none() {
                    Err("requires active project")
                } else {
                    Ok(())
                }
            }
            CommandId::OpenSource => {
                if app.sources.is_empty() || app.selected_source_index >= app.sources.len() {
                    Err("no source selected")
                } else {
                    Ok(())
                }
            }
            CommandId::CiteSource => {
                if app.project_root.is_none() {
                    Err("requires active project")
                } else if app.sources.is_empty() || app.selected_source_index >= app.sources.len() {
                    Err("no source selected")
                } else {
                    Ok(())
                }
            }
            CommandId::CaptureNote => {
                if app.project_root.is_none() {
                    Err("requires active project")
                } else if app.sources.is_empty() || app.selected_source_index >= app.sources.len() {
                    Err("no source selected")
                } else {
                    Ok(())
                }
            }
            CommandId::RefreshDigest => {
                if app.project_root.is_none() {
                    Err("requires active project")
                } else {
                    Ok(())
                }
            }
            CommandId::OpenExternalEditor | CommandId::Undo => {
                if app.project_root.is_none() {
                    Err("requires active project")
                } else {
                    Ok(())
                }
            }
        }
    }
}

/// Static registry listing all registered executable commands.
pub fn all_commands() -> &'static [CommandSpec] {
    &[
        CommandSpec {
            id: CommandId::OpenPalette,
            title: "Open Command Palette",
            aliases: &["palette", "commands", "menu"],
            default_keys: ":, Ctrl+K",
            tab: None,
            description: "Search and execute commands by name",
        },
        CommandSpec {
            id: CommandId::SaveAll,
            title: "Save All",
            aliases: &["save", "write"],
            default_keys: "Ctrl+S, s",
            tab: None,
            description: "Save all modified settings, draft, and project state",
        },
        CommandSpec {
            id: CommandId::Undo,
            title: "Undo",
            aliases: &["undo", "revert", "z"],
            default_keys: "Ctrl+Z",
            tab: None,
            description: "Undo the last file deletion or insertion mutation",
        },
        CommandSpec {
            id: CommandId::OpenHelp,
            title: "Help Overlay",
            aliases: &["help", "keymap", "shortcuts"],
            default_keys: "?, F1",
            tab: None,
            description: "Toggle contextual keyboard help overlay",
        },
        CommandSpec {
            id: CommandId::OpenJobHistory,
            title: "Background Job History",
            aliases: &["jobs", "history", "tasks"],
            default_keys: "J",
            tab: None,
            description: "View and retry background fetch, parse, and hydration jobs",
        },
        CommandSpec {
            id: CommandId::Reload,
            title: "Reload Project State",
            aliases: &["reload", "refresh"],
            default_keys: "R",
            tab: None,
            description: "Reload sources, references.bib, and dashboard from disk",
        },
        CommandSpec {
            id: CommandId::AddSourceLink,
            title: "Add Source Link / Download",
            aliases: &["add", "download", "fetch", "import"],
            default_keys: "a",
            tab: Some(ActiveTab::Sources),
            description: "Fetch and download a source document via DOI, arXiv, or URL",
        },
        CommandSpec {
            id: CommandId::ParseSelected,
            title: "Parse Selected Source",
            aliases: &["parse", "extract"],
            default_keys: "e",
            tab: Some(ActiveTab::Sources),
            description: "Extract text and references from selected source document",
        },
        CommandSpec {
            id: CommandId::ParseAll,
            title: "Parse All Sources",
            aliases: &["parse-all", "batch-parse"],
            default_keys: "Shift+E",
            tab: Some(ActiveTab::Sources),
            description: "Queue background parsing for all unparsed source documents",
        },
        CommandSpec {
            id: CommandId::OpenSource,
            title: "Open Source Markdown",
            aliases: &["read", "view", "open"],
            default_keys: "Enter",
            tab: Some(ActiveTab::Sources),
            description: "Read full text of selected source document in Markdown viewer",
        },
        CommandSpec {
            id: CommandId::CiteSource,
            title: "Cite Source in references.bib",
            aliases: &["cite", "bib", "add-bib"],
            default_keys: "b",
            tab: Some(ActiveTab::Sources),
            description: "Append selected source document citation to references.bib",
        },
        CommandSpec {
            id: CommandId::CaptureNote,
            title: "Capture Note on Paper Draft",
            aliases: &["note", "park", "draft-note"],
            default_keys: "n",
            tab: Some(ActiveTab::Sources),
            description: "Park an idea note linked to selected source onto paper_draft.tex",
        },
        CommandSpec {
            id: CommandId::RefreshDigest,
            title: "Refresh Literature Digest",
            aliases: &["digest", "refresh-digest", "literature"],
            default_keys: "",
            tab: Some(ActiveTab::Dashboard),
            description: "Query OpenAlex for recent publications matching digest query",
        },
        CommandSpec {
            id: CommandId::OpenExternalEditor,
            title: "Open Draft in External Editor",
            aliases: &["editor", "external-editor", "nvim", "vim", "helix"],
            default_keys: "v, o",
            tab: Some(ActiveTab::PaperDraft),
            description: "Open paper_draft.tex in external $EDITOR (nvim / helix / vim)",
        },
        CommandSpec {
            id: CommandId::Quit,
            title: "Quit",
            aliases: &["exit", "close", "quit"],
            default_keys: "q, Esc",
            tab: None,
            description: "Exit scientist-in-loop",
        },
    ]
}
