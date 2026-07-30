//! `sil` — thin CLI wiring only. Domain logic lives in library crates.

use anyhow::Result;
use clap::Parser;
use sil_agent::ContextFlags;

mod cli;
mod commands;
mod init;
mod templates;
mod util;

use cli::{Cli, Commands, SourceCmd, StructureCmd};
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
        Commands::Status { json } => commands::status(json, ui.as_ref()),
        Commands::Parse { path } => commands::parse(path, ui.as_ref()),
        Commands::Source { action } => match action {
            SourceCmd::Fetch { target, no_parse } => {
                commands::source_fetch(&target, no_parse, ui.as_ref())
            }
            SourceCmd::List { json } => commands::source_list(json, ui.as_ref()),
            SourceCmd::Remove { id, delete_file } => {
                commands::source_remove(&id, delete_file, ui.as_ref())
            }
        },
        Commands::Search { query, limit } => commands::search(&query, limit, ui.as_ref()),
        Commands::Build { release } => commands::build(release, ui.as_ref()),
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
        Commands::Split => commands::split(ui.as_ref()),
        Commands::Propose {
            action,
            message,
            body,
        } => commands::propose(action.as_deref(), message.as_deref(), body.as_deref(), ui.as_ref()),
        Commands::Promote { force } => commands::promote(force, ui.as_ref()),
        Commands::Structure { action } => match action {
            StructureCmd::List => commands::structure_list(ui.as_ref()),
            StructureCmd::Set {
                section_id,
                completion,
            } => commands::structure_set(&section_id, &completion, ui.as_ref()),
        },
        Commands::Template {
            action,
            target,
            input,
            output,
        } => commands::template(action, target, input, output, ui.as_ref()),
        Commands::Cite {
            target,
            append,
            json,
        } => commands::cite(&target, append, json, ui.as_ref()),
        Commands::Doctor { json } => commands::doctor(json, ui.as_ref()),
        Commands::Settings => commands::settings(),
    }
}
