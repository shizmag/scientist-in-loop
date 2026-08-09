//! L0 heuristic manuscript estimation (read-only).
//!
//! Inspired by multi-perspective academic review methodology (ARS
//! `academic-paper-reviewer`) but implemented natively for sil: offline
//! structure/health signals produce dimension priors and findings without an
//! external LLM. Agents may refine scores using `agent/skills/review.md` (L1).

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use sil_core::{
    ProjectPaths, SciAction, SectionCompletion, Structure, paths::rel,
};
use sil_latex::audit_manuscript;

use crate::error::ContextError;
use crate::paper::paper_subsections;

/// Estimate mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EstimateMode {
    /// Fast structure + health pass.
    #[default]
    Quick,
    /// Full multi-dimension heuristic with DA-style findings.
    Full,
    /// Emphasize methods/experiments/reproducibility signals.
    Methodology,
}

impl EstimateMode {
    /// Parse from CLI/MCP string.
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "full" => Self::Full,
            "methodology" | "methodology-focus" | "methods" => Self::Methodology,
            _ => Self::Quick,
        }
    }

    /// Snake-case name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Full => "full",
            Self::Methodology => "methodology",
        }
    }
}

/// Editorial decision mapping (ARS-compatible thresholds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EstimateDecision {
    /// Overall ≥ 80.
    Accept,
    /// 65–79.
    MinorRevision,
    /// 50–64.
    MajorRevision,
    /// < 50.
    Reject,
}

impl EstimateDecision {
    /// Map overall score to decision.
    pub fn from_score(score: u8) -> Self {
        if score >= 80 {
            Self::Accept
        } else if score >= 65 {
            Self::MinorRevision
        } else if score >= 50 {
            Self::MajorRevision
        } else {
            Self::Reject
        }
    }

    /// Snake-case wire value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::MinorRevision => "minor_revision",
            Self::MajorRevision => "major_revision",
            Self::Reject => "reject",
        }
    }
}

/// Seven review dimensions (0–100).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EstimateDimensions {
    /// Significance / importance of contribution.
    pub significance: u8,
    /// Novelty.
    pub novelty: u8,
    /// Methodology rigor.
    pub methodology: u8,
    /// Clarity of writing.
    pub clarity: u8,
    /// Related work / literature positioning.
    pub related_work: u8,
    /// Reproducibility.
    pub reproducibility: u8,
    /// Ethics / responsible claims.
    pub ethics: u8,
}

impl EstimateDimensions {
    /// Mean rounded overall score.
    pub fn overall(&self) -> u8 {
        let sum = self.significance as u32
            + self.novelty as u32
            + self.methodology as u32
            + self.clarity as u32
            + self.related_work as u32
            + self.reproducibility as u32
            + self.ethics as u32;
        (sum / 7) as u8
    }
}

/// Single finding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EstimateFinding {
    /// Stable id (e.g. `F1`).
    pub id: String,
    /// Persona / lens.
    pub persona: String,
    /// Severity.
    pub severity: String,
    /// Location hint.
    pub location: String,
    /// Short summary.
    pub summary: String,
    /// Actionable suggestion.
    pub suggestion: String,
}

/// Full estimate report (JSON schema version 1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateReport {
    /// Schema version.
    pub schema_version: u32,
    /// Mode used.
    pub mode: String,
    /// Decision.
    pub decision: String,
    /// Overall 0–100.
    pub overall_score: u8,
    /// Dimension scores.
    pub dimensions: EstimateDimensions,
    /// Findings.
    pub findings: Vec<EstimateFinding>,
    /// Devil's-advocate CRITICAL items.
    pub da_critical: Vec<String>,
    /// Prioritized revision steps.
    pub revision_roadmap: Vec<String>,
    /// Always true for estimate path.
    pub read_only: bool,
    /// L0 heuristic vs L1 agent-refined.
    pub layer: String,
    /// Attribution / license note.
    pub attribution: String,
    /// Word count from draft.
    pub word_count: usize,
    /// Draft content hash (first 16 hex of FNV-like).
    pub draft_hash: String,
}

/// Input for L0 estimate.
pub struct EstimateInput<'a> {
    /// Project root.
    pub root: &'a Utf8Path,
    /// Mode.
    pub mode: EstimateMode,
    /// Optional structure.
    pub structure: Option<&'a Structure>,
}

const ATTRIBUTION: &str = "Inspired by Academic Research Skills academic-paper-reviewer (CC-BY-NC 4.0); sil-native L0 heuristic implementation.";

/// Run offline L0 estimate. Never mutates `paper_draft.tex`.
pub fn run_heuristic_estimate(input: &EstimateInput<'_>) -> Result<EstimateReport, ContextError> {
    let paths = ProjectPaths::new(input.root);
    let draft_path = paths.paper_draft();
    if !draft_path.is_file() {
        return Err(ContextError::Io(format!(
            "missing {}",
            rel::PAPER_DRAFT
        )));
    }
    let tex = fs::read_to_string(draft_path.as_str())
        .map_err(|e| ContextError::Io(format!("{draft_path}: {e}")))?;
    let draft_hash = short_hash(&tex);

    let bib_path = paths.join(rel::REFERENCES);
    let health = audit_manuscript(
        &draft_path,
        if bib_path.is_file() {
            Some(bib_path.as_path())
        } else {
            None
        },
    )
    .map_err(|e| ContextError::Io(format!("health audit: {e}")))?;

    let sections = paper_subsections(&tex);
    let empty_sections: Vec<_> = sections
        .iter()
        .filter(|s| s.body.trim().is_empty())
        .map(|s| s.title.clone())
        .collect();
    let todo_count = health.todo_ideas_count;
    let missing_cites = health.missing_citations_count;
    let word_count = health.word_count;

    let mut dims = EstimateDimensions {
        significance: 70,
        novelty: 68,
        methodology: 72,
        clarity: 75,
        related_work: 70,
        reproducibility: 65,
        ethics: 78,
    };

    // Structure completion signal
    if let Some(st) = input.structure {
        let summary = st.completion_summary();
        let total = st.sections.len().max(1);
        let polished = st
            .sections
            .iter()
            .filter(|s| s.completion == SectionCompletion::Polished)
            .count();
        let draftish = st
            .sections
            .iter()
            .filter(|s| {
                matches!(
                    s.completion,
                    SectionCompletion::Draft | SectionCompletion::Polished
                )
            })
            .count();
        let empty = st
            .sections
            .iter()
            .filter(|s| s.completion == SectionCompletion::Empty)
            .count();
        let _ = summary;
        let draft_ratio = draftish as f32 / total as f32;
        let empty_ratio = empty as f32 / total as f32;
        dims.significance = adjust(dims.significance, draft_ratio * 15.0 - empty_ratio * 20.0);
        dims.clarity = adjust(dims.clarity, draft_ratio * 10.0 - empty_ratio * 15.0);
        dims.novelty = adjust(dims.novelty, (polished as f32 / total as f32) * 8.0);
    }

    // Length priors
    if word_count < 800 {
        dims.significance = adjust(dims.significance, -12.0);
        dims.clarity = adjust(dims.clarity, -8.0);
    } else if word_count > 4000 {
        dims.clarity = adjust(dims.clarity, 4.0);
    }

    // Empty latex sections
    if !empty_sections.is_empty() {
        let pen = (empty_sections.len() as f32 * 6.0).min(24.0);
        dims.clarity = adjust(dims.clarity, -pen);
        dims.methodology = adjust(dims.methodology, -pen * 0.5);
    }

    // TODOs
    if todo_count > 0 {
        dims.clarity = adjust(dims.clarity, -(todo_count as f32 * 2.0).min(15.0));
        dims.reproducibility = adjust(dims.reproducibility, -(todo_count as f32).min(10.0));
    }

    // Missing citations
    if missing_cites > 0 {
        dims.related_work = adjust(dims.related_work, -(missing_cites as f32 * 4.0).min(25.0));
        dims.ethics = adjust(dims.ethics, -(missing_cites as f32 * 2.0).min(12.0));
    }

    // Mode tweaks
    match input.mode {
        EstimateMode::Methodology => {
            dims.methodology = adjust(dims.methodology, 5.0);
            if word_count < 1500 {
                dims.reproducibility = adjust(dims.reproducibility, -8.0);
            }
        }
        EstimateMode::Full => {
            // Slightly harsher empty-section penalty already applied
        }
        EstimateMode::Quick => {}
    }

    let mut findings = Vec::new();
    let mut n = 1u32;
    let mut push = |persona: &str, severity: &str, location: &str, summary: &str, suggestion: &str| {
        findings.push(EstimateFinding {
            id: format!("F{n}"),
            persona: persona.into(),
            severity: severity.into(),
            location: location.into(),
            summary: summary.into(),
            suggestion: suggestion.into(),
        });
        n += 1;
    };

    for title in &empty_sections {
        push(
            "clarity",
            "major",
            &format!("section:{title}"),
            &format!("Section `{title}` has empty body in paper_draft.tex"),
            "Draft prose or mark completion as empty intentionally in structure.yaml",
        );
    }
    if missing_cites > 0 {
        push(
            "domain",
            "major",
            "citations",
            &format!("{missing_cites} \\cite{{}} key(s) missing from references.bib"),
            "Add BibTeX entries or fix cite keys; use sil source cite / sil_upsert_bib",
        );
    }
    if todo_count > 0 {
        push(
            "journal_fit",
            "minor",
            "todos",
            &format!("{todo_count} active # -- X -- # idea/TODO block(s)"),
            "Resolve or remove before submission; track via sil paper todo",
        );
    }
    if word_count < 800 {
        push(
            "journal_fit",
            "major",
            "length",
            &format!("Draft is short ({word_count} words)"),
            "Expand core claims, methods, and related work",
        );
    }

    let mut da_critical = Vec::new();
    if missing_cites > 3 {
        da_critical.push(format!(
            "High volume of undefined citations ({missing_cites}) undermines claim support"
        ));
        push(
            "devils_advocate",
            "critical",
            "citations",
            "Multiple undefined citations — claims may be unsupported",
            "Ground every claim with resolvable bibliography entries",
        );
    }
    if empty_sections.len() >= 3 {
        da_critical.push(format!(
            "{} empty sections suggest incomplete argument chain",
            empty_sections.len()
        ));
    }

    // Methodology mode: extra findings on experiments/data language
    if matches!(input.mode, EstimateMode::Methodology | EstimateMode::Full) {
        let lower = tex.to_ascii_lowercase();
        if !lower.contains("experiment")
            && !lower.contains("evaluation")
            && !lower.contains("dataset")
        {
            push(
                "methodology",
                "major",
                "methods",
                "Little explicit evaluation/experiment language detected",
                "Add evaluation protocol, datasets, and metrics if empirical",
            );
            dims.methodology = adjust(dims.methodology, -10.0);
            dims.reproducibility = adjust(dims.reproducibility, -8.0);
        }
    }

    let overall = dims.overall();
    // DA critical blocks Accept in summary decision presentation
    let mut decision = EstimateDecision::from_score(overall);
    if !da_critical.is_empty() && matches!(decision, EstimateDecision::Accept) {
        decision = EstimateDecision::MinorRevision;
    }

    let mut roadmap = Vec::new();
    for f in findings.iter().filter(|f| f.severity != "minor").take(8) {
        roadmap.push(format!("[{}] {}: {}", f.severity, f.location, f.suggestion));
    }
    if roadmap.is_empty() {
        roadmap.push("Polish prose and re-run sil paper estimate before submission".into());
    }

    Ok(EstimateReport {
        schema_version: 1,
        mode: input.mode.as_str().into(),
        decision: decision.as_str().into(),
        overall_score: overall,
        dimensions: dims,
        findings,
        da_critical,
        revision_roadmap: roadmap,
        read_only: true,
        layer: "L0_heuristic".into(),
        attribution: ATTRIBUTION.into(),
        word_count,
        draft_hash,
    })
}

/// Render report as markdown for humans.
pub fn report_to_markdown(report: &EstimateReport) -> String {
    let mut out = String::new();
    out.push_str("# Manuscript estimate (sil L0)\n\n");
    out.push_str(&format!(
        "- **Mode:** {}\n- **Layer:** {}\n- **Overall:** {} / 100\n- **Decision:** `{}`\n- **Words:** {}\n- **Draft hash:** `{}`\n- **Read-only:** {}\n\n",
        report.mode,
        report.layer,
        report.overall_score,
        report.decision,
        report.word_count,
        report.draft_hash,
        report.read_only
    ));
    out.push_str("## Dimensions\n\n");
    let d = &report.dimensions;
    out.push_str(&format!(
        "| Dimension | Score |\n|---|---:|\n| significance | {} |\n| novelty | {} |\n| methodology | {} |\n| clarity | {} |\n| related_work | {} |\n| reproducibility | {} |\n| ethics | {} |\n\n",
        d.significance, d.novelty, d.methodology, d.clarity, d.related_work, d.reproducibility, d.ethics
    ));
    if !report.da_critical.is_empty() {
        out.push_str("## Devil's Advocate CRITICAL\n\n");
        for c in &report.da_critical {
            out.push_str(&format!("- {c}\n"));
        }
        out.push('\n');
    }
    out.push_str("## Findings\n\n");
    for f in &report.findings {
        out.push_str(&format!(
            "### {} [{} / {}] — {}\n\n{}\n\n**Suggestion:** {}\n\n",
            f.id, f.persona, f.severity, f.location, f.summary, f.suggestion
        ));
    }
    out.push_str("## Revision roadmap\n\n");
    for (i, step) in report.revision_roadmap.iter().enumerate() {
        out.push_str(&format!("{}. {}\n", i + 1, step));
    }
    out.push_str(&format!("\n---\n\n_{}_\n", report.attribution));
    out
}

/// Write report under `.sil/reviews/review_YYYYMMDD_HHMMSS/` (or unique suffix).
/// Does not modify the manuscript. Returns directory path.
pub fn write_estimate_report(
    root: &Utf8Path,
    report: &EstimateReport,
) -> Result<Utf8PathBuf, ContextError> {
    let paths = ProjectPaths::new(root);
    let reviews = paths.reviews_dir();
    fs::create_dir_all(reviews.as_str())
        .map_err(|e| ContextError::Io(format!("{reviews}: {e}")))?;

    // Ensure README
    let readme = reviews.join("README.md");
    if !readme.is_file() {
        let _ = fs::write(
            readme.as_str(),
            "# Reviews\n\nEstimate reports from `sil paper estimate` / MCP `sil_estimate_paper`.\n\
             Read-only artifacts — do not treat scores as peer-review truth.\n",
        );
    }

    let stamp = timestamp_slug();
    let dir = reviews.join(format!("review_{stamp}"));
    fs::create_dir_all(dir.as_str()).map_err(|e| ContextError::Io(format!("{dir}: {e}")))?;

    let json = serde_json::to_string_pretty(report)
        .map_err(|e| ContextError::Io(format!("serialize report: {e}")))?;
    fs::write(dir.join("report.json").as_str(), json)
        .map_err(|e| ContextError::Io(format!("write report.json: {e}")))?;
    fs::write(dir.join("report.md").as_str(), report_to_markdown(report))
        .map_err(|e| ContextError::Io(format!("write report.md: {e}")))?;

    let meta = format!(
        "mode: {}\nlayer: {}\noverall_score: {}\ndecision: {}\ndraft_hash: {}\n",
        report.mode, report.layer, report.overall_score, report.decision, report.draft_hash
    );
    fs::write(dir.join("meta.yaml").as_str(), meta)
        .map_err(|e| ContextError::Io(format!("write meta.yaml: {e}")))?;

    Ok(dir)
}

/// Sci-Action proposal text for writing an estimate report (metadata only).
pub fn estimate_proposal_message(report_dir: &Utf8Path) -> String {
    format!(
        "Add manuscript estimate report\n\n{}\n\nRecord L0/L1 estimate under {}; never auto-committed.\n",
        SciAction::EstimatePaper.trailer_line(),
        report_dir
    )
}

fn adjust(base: u8, delta: f32) -> u8 {
    let v = base as f32 + delta;
    v.clamp(0.0, 100.0).round() as u8
}

fn short_hash(s: &str) -> String {
    // Simple FNV-1a 64-bit for stability without extra deps.
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn timestamp_slug() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // YYYYMMDD_HHMMSS UTC-ish from epoch formatting without chrono
    // Use raw epoch if we lack time crate; still unique enough with pid
    format!("{secs}_{}", std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sil_core::{Section, SectionCompletion, Structure};
    use tempfile::tempdir;

    fn write_project(root: &Utf8Path, tex: &str) {
        fs::create_dir_all(root.join(".sil").as_str()).unwrap();
        fs::write(root.join("paper_draft.tex").as_str(), tex).unwrap();
        fs::write(root.join("references.bib").as_str(), "@article{ok,\n  title={T},\n}\n").unwrap();
    }

    #[test]
    fn decision_thresholds() {
        assert_eq!(EstimateDecision::from_score(80).as_str(), "accept");
        assert_eq!(EstimateDecision::from_score(65).as_str(), "minor_revision");
        assert_eq!(EstimateDecision::from_score(50).as_str(), "major_revision");
        assert_eq!(EstimateDecision::from_score(49).as_str(), "reject");
    }

    #[test]
    fn heuristic_empty_sections_findings() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        write_project(
            &root,
            r#"\section{Intro}
Some text with \cite{missing}.
\section{Methods}
"#,
        );
        let report = run_heuristic_estimate(&EstimateInput {
            root: &root,
            mode: EstimateMode::Full,
            structure: None,
        })
        .unwrap();
        assert!(report.read_only);
        assert_eq!(report.layer, "L0_heuristic");
        assert!(report.findings.iter().any(|f| f.location.contains("Methods")));
        assert!(report.findings.iter().any(|f| f.location == "citations"));
        assert!(report.overall_score <= 100);
    }

    #[test]
    fn write_report_does_not_touch_draft() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let tex = "\\section{A}\nHello world words enough here for a sentence.\n";
        write_project(&root, tex);
        let report = run_heuristic_estimate(&EstimateInput {
            root: &root,
            mode: EstimateMode::Quick,
            structure: None,
        })
        .unwrap();
        let out = write_estimate_report(&root, &report).unwrap();
        assert!(out.join("report.json").is_file());
        assert!(out.join("report.md").is_file());
        let after = fs::read_to_string(root.join("paper_draft.tex").as_str()).unwrap();
        assert_eq!(after, tex);
    }

    #[test]
    fn structure_affects_scores() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        write_project(
            &root,
            "\\section{Intro}\nLong enough text about the contribution and methods evaluation dataset experiment.\n".repeat(20).as_str(),
        );
        let st = Structure {
            title: "t".into(),
            status: Default::default(),
            sections: vec![Section {
                id: "intro".into(),
                title: "Intro".into(),
                level: 1,
                completion: SectionCompletion::Polished,
                main_claim: String::new(),
                secondary_points: vec![],
                required_content: vec![],
            }],
        };
        let report = run_heuristic_estimate(&EstimateInput {
            root: &root,
            mode: EstimateMode::Quick,
            structure: Some(&st),
        })
        .unwrap();
        assert!(report.overall_score > 50);
    }
}
