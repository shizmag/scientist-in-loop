//! MCP Action Envelope and Error Taxonomy contracts.
//!
//! Provides the core data structures for `PR-C` (MCP Action Result Envelope & Error Taxonomy)
//! including [`McpActionResult`], [`McpErrorCode`], [`McpActionStatus`], [`PreconditionResult`],
//! [`VerificationResult`], [`NextAction`], and [`CommitProposalSummary`].

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::ValidationError;

/// Canonical schema version for MCP action results.
pub const MCP_ACTION_SCHEMA_VERSION: &str = "sil.dev/mcp-action/v1";

/// Machine-readable error codes for MCP tool operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpErrorCode {
    /// Invalid arguments, parameters, or malformed input payload.
    InvalidInput,
    /// Target path is outside the allowed project root boundary.
    NotInProject,
    /// A required parameter or prerequisite file is missing.
    MissingInput,
    /// A required precondition or invariant check failed prior to execution.
    PreconditionFailed,
    /// An advisory lock or edit conflict prevents this mutation.
    ConflictDetected,
    /// Required host toolchain capability (e.g. tectonic, marker) is unavailable.
    CapabilityUnavailable,
    /// Remote network provider or academic search API is unreachable or returned an error.
    ProviderUnavailable,
    /// Requested entity (paper, citation key, TODO item, section) was not found.
    NotFound,
    /// Mutation was previously applied and repeating it is a safe no-op.
    AlreadyApplied,
    /// Postcondition verification failed after mutation was performed.
    VerificationFailed,
    /// Internal unexpected server or I/O failure.
    InternalFailure,
}

impl McpErrorCode {
    /// Canonical snake_case representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid_input",
            Self::NotInProject => "not_in_project",
            Self::MissingInput => "missing_input",
            Self::PreconditionFailed => "precondition_failed",
            Self::ConflictDetected => "conflict_detected",
            Self::CapabilityUnavailable => "capability_unavailable",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::NotFound => "not_found",
            Self::AlreadyApplied => "already_applied",
            Self::VerificationFailed => "verification_failed",
            Self::InternalFailure => "internal_failure",
        }
    }
}

impl fmt::Display for McpErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for McpErrorCode {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "invalid_input" => Ok(Self::InvalidInput),
            "not_in_project" => Ok(Self::NotInProject),
            "missing_input" => Ok(Self::MissingInput),
            "precondition_failed" => Ok(Self::PreconditionFailed),
            "conflict_detected" => Ok(Self::ConflictDetected),
            "capability_unavailable" => Ok(Self::CapabilityUnavailable),
            "provider_unavailable" => Ok(Self::ProviderUnavailable),
            "not_found" => Ok(Self::NotFound),
            "already_applied" => Ok(Self::AlreadyApplied),
            "verification_failed" => Ok(Self::VerificationFailed),
            "internal_failure" => Ok(Self::InternalFailure),
            other => Err(ValidationError::Message(format!(
                "invalid MCP error code: '{other}'"
            ))),
        }
    }
}

/// High-level execution status of an MCP action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpActionStatus {
    /// Action completed successfully.
    Success,
    /// Action was not executed because one or more preconditions failed.
    PreconditionFailed,
    /// Action was skipped because the desired state is already applied.
    AlreadyApplied,
    /// Mutation was executed and postcondition verification succeeded.
    Verified,
    /// Action execution failed or postcondition verification failed.
    Failed,
    /// Action was blocked by policy, workspace lock, or conflict.
    Blocked,
    /// Action inputs failed validation.
    InvalidInput,
}

impl McpActionStatus {
    /// Canonical snake_case representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::PreconditionFailed => "precondition_failed",
            Self::AlreadyApplied => "already_applied",
            Self::Verified => "verified",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::InvalidInput => "invalid_input",
        }
    }
}

impl fmt::Display for McpActionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for McpActionStatus {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "success" => Ok(Self::Success),
            "precondition_failed" => Ok(Self::PreconditionFailed),
            "already_applied" => Ok(Self::AlreadyApplied),
            "verified" => Ok(Self::Verified),
            "failed" => Ok(Self::Failed),
            "blocked" => Ok(Self::Blocked),
            "invalid_input" => Ok(Self::InvalidInput),
            other => Err(ValidationError::Message(format!(
                "invalid MCP action status: '{other}'"
            ))),
        }
    }
}

/// Outcome of evaluating a single precondition before action execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreconditionResult {
    /// Precondition name / identifier.
    pub name: String,
    /// Whether the precondition was satisfied.
    pub satisfied: bool,
    /// Human-readable explanation or failure reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl PreconditionResult {
    /// Construct a satisfied precondition.
    pub fn ok(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            satisfied: true,
            message: None,
        }
    }

    /// Construct a failed precondition with an explanation.
    pub fn failed(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            satisfied: false,
            message: Some(message.into()),
        }
    }
}

/// Result of verifying postconditions following an action execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether the postcondition verification passed.
    pub passed: bool,
    /// Machine-readable check code (e.g. "latex.syntax", "bib.valid").
    pub check_code: String,
    /// Summary explanation of verification outcome.
    pub summary: String,
    /// Optional structured verification details or findings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl VerificationResult {
    /// Construct a passed verification result.
    pub fn passed(check_code: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            passed: true,
            check_code: check_code.into(),
            summary: summary.into(),
            details: None,
        }
    }

    /// Construct a failed verification result.
    pub fn failed(check_code: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            passed: false,
            check_code: check_code.into(),
            summary: summary.into(),
            details: None,
        }
    }

    /// Attach structured details to the verification result.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

/// Recommended next action following this operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NextAction {
    /// Recommended action identifier (e.g. "verify", "check", "compile").
    pub action_id: String,
    /// Explanation of why this next action is suggested.
    pub reason: String,
    /// List of required input parameter keys for the action.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_inputs: Vec<String>,
}

impl NextAction {
    /// Construct a new next action recommendation.
    pub fn new(
        action_id: impl Into<String>,
        reason: impl Into<String>,
        required_inputs: Vec<String>,
    ) -> Self {
        Self {
            action_id: action_id.into(),
            reason: reason.into(),
            required_inputs,
        }
    }
}

/// Summary of a proposed Git commit / Sci-Action trailer following mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitProposalSummary {
    /// Sci-Action classification (e.g. "draft", "cite", "todo").
    pub action: String,
    /// Suggested commit subject line.
    pub subject: String,
    /// Extended commit message explanation including Sci-Action trailer.
    pub message: String,
    /// List of project-relative paths modified.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_paths: Vec<String>,
    /// Whether the mutation was actually applied to disk (false for dry-run).
    pub applied: bool,
    /// Whether postcondition verification passed.
    pub verified: bool,
}

/// Standard envelope for MCP tool execution results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpActionResult {
    /// Unique operation identifier for tracing and idempotency.
    pub operation_id: String,
    /// Envelope schema version ("sil.dev/mcp-action/v1").
    pub schema_version: String,
    /// High-level execution status.
    pub status: McpActionStatus,
    /// Machine-readable error code if operation failed or was blocked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<McpErrorCode>,
    /// Human-readable summary of outcome.
    pub summary: String,
    /// Preconditions evaluated prior to execution.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preconditions: Vec<PreconditionResult>,
    /// Project-relative paths modified by this operation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_paths: Vec<String>,
    /// Identifiers created or updated (e.g. cite keys, section IDs, source IDs).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub created_or_updated_ids: Vec<String>,
    /// Postcondition verification result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<VerificationResult>,
    /// Warnings or non-fatal observations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    /// Suggested next safe actions for the agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub next_actions: Vec<NextAction>,
    /// Proposed Git commit metadata (never auto-committed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit_proposal: Option<CommitProposalSummary>,
    /// Optional structured payload for action-specific data and backwards compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

impl McpActionResult {
    /// Create a successful action result.
    pub fn success(operation_id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            schema_version: MCP_ACTION_SCHEMA_VERSION.to_string(),
            status: McpActionStatus::Success,
            error_code: None,
            summary: summary.into(),
            preconditions: Vec::new(),
            changed_paths: Vec::new(),
            created_or_updated_ids: Vec::new(),
            verification: None,
            warnings: Vec::new(),
            next_actions: Vec::new(),
            commit_proposal: None,
            payload: None,
        }
    }

    /// Create a failed action result with an error code and summary.
    pub fn error(
        operation_id: impl Into<String>,
        error_code: McpErrorCode,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            operation_id: operation_id.into(),
            schema_version: MCP_ACTION_SCHEMA_VERSION.to_string(),
            status: McpActionStatus::Failed,
            error_code: Some(error_code),
            summary: summary.into(),
            preconditions: Vec::new(),
            changed_paths: Vec::new(),
            created_or_updated_ids: Vec::new(),
            verification: None,
            warnings: Vec::new(),
            next_actions: Vec::new(),
            commit_proposal: None,
            payload: None,
        }
    }

    /// Create a blocked action result.
    pub fn blocked(
        operation_id: impl Into<String>,
        error_code: McpErrorCode,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            operation_id: operation_id.into(),
            schema_version: MCP_ACTION_SCHEMA_VERSION.to_string(),
            status: McpActionStatus::Blocked,
            error_code: Some(error_code),
            summary: summary.into(),
            preconditions: Vec::new(),
            changed_paths: Vec::new(),
            created_or_updated_ids: Vec::new(),
            verification: None,
            warnings: Vec::new(),
            next_actions: Vec::new(),
            commit_proposal: None,
            payload: None,
        }
    }

    /// Create an already applied result (idempotent no-op).
    pub fn already_applied(operation_id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            schema_version: MCP_ACTION_SCHEMA_VERSION.to_string(),
            status: McpActionStatus::AlreadyApplied,
            error_code: None,
            summary: summary.into(),
            preconditions: Vec::new(),
            changed_paths: Vec::new(),
            created_or_updated_ids: Vec::new(),
            verification: None,
            warnings: Vec::new(),
            next_actions: Vec::new(),
            commit_proposal: None,
            payload: None,
        }
    }

    /// Create an invalid input result.
    pub fn invalid_input(operation_id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            schema_version: MCP_ACTION_SCHEMA_VERSION.to_string(),
            status: McpActionStatus::InvalidInput,
            error_code: Some(McpErrorCode::InvalidInput),
            summary: summary.into(),
            preconditions: Vec::new(),
            changed_paths: Vec::new(),
            created_or_updated_ids: Vec::new(),
            verification: None,
            warnings: Vec::new(),
            next_actions: Vec::new(),
            commit_proposal: None,
            payload: None,
        }
    }

    /// Builder method to add a precondition.
    pub fn with_precondition(mut self, precondition: PreconditionResult) -> Self {
        self.preconditions.push(precondition);
        self
    }

    /// Builder method to add a changed path.
    pub fn with_changed_path(mut self, path: impl Into<String>) -> Self {
        self.changed_paths.push(path.into());
        self
    }

    /// Builder method to add a created or updated ID.
    pub fn with_created_or_updated_id(mut self, id: impl Into<String>) -> Self {
        self.created_or_updated_ids.push(id.into());
        self
    }

    /// Builder method to set postcondition verification.
    pub fn with_verification(mut self, verification: VerificationResult) -> Self {
        if verification.passed {
            self.status = McpActionStatus::Verified;
        } else {
            self.status = McpActionStatus::Failed;
            self.error_code = Some(McpErrorCode::VerificationFailed);
        }
        self.verification = Some(verification);
        self
    }

    /// Builder method to add a warning.
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Builder method to add a next action recommendation.
    pub fn with_next_action(mut self, next: NextAction) -> Self {
        self.next_actions.push(next);
        self
    }

    /// Builder method to set a commit proposal.
    pub fn with_commit_proposal(mut self, proposal: CommitProposalSummary) -> Self {
        self.commit_proposal = Some(proposal);
        self
    }

    /// Builder method to set a typed or JSON payload.
    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = Some(payload);
        self
    }

    /// Check if the action was successful or verified.
    pub fn is_success(&self) -> bool {
        matches!(
            self.status,
            McpActionStatus::Success | McpActionStatus::Verified | McpActionStatus::AlreadyApplied
        )
    }

    /// Check if the action was blocked.
    pub fn is_blocked(&self) -> bool {
        matches!(
            self.status,
            McpActionStatus::Blocked | McpActionStatus::PreconditionFailed
        )
    }

    /// Check if the action failed.
    pub fn is_error(&self) -> bool {
        matches!(
            self.status,
            McpActionStatus::Failed | McpActionStatus::InvalidInput
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mcp_error_code_serde_and_str() {
        let codes = [
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

        for (code, expected) in codes {
            assert_eq!(code.as_str(), expected);
            assert_eq!(code.to_string(), expected);
            assert_eq!(McpErrorCode::from_str(expected).unwrap(), code);
            let json = serde_json::to_string(&code).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            let de: McpErrorCode = serde_json::from_str(&json).unwrap();
            assert_eq!(de, code);
        }
        assert!(McpErrorCode::from_str("non_existent_error").is_err());
    }

    #[test]
    fn test_mcp_action_status_serde_and_str() {
        let statuses = [
            (McpActionStatus::Success, "success"),
            (McpActionStatus::PreconditionFailed, "precondition_failed"),
            (McpActionStatus::AlreadyApplied, "already_applied"),
            (McpActionStatus::Verified, "verified"),
            (McpActionStatus::Failed, "failed"),
            (McpActionStatus::Blocked, "blocked"),
            (McpActionStatus::InvalidInput, "invalid_input"),
        ];

        for (status, expected) in statuses {
            assert_eq!(status.as_str(), expected);
            assert_eq!(status.to_string(), expected);
            assert_eq!(McpActionStatus::from_str(expected).unwrap(), status);
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            let de: McpActionStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(de, status);
        }
        assert!(McpActionStatus::from_str("non_existent_status").is_err());
    }

    #[test]
    fn test_mcp_action_result_roundtrip() {
        let result = McpActionResult::success("op-100", "Added citation successfully")
            .with_precondition(PreconditionResult::ok("file_exists"))
            .with_changed_path("references.bib")
            .with_created_or_updated_id("smith2026")
            .with_verification(VerificationResult::passed("bib.valid", "Syntax is valid"))
            .with_warning("Key was normalized")
            .with_next_action(NextAction::new(
                "compile",
                "Rebuild manuscript to verify citation",
                vec![],
            ))
            .with_commit_proposal(CommitProposalSummary {
                action: "cite".to_string(),
                subject: "Add reference for smith2026".to_string(),
                message: "Add reference for smith2026\n\nSci-Action: cite".to_string(),
                changed_paths: vec!["references.bib".to_string()],
                applied: true,
                verified: true,
            })
            .with_payload(json!({"cite_key": "smith2026"}));

        assert_eq!(result.status, McpActionStatus::Verified);
        assert!(result.is_success());
        assert!(!result.is_blocked());
        assert!(!result.is_error());

        let json = serde_json::to_string_pretty(&result).unwrap();
        let de: McpActionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, de);
    }

    #[test]
    fn test_mcp_action_result_error_and_blocked() {
        let err_res = McpActionResult::error("op-err", McpErrorCode::NotFound, "Source not found")
            .with_precondition(PreconditionResult::failed(
                "source_exists",
                "Path does not exist",
            ));
        assert_eq!(err_res.status, McpActionStatus::Failed);
        assert_eq!(err_res.error_code, Some(McpErrorCode::NotFound));
        assert!(err_res.is_error());

        let blocked_res = McpActionResult::blocked(
            "op-blk",
            McpErrorCode::ConflictDetected,
            "Workspace is locked",
        );
        assert_eq!(blocked_res.status, McpActionStatus::Blocked);
        assert_eq!(blocked_res.error_code, Some(McpErrorCode::ConflictDetected));
        assert!(blocked_res.is_blocked());
    }

    #[test]
    fn test_mcp_action_result_verification_failure() {
        let res = McpActionResult::success("op-vf", "Wrote file").with_verification(
            VerificationResult::failed("latex.syntax", "Unclosed bracket on line 12"),
        );
        assert_eq!(res.status, McpActionStatus::Failed);
        assert_eq!(res.error_code, Some(McpErrorCode::VerificationFailed));
        assert!(res.is_error());
    }
}
