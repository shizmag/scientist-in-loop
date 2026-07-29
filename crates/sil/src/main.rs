//! `sil` — scientist-in-loop CLI binary (thin clap wiring only).
//!
//! Stages 0–5 complete: all MVP commands are wired here; domain logic lives in
//! library crates.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};
use camino::{Utf8Path, Utf8PathBuf};
use clap::{Parser, Subcommand};

use sil_agent::{ContextFlags, ContextInput, SkillSelection, generate_context, sources_summary};
use sil_core::{
    Config, NullUi, ProjectPaths, SciAction, SilProject, SilUi, StdUi, Structure, paths::rel,
};
use sil_db::SilDb;
use sil_git::{CommitProposal, log_entries, path_has_changes, status as git_status};
use sil_latex::build as latex_build;
use sil_parse::{
    MarkerRunner, PythonMarkerRunner, list_unparsed_pdfs, parse_many, parse_one,
    select_pdfs_interactive,
};

mod init;
mod templates;

/// scientist-in-loop — turn a paper folder into an agent-friendly workspace.
#[derive(Debug, Parser)]
#[command(name = "sil")]
#[command(version, about = "scientist-in-loop: agent-friendly scientific paper workspace", long_about = None)]
struct Cli {
    /// Disable colors and progress (also set by NO_COLOR / SIL_NO_COLOR / non-TTY).
    #[arg(long, global = true)]
    plain: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create a new sil-managed paper project
    Init {
        /// Project directory name (default: current directory)
        name: Option<String>,
    },
    /// Show project stage, git status, sources, and structure summary
    Status,
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
    Build,
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
}

#[derive(Debug, Subcommand)]
enum SourceCmd {
    /// Download a PDF by DOI, arXiv id, or URL into sources/
    Fetch {
        /// DOI, arXiv identifier, or direct URL
        target: String,
        /// Skip interactive parse offer after download
        #[arg(long)]
        no_parse: bool,
    },
}

fn make_ui(plain: bool) -> Box<dyn SilUi> {
    if plain
        || std::env::var_os("NO_COLOR").is_some()
        || std::env::var("SIL_NO_COLOR").map(|v| v == "1").unwrap_or(false)
        || std::env::var("SIL_NONINTERACTIVE")
            .map(|v| v == "1")
            .unwrap_or(false)
    {
        Box::new(StdUi::plain())
    } else {
        Box::new(StdUi::new())
    }
}

fn main() {
    if let Err(e) = run() {
        // Prefer SilUi if we can; fall back to eprintln.
        eprintln!("✖ {e}");
        // Print causes.
        let mut source = e.source();
        while let Some(s) = source {
            eprintln!("  ↳ {s}");
            source = s.source();
        }
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let ui = make_ui(cli.plain);
    match cli.command {
        Commands::Init { name } => cmd_init(name, ui.as_ref()),
        Commands::Status => cmd_status(ui.as_ref()),
        Commands::Parse { path } => cmd_parse(path, ui.as_ref()),
        Commands::Source {
            action: SourceCmd::Fetch { target, no_parse },
        } => cmd_source_fetch(&target, no_parse, ui.as_ref()),
        Commands::Search { query, limit } => cmd_search(&query, limit, ui.as_ref()),
        Commands::Build => cmd_build(ui.as_ref()),
        Commands::Log {
            limit,
            sci_only,
            all,
        } => cmd_log(limit, if all { false } else { sci_only }, ui.as_ref()),
        Commands::Context {
            paper,
            agent,
            skill_paper,
            skill_agent_code,
            skills,
            task,
        } => cmd_context(
            ContextFlags {
                paper,
                agent,
                skill_paper,
                skill_agent_code,
                skills,
            },
            task.as_deref(),
            ui.as_ref(),
        ),
    }
}

fn load_project() -> Result<(Utf8PathBuf, Config, ProjectPaths)> {
    let root = sil_core::project_root_from_cwd().map_err(|e| anyhow::anyhow!("{e}"))?;
    let paths = ProjectPaths::new(&root);
    let config = Config::load(&paths.config()).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok((root, config, paths))
}

fn print_proposal(ui: &dyn SilUi, proposal: &CommitProposal) {
    ui.println("");
    ui.info("Commit proposal (not applied — never auto-committed):");
    ui.muted("---");
    for line in proposal.message().lines() {
        ui.muted(line);
    }
    ui.muted("---");
    ui.muted("To apply: git add -A && git commit with the message above.");
}

// ── init ────────────────────────────────────────────────────────────────────

fn cmd_init(name: Option<String>, ui: &dyn SilUi) -> Result<()> {
    let target = match name {
        Some(n) => {
            let p = Utf8PathBuf::from(&n);
            if p.is_absolute() {
                p
            } else {
                let cwd = std::env::current_dir()?;
                let cwd = Utf8PathBuf::from_path_buf(cwd)
                    .map_err(|_| anyhow::anyhow!("cwd not utf-8"))?;
                cwd.join(n)
            }
        }
        None => {
            let cwd = std::env::current_dir()?;
            Utf8PathBuf::from_path_buf(cwd).map_err(|_| anyhow::anyhow!("cwd not utf-8"))?
        }
    };
    init::init_project(&target, ui)?;
    Ok(())
}

// ── status ──────────────────────────────────────────────────────────────────

fn cmd_status(ui: &dyn SilUi) -> Result<()> {
    let (root, config, paths) = load_project()?;
    let structure = Structure::load(&paths.structure()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let source_count = db.source_count().map_err(|e| anyhow::anyhow!("{e}"))?;
    let parsed_count = db.parsed_count().map_err(|e| anyhow::anyhow!("{e}"))?;
    let git = git_status(&root).map_err(|e| anyhow::anyhow!("{e}"))?;
    let draft_dirty = path_has_changes(&root, rel::PAPER_DRAFT).unwrap_or(false);
    let summary = structure.completion_summary();

    ui.println("");
    ui.info(&format!("Project: {}", root));
    ui.println(&format!("  title:  {}", config.project.title));
    ui.println(&format!("  stage:  {}", config.project.stage));
    ui.println(&format!(
        "  latex:  {} → {}",
        config.latex.engine, config.latex.main
    ));
    ui.println("");
    ui.info("Sources");
    ui.println(&format!(
        "  database: {source_count} source(s), {parsed_count} parsed"
    ));
    ui.println("");
    ui.info("Structure");
    ui.println(&format!("  {}", summary));
    for sec in &structure.sections {
        ui.muted(&format!(
            "  - [{}] {} ({})",
            sec.completion, sec.id, sec.title
        ));
    }
    ui.println("");
    ui.info("Git");
    if !git.is_repo {
        ui.warn("  not a git repository");
    } else if git.clean {
        ui.success("  working tree clean");
    } else {
        ui.warn(&format!("  {} uncommitted change(s)", git.entries.len()));
        for e in git.entries.iter().take(12) {
            ui.muted(&format!("    {e}"));
        }
        if git.entries.len() > 12 {
            ui.muted(&format!("    … {} more", git.entries.len() - 12));
        }
    }
    if draft_dirty {
        ui.warn("  paper_draft.tex has uncommitted changes");
    } else {
        ui.muted("  paper_draft.tex: no uncommitted changes");
    }
    ui.println("");
    Ok(())
}

// ── parse ───────────────────────────────────────────────────────────────────

fn marker_runner() -> Result<Box<dyn MarkerRunner>> {
    if let Ok(stub) = std::env::var("SIL_MARKER_STUB") {
        return Ok(Box::new(sil_parse::StubMarkerRunner { content: stub }));
    }
    let runner = PythonMarkerRunner::discover().map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(Box::new(runner))
}

fn cmd_parse(path: Option<PathBuf>, ui: &dyn SilUi) -> Result<()> {
    let (root, config, paths) = load_project()?;
    let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let runner = marker_runner()?;
    let sources_dir = paths.sources(&config);

    let to_parse: Vec<Utf8PathBuf> = if let Some(p) = path {
        let utf = Utf8PathBuf::from_path_buf(p).map_err(|_| anyhow::anyhow!("path not utf-8"))?;
        let abs = if utf.is_absolute() {
            utf
        } else {
            let cwd = std::env::current_dir()?;
            Utf8PathBuf::from_path_buf(cwd.join(utf)).map_err(|_| anyhow::anyhow!("path not utf-8"))?
        };
        vec![abs]
    } else {
        let unparsed = list_unparsed_pdfs(&sources_dir, &db).map_err(|e| anyhow::anyhow!("{e}"))?;
        let selected = select_pdfs_interactive(&unparsed, ui).map_err(|e| anyhow::anyhow!("{e}"))?;
        selected.into_iter().map(|i| unparsed[i].clone()).collect()
    };

    if to_parse.is_empty() {
        ui.warn("Nothing to parse.");
        return Ok(());
    }

    if to_parse.len() == 1 {
        match parse_one(&to_parse[0], &db, runner.as_ref(), ui) {
            Ok(r) => {
                ui.success(&format!("Parsed {}", r.document.filename));
                let proposal = CommitProposal::new(
                    format!("Parse PDF: {}", r.document.filename),
                    SciAction::ParsePdf,
                )
                .with_body(format!("Ingested {} into SQLite + FTS5.", r.document.filename));
                print_proposal(ui, &proposal);
            }
            Err(e) => {
                bail!("{e}");
            }
        }
    } else {
        let (ok, failed, errors) = parse_many(&to_parse, &db, runner.as_ref(), ui);
        for (p, err) in &errors {
            ui.error(&format!("{}: {err}", p.file_name().unwrap_or(p.as_str())));
        }
        if ok > 0 {
            let proposal = CommitProposal::new(
                format!("Parse {ok} PDF(s)"),
                SciAction::ParsePdf,
            )
            .with_body(format!("Parsed {ok} file(s), {failed} failed."));
            print_proposal(ui, &proposal);
        }
        if failed > 0 {
            bail!("Parsed {ok} PDF(s), {failed} failed");
        }
        ui.success(&format!("Parsed {ok} PDF(s)"));
    }
    let _ = root;
    Ok(())
}

// ── search ──────────────────────────────────────────────────────────────────

fn cmd_search(query: &str, limit: usize, ui: &dyn SilUi) -> Result<()> {
    let (_root, _config, paths) = load_project()?;
    let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let hits = db.search(query, limit).map_err(|e| anyhow::anyhow!("{e}"))?;
    if hits.is_empty() {
        ui.warn(&format!("No results for “{query}”"));
        return Ok(());
    }
    ui.info(&format!("{} result(s) for “{query}”", hits.len()));
    ui.println("");
    for (i, h) in hits.iter().enumerate() {
        let title = h.title.as_deref().unwrap_or("");
        ui.println(&format!("{}. {} {}", i + 1, h.filename, title));
        ui.muted(&format!("   {}", h.snippet.replace('\n', " ")));
    }
    ui.println("");
    Ok(())
}

// ── source fetch ────────────────────────────────────────────────────────────

fn cmd_source_fetch(target: &str, no_parse: bool, ui: &dyn SilUi) -> Result<()> {
    let (root, config, paths) = load_project()?;
    let sources_dir = paths.sources(&config);
    fs::create_dir_all(sources_dir.as_str())?;

    let script = discover_download_script()?;
    let python = std::env::var("SIL_PYTHON").unwrap_or_else(|_| "python3".into());

    let mut spinner = ui.spinner(&format!("Fetching {target}"));
    let output = Command::new(&python)
        .arg(script.as_str())
        .arg(target)
        .arg(sources_dir.as_str())
        .output()
        .with_context(|| format!("failed to spawn {python} {script}"))?;
    if !output.status.success() {
        spinner.finish_error("fetch failed");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!(
            "download failed: {}\n{}",
            stderr.trim(),
            stdout.trim()
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let saved = stdout
        .lines()
        .rev()
        .find(|l| l.trim().ends_with(".pdf") || l.contains("sources/"))
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| stdout.trim().to_string());
    spinner.finish_success(&format!("Downloaded → {saved}"));

    let proposal = CommitProposal::new(format!("Fetch source: {target}"), SciAction::FetchSource)
        .with_body(format!("Saved to {saved}"));
    print_proposal(ui, &proposal);

    if !no_parse {
        // Offer parse: non-interactive auto-parse when SIL_NONINTERACTIVE / plain.
        let pdf_path = {
            let p = Utf8Path::new(saved.trim());
            if p.is_absolute() {
                p.to_path_buf()
            } else if sources_dir.join(p.file_name().unwrap_or(p.as_str())).exists() {
                sources_dir.join(p.file_name().unwrap_or(p.as_str()))
            } else {
                root.join(p)
            }
        };
        if pdf_path.exists() {
            ui.info("Parsing downloaded PDF…");
            let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;
            let runner = marker_runner()?;
            match parse_one(&pdf_path, &db, runner.as_ref(), ui) {
                Ok(r) => {
                    ui.success(&format!("Parsed {}", r.document.filename));
                    let p2 = CommitProposal::new(
                        format!("Parse PDF: {}", r.document.filename),
                        SciAction::ParsePdf,
                    );
                    print_proposal(ui, &p2);
                }
                Err(e) => ui.warn(&format!("Parse skipped/failed: {e}")),
            }
        }
    }
    Ok(())
}

fn discover_download_script() -> Result<Utf8PathBuf> {
    if let Ok(p) = std::env::var("SIL_DOWNLOAD_SCRIPT") {
        return Ok(Utf8PathBuf::from(p));
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let m = PathBuf::from(manifest);
        candidates.push(m.join("../../python/download_pdf.py"));
        candidates.push(m.join("../python/download_pdf.py"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("python/download_pdf.py"));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        candidates.push(dir.join("../../python/download_pdf.py"));
        candidates.push(dir.join("../python/download_pdf.py"));
    }
    for c in candidates {
        if c.is_file() {
            return Utf8PathBuf::from_path_buf(c)
                .map_err(|_| anyhow::anyhow!("download script path not utf-8"));
        }
    }
    bail!("could not locate python/download_pdf.py; set SIL_DOWNLOAD_SCRIPT");
}

// ── build ───────────────────────────────────────────────────────────────────

fn cmd_build(ui: &dyn SilUi) -> Result<()> {
    let (root, config, _paths) = load_project()?;
    let main = &config.latex.main;
    let engine = config.latex.engine;
    ui.info(&format!("Building {} with {}", main, engine));
    let mut spinner = ui.spinner("Compiling LaTeX…");
    match latex_build(engine, main, &root) {
        Ok(pdf) => {
            spinner.finish_success(&format!("Built {}", pdf));
            ui.success(&format!("PDF: {pdf}"));
        }
        Err(e) => {
            spinner.finish_error("build failed");
            bail!("{e}");
        }
    }
    Ok(())
}

// ── log ─────────────────────────────────────────────────────────────────────

fn cmd_log(limit: usize, sci_only: bool, ui: &dyn SilUi) -> Result<()> {
    let (root, _config, _paths) = load_project()?;
    let entries = log_entries(&root, limit, sci_only).map_err(|e| anyhow::anyhow!("{e}"))?;
    if entries.is_empty() {
        ui.warn("No matching commits.");
        return Ok(());
    }
    ui.info(&format!(
        "Git log{} (limit {limit})",
        if sci_only { " [Sci-Action]" } else { "" }
    ));
    ui.println("");
    for e in entries {
        let act = e
            .action
            .map(|a| format!("[{}] ", a.as_str()))
            .unwrap_or_default();
        ui.println(&format!("{} {}{}", e.hash, act, e.subject));
    }
    ui.println("");
    Ok(())
}

// ── context ─────────────────────────────────────────────────────────────────

fn cmd_context(flags: ContextFlags, task: Option<&str>, ui: &dyn SilUi) -> Result<()> {
    let (root, _config, paths) = load_project()?;
    let config_yaml = fs::read_to_string(paths.config().as_str())?;
    let structure_yaml = fs::read_to_string(paths.structure().as_str())?;
    let structure = Structure::load(&paths.structure()).ok();
    let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let summary = sources_summary(&db).map_err(|e| anyhow::anyhow!("{e}"))?;
    let log = log_entries(&root, 15, true).unwrap_or_default();

    let mut skills = if let Some(t) = task {
        SkillSelection::from_task(t)
    } else {
        SkillSelection::always()
    };
    skills.merge_flags(&flags);

    let input = ContextInput {
        root: &root,
        config_yaml: &config_yaml,
        structure_yaml: &structure_yaml,
        structure: structure.as_ref(),
        sources_summary: &summary,
        log_entries: &log,
        flags: &flags,
        skills,
    };
    let ctx = generate_context(&input).map_err(|e| anyhow::anyhow!("{e}"))?;
    // Context is primary payload — print plain for piping.
    println!("{ctx}");
    let _ = ui;
    Ok(())
}

// Silence unused import warnings for types re-exported for tests.
#[allow(dead_code)]
fn _stage0_markers() {
    let _ = NullUi::new();
    let _ = SilProject::new(Utf8PathBuf::from("."), Config::default());
}
