//! Deterministic manuscript check contracts and profile policy.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Maximum number of findings shown by the default compact formatter.
pub const DEFAULT_FINDING_LIMIT: usize = 20;

/// The kind of condition represented by a check finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingClass {
    /// The current project state is not coherent enough to establish a result.
    InvariantError,
    /// A condition a caller may choose to make blocking.
    ActionableWarning,
    /// Information that never affects the exit policy.
    Observation,
}

/// A stable, structured check finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckFinding {
    /// Stable machine-readable finding code.
    pub code: String,
    /// Finding policy class.
    pub class: FindingClass,
    /// Project-relative path, when the finding belongs to a file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// One-based source line, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Human-readable explanation.
    pub message: String,
    /// Optional remediation guidance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// Machine-readable evidence. Object keys are serialized deterministically.
    #[serde(default)]
    pub evidence: Value,
}

impl CheckFinding {
    /// Return the identity used for deterministic deduplication.
    pub fn dedup_key(&self) -> String {
        format!(
            "{}\u{1f}{}\u{1f}{}\u{1f}{}",
            self.code,
            self.path.as_deref().unwrap_or_default(),
            self.line.map_or_else(String::new, |line| line.to_string()),
            canonical_json(&self.evidence)
        )
    }
}

/// The explicit policy profile used to decide whether a report passes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckProfile {
    /// Only invariant errors block.
    #[default]
    Draft,
    /// Invariant errors and the normative warning codes block.
    Submission,
    /// Invariant errors and all actionable warnings block.
    Strict,
}

impl CheckProfile {
    /// Return whether a finding blocks this profile.
    pub fn blocks(self, finding: &CheckFinding, template_blocking_codes: &[String]) -> bool {
        match self {
            Self::Draft => finding.class == FindingClass::InvariantError,
            Self::Strict => finding.class != FindingClass::Observation,
            Self::Submission => {
                finding.class == FindingClass::InvariantError
                    || (finding.class == FindingClass::ActionableWarning
                        && (SUBMISSION_BLOCKING_CODES.contains(&finding.code.as_str())
                            || template_blocking_codes
                                .iter()
                                .any(|code| code == &finding.code)))
            }
        }
    }
}

const SUBMISSION_BLOCKING_CODES: &[&str] = &[
    "latex.citation.undefined",
    "latex.reference.undefined",
    "latex.label.duplicate",
    "latex.bib.key_duplicate",
    "latex.dependency.missing",
    "latex.asset.missing",
    "template.constraint.violation",
];

/// Counts of findings by class.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckSummary {
    /// Number of invariant errors.
    pub errors: usize,
    /// Number of actionable warnings.
    pub warnings: usize,
    /// Number of observations.
    pub observations: usize,
}

impl CheckSummary {
    /// Count findings into a summary.
    pub fn from_findings(findings: &[CheckFinding]) -> Self {
        let mut summary = Self::default();
        for finding in findings {
            match finding.class {
                FindingClass::InvariantError => summary.errors += 1,
                FindingClass::ActionableWarning => summary.warnings += 1,
                FindingClass::Observation => summary.observations += 1,
            }
        }
        summary
    }

    /// Return whether this summary contains any invariant errors.
    pub fn has_errors(&self) -> bool {
        self.errors > 0
    }
}

/// The canonical deterministic portion of a check report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckStaticReport {
    /// Check contract schema version.
    pub schema_version: u32,
    /// Profile used to produce the report.
    pub profile: CheckProfile,
    /// SHA-256 fingerprint of normalized check inputs.
    pub input_fingerprint: String,
    /// Counts corresponding to `findings`.
    pub summary: CheckSummary,
    /// Sorted and deduplicated findings.
    pub findings: Vec<CheckFinding>,
    /// Deterministic checker metrics.
    #[serde(default)]
    pub metrics: BTreeMap<String, Value>,
    /// Resolved project-relative dependencies.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Selected template metadata, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<Value>,
}

impl CheckStaticReport {
    /// Construct a report and normalize finding order, duplicates, and summary.
    pub fn new(
        profile: CheckProfile,
        input_fingerprint: impl Into<String>,
        mut findings: Vec<CheckFinding>,
    ) -> Self {
        sort_and_deduplicate(&mut findings);
        Self {
            schema_version: 1,
            profile,
            input_fingerprint: input_fingerprint.into(),
            summary: CheckSummary::from_findings(&findings),
            findings,
            metrics: BTreeMap::new(),
            dependencies: Vec::new(),
            template: None,
        }
    }

    /// Return whether the report passes the selected profile.
    pub fn passes(&self, template_blocking_codes: &[String]) -> bool {
        !self
            .findings
            .iter()
            .any(|finding| self.profile.blocks(finding, template_blocking_codes))
    }

    /// Format compact human output, hiding observations and capping findings by default.
    pub fn format_compact(&self, all: bool, verbose: bool) -> String {
        let mut output = format!(
            "{} error(s), {} warning(s), {} observation(s)",
            self.summary.errors, self.summary.warnings, self.summary.observations
        );
        let findings = self
            .findings
            .iter()
            .filter(|finding| verbose || finding.class != FindingClass::Observation);
        let findings: Vec<_> = if all {
            findings.collect()
        } else {
            findings.take(DEFAULT_FINDING_LIMIT).collect()
        };
        for finding in findings {
            let location = finding.path.as_deref().unwrap_or("<project>");
            let line = finding
                .line
                .map_or(String::new(), |line| format!(":{line}"));
            output.push_str(&format!(
                "\n{} {}{}: {}",
                finding.code, location, line, finding.message
            ));
        }
        output
    }
}

/// Volatile execution metadata kept outside the canonical static artifact.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckRunMetadata {
    /// UTC timestamp for this execution.
    pub checked_at: String,
    /// Build result and logs, if a build was requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<Value>,
    /// Online provider result, if online checks were requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub online: Option<Value>,
}

/// Complete report containing canonical data and volatile execution metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckReport {
    /// Canonical deterministic report.
    #[serde(rename = "static")]
    pub r#static: CheckStaticReport,
    /// Volatile execution envelope.
    pub run: CheckRunMetadata,
}

impl CheckReport {
    /// Return whether the canonical report passes its profile policy.
    pub fn passes(&self, template_blocking_codes: &[String]) -> bool {
        self.r#static.passes(template_blocking_codes)
    }
}

/// Sort and deduplicate findings by their stable identity.
pub fn sort_and_deduplicate(findings: &mut Vec<CheckFinding>) {
    for finding in findings.iter_mut() {
        finding.evidence = normalize_json(&finding.evidence);
    }
    findings.sort_by_key(|finding| {
        (
            finding.dedup_key(),
            format!("{:?}", finding.class),
            finding.message.clone(),
            finding.hint.clone().unwrap_or_default(),
        )
    });
    findings.dedup_by(|right, left| right.dedup_key() == left.dedup_key());
}

/// Compute a `sha256:` fingerprint for normalized input bytes.
pub fn input_fingerprint(input: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(input))
}

/// Serialize a value and compute its deterministic input fingerprint.
pub fn serialized_input_fingerprint<T: Serialize>(input: &T) -> Result<String, serde_json::Error> {
    serde_json::to_value(input).map(|value| input_fingerprint(canonical_json(&value).as_bytes()))
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(object) => {
            let mut sorted = Map::new();
            let mut entries: Vec<_> = object.iter().collect();
            entries.sort_by_key(|(key, _)| *key);
            for (key, value) in entries {
                sorted.insert(
                    key.clone(),
                    serde_json::from_str(&canonical_json(value)).unwrap(),
                );
            }
            serde_json::to_string(&sorted).unwrap_or_default()
        }
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn normalize_json(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut normalized = Map::new();
            let mut entries: Vec<_> = object.iter().collect();
            entries.sort_by_key(|(key, _)| *key);
            for (key, value) in entries {
                normalized.insert(key.clone(), normalize_json(value));
            }
            Value::Object(normalized)
        }
        Value::Array(values) => Value::Array(values.iter().map(normalize_json).collect()),
        value => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(class: FindingClass, code: &str, path: &str, line: usize) -> CheckFinding {
        CheckFinding {
            code: code.into(),
            class,
            path: Some(path.into()),
            line: Some(line),
            message: code.into(),
            hint: None,
            evidence: serde_json::json!({"key": code}),
        }
    }

    #[test]
    fn report_round_trips_and_orders_findings() {
        let report = CheckStaticReport::new(
            CheckProfile::Draft,
            "sha256:test",
            vec![
                finding(FindingClass::Observation, "z.code", "b.tex", 2),
                finding(FindingClass::InvariantError, "a.code", "a.tex", 1),
                finding(FindingClass::InvariantError, "a.code", "a.tex", 1),
            ],
        );
        assert_eq!(report.findings.len(), 2);
        assert_eq!(report.findings[0].code, "a.code");
        let json = serde_json::to_string(&report).unwrap();
        assert_eq!(
            serde_json::from_str::<CheckStaticReport>(&json).unwrap(),
            report
        );
    }

    #[test]
    fn profile_policy_is_explicit() {
        let warning = finding(
            FindingClass::ActionableWarning,
            "latex.citation.undefined",
            "paper.tex",
            3,
        );
        let other = finding(
            FindingClass::ActionableWarning,
            "custom.warning",
            "paper.tex",
            4,
        );
        let error = finding(FindingClass::InvariantError, "project.missing", "", 0);
        assert!(!CheckProfile::Draft.blocks(&warning, &[]));
        assert!(CheckProfile::Submission.blocks(&warning, &[]));
        assert!(!CheckProfile::Submission.blocks(&other, &[]));
        assert!(CheckProfile::Submission.blocks(&other, &["custom.warning".into()]));
        assert!(CheckProfile::Strict.blocks(&other, &[]));
        assert!(
            !CheckProfile::Strict.blocks(&finding(FindingClass::Observation, "info", "", 0), &[])
        );
        assert!(CheckProfile::Draft.blocks(&error, &[]));
    }

    #[test]
    fn compact_output_caps_findings_but_report_keeps_all() {
        let findings = (0..25)
            .map(|line| {
                finding(
                    FindingClass::ActionableWarning,
                    &format!("warning.{line:02}"),
                    "paper.tex",
                    line,
                )
            })
            .collect();
        let report = CheckStaticReport::new(CheckProfile::Draft, "sha256:test", findings);
        assert_eq!(report.findings.len(), 25);
        assert_eq!(report.format_compact(false, false).lines().count(), 21);
        assert_eq!(report.format_compact(true, false).lines().count(), 26);
    }

    #[test]
    fn observations_never_block_and_fingerprint_is_sha256() {
        let observation = finding(FindingClass::Observation, "metrics.words", "", 0);
        let report = CheckStaticReport::new(
            CheckProfile::Strict,
            input_fingerprint(b"input"),
            vec![observation],
        );
        assert!(report.passes(&[]));
        assert_eq!(
            input_fingerprint(b"abc"),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
