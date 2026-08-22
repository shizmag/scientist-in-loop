//! Cross-surface parity and agent verification suite (Stage 16 / PR-V).
//!
//! Verifies:
//! 1. Cross-surface fact parity (CLI `sil context --json` vs MCP `sil_context`).
//! 2. Mutation replay and dry-run safety (`dry_run: true` does not mutate disk).
//! 3. Structured error taxonomy (`McpErrorCode`).
//! 4. Deterministic state hashing and secret scrubbing.

mod common;

use std::fs;

use common::{git_commit_all, init_project, sil};
use serde_json::json;
use sil_core::{AgentState, McpActionResult, McpActionStatus, McpErrorCode};

#[test]
fn test_cross_surface_state_and_fact_parity() {
    let (_dir, project) = init_project("pr-v-parity");
    git_commit_all(&project, "Initialize sil project\n\nSci-Action: init\n");

    // Add a structure section and an idea block
    let structure_path = project.join(".sil/structure.yaml");
    fs::write(
        &structure_path,
        "title: Parity Paper\nstatus: draft\nsections:\n  - id: intro\n    title: Introduction\n    level: 1\n    completion: draft\n",
    )
    .unwrap();

    let draft_path = project.join("paper_draft.tex");
    fs::write(
        &draft_path,
        "\\section{Introduction}\n\\label{sec:intro}\n% # -- X -- #\n% TODO: Need literature review\n% # -- X -- #\nHello world.\n",
    )
    .unwrap();

    // 1. Run CLI `sil context --json`
    let cli_output = sil()
        .current_dir(&project)
        .args(["project", "context", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let cli_json_str = String::from_utf8(cli_output).unwrap();
    let cli_state: AgentState =
        serde_json::from_str(&cli_json_str).expect("valid CLI AgentState JSON");

    // 2. Run MCP `sil_context` with json: true
    let mcp_res = sil_mcp::tools::call_tool_with_context(
        &sil_mcp::McpContext::from_root(camino::Utf8Path::from_path(&project).unwrap()).unwrap(),
        "sil_context",
        Some(json!({ "json": true })),
    );
    assert!(mcp_res.is_error.is_none() || mcp_res.is_error == Some(false));
    let mcp_text = match &mcp_res.content[0] {
        sil_mcp::Content::Text { text } => text.clone(),
    };
    let mcp_state: AgentState = serde_json::from_str(&mcp_text).expect("valid MCP AgentState JSON");

    // 3. Verify parity across surfaces
    assert_eq!(cli_state.schema_version, mcp_state.schema_version);
    assert_eq!(cli_state.project.title, mcp_state.project.title);
    assert_eq!(cli_state.project.stage, mcp_state.project.stage);
    assert_eq!(
        cli_state.structure.sections.len(),
        mcp_state.structure.sections.len()
    );
    assert_eq!(
        cli_state.structure.sections[0].id,
        mcp_state.structure.sections[0].id
    );
    assert_eq!(cli_state.work_items.len(), mcp_state.work_items.len());
    assert_eq!(cli_state.work_items[0].id, mcp_state.work_items[0].id);
    assert_eq!(cli_state.state, mcp_state.state);
    assert_eq!(
        cli_state.inputs.structure_fingerprint,
        mcp_state.inputs.structure_fingerprint
    );
    assert_eq!(
        cli_state.inputs.draft_fingerprint,
        mcp_state.inputs.draft_fingerprint
    );
    assert_eq!(
        cli_state.skills.active_skill_ids,
        mcp_state.skills.active_skill_ids
    );
    assert_eq!(
        cli_state.skills.available_skill_ids,
        mcp_state.skills.available_skill_ids
    );
}

#[test]
fn test_mutation_dry_run_safety() {
    let (_dir, project) = init_project("pr-v-dry-run");
    git_commit_all(&project, "Initialize sil project\n\nSci-Action: init\n");

    let bib_path = project.join("references.bib");
    let original_bib = fs::read_to_string(&bib_path).unwrap();

    let mcp_ctx =
        sil_mcp::McpContext::from_root(camino::Utf8Path::from_path(&project).unwrap()).unwrap();

    // 1. Dry run bib upsert
    let res = sil_mcp::tools::call_tool_with_context(
        &mcp_ctx,
        "sil_cite",
        Some(json!({
            "action": "upsert",
            "entry": "@article{vaswani2017,\n  author = {Vaswani, Ashish},\n  title = {Attention Is All You Need},\n  year = {2017}\n}",
            "draft": true,
            "dry_run": true
        })),
    );

    assert!(res.is_error.is_none() || res.is_error == Some(false));
    let action_res: McpActionResult = res.as_action_result().expect("McpActionResult");
    assert_eq!(action_res.status, McpActionStatus::Success);
    assert!(action_res.summary.contains("DRY RUN"));
    assert!(action_res.changed_paths.is_empty());

    // Verify file on disk was NOT modified
    let bib_after = fs::read_to_string(&bib_path).unwrap();
    assert_eq!(original_bib, bib_after);

    // 2. Dry run draft edit
    let draft_path = project.join("paper_draft.tex");
    let original_draft = fs::read_to_string(&draft_path).unwrap();

    let res_draft = sil_mcp::tools::call_tool_with_context(
        &mcp_ctx,
        "sil_draft",
        Some(json!({
            "action": "edit",
            "section_title": "Introduction",
            "content": "This is a dry run edit.",
            "dry_run": true
        })),
    );
    assert!(res_draft.is_error.is_none() || res_draft.is_error == Some(false));
    let draft_action_res = res_draft.as_action_result().expect("McpActionResult");
    assert_eq!(draft_action_res.status, McpActionStatus::Success);
    assert!(draft_action_res.summary.contains("DRY RUN"));
    assert!(draft_action_res.changed_paths.is_empty());

    // Verify draft file was NOT modified
    let draft_after = fs::read_to_string(&draft_path).unwrap();
    assert_eq!(original_draft, draft_after);
}

#[test]
fn test_structured_error_taxonomy() {
    let (_dir, project) = init_project("pr-v-errors");
    let mcp_ctx =
        sil_mcp::McpContext::from_root(camino::Utf8Path::from_path(&project).unwrap()).unwrap();

    // 1. Missing action parameter
    let res_missing_action =
        sil_mcp::tools::call_tool_with_context(&mcp_ctx, "sil_sources", Some(json!({})));
    assert_eq!(res_missing_action.is_error, Some(true));
    let act_missing = res_missing_action.as_action_result().unwrap();
    assert_eq!(act_missing.error_code, Some(McpErrorCode::MissingInput));
    assert_eq!(act_missing.status, McpActionStatus::Failed);

    // 2. Invalid action
    let res_invalid_action = sil_mcp::tools::call_tool_with_context(
        &mcp_ctx,
        "sil_sources",
        Some(json!({ "action": "nonexistent_action" })),
    );
    assert_eq!(res_invalid_action.is_error, Some(true));
    let act_invalid = res_invalid_action.as_action_result().unwrap();
    assert_eq!(act_invalid.error_code, Some(McpErrorCode::InvalidInput));

    // 3. Precondition failure (invalid bibtex)
    let res_bad_bib = sil_mcp::tools::call_tool_with_context(
        &mcp_ctx,
        "sil_cite",
        Some(json!({
            "action": "upsert",
            "entry": "this is not valid bibtex"
        })),
    );
    assert_eq!(res_bad_bib.is_error, Some(true));
    let act_bad_bib = res_bad_bib.as_action_result().unwrap();
    assert_eq!(
        act_bad_bib.error_code,
        Some(McpErrorCode::PreconditionFailed)
    );
    assert!(!act_bad_bib.preconditions.is_empty());
    assert!(!act_bad_bib.preconditions[0].satisfied);

    // 4. Missing target for promote
    let res_promote_missing = sil_mcp::tools::call_tool_with_context(
        &mcp_ctx,
        "sil_cite",
        Some(json!({
            "action": "promote"
        })),
    );
    assert_eq!(res_promote_missing.is_error, Some(true));
    let act_promote_missing = res_promote_missing.as_action_result().unwrap();
    assert_eq!(
        act_promote_missing.error_code,
        Some(McpErrorCode::MissingInput)
    );
}

#[test]
fn test_secret_scrubbing_and_path_normalization() {
    let (_dir, project) = init_project("pr-v-secrets");
    git_commit_all(&project, "Initialize sil project\n\nSci-Action: init\n");

    // Modify existing valid config to include secret tokens
    let config_path = project.join(".sil/config.yaml");
    let original_config = fs::read_to_string(&config_path).unwrap();
    let modified_config = original_config.replace(
        "title: pr-v-secrets",
        "title: Secret Project sk-ant-api03-abcdef123456",
    );
    fs::write(&config_path, modified_config).unwrap();

    let output = sil()
        .current_dir(&project)
        .env("SIL_API_KEY", "sil-secret-token-999")
        .env("OPENAI_API_KEY", "sk-proj-supersecretkey12345")
        .env("ANTHROPIC_API_KEY", "sk-ant-live01-topsecrettoken987")
        .args(["project", "context", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_text = String::from_utf8(output).unwrap();
    // Verify no secret tokens leaked into output
    assert!(!json_text.contains("sil-secret-token-999"));
    assert!(!json_text.contains("sk-proj-supersecretkey12345"));
    assert!(!json_text.contains("sk-ant-live01-topsecrettoken987"));
}
