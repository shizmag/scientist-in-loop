//! Agent state contract and deterministic context types.
//!
//! Provides the core data structures for `PR-A` (Agent State contract)
//! including [`AgentState`], [`AgentStateKind`], summaries of health, structure,
//! work items, literature, skills, capabilities, jobs, actions, and findings.
//! Also provides stable fingerprinting, secret redaction, and path normalization.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::check::{CheckFinding, FindingClass, serialized_input_fingerprint};
use crate::error::ValidationError;
use crate::health::ManuscriptHealthReport;
use crate::stage::Stage;
use crate::structure::Structure;
use crate::types::{LatexEngine, PaperKind};

/// Canonical schema version for AgentState.
pub const AGENT_STATE_SCHEMA_VERSION: &str = "sil.dev/agent-state/v1";

/// High-level lifecycle state machine of an agent in a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentStateKind {
    /// Required inputs exist and at least one safe action is available.
    #[default]
    Ready,
    /// A human choice or missing argument is required.
    NeedsInput,
    /// A precondition, invariant, lock, or capability prevents the action.
    Blocked,
    /// A mutation completed but verification is still pending.
    Changed,
    /// The requested operation completed and its postcondition passed.
    Verified,
    /// The operation did not complete; details and retry guidance are present.
    Failed,
    /// The state was computed before a relevant file/job/config change.
    Stale,
}

impl AgentStateKind {
    /// String representation in snake_case.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::NeedsInput => "needs_input",
            Self::Blocked => "blocked",
            Self::Changed => "changed",
            Self::Verified => "verified",
            Self::Failed => "failed",
            Self::Stale => "stale",
        }
    }
}

impl fmt::Display for AgentStateKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AgentStateKind {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ready" => Ok(Self::Ready),
            "needs_input" => Ok(Self::NeedsInput),
            "blocked" => Ok(Self::Blocked),
            "changed" => Ok(Self::Changed),
            "verified" => Ok(Self::Verified),
            "failed" => Ok(Self::Failed),
            "stale" => Ok(Self::Stale),
            other => Err(ValidationError::Message(format!(
                "invalid agent state kind: '{other}'"
            ))),
        }
    }
}

/// Stable identity and configuration facts for a project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProjectIdentity {
    /// Human-readable title of the manuscript/project.
    #[serde(default)]
    pub title: String,
    /// Current lifecycle stage (e.g. draft, prep, review, final).
    #[serde(default)]
    pub stage: Stage,
    /// Paper artifact addressed (draft or final).
    #[serde(default)]
    pub paper_kind: PaperKind,
    /// LaTeX engine configured.
    #[serde(default)]
    pub latex_engine: LatexEngine,
    /// Target article template (e.g. "standard", "neurips").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Normalized project-relative root directory (e.g. ".").
    #[serde(default)]
    pub relative_root: String,
}

/// Fingerprints and presence flags for canonical source-of-truth files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InputSnapshot {
    /// SHA-256 fingerprint of config.yaml.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_fingerprint: Option<String>,
    /// SHA-256 fingerprint of structure.yaml.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structure_fingerprint: Option<String>,
    /// SHA-256 fingerprint of paper_draft.tex.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_fingerprint: Option<String>,
    /// SHA-256 fingerprint of references.bib.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bib_fingerprint: Option<String>,
    /// Normalized relative paths of files detected in project.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files_present: Vec<String>,
    /// Count of source files in `sources/`.
    #[serde(default)]
    pub sources_count: usize,
    /// Count of parsed source files.
    #[serde(default)]
    pub parsed_sources_count: usize,
    /// Whether a skill lock file (`skill.lock`) is present.
    #[serde(default)]
    pub skill_lock_present: bool,
    /// Whether a template lock file (`template.lock`) is present.
    #[serde(default)]
    pub template_lock_present: bool,
}

/// Compact deterministic summary of manuscript health and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HealthSummary {
    /// Total word count of the manuscript draft prose.
    #[serde(default)]
    pub word_count: usize,
    /// Number of missing citations detected.
    #[serde(default)]
    pub missing_citations_count: usize,
    /// Number of unreferenced figure/table labels.
    #[serde(default)]
    pub unreferenced_labels_count: usize,
    /// Number of active TODO/idea blocks.
    #[serde(default)]
    pub todo_ideas_count: usize,
    /// Total bibliography keys in references.bib.
    #[serde(default)]
    pub total_bib_keys_count: usize,
    /// Number of bibliography keys actually cited in manuscript.
    #[serde(default)]
    pub cited_bib_keys_count: usize,
    /// Whether any critical errors are present in diagnostics.
    #[serde(default)]
    pub has_errors: bool,
    /// Number of diagnostic warnings.
    #[serde(default)]
    pub warning_count: usize,
    /// Number of diagnostic errors.
    #[serde(default)]
    pub error_count: usize,
}

impl From<&ManuscriptHealthReport> for HealthSummary {
    fn from(r: &ManuscriptHealthReport) -> Self {
        let errs = r
            .diagnostics
            .iter()
            .filter(|d| d.level == crate::health::DiagnosticLevel::Error)
            .count();
        Self {
            word_count: r.word_count,
            missing_citations_count: r.missing_citations_count,
            unreferenced_labels_count: r.unreferenced_labels_count,
            todo_ideas_count: r.todo_ideas_count,
            total_bib_keys_count: r.total_bib_keys_count,
            cited_bib_keys_count: r.cited_bib_keys_count,
            has_errors: r.has_errors(),
            warning_count: r.warning_count(),
            error_count: errs,
        }
    }
}

/// Summary of project structure and section completion progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StructureSummary {
    /// Human-readable title from structure.yaml.
    #[serde(default)]
    pub title: String,
    /// Total sections declared in structure.yaml.
    #[serde(default)]
    pub total_sections: usize,
    /// Count of sections marked completed.
    #[serde(default)]
    pub completed_sections: usize,
    /// Count of sections currently in progress.
    #[serde(default)]
    pub in_progress_sections: usize,
    /// Count of sections planned (not started).
    #[serde(default)]
    pub planned_sections: usize,
    /// Overall completion percentage (0-100).
    #[serde(default)]
    pub completion_percent: u32,
    /// List of section items with completion status.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<SectionSummaryItem>,
}

impl From<&Structure> for StructureSummary {
    fn from(st: &Structure) -> Self {
        let summary = st.completion_summary();
        let total = summary.total;
        let completed = summary.polished;
        let in_progress = summary.outline + summary.draft;
        let planned = summary.empty;
        let percent = if total == 0 {
            0
        } else {
            (((completed as f64 + (in_progress as f64 * 0.5)) / total as f64) * 100.0).round()
                as u32
        };
        let items = st
            .sections
            .iter()
            .map(|s| SectionSummaryItem {
                id: s.id.clone(),
                title: s.title.clone(),
                level: s.level as usize,
                completion: s.completion.as_str().to_string(),
                path: None,
            })
            .collect();
        Self {
            title: st.title.clone(),
            total_sections: total,
            completed_sections: completed,
            in_progress_sections: in_progress,
            planned_sections: planned,
            completion_percent: percent,
            sections: items,
        }
    }
}

/// Brief status of an individual section in structure.yaml.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionSummaryItem {
    /// Unique section ID.
    pub id: String,
    /// Section title.
    pub title: String,
    /// Hierarchy depth level (1-indexed).
    pub level: usize,
    /// Completion state string ("completed", "in_progress", "planned").
    pub completion: String,
    /// Optional relative path to subsection file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Summary of a single work item (TODO item, idea block, or review item).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItemSummary {
    /// Stable work item identifier.
    pub id: String,
    /// Item classification (e.g. "todo", "idea", "action").
    pub kind: String,
    /// Associated section ID if located within a section.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_id: Option<String>,
    /// Starting line in source file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_start: Option<usize>,
    /// Ending line in source file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_end: Option<usize>,
    /// Item content or instruction text.
    pub content: String,
    /// Whether the work item is resolved.
    #[serde(default)]
    pub resolved: bool,
}

/// Summary of sources, indexed papers, and bibliography statistics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LiteratureSummary {
    /// Total source files tracked in database or filesystem.
    #[serde(default)]
    pub total_sources: usize,
    /// Number of source files successfully parsed into text/markdown.
    #[serde(default)]
    pub parsed_sources: usize,
    /// Number of unparsed source files awaiting extraction.
    #[serde(default)]
    pub unparsed_sources: usize,
    /// Total unique cite keys in references.bib.
    #[serde(default)]
    pub total_bib_keys: usize,
    /// Count of bibliography keys cited in manuscript.
    #[serde(default)]
    pub cited_bib_keys: usize,
    /// Count of bibliography keys unmentioned in manuscript.
    #[serde(default)]
    pub unmentioned_bib_keys: usize,
    /// Number of candidate publications from recent discovery/digest.
    #[serde(default)]
    pub recent_candidates_count: usize,
}

/// Status of a skill in the context of the current project state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillStatus {
    /// Selected and active for the current task/context.
    Selected,
    /// Available in registry/workspace but not selected.
    Available,
    /// Required or requested skill is missing from workspace.
    Missing,
    /// Incompatible with other selected skills or project stage.
    Incompatible,
    /// Unsupported by host platform or missing required host tools.
    Unsupported,
}

impl SkillStatus {
    /// String representation in snake_case.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Selected => "selected",
            Self::Available => "available",
            Self::Missing => "missing",
            Self::Incompatible => "incompatible",
            Self::Unsupported => "unsupported",
        }
    }
}

impl fmt::Display for SkillStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An individual skill considered during skill selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedSkillItem {
    /// Stable skill identifier (e.g. "SYSTEM", "paper", "review", "visualize-article").
    pub id: String,
    /// Pack or skill version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Selection status.
    pub status: SkillStatus,
    /// Explanation for why the skill was selected, rejected, or unsupported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Project-relative managed or local skill path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Capabilities required by this skill.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_capabilities: Vec<String>,
    /// Conflicting skill IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
}

impl SelectedSkillItem {
    /// Create a new SelectedSkillItem.
    pub fn new(id: impl Into<String>, status: SkillStatus) -> Self {
        Self {
            id: id.into(),
            version: None,
            status,
            reason: None,
            path: None,
            required_capabilities: Vec::new(),
            conflicts: Vec::new(),
        }
    }

    /// Set version.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set reason.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Set path.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set required capabilities.
    pub fn with_required_capabilities(mut self, caps: Vec<String>) -> Self {
        self.required_capabilities = caps;
        self
    }

    /// Set conflicts.
    pub fn with_conflicts(mut self, conflicts: Vec<String>) -> Self {
        self.conflicts = conflicts;
        self
    }
}

/// Summary of selected and available agent skills.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SkillSelectionSummary {
    /// Active skill IDs loaded into context.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_skill_ids: Vec<String>,
    /// Available skill IDs in workspace or registry.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub available_skill_ids: Vec<String>,
    /// All evaluated skills with status and selection reasons.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_skills: Vec<SelectedSkillItem>,
    /// Any conflicting skills detected.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
    /// Missing requirements across evaluated skills.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_requirements: Vec<String>,
    /// Version of the skill registry or pack schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_version: Option<String>,
}

/// Available toolchain capabilities on the current host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CapabilitySummary {
    /// Whether LaTeX compiler (tectonic, pdflatex, etc.) is available.
    #[serde(default)]
    pub latex_available: bool,
    /// Whether PDF parser (marker) is available.
    #[serde(default)]
    pub parser_available: bool,
    /// Whether git CLI is available.
    #[serde(default)]
    pub git_available: bool,
    /// Whether online academic search providers are reachable/configured.
    #[serde(default)]
    pub online_search_available: bool,
    /// Whether an LLM provider / embedding model is available.
    #[serde(default)]
    pub llm_provider_available: bool,
    /// List of supported MCP/CLI action IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_actions: Vec<String>,
}

/// Summary of advisory workspace lock status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LockSummary {
    /// Whether the lock file exists and is currently active.
    #[serde(default)]
    pub locked: bool,
    /// Identifier of the lock holder (e.g. "agent", "tui", "cli").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holder: Option<String>,
    /// Process ID holding the lock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// Purpose/reason for taking the lock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Whether the lock is considered stale (e.g. process is dead).
    #[serde(default)]
    pub stale: bool,
}

/// Summary of background/asynchronous jobs and workspace locking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct JobSummary {
    /// Count of pending jobs in queue.
    #[serde(default)]
    pub pending_jobs_count: usize,
    /// Count of currently running jobs.
    #[serde(default)]
    pub running_jobs_count: usize,
    /// Count of failed jobs.
    #[serde(default)]
    pub failed_jobs_count: usize,
    /// Active job identifier if one is running.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_job_id: Option<String>,
    /// Advisory workspace lock state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_lock: Option<LockSummary>,
}

/// An action that an agent can safely or conditionally execute in the workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvailableAction {
    /// Action identifier (e.g. "check", "compile", "upsert_bib", "parse_source").
    pub id: String,
    /// Human-readable description of what this action does.
    pub description: String,
    /// Reason explaining why this action is currently recommended or available.
    pub reason: String,
    /// Whether the action is read-only / free of side effects.
    pub safe: bool,
    /// Whether the action mutates project files or workspace state.
    pub mutating: bool,
    /// Required input parameters/keys for executing this action.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_inputs: Vec<String>,
}

/// A finding or warning surfaced in agent context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentFinding {
    /// Stable finding code (e.g. "latex.citation.undefined").
    pub code: String,
    /// Finding severity / policy class.
    pub class: FindingClass,
    /// Human-readable explanation.
    pub message: String,
    /// Project-relative path when the finding is located in a file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// 1-indexed source line number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Optional remediation suggestion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl From<&CheckFinding> for AgentFinding {
    fn from(f: &CheckFinding) -> Self {
        Self {
            code: f.code.clone(),
            class: f.class,
            message: f.message.clone(),
            path: f.path.clone(),
            line: f.line,
            hint: f.hint.clone(),
        }
    }
}

/// Root deterministic state contract for agent context inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentState {
    /// Schema version identifier ("sil.dev/agent-state/v1").
    pub schema_version: String,
    /// High-level lifecycle state.
    pub state: AgentStateKind,
    /// Project identity and configuration facts.
    pub project: ProjectIdentity,
    /// Inputs and content fingerprints.
    pub inputs: InputSnapshot,
    /// Manuscript health summary.
    pub health: HealthSummary,
    /// Structure and section progress summary.
    pub structure: StructureSummary,
    /// Open work items, ideas, and TODOs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub work_items: Vec<WorkItemSummary>,
    /// Literature, citations, and source status.
    pub literature: LiteratureSummary,
    /// Selected skills and routing rationale.
    pub skills: SkillSelectionSummary,
    /// Host capabilities.
    pub capabilities: CapabilitySummary,
    /// Background jobs and lock status.
    pub jobs: JobSummary,
    /// Safe next actions available to the agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<AvailableAction>,
    /// Warnings and actionable findings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<AgentFinding>,
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            schema_version: AGENT_STATE_SCHEMA_VERSION.to_string(),
            state: AgentStateKind::Ready,
            project: ProjectIdentity::default(),
            inputs: InputSnapshot::default(),
            health: HealthSummary::default(),
            structure: StructureSummary::default(),
            work_items: Vec::new(),
            literature: LiteratureSummary::default(),
            skills: SkillSelectionSummary::default(),
            capabilities: CapabilitySummary::default(),
            jobs: JobSummary::default(),
            actions: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

impl AgentState {
    /// Compute a deterministic SHA-256 fingerprint over canonical serialized JSON.
    ///
    /// Excludes any volatile execution fields by operating directly on the canonical `AgentState`.
    pub fn stable_fingerprint(&self) -> String {
        serialized_input_fingerprint(self).unwrap_or_else(|_| "sha256:unknown".to_string())
    }
}

/// Volatile execution metadata recorded during context generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AgentExecutionMetadata {
    /// UTC ISO-8601 timestamp when the state was evaluated.
    pub checked_at: String,
    /// Wall-clock evaluation duration in milliseconds.
    pub duration_ms: u64,
    /// Active job identifier, if evaluated in the context of a job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    /// Optional host platform information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_info: Option<String>,
}

/// Context envelope packaging canonical deterministic `state` with volatile `execution` metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentContextEnvelope {
    /// Canonical deterministic agent state.
    pub state: AgentState,
    /// Volatile execution metadata (excluded from stable fingerprint).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<AgentExecutionMetadata>,
}

/// Type alias for AgentContextEnvelope.
pub type AgentStateReport = AgentContextEnvelope;

/// Normalize a file path for deterministic context serialization:
/// - Replaces Windows backslashes with `/`
/// - Strips absolute host prefixes if `project_root` is provided
/// - Strips leading `./` or redundant slashes
/// - Returns normalized project-relative path
pub fn normalize_path(path: &str, project_root: Option<&str>) -> String {
    let mut normalized = path.replace('\\', "/");
    if let Some(root) = project_root {
        let root_norm = root.replace('\\', "/");
        let root_trimmed = root_norm.trim_end_matches('/');
        if !root_trimmed.is_empty() && normalized.starts_with(root_trimmed) {
            normalized = normalized[root_trimmed.len()..].to_string();
        }
    }
    let trimmed = normalized.trim_start_matches('/');
    let trimmed = trimmed.strip_prefix("./").unwrap_or(trimmed);
    if trimmed.is_empty() {
        ".".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Redact sensitive secrets (e.g. API keys, bearer tokens, passwords) from text.
pub fn redact_secrets(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(&_ch) = chars.peek() {
        let remaining: String = chars.clone().take(40).collect();
        let remaining_lower = remaining.to_ascii_lowercase();

        // 1. Bearer tokens
        if remaining_lower.starts_with("bearer ") {
            for _ in 0..7 {
                result.push(chars.next().unwrap());
            }
            while let Some(&c) = chars.peek() {
                if c == ' ' || c == '\t' {
                    result.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            let mut token = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric()
                    || c == '_'
                    || c == '-'
                    || c == '.'
                    || c == '~'
                    || c == '+'
                    || c == '/'
                    || c == '='
                {
                    token.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            if token.len() >= 10 {
                result.push_str("[REDACTED]");
            } else {
                result.push_str(&token);
            }
            continue;
        }

        // 2. Basic auth
        if remaining_lower.starts_with("basic ") {
            for _ in 0..6 {
                result.push(chars.next().unwrap());
            }
            while let Some(&c) = chars.peek() {
                if c == ' ' || c == '\t' {
                    result.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            let mut token = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '+' || c == '/' || c == '=' {
                    token.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            if token.len() >= 10 {
                result.push_str("[REDACTED]");
            } else {
                result.push_str(&token);
            }
            continue;
        }

        // 3. Standalone known key patterns: sk-, ghp_, gho_, github_pat_, AKIA
        if remaining.starts_with("sk-")
            || remaining.starts_with("ghp_")
            || remaining.starts_with("gho_")
            || remaining.starts_with("github_pat_")
            || remaining.starts_with("AKIA")
        {
            let mut token = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '_' || c == '-' {
                    token.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            if (token.starts_with("sk-") && token.len() >= 20)
                || (token.starts_with("ghp_") && token.len() >= 20)
                || (token.starts_with("gho_") && token.len() >= 20)
                || (token.starts_with("github_pat_") && token.len() >= 20)
                || (token.starts_with("AKIA") && token.len() == 20)
            {
                result.push_str("[REDACTED]");
            } else {
                result.push_str(&token);
            }
            continue;
        }

        // 4. Key-value pairs
        let kv_keys = [
            "api_key",
            "apikey",
            "api-key",
            "access_token",
            "auth_token",
            "token",
            "password",
            "passwd",
            "secret",
            "private_key",
        ];
        let mut matched_kv = None;
        for &k in &kv_keys {
            if let Some(after) = remaining_lower.strip_prefix(k) {
                let trimmed_after = after.trim_start();
                if trimmed_after.starts_with('=') || trimmed_after.starts_with(':') {
                    matched_kv = Some((k, after.len() - trimmed_after.len()));
                    break;
                }
            }
        }

        if let Some((k, spaces_len)) = matched_kv {
            for _ in 0..k.len() {
                result.push(chars.next().unwrap());
            }
            for _ in 0..spaces_len {
                result.push(chars.next().unwrap());
            }
            if let Some(&sep) = chars.peek()
                && (sep == '=' || sep == ':')
            {
                result.push(chars.next().unwrap());
            }
            while let Some(&c) = chars.peek() {
                if c == ' ' || c == '\t' {
                    result.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            let quote = if let Some(&c) = chars.peek() {
                if c == '"' || c == '\'' {
                    Some(chars.next().unwrap())
                } else {
                    None
                }
            } else {
                None
            };

            let mut val = String::new();
            while let Some(&c) = chars.peek() {
                if let Some(q) = quote {
                    if c == q {
                        break;
                    }
                    val.push(chars.next().unwrap());
                } else {
                    if c.is_whitespace() || c == ',' || c == ';' || c == '}' || c == ')' {
                        break;
                    }
                    val.push(chars.next().unwrap());
                }
            }

            if !val.is_empty() {
                if let Some(q) = quote {
                    result.push(q);
                    result.push_str("[REDACTED]");
                    if let Some(&c) = chars.peek()
                        && c == q
                    {
                        result.push(chars.next().unwrap());
                    }
                } else {
                    result.push_str("[REDACTED]");
                }
            }
            continue;
        }

        result.push(chars.next().unwrap());
    }

    result
}

/// Sanitize an `AgentState` in-place by normalizing all file paths and redacting secrets from all human-readable text fields.
pub fn sanitize_agent_state(state: &mut AgentState, project_root: Option<&str>) {
    state.project.title = redact_secrets(&state.project.title);
    state.project.relative_root = normalize_path(&state.project.relative_root, project_root);
    if let Some(template) = &mut state.project.template {
        *template = redact_secrets(template);
    }

    for file in &mut state.inputs.files_present {
        *file = normalize_path(file, project_root);
    }

    state.structure.title = redact_secrets(&state.structure.title);
    for sec in &mut state.structure.sections {
        sec.title = redact_secrets(&sec.title);
        if let Some(p) = &mut sec.path {
            *p = normalize_path(p, project_root);
        }
    }

    for item in &mut state.work_items {
        item.content = redact_secrets(&item.content);
        if let Some(sec_id) = &mut item.section_id {
            *sec_id = redact_secrets(sec_id);
        }
    }

    for skill in &mut state.skills.selected_skills {
        if let Some(r) = &mut skill.reason {
            *r = redact_secrets(r);
        }
        if let Some(p) = &mut skill.path {
            *p = normalize_path(p, project_root);
        }
    }

    if let Some(lock) = &mut state.jobs.workspace_lock {
        if let Some(holder) = &mut lock.holder {
            *holder = redact_secrets(holder);
        }
        if let Some(reason) = &mut lock.reason {
            *reason = redact_secrets(reason);
        }
    }

    for act in &mut state.actions {
        act.description = redact_secrets(&act.description);
        act.reason = redact_secrets(&act.reason);
    }

    for warn in &mut state.warnings {
        warn.message = redact_secrets(&warn.message);
        if let Some(h) = &mut warn.hint {
            *h = redact_secrets(h);
        }
        if let Some(p) = &mut warn.path {
            *p = normalize_path(p, project_root);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_state_kind_serde_and_str() {
        for (kind, expected) in [
            (AgentStateKind::Ready, "ready"),
            (AgentStateKind::NeedsInput, "needs_input"),
            (AgentStateKind::Blocked, "blocked"),
            (AgentStateKind::Changed, "changed"),
            (AgentStateKind::Verified, "verified"),
            (AgentStateKind::Failed, "failed"),
            (AgentStateKind::Stale, "stale"),
        ] {
            assert_eq!(kind.as_str(), expected);
            assert_eq!(kind.to_string(), expected);
            assert_eq!(AgentStateKind::from_str(expected).unwrap(), kind);
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            let de: AgentStateKind = serde_json::from_str(&json).unwrap();
            assert_eq!(de, kind);
        }
        assert!(AgentStateKind::from_str("unknown_state").is_err());
    }

    #[test]
    fn test_skill_status_serde() {
        for (status, expected) in [
            (SkillStatus::Selected, "selected"),
            (SkillStatus::Available, "available"),
            (SkillStatus::Missing, "missing"),
            (SkillStatus::Incompatible, "incompatible"),
            (SkillStatus::Unsupported, "unsupported"),
        ] {
            assert_eq!(status.as_str(), expected);
            assert_eq!(status.to_string(), expected);
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            let de: SkillStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(de, status);
        }
    }

    #[test]
    fn test_agent_state_roundtrip() {
        let state = AgentState {
            schema_version: AGENT_STATE_SCHEMA_VERSION.to_string(),
            state: AgentStateKind::Ready,
            project: ProjectIdentity {
                title: "Test Paper".to_string(),
                stage: Stage::Draft,
                paper_kind: PaperKind::Draft,
                latex_engine: LatexEngine::Tectonic,
                template: Some("standard".to_string()),
                relative_root: ".".to_string(),
            },
            inputs: InputSnapshot {
                config_fingerprint: Some("sha256:cfg123".to_string()),
                structure_fingerprint: Some("sha256:str123".to_string()),
                draft_fingerprint: Some("sha256:tex123".to_string()),
                bib_fingerprint: Some("sha256:bib123".to_string()),
                files_present: vec!["config.yaml".to_string(), "paper_draft.tex".to_string()],
                sources_count: 2,
                parsed_sources_count: 1,
                skill_lock_present: true,
                template_lock_present: true,
            },
            health: HealthSummary {
                word_count: 1500,
                missing_citations_count: 0,
                unreferenced_labels_count: 0,
                todo_ideas_count: 1,
                total_bib_keys_count: 5,
                cited_bib_keys_count: 5,
                has_errors: false,
                warning_count: 1,
                error_count: 0,
            },
            structure: StructureSummary {
                title: "Test Paper".to_string(),
                total_sections: 3,
                completed_sections: 1,
                in_progress_sections: 1,
                planned_sections: 1,
                completion_percent: 33,
                sections: vec![SectionSummaryItem {
                    id: "sec.intro".to_string(),
                    title: "Introduction".to_string(),
                    level: 1,
                    completion: "completed".to_string(),
                    path: Some("sections/intro.tex".to_string()),
                }],
            },
            work_items: vec![WorkItemSummary {
                id: "todo.1".to_string(),
                kind: "todo".to_string(),
                section_id: Some("sec.methods".to_string()),
                line_start: Some(42),
                line_end: Some(44),
                content: "Add ablation study".to_string(),
                resolved: false,
            }],
            literature: LiteratureSummary {
                total_sources: 2,
                parsed_sources: 1,
                unparsed_sources: 1,
                total_bib_keys: 5,
                cited_bib_keys: 5,
                unmentioned_bib_keys: 0,
                recent_candidates_count: 0,
            },
            skills: SkillSelectionSummary {
                active_skill_ids: vec!["SYSTEM".to_string(), "paper".to_string()],
                available_skill_ids: vec!["SYSTEM".to_string(), "paper".to_string()],
                selected_skills: vec![SelectedSkillItem {
                    id: "SYSTEM".to_string(),
                    version: Some("1.0.0".to_string()),
                    status: SkillStatus::Selected,
                    reason: Some("Mandatory system instructions".to_string()),
                    path: Some("agent/skills/SYSTEM.md".to_string()),
                    required_capabilities: vec![],
                    conflicts: vec![],
                }],
                conflicts: vec![],
                missing_requirements: vec![],
                registry_version: Some("1.0.0".to_string()),
            },
            capabilities: CapabilitySummary {
                latex_available: true,
                parser_available: true,
                git_available: true,
                online_search_available: false,
                llm_provider_available: true,
                supported_actions: vec!["check".to_string(), "compile".to_string()],
            },
            jobs: JobSummary {
                pending_jobs_count: 0,
                running_jobs_count: 0,
                failed_jobs_count: 0,
                active_job_id: None,
                workspace_lock: Some(LockSummary {
                    locked: false,
                    holder: None,
                    pid: None,
                    reason: None,
                    stale: false,
                }),
            },
            actions: vec![AvailableAction {
                id: "check".to_string(),
                description: "Run deterministic manuscript checks".to_string(),
                reason: "Validate syntax and references".to_string(),
                safe: true,
                mutating: false,
                required_inputs: vec![],
            }],
            warnings: vec![AgentFinding {
                code: "latex.warning".to_string(),
                class: FindingClass::ActionableWarning,
                message: "A minor warning".to_string(),
                path: Some("paper_draft.tex".to_string()),
                line: Some(10),
                hint: Some("Check syntax".to_string()),
            }],
        };

        let json = serde_json::to_string_pretty(&state).unwrap();
        let de: AgentState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, de);
    }

    #[test]
    fn test_stable_fingerprint_determinism() {
        let state1 = AgentState::default();
        let state2 = AgentState::default();
        assert_eq!(state1.stable_fingerprint(), state2.stable_fingerprint());
        assert!(state1.stable_fingerprint().starts_with("sha256:"));

        let envelope1 = AgentContextEnvelope {
            state: state1.clone(),
            execution: Some(AgentExecutionMetadata {
                checked_at: "2026-08-22T10:00:00Z".to_string(),
                duration_ms: 120,
                job_id: Some("job-1".to_string()),
                host_info: Some("darwin-arm64".to_string()),
            }),
        };

        let envelope2 = AgentContextEnvelope {
            state: state2.clone(),
            execution: Some(AgentExecutionMetadata {
                checked_at: "2026-08-22T10:05:00Z".to_string(),
                duration_ms: 450,
                job_id: None,
                host_info: None,
            }),
        };

        // Execution metadata differences do not change the stable fingerprint of state
        assert_eq!(
            envelope1.state.stable_fingerprint(),
            envelope2.state.stable_fingerprint()
        );
    }

    #[test]
    fn test_path_normalization() {
        assert_eq!(
            normalize_path(
                "/Users/researcher/paper/sections/intro.tex",
                Some("/Users/researcher/paper")
            ),
            "sections/intro.tex"
        );
        assert_eq!(
            normalize_path(
                "C:\\Users\\researcher\\paper\\figures\\plot.png",
                Some("C:\\Users\\researcher\\paper")
            ),
            "figures/plot.png"
        );
        assert_eq!(
            normalize_path("./paper_draft.tex", Some("/any/root")),
            "paper_draft.tex"
        );
        assert_eq!(normalize_path("", None), ".");
    }

    #[test]
    fn test_secret_redaction() {
        let raw = "Config has api_key: 'sk-proj-1234567890abcdef1234567890' and Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let redacted = redact_secrets(raw);
        assert!(!redacted.contains("sk-proj-1234567890abcdef1234567890"));
        assert!(!redacted.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
        assert!(redacted.contains("[REDACTED]"));

        let normal = "Normal section heading: Introduction to RAG models";
        assert_eq!(redact_secrets(normal), normal);

        let ghp = "Token: ghp_123456789012345678901234567890";
        assert_eq!(redact_secrets(ghp), "Token: [REDACTED]");
    }

    #[test]
    fn test_sanitize_agent_state() {
        let mut state = AgentState {
            project: ProjectIdentity {
                title: "Paper with password=secret12345".to_string(),
                relative_root: "/Users/alice/repo".to_string(),
                ..Default::default()
            },
            inputs: InputSnapshot {
                files_present: vec!["/Users/alice/repo/paper_draft.tex".to_string()],
                ..Default::default()
            },
            warnings: vec![AgentFinding {
                code: "leak".to_string(),
                class: FindingClass::ActionableWarning,
                message: "Found api_key: 'sk-123456789012345678901234'".to_string(),
                path: Some("/Users/alice/repo/config.yaml".to_string()),
                line: Some(5),
                hint: None,
            }],
            ..Default::default()
        };

        sanitize_agent_state(&mut state, Some("/Users/alice/repo"));

        assert_eq!(state.project.relative_root, ".");
        assert_eq!(state.inputs.files_present, vec!["paper_draft.tex"]);
        assert_eq!(state.warnings[0].path.as_deref(), Some("config.yaml"));
        assert!(!state.project.title.contains("secret12345"));
        assert!(
            !state.warnings[0]
                .message
                .contains("sk-123456789012345678901234")
        );
    }
}
