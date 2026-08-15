//! Versioned canonical venue identities and deterministic alias resolution.
#![allow(missing_docs)]

use html_escape::decode_html_entities;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use unicode_normalization::UnicodeNormalization;

pub const CATALOGUE_SCHEMA_VERSION: u32 = 1;
pub const CATALOGUE_VERSION: &str = "2026.08.15-seed";
pub const NORMALIZER_VERSION: u32 = 1;

/// Load the reviewed seed catalogue shipped with the crate.
pub fn builtin_catalogue() -> Result<Catalogue, Vec<CatalogueIssue>> {
    Catalogue::from_yaml(include_str!("../data/venues.yaml"))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VenueId(pub String);
impl VenueId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VenueKind {
    ConferenceSeries,
    ConferenceEdition,
    WorkshopSeries,
    WorkshopEdition,
    Journal,
    Repository,
    HostingPlatform,
    Track,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VenueParent {
    pub id: VenueId,
    #[serde(default)]
    pub relation: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VenueEdition {
    pub year: i32,
    #[serde(default)]
    pub label: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VenueTrack {
    pub id: VenueId,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalId {
    pub namespace: String,
    pub value: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub url: String,
    pub evidence_type: String,
    pub curated_by: String,
    pub reviewed_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AliasKind {
    Official,
    HistoricalAcronym,
    ProceedingsTitle,
    CommonAbbreviation,
    WorkshopName,
    PlatformName,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VenueAlias {
    pub value: String,
    pub kind: AliasKind,
    #[serde(default)]
    pub valid_from: Option<i32>,
    #[serde(default)]
    pub valid_to: Option<i32>,
    pub provenance: Provenance,
    #[serde(default)]
    pub context: Vec<String>,
    #[serde(default)]
    pub additional_provenance: Vec<Provenance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Venue {
    pub id: VenueId,
    pub canonical_name: String,
    pub short_name: String,
    pub kind: VenueKind,
    #[serde(default)]
    pub parent: Option<VenueParent>,
    #[serde(default)]
    pub editions: Vec<VenueEdition>,
    #[serde(default)]
    pub tracks: Vec<VenueTrack>,
    pub aliases: Vec<VenueAlias>,
    #[serde(default)]
    pub external_ids: Vec<ExternalId>,
    #[serde(default)]
    pub collections: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalogue {
    pub schema_version: u32,
    pub catalogue_version: String,
    pub normalizer_version: u32,
    pub venues: Vec<Venue>,
    #[serde(default)]
    pub collections: Vec<Collection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogueIssue {
    pub code: String,
    pub venue_id: Option<VenueId>,
    pub alias: Option<String>,
    pub message: String,
}

impl Catalogue {
    pub fn from_yaml(yaml: &str) -> Result<Self, Vec<CatalogueIssue>> {
        let catalogue: Self = serde_yaml::from_str(yaml)
            .map_err(|e| vec![issue("yaml.invalid", e.to_string(), None, None)])?;
        catalogue.validate().map(|_| catalogue)
    }

    pub fn validate(&self) -> Result<(), Vec<CatalogueIssue>> {
        let mut issues = Vec::new();
        if self.schema_version != CATALOGUE_SCHEMA_VERSION {
            issues.push(issue(
                "schema.version",
                "unsupported schema version",
                None,
                None,
            ));
        }
        if self.catalogue_version.trim().is_empty() {
            issues.push(issue(
                "catalogue.version",
                "catalogue version is required",
                None,
                None,
            ));
        }
        if self.normalizer_version != NORMALIZER_VERSION {
            issues.push(issue(
                "normalizer.version",
                "unsupported normalizer version",
                None,
                None,
            ));
        }
        let ids: BTreeSet<_> = self.venues.iter().map(|v| v.id.clone()).collect();
        if ids.len() != self.venues.len() {
            issues.push(issue(
                "venue.duplicate_id",
                "venue IDs must be unique",
                None,
                None,
            ));
        }
        for venue in &self.venues {
            if venue.id.0.trim().is_empty() || !venue.id.0.contains('.') {
                issues.push(issue(
                    "venue.id",
                    "venue ID must be a non-empty dotted stable ID",
                    Some(venue.id.clone()),
                    None,
                ));
            }
            if venue.canonical_name.trim().is_empty() || venue.short_name.trim().is_empty() {
                issues.push(issue(
                    "venue.name",
                    "canonical and short names are required",
                    Some(venue.id.clone()),
                    None,
                ));
            }
            if let Some(parent) = &venue.parent
                && !ids.contains(&parent.id)
            {
                issues.push(issue(
                    "venue.parent_missing",
                    "parent ID does not exist",
                    Some(venue.id.clone()),
                    None,
                ));
            }
            for alias in &venue.aliases {
                if alias.value.trim().is_empty() {
                    issues.push(issue(
                        "alias.empty",
                        "alias value is required",
                        Some(venue.id.clone()),
                        None,
                    ));
                }
                if alias
                    .valid_from
                    .zip(alias.valid_to)
                    .is_some_and(|(a, b)| a > b)
                {
                    issues.push(issue(
                        "alias.validity",
                        "valid_from must not be after valid_to",
                        Some(venue.id.clone()),
                        Some(alias.value.clone()),
                    ));
                }
                if alias.provenance.url.trim().is_empty()
                    || alias.provenance.curated_by.trim().is_empty()
                    || alias.provenance.reviewed_at.trim().is_empty()
                {
                    issues.push(issue(
                        "alias.provenance",
                        "alias provenance URL, curator, and review date are required",
                        Some(venue.id.clone()),
                        Some(alias.value.clone()),
                    ));
                }
                if normalize(&alias.value).chars().count() <= 4
                    && alias.context.is_empty()
                    && alias.additional_provenance.is_empty()
                {
                    issues.push(issue(
                        "alias.short_evidence",
                        "short aliases require context or independent evidence",
                        Some(venue.id.clone()),
                        Some(alias.value.clone()),
                    ));
                }
            }
        }
        let mut aliases: BTreeMap<String, Vec<&Venue>> = BTreeMap::new();
        for venue in &self.venues {
            for alias in &venue.aliases {
                aliases
                    .entry(normalize(&alias.value))
                    .or_default()
                    .push(venue);
            }
        }
        for (alias, matches) in aliases {
            if matches.iter().map(|v| &v.id).collect::<BTreeSet<_>>().len() > 1 {
                let constrained = matches.iter().all(|v| {
                    v.aliases
                        .iter()
                        .filter(|a| normalize(&a.value) == alias)
                        .all(|a| !a.context.is_empty() || !a.additional_provenance.is_empty())
                });
                if !constrained {
                    issues.push(issue(
                        "alias.collision",
                        "normalized alias collides without context or independent evidence",
                        None,
                        Some(alias),
                    ));
                }
            }
        }
        for venue in &self.venues {
            let mut seen = BTreeSet::new();
            let mut current = Some(&venue.id);
            while let Some(id) = current {
                if !seen.insert(id) {
                    issues.push(issue(
                        "venue.parent_cycle",
                        "parent graph contains a cycle",
                        Some(venue.id.clone()),
                        None,
                    ));
                    break;
                }
                current = self
                    .venues
                    .iter()
                    .find(|v| &v.id == id)
                    .and_then(|v| v.parent.as_ref())
                    .map(|p| &p.id);
            }
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }

    pub fn resolve(&self, raw: &str, year: Option<i32>, context: &[&str]) -> Resolution {
        let normalized = normalize(raw);
        let mut candidates = BTreeSet::new();
        let mut evidence = Vec::new();
        for venue in &self.venues {
            for alias in &venue.aliases {
                if normalize(&alias.value) == normalized
                    && valid_year(alias, year)
                    && context_matches(alias, context)
                {
                    candidates.insert(venue.id.clone());
                    evidence.push(ResolutionEvidence {
                        venue_id: venue.id.clone(),
                        alias: alias.value.clone(),
                        provenance: alias.provenance.clone(),
                    });
                }
            }
        }
        let status = match candidates.len() {
            0 => ResolutionStatus::Unknown,
            1 => ResolutionStatus::Resolved,
            _ => ResolutionStatus::Ambiguous,
        };
        let selected_alias = if status == ResolutionStatus::Resolved {
            evidence.first().map(|e| e.alias.clone())
        } else {
            None
        };
        Resolution {
            raw: raw.into(),
            normalized,
            status,
            candidate_ids: candidates.into_iter().collect(),
            selected_alias,
            evidence,
            catalogue_version: self.catalogue_version.clone(),
            normalizer_version: self.normalizer_version,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Normalizer;
impl Normalizer {
    pub fn normalize(&self, value: &str) -> String {
        normalize(value)
    }
    pub const fn version(&self) -> u32 {
        NORMALIZER_VERSION
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResolutionStatus {
    Resolved,
    Ambiguous,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionEvidence {
    pub venue_id: VenueId,
    pub alias: String,
    pub provenance: Provenance,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    pub raw: String,
    pub normalized: String,
    pub status: ResolutionStatus,
    pub candidate_ids: Vec<VenueId>,
    pub selected_alias: Option<String>,
    pub evidence: Vec<ResolutionEvidence>,
    pub catalogue_version: String,
    pub normalizer_version: u32,
}

fn issue(
    code: &str,
    message: impl Into<String>,
    venue_id: Option<VenueId>,
    alias: Option<String>,
) -> CatalogueIssue {
    CatalogueIssue {
        code: code.into(),
        venue_id,
        alias,
        message: message.into(),
    }
}
fn valid_year(a: &VenueAlias, year: Option<i32>) -> bool {
    year.is_none_or(|y| a.valid_from.is_none_or(|v| y >= v) && a.valid_to.is_none_or(|v| y <= v))
}
fn context_matches(a: &VenueAlias, context: &[&str]) -> bool {
    a.context.is_empty()
        || a.context
            .iter()
            .any(|c| context.iter().any(|x| normalize(c) == normalize(x)))
}

fn normalize(value: &str) -> String {
    let decoded = decode_html_entities(value)
        .replace("\\&", "&")
        .replace("\\textampersand", "&")
        .replace('&', " and ");
    decoded
        .nfkc()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            '&' => ' ',
            '\u{2010}'..='\u{2015}'
            | '\u{2212}'
            | '_'
            | '/'
            | ':'
            | ';'
            | ','
            | '.'
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '\''
            | '"' => ' ',
            c if c.is_whitespace() => ' ',
            c => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p() -> Provenance {
        Provenance {
            url: "https://example.test/evidence".into(),
            evidence_type: "official".into(),
            curated_by: "test".into(),
            reviewed_at: "2026-08-15".into(),
        }
    }
    fn v(id: &str, aliases: &[&str]) -> Venue {
        Venue {
            id: VenueId::new(id),
            canonical_name: id.into(),
            short_name: id.into(),
            kind: VenueKind::ConferenceSeries,
            parent: None,
            editions: vec![],
            tracks: vec![],
            aliases: aliases
                .iter()
                .map(|x| VenueAlias {
                    value: (*x).into(),
                    kind: AliasKind::Official,
                    valid_from: None,
                    valid_to: None,
                    provenance: p(),
                    context: vec!["conference".into()],
                    additional_provenance: vec![],
                })
                .collect(),
            external_ids: vec![],
            collections: vec![],
        }
    }
    fn c(venues: Vec<Venue>) -> Catalogue {
        Catalogue {
            schema_version: 1,
            catalogue_version: "test".into(),
            normalizer_version: 1,
            venues,
            collections: vec![],
        }
    }
    #[test]
    fn normalization_is_unicode_aware_and_idempotent() {
        let n = Normalizer;
        let x = n.normalize("  NIPS &amp; Proceedings – 2024  ");
        assert_eq!(x, n.normalize(&x));
        assert_eq!(x, "nips and proceedings 2024");
        assert_eq!(
            n.normalize("Research & Development"),
            n.normalize("Research and Development")
        );
        assert_eq!(n.normalize("\u{212B}"), "å");
    }
    #[test]
    fn resolves_exact_alias_and_preserves_raw() {
        let cat = c(vec![v("conf.neurips", &["NIPS", "NeurIPS"])]);
        let r = cat.resolve(" NIPS ", None, &["conference"]);
        assert_eq!(r.status, ResolutionStatus::Resolved);
        assert_eq!(r.candidate_ids, vec![VenueId::new("conf.neurips")]);
        assert_eq!(r.raw, " NIPS ");
    }
    #[test]
    fn ambiguity_does_not_select() {
        let cat = c(vec![v("conf.acl", &["ACL"]), v("journal.acl", &["ACL"])]);
        let r = cat.resolve("ACL", None, &["conference"]);
        assert_eq!(r.status, ResolutionStatus::Ambiguous);
        assert!(r.selected_alias.is_none());
    }
    #[test]
    fn no_substring_guessing() {
        let cat = c(vec![v("journal.nature", &["Nature"])]);
        assert_eq!(
            cat.resolve("Nature Machine Intelligence", None, &[]).status,
            ResolutionStatus::Unknown
        );
    }
    #[test]
    fn validation_catches_cycles_and_collisions() {
        let mut a = v("conf.a", &["same"]);
        a.aliases[0].context.clear();
        a.parent = Some(VenueParent {
            id: VenueId::new("conf.b"),
            relation: None,
        });
        let mut b = v("conf.b", &["same"]);
        b.aliases[0].context.clear();
        b.parent = Some(VenueParent {
            id: VenueId::new("conf.a"),
            relation: None,
        });
        let errors = c(vec![a, b]).validate().unwrap_err();
        assert!(errors.iter().any(|e| e.code == "alias.collision"));
        assert!(errors.iter().any(|e| e.code == "venue.parent_cycle"));
    }
    #[test]
    fn builtin_catalogue_validates_and_keeps_platforms_distinct() {
        let cat = builtin_catalogue().expect("seed catalogue should validate");
        assert!(cat.venues.len() >= 8);
        assert_eq!(
            cat.resolve("OpenReview", None, &[]).candidate_ids,
            vec![VenueId::new("platform.openreview")]
        );
        assert_eq!(
            cat.resolve("Nature Machine Intelligence", None, &[]).status,
            ResolutionStatus::Resolved
        );
    }
    #[test]
    fn validity_window_filters_alias() {
        let mut venue = v("conf.nips", &["NIPS"]);
        venue.aliases[0].valid_to = Some(2017);
        let cat = c(vec![venue]);
        assert_eq!(
            cat.resolve("NIPS", Some(2018), &["conference"]).status,
            ResolutionStatus::Unknown
        );
        assert_eq!(
            cat.resolve("NIPS", Some(2017), &["conference"]).status,
            ResolutionStatus::Resolved
        );
    }

    #[test]
    fn hard_aliases_and_name_variants_resolve_without_substring_matching() {
        let cat = builtin_catalogue().unwrap();
        for (raw, year) in [
            ("NIPS", Some(2017)),
            ("NeurIPS", Some(2024)),
            (
                "Advances in Neural Information Processing Systems",
                Some(2024),
            ),
        ] {
            assert_eq!(
                cat.resolve(raw, year, &["conference"]).status,
                ResolutionStatus::Resolved,
                "failed to resolve {raw}"
            );
        }
        assert_eq!(
            cat.resolve("Nature Machine Intelligence", None, &[]).status,
            ResolutionStatus::Resolved
        );
        assert_eq!(
            cat.resolve("Nature", None, &[]).status,
            ResolutionStatus::Resolved
        );
        assert_ne!(
            cat.resolve("Nature", None, &[]).candidate_ids,
            cat.resolve("Nature Machine Intelligence", None, &[])
                .candidate_ids
        );
    }

    #[test]
    #[ignore = "Stage 15 acceptance gate: seed catalogue must reach 200-300 venues and 1,000 aliases"]
    fn catalogue_acceptance_target_is_met() {
        let cat = builtin_catalogue().unwrap();
        let aliases = cat
            .venues
            .iter()
            .map(|venue| venue.aliases.len())
            .sum::<usize>();
        assert!((200..=300).contains(&cat.venues.len()));
        assert!(aliases >= 1_000);
    }
}
