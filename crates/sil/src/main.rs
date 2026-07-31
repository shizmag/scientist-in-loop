//! `sil` — thin CLI wiring only. Domain logic lives in library crates.

use anyhow::Result;
use clap::Parser;
use sil_agent::ContextFlags;

mod cli;
mod commands;
mod init;
mod templates;
mod util;

use cli::{Cli, Commands, GitCmd, PaperCmd, ProjectCmd, SourceCmd, StructureCmd};
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
        Commands::Source { action } => match action {
            SourceCmd::Fetch { target, no_parse } => {
                commands::source_fetch(&target, no_parse, ui.as_ref())
            }
            SourceCmd::List { json } => commands::source_list(json, ui.as_ref()),
            SourceCmd::Remove { id, delete_file } => {
                commands::source_remove(&id, delete_file, ui.as_ref())
            }
            SourceCmd::Parse { path } => commands::parse(path, ui.as_ref()),
            SourceCmd::Search { query, limit } => commands::search(&query, limit, ui.as_ref()),
            SourceCmd::Cite {
                target,
                append,
                json,
            } => commands::cite(&target, append, json, ui.as_ref()),
            SourceCmd::Digest { query, limit } => commands::digest(&query, limit, ui.as_ref()),
            SourceCmd::Read { id } => commands::source_read(&id, ui.as_ref()),
            SourceCmd::Doctor { id } => commands::source_doctor(id, ui.as_ref()),
        },
        Commands::Paper { action } => match action {
            PaperCmd::Build { target, release } => commands::build(target, release, ui.as_ref()),
            PaperCmd::Split => commands::split(ui.as_ref()),
            PaperCmd::Promote { force } => commands::promote(force, ui.as_ref()),
            PaperCmd::Todo { json } => commands::todo(json, ui.as_ref()),
            PaperCmd::Structure { action } => match action {
                StructureCmd::List => commands::structure_list(ui.as_ref()),
                StructureCmd::Set {
                    section_id,
                    completion,
                } => commands::structure_set(&section_id, &completion, ui.as_ref()),
            },
            PaperCmd::Template {
                action,
                target,
                input,
                output,
            } => commands::template(action, target, input, output, ui.as_ref()),
        },
        Commands::Project { action } => match action {
            ProjectCmd::Doctor { json } => commands::doctor(json, ui.as_ref()),
            ProjectCmd::Context {
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
            ProjectCmd::Mcp { quiet } => commands::mcp(quiet),
        },
        Commands::Git { action } => match action {
            GitCmd::Log {
                limit,
                sci_only,
                all,
            } => commands::log(limit, if all { false } else { sci_only }, ui.as_ref()),
            GitCmd::Propose {
                action,
                message,
                body,
            } => commands::propose(
                action.as_deref(),
                message.as_deref(),
                body.as_deref(),
                ui.as_ref(),
            ),
        },
        Commands::Tui { action: _ } => commands::settings(),
    }
}
