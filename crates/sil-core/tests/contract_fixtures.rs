//! Contract schema and golden fixture tests for Stage 16 Wave 0 (PR-A and PR-C).

use sil_core::agent::{
    AGENT_STATE_SCHEMA_VERSION, AgentContextEnvelope, AgentExecutionMetadata, AgentFinding,
    AgentState, AgentStateKind, AvailableAction, CapabilitySummary, HealthSummary, InputSnapshot,
    JobSummary, LiteratureSummary, LockSummary, ProjectIdentity, SectionSummaryItem,
    SelectedSkillItem, SkillSelectionSummary, SkillStatus, StructureSummary, WorkItemSummary,
    normalize_path, redact_secrets, sanitize_agent_state,
};
use sil_core::check::FindingClass;
use sil_core::mcp::{
    CommitProposalSummary, MCP_ACTION_SCHEMA_VERSION, McpActionResult, McpActionStatus,
    McpErrorCode, NextAction, PreconditionResult, VerificationResult,
};
use sil_core::{LatexEngine, PaperKind, Stage};
use std::str::FromStr;

const GOLDEN_AGENT_STATE_JSON: &str =
    include_str!("../../../tests/fixtures/pr-v/agent_state_schema.json");

#[test]
fn test_golden_agent_state_fixture_deserialization() {
    let state: AgentState =
        serde_json::from_str(GOLDEN_AGENT_STATE_JSON).expect("valid golden AgentState JSON");

    assert_eq!(state.schema_version, AGENT_STATE_SCHEMA_VERSION);
    assert_eq!(state.state, AgentStateKind::Ready);
    assert_eq!(state.project.title, "Offline Verification Fixture");
    assert_eq!(state.project.stage, Stage::Draft);
    assert_eq!(state.project.paper_kind, PaperKind::Draft);
    assert_eq!(state.project.latex_engine, LatexEngine::Tectonic);
    assert_eq!(state.project.template.as_deref(), Some("standard"));
    assert_eq!(state.project.relative_root, ".");

    assert_eq!(state.inputs.files_present.len(), 5);
    assert!(state.inputs.skill_lock_present);
    assert!(state.inputs.template_lock_present);

    assert_eq!(state.health.word_count, 32);
    assert_eq!(state.health.total_bib_keys_count, 1);
    assert_eq!(state.health.cited_bib_keys_count, 1);
    assert!(!state.health.has_errors);

    assert_eq!(state.structure.total_sections, 2);
    assert_eq!(state.structure.sections.len(), 2);
    assert_eq!(state.structure.sections[0].id, "sec.intro");
    assert_eq!(state.structure.sections[1].id, "sec.methods");

    assert_eq!(state.literature.total_sources, 1);
    assert_eq!(state.literature.parsed_sources, 1);

    assert_eq!(state.skills.active_skill_ids, vec!["SYSTEM", "paper"]);
    assert_eq!(state.skills.selected_skills.len(), 2);
    assert_eq!(state.skills.selected_skills[0].id, "SYSTEM");
    assert_eq!(
        state.skills.selected_skills[0].status,
        SkillStatus::Selected
    );

    assert!(state.capabilities.latex_available);
    assert!(state.capabilities.parser_available);
    assert!(state.capabilities.git_available);
    assert!(!state.capabilities.online_search_available);

    assert_eq!(state.actions.len(), 2);
    assert_eq!(state.actions[0].id, "check");
    assert_eq!(state.actions[1].id, "compile");
}

#[test]
fn test_golden_agent_state_roundtrip_byte_parity() {
    let state: AgentState =
        serde_json::from_str(GOLDEN_AGENT_STATE_JSON).expect("valid golden AgentState JSON");

    let serialized = serde_json::to_string_pretty(&state).expect("serialize AgentState");
    let state_reparsed: AgentState =
        serde_json::from_str(&serialized).expect("deserialize re-serialized AgentState");

    assert_eq!(state, state_reparsed);

    // Byte parity: re-serializing parsed golden json produces identical canonical value tree
    let golden_val: serde_json::Value = serde_json::from_str(GOLDEN_AGENT_STATE_JSON).unwrap();
    let re_val: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(golden_val, re_val);
}

#[test]
fn test_stable_fingerprint_isolation_from_volatile_execution() {
    let state: AgentState =
        serde_json::from_str(GOLDEN_AGENT_STATE_JSON).expect("valid golden AgentState JSON");

    let fp1 = state.stable_fingerprint();
    assert!(fp1.starts_with("sha256:"));

    let env1 = AgentContextEnvelope {
        state: state.clone(),
        execution: Some(AgentExecutionMetadata {
            checked_at: "2026-08-22T08:00:00Z".to_string(),
            duration_ms: 50,
            job_id: Some("job-101".to_string()),
            host_info: Some("darwin".to_string()),
        }),
    };

    let env2 = AgentContextEnvelope {
        state: state.clone(),
        execution: Some(AgentExecutionMetadata {
            checked_at: "2026-08-22T09:30:15Z".to_string(),
            duration_ms: 1250,
            job_id: None,
            host_info: None,
        }),
    };

    let env3 = AgentContextEnvelope {
        state: state.clone(),
        execution: None,
    };

    assert_eq!(env1.state.stable_fingerprint(), fp1);
    assert_eq!(env2.state.stable_fingerprint(), fp1);
    assert_eq!(env3.state.stable_fingerprint(), fp1);
}

#[test]
fn test_mcp_error_taxonomy_coverage() {
    let all_codes = [
        (McpErrorCode::InvalidInput, "invalid_input"),
        (McpErrorCode::NotInProject, "not_in_project"),
        (McpErrorCode::MissingInput, "missing_input"),
        (McpErrorCode::PreconditionFailed, "precondition_failed"),
        (McpErrorCode::ConflictDetected, "conflict_detected"),
        (
            McpErrorCode::CapabilityUnavailable,
            "capability_unavailable",
        ),
        (McpErrorCode::ProviderUnavailable, "provider_unavailable"),
        (McpErrorCode::NotFound, "not_found"),
        (McpErrorCode::AlreadyApplied, "already_applied"),
        (McpErrorCode::VerificationFailed, "verification_failed"),
        (McpErrorCode::InternalFailure, "internal_failure"),
    ];

    assert_eq!(all_codes.len(), 11);

    for (code, expected_str) in all_codes {
        assert_eq!(code.as_str(), expected_str);
        assert_eq!(code.to_string(), expected_str);
        assert_eq!(McpErrorCode::from_str(expected_str).unwrap(), code);

        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, format!("\"{expected_str}\""));
        let de: McpErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(de, code);
    }
}

#[test]
fn test_mcp_action_status_coverage() {
    let all_statuses = [
        (McpActionStatus::Success, "success"),
        (McpActionStatus::PreconditionFailed, "precondition_failed"),
        (McpActionStatus::AlreadyApplied, "already_applied"),
        (McpActionStatus::Verified, "verified"),
        (McpActionStatus::Failed, "failed"),
        (McpActionStatus::Blocked, "blocked"),
        (McpActionStatus::InvalidInput, "invalid_input"),
    ];

    assert_eq!(all_statuses.len(), 7);

    for (status, expected_str) in all_statuses {
        assert_eq!(status.as_str(), expected_str);
        assert_eq!(status.to_string(), expected_str);
        assert_eq!(McpActionStatus::from_str(expected_str).unwrap(), status);

        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, format!("\"{expected_str}\""));
        let de: McpActionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, status);
    }
}

#[test]
fn test_mcp_action_result_envelope_builder_and_contracts() {
    let op_id = "op-stage16-test";
    let res = McpActionResult::success(op_id, "Successfully parsed source document")
        .with_precondition(PreconditionResult::ok("source_file_exists"))
        .with_changed_path("sources/paper.pdf")
        .with_created_or_updated_id("src-123")
        .with_verification(VerificationResult::passed(
            "source.extracted",
            "Extracted 4500 characters of Markdown",
        ))
        .with_warning("Non-standard font encoding mapped")
        .with_next_action(NextAction::new(
            "cite",
            "Inspect extracted metadata and add citation",
            vec!["source_id".to_string()],
        ))
        .with_commit_proposal(CommitProposalSummary {
            action: "parse".to_string(),
            subject: "Parse source paper.pdf".to_string(),
            message: "Parse source paper.pdf\n\nSci-Action: parse".to_string(),
            changed_paths: vec!["sources/paper.pdf".to_string()],
            applied: true,
            verified: true,
        })
        .with_payload(serde_json::json!({
            "source_id": "src-123",
            "char_count": 4500
        }));

    assert_eq!(res.schema_version, MCP_ACTION_SCHEMA_VERSION);
    assert_eq!(res.operation_id, op_id);
    assert_eq!(res.status, McpActionStatus::Verified);
    assert!(res.is_success());
    assert!(!res.is_blocked());
    assert!(!res.is_error());

    let json = serde_json::to_string_pretty(&res).unwrap();
    let de: McpActionResult = serde_json::from_str(&json).unwrap();
    assert_eq!(res, de);
}

#[test]
fn test_mcp_action_result_idempotent_already_applied() {
    let res = McpActionResult::already_applied(
        "op-dup-1",
        "Citation 'vaswani2017' already present in references.bib",
    )
    .with_created_or_updated_id("vaswani2017");

    assert_eq!(res.status, McpActionStatus::AlreadyApplied);
    assert!(res.is_success());
    assert!(!res.is_error());
}

#[test]
fn test_mcp_action_result_blocked_with_precondition() {
    let res = McpActionResult::blocked(
        "op-lock-1",
        McpErrorCode::ConflictDetected,
        "Cannot mutate draft while TUI holds active lock",
    )
    .with_precondition(PreconditionResult::failed(
        "workspace_unlocked",
        "Lock held by PID 12345 (TUI session)",
    ));

    assert_eq!(res.status, McpActionStatus::Blocked);
    assert_eq!(res.error_code, Some(McpErrorCode::ConflictDetected));
    assert!(res.is_blocked());
    assert!(!res.is_success());
}

#[test]
fn test_secret_redaction_and_path_normalization() {
    let test_cases = [
        (
            "Authorization: Bearer sk-ant-api03-abcdefghijklmnopqrstuvwxyz123456",
            "Authorization: Bearer [REDACTED]",
        ),
        (
            "OPENAI_API_KEY=sk-proj-123456789012345678901234",
            "OPENAI_API_KEY=[REDACTED]",
        ),
        (
            "github_token: 'ghp_123456789012345678901234567890'",
            "github_token: '[REDACTED]'",
        ),
        (
            "password = \"super_secret_db_password_123\"",
            "password = \"[REDACTED]\"",
        ),
        (
            "Normal text in LaTeX: \\section{Introduction}",
            "Normal text in LaTeX: \\section{Introduction}",
        ),
    ];

    for (raw, expected) in test_cases {
        assert_eq!(redact_secrets(raw), expected);
    }

    assert_eq!(
        normalize_path(
            "/Users/user/project/paper_draft.tex",
            Some("/Users/user/project")
        ),
        "paper_draft.tex"
    );
    assert_eq!(
        normalize_path(
            "C:\\Users\\user\\project\\references.bib",
            Some("C:\\Users\\user\\project")
        ),
        "references.bib"
    );
    assert_eq!(
        normalize_path("./figures/chart.png", Some("/Users/user/project")),
        "figures/chart.png"
    );
}

#[test]
fn test_sanitize_agent_state_full_pass() {
    let mut state = AgentState {
        schema_version: AGENT_STATE_SCHEMA_VERSION.to_string(),
        state: AgentStateKind::Ready,
        project: ProjectIdentity {
            title: "Project with secret api_key: 'sk-123456789012345678901234'".to_string(),
            stage: Stage::Draft,
            paper_kind: PaperKind::Draft,
            latex_engine: LatexEngine::Tectonic,
            template: Some("standard".to_string()),
            relative_root: "/Users/dev/sil-project".to_string(),
        },
        inputs: InputSnapshot {
            files_present: vec![
                "/Users/dev/sil-project/config.yaml".to_string(),
                "/Users/dev/sil-project/paper_draft.tex".to_string(),
            ],
            ..Default::default()
        },
        health: HealthSummary::default(),
        structure: StructureSummary {
            title: "Structure with password=xyz123".to_string(),
            total_sections: 1,
            completed_sections: 0,
            in_progress_sections: 1,
            planned_sections: 0,
            completion_percent: 0,
            sections: vec![SectionSummaryItem {
                id: "sec.intro".to_string(),
                title: "Introduction to token: abc123456789".to_string(),
                level: 1,
                completion: "draft".to_string(),
                path: Some("/Users/dev/sil-project/sections/intro.tex".to_string()),
            }],
        },
        work_items: vec![WorkItemSummary {
            id: "todo-1".to_string(),
            kind: "todo".to_string(),
            section_id: Some("sec.intro".to_string()),
            line_start: Some(10),
            line_end: Some(12),
            content: "Fix token = 'ghp_123456789012345678901234567890' in example".to_string(),
            resolved: false,
        }],
        literature: LiteratureSummary::default(),
        skills: SkillSelectionSummary {
            active_skill_ids: vec!["paper".to_string()],
            selected_skills: vec![SelectedSkillItem {
                id: "paper".to_string(),
                version: Some("1.0.0".to_string()),
                status: SkillStatus::Selected,
                reason: Some("Selected for api_key=topsecret".to_string()),
                required_capabilities: vec![],
                conflicts: vec![],
            }],
            registry_version: None,
        },
        capabilities: CapabilitySummary::default(),
        jobs: JobSummary {
            workspace_lock: Some(LockSummary {
                locked: true,
                holder: Some("agent-secret-id-12345".to_string()),
                pid: Some(999),
                reason: Some("Running task with password=mypass".to_string()),
                stale: false,
            }),
            ..Default::default()
        },
        actions: vec![AvailableAction {
            id: "compile".to_string(),
            description: "Compile draft with secret=pass123".to_string(),
            reason: "Validate with token: 123456789".to_string(),
            safe: true,
            mutating: false,
            required_inputs: vec![],
        }],
        warnings: vec![AgentFinding {
            code: "leak".to_string(),
            class: FindingClass::ActionableWarning,
            message: "Found leak with api_key=sk-123456789012345678901234".to_string(),
            path: Some("/Users/dev/sil-project/paper_draft.tex".to_string()),
            line: Some(20),
            hint: Some("Remove secret password=1234".to_string()),
        }],
    };

    sanitize_agent_state(&mut state, Some("/Users/dev/sil-project"));

    assert_eq!(state.project.relative_root, ".");
    assert_eq!(
        state.inputs.files_present,
        vec!["config.yaml", "paper_draft.tex"]
    );
    assert_eq!(
        state.structure.sections[0].path.as_deref(),
        Some("sections/intro.tex")
    );
    assert_eq!(state.warnings[0].path.as_deref(), Some("paper_draft.tex"));

    // Assert no raw secrets remain in text fields
    assert!(!state.project.title.contains("sk-123456789012345678901234"));
    assert!(!state.structure.title.contains("xyz123"));
    assert!(
        !state.work_items[0]
            .content
            .contains("ghp_123456789012345678901234567890")
    );
    assert!(
        !state.skills.selected_skills[0]
            .reason
            .as_ref()
            .unwrap()
            .contains("topsecret")
    );
    assert!(
        !state
            .jobs
            .workspace_lock
            .as_ref()
            .unwrap()
            .reason
            .as_ref()
            .unwrap()
            .contains("mypass")
    );
    assert!(!state.actions[0].description.contains("pass123"));
    assert!(
        !state.warnings[0]
            .message
            .contains("sk-123456789012345678901234")
    );
    assert!(!state.warnings[0].hint.as_ref().unwrap().contains("1234"));
}
