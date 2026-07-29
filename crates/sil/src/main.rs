//! `sil` — thin CLI wiring only. Domain logic lives in library crates.

use anyhow::Result;
use clap::Parser;
use sil_agent::ContextFlags;

mod cli;
mod commands;
mod init;
mod templates;
mod util;

use cli::{Cli, Commands, SourceCmd};
use util::make_ui;

fn main() {
    if let Err(e) = run() {
        eprintln!("✖ {e}");
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
        Commands::Init { name, update } => commands::init(name, update, ui.as_ref()),
        Commands::Status => commands::status(ui.as_ref()),
        Commands::Parse { path } => commands::parse(path, ui.as_ref()),
        Commands::Source {
            action: SourceCmd::Fetch { target, no_parse },
        } => commands::source_fetch(&target, no_parse, ui.as_ref()),
        Commands::Search { query, limit } => commands::search(&query, limit, ui.as_ref()),
        Commands::Build => commands::build(ui.as_ref()),
        Commands::Log {
            limit,
            sci_only,
            all,
        } => commands::log(limit, if all { false } else { sci_only }, ui.as_ref()),
        Commands::Context {
            paper,
            agent,
            skill_paper,
            skill_agent_code,
            skills,
            task,
        } => commands::context(
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
