//! Clap CLI definitions for `sil`.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// scientist-in-loop — turn a paper folder into an agent-friendly workspace.
#[derive(Debug, Parser)]
#[command(name = "sil")]
#[command(
    version,
    about = "scientist-in-loop: agent-friendly scientific paper workspace",
    long_about = None
)]
pub struct Cli {
    /// Disable colors and progress (also set by NO_COLOR / SIL_NO_COLOR / non-TTY).
    #[arg(long, global = true)]
    pub plain: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Create a new sil-managed paper project
    Init {
        /// Project directory name (default: current directory; with --update: project root)
        name: Option<String>,
        /// Upgrade an existing project to the current sil template version
        #[arg(long)]
        update: bool,
    },
    /// Show project stage, git status, sources, and structure summary
    Status {
        /// Machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Manage literature sources and parsing (fetch, list, remove, parse, search, cite, digest, read)
    Source {
        #[command(subcommand)]
        action: SourceCmd,
    },
    /// Manage paper draft, compilation, structure, and templates
    Paper {
        #[command(subcommand)]
        action: PaperCmd,
    },
    /// Project and workspace administration tools (doctor, context, mcp)
    Project {
        #[command(subcommand)]
        action: ProjectCmd,
    },
    /// Source control and sci-actions (log, propose)
    Git {
        #[command(subcommand)]
        action: GitCmd,
    },
    /// Interactive terminal user interfaces (dashboard, settings)
    Tui {
        #[command(subcommand)]
        action: Option<TuiCmd>,
    },
}

/// `sil source` subcommands.
#[derive(Debug, Subcommand)]
pub enum SourceCmd {
    /// Download a scientific paper or source file into sources/
    Fetch {
        /// Target identifier, URL, or file path.
        target: String,
        /// Skip interactive parse offer after download
        #[arg(long)]
        no_parse: bool,
    },
    /// List sources with parsed vs unparsed visibility
    List {
        /// Machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Remove a source from the database (enables reparse); optional file delete
    Remove {
        /// Source id or filename (e.g. paper.pdf)
        id: String,
        /// Also delete the PDF under sources/
        #[arg(long)]
        delete_file: bool,
    },
    /// Parse PDF source(s) into SQLite + FTS5 via Marker
    Parse {
        /// Path to a specific PDF (omit for interactive selection of unparsed sources/)
        path: Option<PathBuf>,
    },
    /// Full-text search over parsed sources
    Search {
        /// FTS5 query string
        query: String,
        /// Max results
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    /// Suggest BibTeX and `\cite{...}` from a source filename or query
    Cite {
        /// Source filename/id or free-text query
        target: String,
        /// Append the stub entry to references.bib
        #[arg(long)]
        append: bool,
        /// Promote an existing entry in references.bib by removing % [sil: tui-added] marker
        #[arg(long)]
        promote: bool,
        /// Machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Fetch top peer-reviewed journal publications digest
    Digest {
        /// Search query or topic (default: machine learning)
        #[arg(default_value = "machine learning")]
        query: String,
        /// Max publications to fetch
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
    /// Open a source document in the interactive TUI markdown reader
    Read {
        /// Source ID, filename, or file path to read
        id: String,
    },
    /// Heal parsed sources: re-extract references and fetch missing metadata via DOI
    Doctor {
        /// Source ID or filename to repair (omit to process all parsed sources)
        id: Option<String>,
    },
    /// Rank extracted references by cosine similarity against paper_draft.tex
    RankDraft {
        /// Filter by minimum similarity score threshold (0.0 to 1.0)
        #[arg(long)]
        min_score: Option<f32>,
        /// Machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
}

/// `sil paper` subcommands.
#[derive(Debug, Subcommand)]
pub enum PaperCmd {
    /// Compile the LaTeX main file from config
    Build {
        /// Target build mode ("release" or "draft")
        target: Option<String>,
        /// Format with target template from config before compiling (legacy flag)
        #[arg(long, hide = true)]
        release: bool,
    },
    /// Split paper_draft.tex into agent-readable files under .sil/draft_sections/
    Split,
    /// Copy paper_draft.tex → paper.tex and propose promote-to-final
    Promote {
        /// Skip structure completion guardrail (sections should be draft/polished)
        #[arg(long)]
        force: bool,
    },
    /// List active # -- X -- # idea and TODO blocks parsed from paper_draft.tex
    Todo {
        /// Machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Inspect or update `.sil/structure.yaml`
    Structure {
        #[command(subcommand)]
        action: StructureCmd,
    },
    /// Collect manuscript prose into conference/journal article templates
    Template {
        #[command(subcommand)]
        action: Option<TemplateCmd>,
        /// Target template (neurips, icml, iclr, ieee, arxiv, standard)
        #[arg(long, short = 't')]
        target: Option<String>,
        /// Source manuscript file
        #[arg(long, short = 'i')]
        input: Option<camino::Utf8PathBuf>,
        /// Output file path
        #[arg(long, short = 'o')]
        output: Option<camino::Utf8PathBuf>,
    },
}

/// `sil project` subcommands.
#[derive(Debug, Subcommand)]
pub enum ProjectCmd {
    /// Check project layout, host dependencies, and manuscript health
    Doctor {
        /// Machine-readable JSON output
        #[arg(long)]
        json: bool,
        /// Provide ONNX model cache bootstrap directories and export recipe
        #[arg(long = "fix-rag")]
        fix_rag: bool,
    },
    /// Generate structured context for a human or agent
    Context {
        /// Include paper_draft.tex split into subsections
        #[arg(long)]
        paper: bool,
        /// Include agent/ listing and README
        #[arg(long)]
        agent: bool,
        /// Include paper.md skill
        #[arg(long)]
        skill_paper: bool,
        /// Include agent-code.md skill
        #[arg(long)]
        skill_agent_code: bool,
        /// Additional skill basenames to load (e.g. paper.md)
        #[arg(long = "skill")]
        skills: Vec<String>,
        /// Free-text task hint for dynamic skill loading
        #[arg(long)]
        task: Option<String>,
    },
    /// Start Model Context Protocol (MCP) stdio server
    Mcp {
        /// Quiet mode (suppress log output on stderr)
        #[arg(short, long)]
        quiet: bool,
    },
}

/// `sil git` subcommands.
#[derive(Debug, Subcommand)]
pub enum GitCmd {
    /// Show git log annotated by Sci-Action trailers
    Log {
        /// Max commits to show
        #[arg(short = 'n', long, default_value_t = 30)]
        limit: usize,
        /// Only show commits that have a Sci-Action trailer
        #[arg(long, default_value_t = true)]
        sci_only: bool,
        /// Include commits without Sci-Action
        #[arg(long)]
        all: bool,
    },
    /// Print a Sci-Action commit proposal (never auto-commits)
    Propose {
        /// Explicit Sci-Action (e.g. edit-draft, update-structure, promote-to-final)
        #[arg(long, short = 'a')]
        action: Option<String>,
        /// Commit subject override
        #[arg(long, short = 'm')]
        message: Option<String>,
        /// Optional body paragraph
        #[arg(long)]
        body: Option<String>,
    },
}

/// `sil tui` subcommands.
#[derive(Debug, Subcommand)]
pub enum TuiCmd {
    /// Launch interactive TUI sources manager
    Sources,
    /// Launch interactive TUI references manager
    References,
    /// Launch interactive TUI settings manager
    Settings,
}

/// `sil paper template` subcommands.
#[derive(Debug, Subcommand)]
pub enum TemplateCmd {
    /// List supported target templates
    List,
    /// Apply target template to manuscript
    Apply {
        /// Target template (neurips, icml, iclr, ieee, arxiv, standard)
        #[arg(long, short = 't')]
        target: Option<String>,
        /// Source manuscript file
        #[arg(long, short = 'i')]
        input: Option<camino::Utf8PathBuf>,
        /// Output file path
        #[arg(long, short = 'o')]
        output: Option<camino::Utf8PathBuf>,
    },
}

/// `sil paper structure` subcommands.
#[derive(Debug, Subcommand)]
pub enum StructureCmd {
    /// List sections and completion levels
    List,
    /// Set a section's completion (`empty`|`outline`|`draft`|`polished`)
    Set {
        /// Section id from structure.yaml
        section_id: String,
        /// New completion level
        completion: String,
    },
}
