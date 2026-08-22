//! Declarative skill metadata, capability checking, scoring, deterministic tie-breaking, and routing.

use std::collections::BTreeSet;
use std::fs;

use camino::Utf8Path;
pub use sil_core::agent::{SelectedSkillItem, SkillSelectionSummary, SkillStatus};
use sil_core::paths::rel;

use crate::error::ContextError;
use crate::registry::{HostCapabilities, SkillRegistry, SkillRegistryError};

/// Which optional context sections to include.
#[derive(Debug, Clone, Default)]
pub struct ContextFlags {
    /// Include paper_draft.tex split into subsections.
    pub paper: bool,
    /// Include agent/ listing + README.
    pub agent: bool,
    /// Include paper.md skill.
    pub skill_paper: bool,
    /// Include agent-code.md skill.
    pub skill_agent_code: bool,
    /// Explicit skill names to load (basename without path).
    pub skills: Vec<String>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

/// Declarative metadata for a workspace skill used by routing, capability validation, and explainability.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SkillDefinition {
    /// Stable skill identifier (e.g. "SYSTEM", "paper", "review", "visualize-article").
    pub id: String,
    /// Human-readable skill name.
    #[serde(default)]
    pub name: String,
    /// Semver package or skill version.
    #[serde(default = "default_version")]
    pub version: String,
    /// Human-readable skill title.
    #[serde(default)]
    pub title: String,
    /// Human-readable skill description.
    #[serde(default)]
    pub description: String,
    /// Project-relative or entrypoint path.
    #[serde(default)]
    pub path: String,
    /// Explicit triggers / goals that activate this skill when present in a task description.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<String>,
    /// Toolchain and host capabilities required by this skill (e.g. "tectonic", "latexmk", "marker", "python", "git", "network").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_capabilities: Vec<String>,
    /// Expected input artifacts or files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
    /// Expected output artifacts or files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<String>,
    /// Permissions needed by this skill.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
    /// Verification command or action ID (e.g. "check", "verify_review").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<String>,
    /// Conflicting skill IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
}

impl SkillDefinition {
    /// Built-in SYSTEM skill definition (mandatory baseline).
    pub fn system() -> Self {
        Self {
            id: "SYSTEM".into(),
            name: "system".into(),
            version: "1.0.0".into(),
            title: "System Grounding Rules".into(),
            description:
                "Mandatory system instructions and core operating rules for all agent tasks.".into(),
            path: "agent/skills/SYSTEM.md".into(),
            triggers: vec!["system".into()],
            required_capabilities: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            permissions: Vec::new(),
            verification: None,
            conflicts: Vec::new(),
        }
    }

    /// Built-in paper skill definition.
    pub fn paper() -> Self {
        Self {
            id: "paper".into(),
            name: "paper".into(),
            version: "1.0.0".into(),
            title: "Manuscript Drafting and Structure".into(),
            description:
                "LaTeX manuscript drafting, subsection writing, and structure.yaml updates.".into(),
            path: "agent/skills/paper.md".into(),
            triggers: vec![
                "paper".into(),
                "draft".into(),
                "latex".into(),
                "structure".into(),
                "tex".into(),
                "write".into(),
                "section".into(),
                "abstract".into(),
                "introduction".into(),
                "structure.yaml".into(),
                "paper_draft.tex".into(),
                "paper.tex".into(),
                "completion".into(),
                "manuscript".into(),
            ],
            required_capabilities: vec!["tectonic".into()],
            inputs: vec!["paper_draft.tex".into(), "structure.yaml".into()],
            outputs: vec!["paper_draft.tex".into(), "structure.yaml".into()],
            permissions: vec![
                "write:paper_draft.tex".into(),
                "write:structure.yaml".into(),
            ],
            verification: Some("sil build".into()),
            conflicts: Vec::new(),
        }
    }

    /// Built-in agent-code skill definition.
    pub fn agent_code() -> Self {
        Self {
            id: "agent-code".into(),
            name: "agent-code".into(),
            version: "1.0.0".into(),
            title: "Agent Reproducibility and Code".into(),
            description:
                "Agent scripts, parser development, and reproducibility pipeline under agent/."
                    .into(),
            path: "agent/skills/agent-code.md".into(),
            triggers: vec![
                "agent-code".into(),
                "agent".into(),
                "python".into(),
                "script".into(),
                "parser".into(),
                "reproducibility".into(),
                "agent/".into(),
                "agent\\".into(),
                "reproducib".into(),
            ],
            required_capabilities: vec!["python".into(), "git".into()],
            inputs: vec!["agent/".into()],
            outputs: vec!["agent/".into()],
            permissions: vec!["write:agent/".into()],
            verification: Some("python -m pytest".into()),
            conflicts: Vec::new(),
        }
    }

    /// Built-in review skill definition.
    pub fn review() -> Self {
        Self {
            id: "review".into(),
            name: "review".into(),
            version: "1.0.0".into(),
            title: "Quality Gate and L0 Estimate".into(),
            description: "Manuscript estimation, peer-review critique, and editorial scoring."
                .into(),
            path: "agent/skills/review.md".into(),
            triggers: vec![
                "review".into(),
                "estimate".into(),
                "rubric".into(),
                "quality".into(),
                "critique".into(),
                "peer review".into(),
                "referee".into(),
                "editorial".into(),
            ],
            required_capabilities: Vec::new(),
            inputs: vec!["paper_draft.tex".into(), "structure.yaml".into()],
            outputs: vec![".sil/reviews/".into()],
            permissions: vec!["write:.sil/reviews/".into()],
            verification: Some("sil estimate".into()),
            conflicts: Vec::new(),
        }
    }

    /// Built-in visualize-article skill definition.
    pub fn visualize_article() -> Self {
        Self {
            id: "visualize-article".into(),
            name: "visualize-article".into(),
            version: "1.0.0".into(),
            title: "Visualize Article & Architecture Diagrams".into(),
            description:
                "Figure and diagram design guidelines for article architectures and visualization."
                    .into(),
            path: "agent/skills/managed/scientist-in-loop__visualize-article/1.0.0/SKILL.md".into(),
            triggers: vec![
                "figure".into(),
                "figures".into(),
                "visualiz".into(),
                "visualize".into(),
                "visualise".into(),
                "visualization".into(),
                "diagram".into(),
                "architecture".into(),
                "pipeline".into(),
            ],
            required_capabilities: vec!["network".into()],
            inputs: vec!["paper_draft.tex".into()],
            outputs: vec!["figures/".into()],
            permissions: vec![
                "read:manuscript".into(),
                "network:external_image_provider".into(),
            ],
            verification: Some("check_figures".into()),
            conflicts: Vec::new(),
        }
    }

    /// Default set of built-in first-party skill definitions.
    pub fn builtins() -> Vec<Self> {
        vec![
            Self::system(),
            Self::paper(),
            Self::agent_code(),
            Self::review(),
            Self::visualize_article(),
        ]
    }

    /// Parse declarative skill metadata from markdown frontmatter or contents.
    pub fn parse_from_markdown(content: &str, default_id: &str) -> Result<Self, ContextError> {
        let trimmed = content.trim_start();
        if let Some(stripped) = trimmed.strip_prefix("---")
            && let Some(end_idx) = stripped.find("\n---")
        {
            let yaml_str = &stripped[..end_idx];
            if let Ok(mut def) = serde_yaml::from_str::<SkillDefinition>(yaml_str) {
                if def.id.is_empty() {
                    def.id = default_id.to_string();
                }
                if def.path.is_empty() {
                    def.path = format!("agent/skills/{default_id}.md");
                }
                return Ok(def);
            }
        }

        let title = content
            .lines()
            .find(|l| l.starts_with("# "))
            .map(|l| l.trim_start_matches("# ").trim().to_string())
            .unwrap_or_else(|| default_id.to_string());

        Ok(Self {
            id: default_id.to_string(),
            name: default_id.to_string(),
            version: "1.0.0".to_string(),
            title,
            description: String::new(),
            path: format!("agent/skills/{default_id}.md"),
            triggers: Vec::new(),
            required_capabilities: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            permissions: Vec::new(),
            verification: None,
            conflicts: Vec::new(),
        })
    }
}

/// Scoring breakdown for a skill candidate during routing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SkillScore {
    /// Matched exact triggers/goals from the task text.
    pub matched_triggers: Vec<String>,
    /// Matched overlapping keywords.
    pub matched_keywords: Vec<String>,
    /// Total computed numeric score.
    pub score: u32,
}

impl SkillScore {
    /// Whether this score represents an active match.
    pub fn is_matched(&self) -> bool {
        self.score > 0
    }
}

fn is_stopword(w: &str) -> bool {
    matches!(
        w,
        "the"
            | "and"
            | "for"
            | "with"
            | "this"
            | "that"
            | "from"
            | "into"
            | "about"
            | "some"
            | "any"
            | "have"
            | "has"
            | "had"
            | "will"
            | "would"
            | "should"
            | "could"
            | "under"
            | "over"
            | "after"
            | "before"
            | "which"
            | "where"
    )
}

/// Declarative matcher and router for workspace skills.
#[derive(Debug, Clone)]
pub struct SkillRouter {
    skills: Vec<SkillDefinition>,
}

impl Default for SkillRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRouter {
    /// Create a new SkillRouter initialized with default built-in skill definitions.
    pub fn new() -> Self {
        Self {
            skills: SkillDefinition::builtins(),
        }
    }

    /// Create a SkillRouter with explicit custom skill definitions.
    pub fn with_skills(skills: Vec<SkillDefinition>) -> Self {
        Self { skills }
    }

    /// Register or replace a skill definition in the router.
    pub fn register(&mut self, skill: SkillDefinition) {
        if let Some(pos) = self.skills.iter().position(|s| s.id == skill.id) {
            self.skills[pos] = skill;
        } else {
            self.skills.push(skill);
        }
    }

    /// Load installed skills from a `SkillRegistry` into this router.
    pub fn load_from_registry(
        &mut self,
        registry: &SkillRegistry,
    ) -> Result<(), SkillRegistryError> {
        if let Ok(list) = registry.list() {
            let mut loaded_manifest_ids = BTreeSet::new();
            for item in list {
                if loaded_manifest_ids.insert((item.id.clone(), item.version.clone()))
                    && let Ok(manifest) = registry.show(&item.id)
                {
                    let base = registry
                        .managed_dir()
                        .join(item.id.replace('/', "__"))
                        .join(&item.version);
                    for def in manifest.to_skill_definitions(Some(&base)) {
                        self.register(def);
                    }
                }
            }
        }
        Ok(())
    }

    /// Calculate the score of a single skill against a task description.
    pub fn score_skill(skill: &SkillDefinition, task: &str) -> SkillScore {
        if skill.id == "SYSTEM" {
            return SkillScore {
                matched_triggers: vec!["system".to_string()],
                matched_keywords: vec![],
                score: u32::MAX,
            };
        }

        let task_lower = task.to_ascii_lowercase();
        let task_words: BTreeSet<&str> = task_lower
            .split(|c: char| {
                !c.is_alphanumeric() && c != '_' && c != '-' && c != '/' && c != '\\' && c != '.'
            })
            .filter(|w| w.len() >= 3 && !is_stopword(w))
            .collect();

        let mut matched_triggers = Vec::new();
        let mut matched_keywords = Vec::new();
        let mut score: u32 = 0;

        // 1. Exact trigger matches
        for trigger in &skill.triggers {
            let tr_lower = trigger.to_ascii_lowercase();
            if task_lower.contains(&tr_lower) {
                if !matched_triggers.contains(trigger) {
                    matched_triggers.push(trigger.clone());
                }
                score = score.saturating_add(100);
            }
        }

        // 2. Exact goal / id / name match
        let id_lower = skill.id.to_ascii_lowercase();
        let name_lower = skill.name.to_ascii_lowercase();
        if task_lower.contains(&id_lower)
            && !matched_triggers
                .iter()
                .any(|t| t.eq_ignore_ascii_case(&skill.id))
        {
            matched_triggers.push(skill.id.clone());
            score = score.saturating_add(50);
        } else if !name_lower.is_empty()
            && task_lower.contains(&name_lower)
            && !matched_triggers
                .iter()
                .any(|t| t.eq_ignore_ascii_case(&skill.name))
        {
            matched_triggers.push(skill.name.clone());
            score = score.saturating_add(50);
        }

        // 3. Keyword overlap from description, title, inputs, outputs
        let mut skill_corpus = format!("{} {}", skill.title, skill.description);
        for input in &skill.inputs {
            skill_corpus.push(' ');
            skill_corpus.push_str(input);
        }
        for output in &skill.outputs {
            skill_corpus.push(' ');
            skill_corpus.push_str(output);
        }
        let corpus_lower = skill_corpus.to_ascii_lowercase();
        let skill_words: BTreeSet<&str> = corpus_lower
            .split(|c: char| {
                !c.is_alphanumeric() && c != '_' && c != '-' && c != '/' && c != '\\' && c != '.'
            })
            .filter(|w| w.len() >= 3 && !is_stopword(w))
            .collect();

        for word in task_words {
            if skill_words.contains(word)
                && !matched_triggers
                    .iter()
                    .any(|t| t.to_ascii_lowercase().contains(word))
            {
                if !matched_keywords.contains(&word.to_string()) {
                    matched_keywords.push(word.to_string());
                }
                score = score.saturating_add(10);
            }
        }

        matched_triggers.sort();
        matched_keywords.sort();

        SkillScore {
            matched_triggers,
            matched_keywords,
            score,
        }
    }

    /// Score all registered skills against a task text.
    pub fn score_task(&self, task: &str) -> Vec<(&SkillDefinition, SkillScore)> {
        let mut scored: Vec<(&SkillDefinition, SkillScore)> = self
            .skills
            .iter()
            .map(|s| (s, Self::score_skill(s, task)))
            .collect();

        scored.sort_by(
            |(s_a, sc_a), (s_b, sc_b)| match sc_b.score.cmp(&sc_a.score) {
                std::cmp::Ordering::Equal => s_a.id.cmp(&s_b.id),
                other => other,
            },
        );

        scored
    }

    /// Perform full declarative routing given optional task, flags, host capabilities, and project root.
    pub fn route(
        &self,
        task: Option<&str>,
        flags: Option<&ContextFlags>,
        host: Option<&HostCapabilities>,
        _root: Option<&Utf8Path>,
    ) -> (SkillSelection, SkillSelectionSummary) {
        let task_text = task.unwrap_or("");
        let mut selection = SkillSelection::always();

        // 1. Collect and score all skills
        let mut candidates: Vec<(&SkillDefinition, SkillScore, bool)> = Vec::new();

        for skill in &self.skills {
            let score = if task_text.is_empty() {
                if skill.id == "SYSTEM" {
                    SkillScore {
                        matched_triggers: vec!["system".into()],
                        matched_keywords: vec![],
                        score: u32::MAX,
                    }
                } else {
                    SkillScore::default()
                }
            } else {
                Self::score_skill(skill, task_text)
            };

            let mut forced = false;
            if let Some(f) = flags {
                if (skill.id == "paper" || skill.name == "paper") && (f.paper || f.skill_paper) {
                    forced = true;
                }
                if (skill.id == "agent-code" || skill.id == "agent" || skill.name == "agent-code")
                    && (f.agent || f.skill_agent_code)
                {
                    forced = true;
                }
                for named in &f.skills {
                    let n = named.to_ascii_lowercase();
                    if skill.id.to_ascii_lowercase() == n
                        || skill.name.to_ascii_lowercase() == n
                        || skill.path.to_ascii_lowercase().ends_with(&n)
                        || n.contains(&skill.id.to_ascii_lowercase())
                    {
                        forced = true;
                    }
                }
            }

            candidates.push((skill, score, forced));
        }

        // 2. Deterministic sort:
        // Priority order:
        // a) SYSTEM always first
        // b) Active (forced || score > 0) before inactive
        // c) Score descending
        // d) Lexical ID ascending (deterministic tie-breaking)
        candidates.sort_by(|(s_a, score_a, forced_a), (s_b, score_b, forced_b)| {
            if s_a.id == "SYSTEM" {
                return std::cmp::Ordering::Less;
            }
            if s_b.id == "SYSTEM" {
                return std::cmp::Ordering::Greater;
            }

            let active_a = *forced_a || score_a.score > 0;
            let active_b = *forced_b || score_b.score > 0;

            match active_b.cmp(&active_a) {
                std::cmp::Ordering::Equal => match score_b.score.cmp(&score_a.score) {
                    std::cmp::Ordering::Equal => s_a.id.cmp(&s_b.id),
                    other => other,
                },
                other => other,
            }
        });

        // 3. Evaluate selection, capabilities, and conflicts
        let mut selected_skills = Vec::new();
        let mut active_skill_ids = Vec::new();
        let mut available_skill_ids = Vec::new();
        let mut all_conflicts = BTreeSet::new();
        let mut all_missing = BTreeSet::new();
        let mut selected_id_set = BTreeSet::new();

        for (skill, score, forced) in candidates {
            available_skill_ids.push(skill.id.clone());

            if skill.id == "SYSTEM" {
                selected_id_set.insert(skill.id.clone());
                active_skill_ids.push(skill.id.clone());
                selected_skills.push(SelectedSkillItem {
                    id: skill.id.clone(),
                    version: Some(skill.version.clone()),
                    status: SkillStatus::Selected,
                    reason: Some("Mandatory system instructions".into()),
                    path: Some(skill.path.clone()),
                    required_capabilities: skill.required_capabilities.clone(),
                    conflicts: skill.conflicts.clone(),
                });
                continue;
            }

            let is_matched = forced || score.score > 0;

            if !is_matched {
                selected_skills.push(SelectedSkillItem {
                    id: skill.id.clone(),
                    version: Some(skill.version.clone()),
                    status: SkillStatus::Available,
                    reason: Some("Available in registry (no matching task trigger)".into()),
                    path: Some(skill.path.clone()),
                    required_capabilities: skill.required_capabilities.clone(),
                    conflicts: skill.conflicts.clone(),
                });
                continue;
            }

            // Check conflicts
            let mut conflict_found = None;
            for conflict_id in &skill.conflicts {
                if selected_id_set.contains(conflict_id) {
                    conflict_found = Some(conflict_id.clone());
                    all_conflicts.insert(format!("{}:{}", skill.id, conflict_id));
                    break;
                }
            }

            if let Some(c) = conflict_found {
                selected_skills.push(SelectedSkillItem {
                    id: skill.id.clone(),
                    version: Some(skill.version.clone()),
                    status: SkillStatus::Incompatible,
                    reason: Some(format!("Conflicts with selected skill: {c}")),
                    path: Some(skill.path.clone()),
                    required_capabilities: skill.required_capabilities.clone(),
                    conflicts: skill.conflicts.clone(),
                });
                continue;
            }

            // Check host capabilities if host is provided
            let mut missing_caps = Vec::new();
            if let Some(h) = host {
                for req in &skill.required_capabilities {
                    if !h.supports(req) {
                        missing_caps.push(req.clone());
                        all_missing.insert(req.clone());
                    }
                }
            }

            if !missing_caps.is_empty() {
                selected_skills.push(SelectedSkillItem {
                    id: skill.id.clone(),
                    version: Some(skill.version.clone()),
                    status: SkillStatus::Incompatible,
                    reason: Some(format!(
                        "Missing required host capability: {}",
                        missing_caps.join(", ")
                    )),
                    path: Some(skill.path.clone()),
                    required_capabilities: skill.required_capabilities.clone(),
                    conflicts: skill.conflicts.clone(),
                });
                continue;
            }

            // Passed all checks: mark as Selected
            let reason = if forced {
                "Explicitly requested via flags".to_string()
            } else if !score.matched_triggers.is_empty() {
                format!(
                    "Matched task triggers: [{}]",
                    score.matched_triggers.join(", ")
                )
            } else if !score.matched_keywords.is_empty() {
                format!(
                    "Matched task keywords: [{}]",
                    score.matched_keywords.join(", ")
                )
            } else {
                "Selected for task".to_string()
            };

            selected_id_set.insert(skill.id.clone());
            active_skill_ids.push(skill.id.clone());
            selected_skills.push(SelectedSkillItem {
                id: skill.id.clone(),
                version: Some(skill.version.clone()),
                status: SkillStatus::Selected,
                reason: Some(reason),
                path: Some(skill.path.clone()),
                required_capabilities: skill.required_capabilities.clone(),
                conflicts: skill.conflicts.clone(),
            });
        }

        // 4. Special rule: if `review` is selected, also ensure `paper` is selected if available
        if selected_id_set.contains("review")
            && !selected_id_set.contains("paper")
            && let Some(paper_item) = selected_skills.iter_mut().find(|s| s.id == "paper")
            && paper_item.status == SkillStatus::Available
        {
            paper_item.status = SkillStatus::Selected;
            paper_item.reason = Some("Draft stage manuscript active".into());
            selected_id_set.insert("paper".to_string());
            active_skill_ids.push("paper".to_string());
        }

        // 5. Update SkillSelection boolean flags
        selection.paper = selected_id_set.contains("paper");
        selection.agent_code = selected_id_set.contains("agent-code")
            || selected_id_set.contains("agent_code")
            || selected_id_set.contains("agent");
        selection.review = selected_id_set.contains("review");
        selection.visualize_article = selected_id_set.contains("visualize-article")
            || selected_id_set.contains("visualize_article")
            || selected_id_set.contains("scientist-in-loop/visualize-article");

        for id in &active_skill_ids {
            if id != "SYSTEM"
                && id != "paper"
                && id != "agent-code"
                && id != "review"
                && id != "visualize-article"
                && id != "scientist-in-loop/visualize-article"
            {
                selection.dynamic_skills.push(id.clone());
            }
        }

        available_skill_ids.sort();

        let summary = SkillSelectionSummary {
            active_skill_ids,
            available_skill_ids,
            selected_skills,
            conflicts: all_conflicts.into_iter().collect(),
            missing_requirements: all_missing.into_iter().collect(),
            registry_version: Some("1.0.0".to_string()),
        };

        (selection, summary)
    }
}

/// Skill loading intent derived from a task description or flags.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillSelection {
    /// Always true when generating context.
    pub system: bool,
    /// Load paper.md.
    pub paper: bool,
    /// Load agent-code.md.
    pub agent_code: bool,
    /// Load review.md (manuscript estimate / peer-review skill).
    pub review: bool,
    /// Load the optional Visualize Article prompt skill.
    pub visualize_article: bool,
    /// Additional dynamically routed skill IDs.
    pub dynamic_skills: Vec<String>,
}

impl SkillSelection {
    /// Always-loaded baseline.
    pub fn always() -> Self {
        Self {
            system: true,
            paper: false,
            agent_code: false,
            review: false,
            visualize_article: false,
            dynamic_skills: Vec::new(),
        }
    }

    /// Apply declarative routing rules from a free-text task description.
    pub fn from_task(task: &str) -> Self {
        let router = SkillRouter::new();
        let (selection, _) = router.route(Some(task), None, None, None);
        selection
    }

    /// Apply declarative routing rules using a task description and an explicit SkillRegistry.
    pub fn from_task_and_registry(task: &str, registry: &SkillRegistry) -> Self {
        let mut router = SkillRouter::new();
        let _ = router.load_from_registry(registry);
        let (selection, _) = router.route(Some(task), None, None, Some(registry.root()));
        selection
    }

    /// Apply declarative routing rules with full host capability checking and explanation summary.
    pub fn from_task_and_registry_with_host(
        task: &str,
        registry: &SkillRegistry,
        host: Option<&HostCapabilities>,
    ) -> (Self, SkillSelectionSummary) {
        let mut router = SkillRouter::new();
        let _ = router.load_from_registry(registry);
        router.route(Some(task), None, host, Some(registry.root()))
    }

    /// Explain the selection decisions for this task as a structured SkillSelectionSummary.
    pub fn explain_from_task(task: &str, host: Option<&HostCapabilities>) -> SkillSelectionSummary {
        let router = SkillRouter::new();
        let (_, summary) = router.route(Some(task), None, host, None);
        summary
    }

    /// Merge explicit CLI flags.
    pub fn merge_flags(&mut self, flags: &ContextFlags) {
        if flags.skill_paper || flags.paper {
            self.paper = true;
        }
        if flags.skill_agent_code || flags.agent {
            self.agent_code = true;
        }
        for name in &flags.skills {
            let n = name.to_ascii_lowercase();
            if n.contains("paper") {
                self.paper = true;
            } else if n.contains("agent") {
                self.agent_code = true;
            } else if n.contains("review") || n.contains("estimate") {
                self.review = true;
            } else if n.contains("visualize") || n.contains("figure") || n.contains("diagram") {
                self.visualize_article = true;
            } else {
                self.dynamic_skills.push(name.clone());
            }
        }
    }
}

/// Load skill file contents from the project.
pub fn load_skill(root: &Utf8Path, name: &str) -> Result<String, ContextError> {
    let registry = crate::SkillRegistry::new(root);
    if let Ok(entries) = registry.list()
        && let Some(entry) = entries.iter().find(|entry| {
            entry.entrypoint == name || entry.path.ends_with(name) || entry.id == name
        })
    {
        return fs::read_to_string(entry.path.as_str())
            .map_err(|e| ContextError::Io(format!("{}: {e}", entry.path)));
    }
    let local = root.join("agent/skills/local").join(name);
    if local.is_file() {
        return fs::read_to_string(local.as_str())
            .map_err(|e| ContextError::Io(format!("{local}: {e}")));
    }
    let local_md = if !name.ends_with(".md") {
        root.join("agent/skills/local").join(format!("{name}.md"))
    } else {
        local.clone()
    };
    if local_md.is_file() {
        return fs::read_to_string(local_md.as_str())
            .map_err(|e| ContextError::Io(format!("{local_md}: {e}")));
    }
    let path = root.join(rel::SKILLS).join(name);
    if path.is_file() {
        return fs::read_to_string(path.as_str())
            .map_err(|e| ContextError::Io(format!("{path}: {e}")));
    }
    let path_md = if !name.ends_with(".md") {
        root.join(rel::SKILLS).join(format!("{name}.md"))
    } else {
        path.clone()
    };
    if path_md.is_file() {
        return fs::read_to_string(path_md.as_str())
            .map_err(|e| ContextError::Io(format!("{path_md}: {e}")));
    }
    Err(ContextError::MissingSkill(path.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;

    #[test]
    fn skill_selection_from_task() {
        let s = SkillSelection::from_task("edit paper_draft.tex introduction");
        assert!(s.system);
        assert!(s.paper);
        let s2 = SkillSelection::from_task("add a parser under agent/");
        assert!(s2.agent_code);
        let s3 = SkillSelection::from_task("list sources");
        assert!(!s3.paper);
        assert!(!s3.agent_code);
    }

    #[test]
    fn from_task_structure_and_completion() {
        assert!(SkillSelection::from_task("update structure.yaml").paper);
        assert!(SkillSelection::from_task("mark section completion").paper);
        assert!(SkillSelection::from_task("edit paper.tex").paper);
        assert!(SkillSelection::from_task("reproducibility script").agent_code);
    }

    #[test]
    fn merge_flags_enables_skills() {
        let mut s = SkillSelection::always();
        s.merge_flags(&ContextFlags {
            paper: true,
            agent: false,
            skill_paper: false,
            skill_agent_code: true,
            skills: vec!["paper.md".into()],
        });
        assert!(s.paper);
        assert!(s.agent_code);
    }

    #[test]
    fn merge_flags_skills_list() {
        let mut s = SkillSelection::always();
        s.merge_flags(&ContextFlags {
            paper: false,
            agent: false,
            skill_paper: false,
            skill_agent_code: false,
            skills: vec!["agent-code.md".into()],
        });
        assert!(s.agent_code);
        assert!(!s.paper);
    }

    #[test]
    fn always_has_system_only() {
        let s = SkillSelection::always();
        assert!(s.system);
        assert!(!s.paper);
        assert!(!s.agent_code);
        assert!(!s.review);
    }

    #[test]
    fn from_task_review_estimate() {
        let s = SkillSelection::from_task("estimate this paper");
        assert!(s.review);
        assert!(s.paper);
        let s2 = SkillSelection::from_task("peer review critique");
        assert!(s2.review);
    }

    #[test]
    fn visualize_article_triggers_route() {
        assert!(SkillSelection::from_task("design a figure architecture").visualize_article);
        let mut selection = SkillSelection::always();
        selection.merge_flags(&ContextFlags {
            skills: vec!["visualize-article".into()],
            ..Default::default()
        });
        assert!(selection.visualize_article);
    }

    #[test]
    fn load_skill_missing() {
        let dir = tempfile::tempdir().unwrap();
        let root = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let err = load_skill(&root, "SYSTEM.md").unwrap_err();
        assert!(matches!(err, ContextError::MissingSkill(_)));
    }

    #[test]
    fn load_skill_ok() {
        let dir = tempfile::tempdir().unwrap();
        let root = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        std::fs::create_dir_all(root.join("agent/skills")).unwrap();
        std::fs::write(root.join("agent/skills/SYSTEM.md"), "hello skill").unwrap();
        let text = load_skill(&root, "SYSTEM.md").unwrap();
        assert_eq!(text, "hello skill");
    }

    #[test]
    fn manifest_serialization_and_deserialization() {
        let skill = SkillDefinition {
            id: "custom-compiler".into(),
            name: "compiler".into(),
            version: "2.1.0".into(),
            title: "Custom Compiler Skill".into(),
            description: "Compiles specialized LaTeX macros and builds assets.".into(),
            path: "agent/skills/custom.md".into(),
            triggers: vec!["compile-custom".into(), "macro".into()],
            required_capabilities: vec!["tectonic".into(), "python".into()],
            inputs: vec!["macros.tex".into()],
            outputs: vec!["macros.pdf".into()],
            permissions: vec!["write:manuscript".into()],
            verification: Some("verify_macros".into()),
            conflicts: vec!["legacy-compiler".into()],
        };

        let yaml = serde_yaml::to_string(&skill).expect("serialize skill YAML");
        let parsed: SkillDefinition = serde_yaml::from_str(&yaml).expect("deserialize skill YAML");
        assert_eq!(skill, parsed);

        let json = serde_json::to_string(&skill).expect("serialize skill JSON");
        let parsed_json: SkillDefinition =
            serde_json::from_str(&json).expect("deserialize skill JSON");
        assert_eq!(skill, parsed_json);
    }

    #[test]
    fn markdown_frontmatter_parser() {
        let md = r#"---
id: custom-math
name: math
version: 1.2.0
title: Mathematical Derivations
description: Verifies mathematical derivations in paper draft.
triggers:
  - equation
  - derivation
required_capabilities:
  - python
---
# Mathematical Derivations
Instructions for proving theorems.
"#;
        let def = SkillDefinition::parse_from_markdown(md, "fallback-id").expect("parse markdown");
        assert_eq!(def.id, "custom-math");
        assert_eq!(def.name, "math");
        assert_eq!(def.version, "1.2.0");
        assert_eq!(def.triggers, vec!["equation", "derivation"]);
        assert_eq!(def.required_capabilities, vec!["python"]);

        // Fallback when no frontmatter
        let plain_md = "# Standard Heading\nSome instructions.";
        let fallback = SkillDefinition::parse_from_markdown(plain_md, "plain-skill").unwrap();
        assert_eq!(fallback.id, "plain-skill");
        assert_eq!(fallback.title, "Standard Heading");
    }

    #[test]
    fn scoring_and_tie_breaking_determinism() {
        let skill_a = SkillDefinition {
            id: "alpha-diagram".into(),
            name: "alpha".into(),
            version: "1.0.0".into(),
            title: "Alpha Diagram Generator".into(),
            description: "Generates diagrams for models".into(),
            path: "alpha.md".into(),
            triggers: vec!["diagram".into()],
            required_capabilities: vec![],
            inputs: vec![],
            outputs: vec![],
            permissions: vec![],
            verification: None,
            conflicts: vec![],
        };

        let skill_b = SkillDefinition {
            id: "beta-diagram".into(),
            name: "beta".into(),
            version: "1.0.0".into(),
            title: "Beta Diagram Generator".into(),
            description: "Generates diagrams for models".into(),
            path: "beta.md".into(),
            triggers: vec!["diagram".into()],
            required_capabilities: vec![],
            inputs: vec![],
            outputs: vec![],
            permissions: vec![],
            verification: None,
            conflicts: vec![],
        };

        let router = SkillRouter::with_skills(vec![skill_b, skill_a]);
        let scored = router.score_task("please generate a diagram");

        // Both matched trigger "diagram" with equal score
        assert_eq!(scored[0].1.score, 100);
        assert_eq!(scored[1].1.score, 100);
        // Tie-breaker: lexical id ordering guarantees alpha-diagram comes first
        assert_eq!(scored[0].0.id, "alpha-diagram");
        assert_eq!(scored[1].0.id, "beta-diagram");
    }

    #[test]
    fn capability_check_failure_handling() {
        let skill = SkillDefinition {
            id: "pdf-marker-tool".into(),
            name: "marker-extractor".into(),
            version: "1.0.0".into(),
            title: "Marker PDF Extractor".into(),
            description: "Extracts markdown from PDFs".into(),
            path: "marker.md".into(),
            triggers: vec!["pdf extraction".into(), "marker".into()],
            required_capabilities: vec!["marker".into(), "python".into()],
            inputs: vec![],
            outputs: vec![],
            permissions: vec![],
            verification: None,
            conflicts: vec![],
        };

        let router = SkillRouter::with_skills(vec![SkillDefinition::system(), skill]);

        // Host missing marker
        let host_without_marker = HostCapabilities {
            tools: ["python".into()].into(),
            process: true,
            ..Default::default()
        };

        let (selection, summary) = router.route(
            Some("run marker pdf extraction"),
            None,
            Some(&host_without_marker),
            None,
        );

        assert!(
            !selection
                .dynamic_skills
                .contains(&"pdf-marker-tool".to_string())
        );
        let marker_item = summary
            .selected_skills
            .iter()
            .find(|s| s.id == "pdf-marker-tool")
            .unwrap();
        assert_eq!(marker_item.status, SkillStatus::Incompatible);
        assert!(
            marker_item
                .reason
                .as_ref()
                .unwrap()
                .contains("Missing required host capability: marker"),
            "reason was: {:?}",
            marker_item.reason
        );
        assert!(summary.missing_requirements.contains(&"marker".to_string()));
    }

    #[test]
    fn conflict_resolution_marks_incompatible() {
        let skill_1 = SkillDefinition {
            id: "diagram-svg".into(),
            name: "diagram-svg".into(),
            version: "1.0.0".into(),
            title: "SVG Diagramming".into(),
            description: "Generates SVG diagrams".into(),
            path: "svg.md".into(),
            triggers: vec!["diagram".into()],
            required_capabilities: vec![],
            inputs: vec![],
            outputs: vec![],
            permissions: vec![],
            verification: None,
            conflicts: vec!["diagram-png".into()],
        };

        let skill_2 = SkillDefinition {
            id: "diagram-png".into(),
            name: "diagram-png".into(),
            version: "1.0.0".into(),
            title: "PNG Diagramming".into(),
            description: "Generates PNG diagrams".into(),
            path: "png.md".into(),
            triggers: vec!["diagram".into()],
            required_capabilities: vec![],
            inputs: vec![],
            outputs: vec![],
            permissions: vec![],
            verification: None,
            conflicts: vec!["diagram-svg".into()],
        };

        // diagram-png has lexical priority over diagram-svg, so diagram-png is selected first
        let router = SkillRouter::with_skills(vec![SkillDefinition::system(), skill_1, skill_2]);
        let (_, summary) = router.route(Some("make a diagram"), None, None, None);

        let png = summary
            .selected_skills
            .iter()
            .find(|s| s.id == "diagram-png")
            .unwrap();
        let svg = summary
            .selected_skills
            .iter()
            .find(|s| s.id == "diagram-svg")
            .unwrap();

        assert_eq!(png.status, SkillStatus::Selected);
        assert_eq!(svg.status, SkillStatus::Incompatible);
        assert!(
            svg.reason
                .as_ref()
                .unwrap()
                .contains("Conflicts with selected skill: diagram-png")
        );
        assert!(
            summary
                .conflicts
                .contains(&"diagram-svg:diagram-png".to_string())
        );
    }

    #[test]
    fn decision_explanation_output_matches_summary() {
        let summary = SkillSelection::explain_from_task(
            "estimate paper draft and design a figure",
            Some(&HostCapabilities::all_available()),
        );

        assert!(summary.active_skill_ids.contains(&"SYSTEM".to_string()));
        assert!(summary.active_skill_ids.contains(&"review".to_string()));
        assert!(summary.active_skill_ids.contains(&"paper".to_string()));
        assert!(
            summary
                .active_skill_ids
                .contains(&"visualize-article".to_string())
        );

        let review_item = summary
            .selected_skills
            .iter()
            .find(|s| s.id == "review")
            .unwrap();
        assert_eq!(review_item.status, SkillStatus::Selected);
        assert!(review_item.reason.is_some());
        assert!(review_item.path.is_some());

        assert!(
            summary
                .available_skill_ids
                .contains(&"agent-code".to_string())
        );
        let agent_code_item = summary
            .selected_skills
            .iter()
            .find(|s| s.id == "agent-code")
            .unwrap();
        assert_eq!(agent_code_item.status, SkillStatus::Available);
    }

    #[test]
    fn from_task_and_registry_integration() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let reg = SkillRegistry::new(&root);

        let (selection, summary) = SkillSelection::from_task_and_registry_with_host(
            "please write the introduction section in paper_draft.tex",
            &reg,
            Some(&HostCapabilities::all_available()),
        );

        assert!(selection.system);
        assert!(selection.paper);
        assert!(!selection.agent_code);
        assert!(summary.active_skill_ids.contains(&"SYSTEM".to_string()));
        assert!(summary.active_skill_ids.contains(&"paper".to_string()));
    }

    #[test]
    fn test_first_party_skill_templates_declarative_frontmatter() {
        let system_md = include_str!("../../../templates/agent/skills/SYSTEM.md");
        let system_def = SkillDefinition::parse_from_markdown(system_md, "SYSTEM").unwrap();
        assert_eq!(system_def.id, "SYSTEM");
        assert_eq!(system_def.version, "1.0.0");
        assert_eq!(system_def.title, "System Grounding Rules");
        assert_eq!(system_def.triggers, vec!["system"]);
        assert!(system_def.required_capabilities.is_empty());
        assert!(system_def.inputs.is_empty());
        assert!(system_def.outputs.is_empty());
        assert!(system_def.permissions.is_empty());
        assert_eq!(system_def.verification, None);

        let paper_md = include_str!("../../../templates/agent/skills/paper.md");
        let paper_def = SkillDefinition::parse_from_markdown(paper_md, "paper").unwrap();
        assert_eq!(paper_def.id, "paper");
        assert_eq!(paper_def.version, "1.0.0");
        assert_eq!(paper_def.title, "Manuscript Drafting and Structure");
        assert_eq!(
            paper_def.triggers,
            vec![
                "paper",
                "draft",
                "latex",
                "structure",
                "tex",
                "write",
                "section",
                "abstract",
                "introduction"
            ]
        );
        assert_eq!(paper_def.required_capabilities, vec!["tectonic"]);
        assert_eq!(paper_def.inputs, vec!["paper_draft.tex", "structure.yaml"]);
        assert_eq!(paper_def.outputs, vec!["paper_draft.tex", "structure.yaml"]);
        assert_eq!(
            paper_def.permissions,
            vec!["write:paper_draft.tex", "write:structure.yaml"]
        );
        assert_eq!(paper_def.verification.as_deref(), Some("sil build"));

        let agent_code_md = include_str!("../../../templates/agent/skills/agent-code.md");
        let agent_code_def =
            SkillDefinition::parse_from_markdown(agent_code_md, "agent-code").unwrap();
        assert_eq!(agent_code_def.id, "agent-code");
        assert_eq!(agent_code_def.version, "1.0.0");
        assert_eq!(agent_code_def.title, "Agent Reproducibility and Code");
        assert_eq!(
            agent_code_def.triggers,
            vec![
                "agent-code",
                "agent",
                "python",
                "script",
                "parser",
                "reproducibility"
            ]
        );
        assert_eq!(agent_code_def.required_capabilities, vec!["python", "git"]);
        assert_eq!(agent_code_def.inputs, vec!["agent/"]);
        assert_eq!(agent_code_def.outputs, vec!["agent/"]);
        assert_eq!(agent_code_def.permissions, vec!["write:agent/"]);
        assert_eq!(
            agent_code_def.verification.as_deref(),
            Some("python -m pytest")
        );

        let review_md = include_str!("../../../templates/agent/skills/review.md");
        let review_def = SkillDefinition::parse_from_markdown(review_md, "review").unwrap();
        assert_eq!(review_def.id, "review");
        assert_eq!(review_def.version, "1.0.0");
        assert_eq!(review_def.title, "Quality Gate and L0 Estimate");
        assert_eq!(
            review_def.triggers,
            vec!["review", "estimate", "rubric", "quality", "critique"]
        );
        assert!(review_def.required_capabilities.is_empty());
        assert_eq!(review_def.inputs, vec!["paper_draft.tex", "structure.yaml"]);
        assert_eq!(review_def.outputs, vec![".sil/reviews/"]);
        assert_eq!(review_def.permissions, vec!["write:.sil/reviews/"]);
        assert_eq!(review_def.verification.as_deref(), Some("sil estimate"));

        let visualize_md = include_str!("../packs/visualize-article/SKILL.md");
        let visualize_def =
            SkillDefinition::parse_from_markdown(visualize_md, "visualize-article").unwrap();
        assert_eq!(visualize_def.id, "visualize-article");
        assert_eq!(visualize_def.version, "1.0.0");
        assert_eq!(visualize_def.title, "Visualize Article");
        assert_eq!(
            visualize_def.required_capabilities,
            vec!["network", "resources"]
        );
        assert_eq!(visualize_def.inputs, vec!["paper_draft.tex"]);
        assert_eq!(visualize_def.outputs, vec!["figures/"]);
        assert_eq!(
            visualize_def.permissions,
            vec![
                "read:manuscript",
                "read:figures",
                "network:external_image_provider"
            ]
        );
        assert_eq!(visualize_def.verification.as_deref(), Some("check_figures"));
    }
}
