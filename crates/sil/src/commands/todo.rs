//! `sil todo` — list and parse active `# -- X -- #` idea and TODO blocks from paper_draft.tex.

use anyhow::Result;
use sil_core::SilUi;
use sil_latex::parse_idea_blocks;

use crate::util::load_project;

/// List active `# -- X -- #` idea and TODO blocks.
pub fn run(json: bool, ui: &dyn SilUi) -> Result<()> {
    let (_root, _config, paths) = load_project()?;
    let draft_path = paths.paper_draft();

    if !draft_path.exists() {
        ui.warn(&format!("Main draft file {draft_path} not found."));
        return Ok(());
    }

    let tex_content = std::fs::read_to_string(draft_path)?;
    let ideas = parse_idea_blocks(&tex_content);

    // Save to SQLite
    if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
        let _ = db.replace_todo_ideas(&ideas);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&ideas)?);
        return Ok(());
    }

    ui.println("");
    ui.info(&format!(
        "Active `# -- X -- #` Idea & TODO Blocks ({})",
        ideas.len()
    ));
    ui.println("─────────────────────────────────────────────────────────────");

    if ideas.is_empty() {
        ui.println("No `# -- X -- #` blocks found in paper_draft.tex.");
        ui.println("Tip: Surround notes or ideas with `# -- X -- #` or `% # -- X -- #` in paper_draft.tex.");
        return Ok(());
    }

    for (idx, idea) in ideas.iter().enumerate() {
        let sec = idea.section_id.as_deref().unwrap_or("General");
        ui.println(&format!(
            "{}. [Lines {}-{}] ({sec})",
            idx + 1,
            idea.line_start,
            idea.line_end
        ));
        for line in idea.content.lines() {
            ui.println(&format!("   {line}"));
        }
        ui.println("");
    }

    Ok(())
}
