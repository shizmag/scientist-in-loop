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
        ///
        /// Refreshes skills, structure.example.yaml, and the sil-managed `.gitignore`
        /// block. Creates any missing scaffold files. Never overwrites config.yaml,
        /// structure.yaml, manuscripts, or custom gitignore rules outside the managed block.
        #[arg(long)]
        update: bool,
    },
    /// Show project stage, git status, sources, and structure summary
    Status {
        /// Machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Parse PDF source(s) into SQLite + FTS5 via Marker
    Parse {
        /// Path to a specific PDF (omit for interactive selection of unparsed sources/)
        path: Option<PathBuf>,
    },
    /// Manage source PDFs
    Source {
        #[command(subcommand)]
        action: SourceCmd,
    },
    /// Full-text search over parsed sources
    Search {
        /// FTS5 query string
        query: String,
        /// Max results
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    /// Compile the LaTeX main file from config
    Build {
        /// Format with target template from config before compiling
        #[arg(long)]
        release: bool,
    },
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
    /// Split paper_draft.tex into agent-readable files under .sil/draft_sections/
    ///
    /// Does not modify paper_draft.tex. Re-run after editing the draft to refresh
    /// the section tree (source of truth stays the draft).
    Split,
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
    /// Copy paper_draft.tex → paper.tex and propose promote-to-final
    Promote {
        /// Skip structure completion guardrail (sections should be draft/polished)
        #[arg(long)]
        force: bool,
    },
    /// Inspect or update `.sil/structure.yaml`
    Structure {
        #[command(subcommand)]
        action: StructureCmd,
    },
    /// Collect manuscript prose into conference/journal article templates (NeurIPS, ICML, ICLR, IEEE/CVPR, arXiv)
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
    /// Suggest BibTeX and `\cite{...}` from a source filename or query
    Cite {
        /// Source filename/id or free-text query
        target: String,
        /// Append the stub entry to references.bib
        #[arg(long)]
        append: bool,
        /// Machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Check project layout and host dependencies
    Doctor {
        /// Machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Launch interactive TUI to manage global/local settings and co-authors cache
    #[command(alias = "tui")]
    Settings,
}

/// `sil template` subcommands.
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

/// `sil source` subcommands.
#[derive(Debug, Subcommand)]
pub enum SourceCmd {
    /// Download a PDF by DOI, arXiv id, or URL into sources/
    Fetch {
        /// DOI, arXiv identifier, or direct URL
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
}

/// `sil structure` subcommands.
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
