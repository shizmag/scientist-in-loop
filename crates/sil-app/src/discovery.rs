//! Candidate discovery orchestration and deterministic candidate policy.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sil_api::{Cancellation, DiscoveryProvider, DiscoveryRequest, RawRecord};
use sil_core::{ResolutionStatus, builtin_catalogue, venue::Normalizer};
use sil_db::{
    Candidate, CandidateState, DiscoveryRun, ProviderRecord, ProviderRequest, SilDb, Work,
    WorkIdentifier, WorkVenue, WorkVersion,
};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::thread;

/// Configuration for one offline-testable discovery run.
#[derive(Debug, Clone)]
pub struct DiscoveryOptions {
    /// Provider-neutral query.
    pub request: DiscoveryRequest,
    /// Maximum number of providers executing at once.
    pub max_concurrency: usize,
    /// Stable run identifier.
    pub run_id: String,
    /// Timestamp supplied by the caller for reproducible tests.
    pub created_at: String,
}

/// A proposed relation that was deliberately not used to merge records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityRelation {
    /// First stable record identity.
    pub left: String,
    /// Second stable record identity.
    pub right: String,
    /// Why the relation was proposed.
    pub reason: String,
}

/// The result of conservative identity resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityDecision {
    /// Stable identity key, or `provider:<provider>:<id>` fallback.
    pub key: String,
    /// The identity rule that produced the key.
    pub basis: String,
    /// Non-merging similarity observations.
    pub proposals: Vec<IdentityRelation>,
}

/// Durable discovery outcome and the records retained for ranking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryResult {
    /// Run status: `complete`, `partial`, or `failed`.
    pub status: String,
    /// Provider errors retained in result order.
    pub errors: Vec<sil_api::PartialError>,
    /// Candidate IDs created by this run.
    pub candidate_ids: Vec<String>,
    /// Identity decisions, in stable record order.
    pub identities: Vec<IdentityDecision>,
}

/// Stored integer ranking components. Values are fixed-point points, not floats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankingComponents {
    /// Token lexical relevance.
    pub lexical_relevance: i64,
    /// Exact phrase match.
    pub exact_phrase: i64,
    /// Explicit requested venue collection match.
    pub venue_collection_match: i64,
    /// Identifier/provider consensus.
    pub provider_identifier_consensus: i64,
    /// Recency component.
    pub recency: i64,
    /// Observed citation signal.
    pub observed_citation_signal: i64,
    /// Open-access availability.
    pub oa_availability: i64,
}

impl RankingComponents {
    /// Sum all visible components.
    pub fn total(&self) -> i64 {
        self.lexical_relevance
            + self.exact_phrase
            + self.venue_collection_match
            + self.provider_identifier_consensus
            + self.recency
            + self.observed_citation_signal
            + self.oa_availability
    }
}

/// A deterministic ranked candidate with an inspectable explanation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankedCandidate {
    /// Stable candidate identifier.
    pub candidate_id: String,
    /// Stable canonical work identifier.
    pub work_id: String,
    /// Display title.
    pub title: String,
    /// Optional publication year.
    pub year: Option<i32>,
    /// Normalized title used by the tie breaker.
    pub normalized_title: String,
    /// Fixed-point components.
    pub components: RankingComponents,
    /// Total fixed-point score.
    pub score: i64,
}

/// Input to the pure ranker. It intentionally contains no database handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateForRanking {
    /// Stable candidate identifier.
    pub candidate_id: String,
    /// Stable work identifier.
    pub work_id: String,
    /// Work title.
    pub title: String,
    /// Publication year.
    pub year: Option<i32>,
    /// Raw venue collection membership.
    pub venue_collection_match: bool,
    /// Number of distinct providers or agreeing identifiers.
    pub provider_identifier_consensus: usize,
    /// Observed citation count.
    pub citation_count: Option<u64>,
    /// Whether open access was observed.
    pub open_access: Option<bool>,
}

/// Resolve a provider record without treating fuzzy metadata as identity.
pub fn identify(record: &RawRecord) -> IdentityDecision {
    let mut ids = record.identifiers.clone();
    ids.extend(record.source_external_ids.clone());
    for (namespace, value) in &ids {
        if namespace.eq_ignore_ascii_case("doi") {
            let value = normalize_doi(value);
            if !value.is_empty() {
                return decision(format!("doi:{value}"), "normalized_doi");
            }
        }
    }
    for (namespace, value) in &ids {
        if namespace.eq_ignore_ascii_case("arxiv") || namespace.eq_ignore_ascii_case("arxiv_id") {
            let value = normalize_arxiv(value);
            if !value.is_empty() {
                return decision(format!("arxiv:{value}"), "arxiv_base_version");
            }
        }
    }
    for namespace in [
        "openreview_forum",
        "openreview_cross_id",
        "forum",
        "cross_id",
    ] {
        if let Some(value) = ids.get(namespace).filter(|v| !v.trim().is_empty()) {
            return decision(
                format!("openreview:{}", normalize_id(value)),
                "openreview_forum_or_cross_id",
            );
        }
    }
    if let Some((namespace, value)) = ids.iter().next() {
        return decision(
            format!("{namespace}:{}", normalize_id(value)),
            "provider_identifier",
        );
    }
    decision(
        format!(
            "provider:{}:{}",
            record.provenance.provider,
            normalize_id(&record.provider_record_id)
        ),
        "provider_record_id",
    )
}

fn decision(key: String, basis: &str) -> IdentityDecision {
    IdentityDecision {
        key,
        basis: basis.into(),
        proposals: Vec::new(),
    }
}

/// Rank candidates using only explicit fixed-point components.
pub fn rank_candidates(query: &str, candidates: &[CandidateForRanking]) -> Vec<RankedCandidate> {
    let query_norm = normalize_text(query);
    let query_tokens: BTreeSet<_> = query_norm.split_whitespace().collect();
    let phrase = !query_norm.is_empty();
    let mut ranked = candidates
        .iter()
        .map(|candidate| {
            let title = normalize_text(&candidate.title);
            let title_tokens: BTreeSet<_> = title.split_whitespace().collect();
            let lexical = if query_tokens.is_empty() {
                0
            } else {
                (query_tokens.intersection(&title_tokens).count() as i64 * 30).min(120)
            };
            let exact = if phrase && title.contains(&query_norm) {
                100
            } else {
                0
            };
            let venue = if candidate.venue_collection_match {
                100
            } else {
                0
            };
            let consensus = (candidate.provider_identifier_consensus.min(4) as i64) * 25;
            let recency = candidate
                .year
                .map(|year| ((year - 2000).clamp(0, 40) * 2) as i64)
                .unwrap_or(0);
            let citation = candidate
                .citation_count
                .map(|n| ((n.min(1000) as f64).sqrt() * 3.0) as i64)
                .unwrap_or(0);
            let oa = i64::from(candidate.open_access == Some(true)) * 75;
            let components = RankingComponents {
                lexical_relevance: lexical,
                exact_phrase: exact,
                venue_collection_match: venue,
                provider_identifier_consensus: consensus,
                recency,
                observed_citation_signal: citation,
                oa_availability: oa,
            };
            RankedCandidate {
                candidate_id: candidate.candidate_id.clone(),
                work_id: candidate.work_id.clone(),
                title: candidate.title.clone(),
                year: candidate.year,
                normalized_title: title,
                score: components.total(),
                components,
            }
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| b.year.is_some().cmp(&a.year.is_some()))
            .then_with(|| b.year.cmp(&a.year))
            .then_with(|| a.normalized_title.cmp(&b.normalized_title))
            .then_with(|| a.work_id.cmp(&b.work_id))
    });
    ranked
}

/// Explicitly transition one lifecycle dimension through the application boundary.
pub fn transition_candidate(
    ctx: &crate::AppContext,
    candidate_id: &str,
    dimension: &str,
    to: CandidateState,
    actor: &str,
    reason: &str,
) -> Result<(), crate::AppError> {
    let db = SilDb::open(&ctx.paths.db()).map_err(|e| crate::AppError::Message(e.to_string()))?;
    db.transition_candidate(candidate_id, dimension, to, actor, reason)
        .map_err(|e| crate::AppError::Message(e.to_string()))
}

/// Rank a frozen input and persist every visible component for later explanation.
pub fn rank_and_store(
    ctx: &crate::AppContext,
    run_id: &str,
    algorithm_version: &str,
    query: &str,
    candidates: &[CandidateForRanking],
) -> Result<Vec<RankedCandidate>, crate::AppError> {
    let ranked = rank_candidates(query, candidates);
    let db = SilDb::open(&ctx.paths.db()).map_err(|e| crate::AppError::Message(e.to_string()))?;
    for candidate in &ranked {
        db.insert_candidate_ranking(
            run_id,
            &candidate.candidate_id,
            algorithm_version,
            candidate.score,
            &serde_json::to_string(&candidate.components)
                .map_err(|e| crate::AppError::Message(e.to_string()))?,
        )
        .map_err(|e| crate::AppError::Message(e.to_string()))?;
    }
    Ok(ranked)
}

/// Run discovery and persist discovery data. This function never reads or writes `references.bib`.
pub fn discover_candidates<C: Cancellation>(
    ctx: &crate::AppContext,
    options: &DiscoveryOptions,
    providers: &[Arc<dyn DiscoveryProvider>],
    cancellation: &C,
) -> Result<DiscoveryResult, crate::AppError> {
    let db = SilDb::open(&ctx.paths.db()).map_err(|e| crate::AppError::Message(e.to_string()))?;
    db.create_discovery_run(&DiscoveryRun {
        id: options.run_id.clone(),
        query: options.request.query.clone(),
        status: "running".into(),
        cursor_json: None,
        created_at: options.created_at.clone(),
    })
    .map_err(|e| crate::AppError::Message(e.to_string()))?;
    let limit = options.max_concurrency.max(1);
    let mut all = Vec::new();
    let mut errors = Vec::new();
    for batch in providers.chunks(limit) {
        thread::scope(|scope| {
            let mut joins = Vec::new();
            for provider in batch {
                let provider = Arc::clone(provider);
                joins.push(scope.spawn(move || provider.discover(&options.request, cancellation)));
            }
            for join in joins {
                match join.join().expect("discovery provider thread panicked") {
                    Ok(records) => all.extend(records),
                    Err(error) => errors.push(error),
                }
            }
        });
    }
    all.sort_by_key(|r| {
        (
            r.provenance.provider.clone(),
            r.provider_record_id.clone(),
            r.raw_payload.clone(),
        )
    });
    let catalogue = builtin_catalogue().map_err(|issues| {
        crate::AppError::Message(format!("venue catalogue invalid: {} issues", issues.len()))
    })?;
    let mut identities = Vec::new();
    let mut candidate_ids = Vec::new();
    let mut seen = BTreeSet::new();
    let mut seen_requests = BTreeSet::new();
    for (index, record) in all.iter().enumerate() {
        let identity = identify(record);
        identities.push(identity.clone());
        let request_id = stable_id(
            "request",
            &format!("{}:{}", options.run_id, record.provenance.request_url),
        );
        let record_id = stable_id(
            "record",
            &format!(
                "{}:{}:{}:{}",
                options.run_id,
                record.provenance.provider,
                record.provider_record_id,
                record.provenance.payload_sha256
            ),
        );
        if seen_requests.insert(request_id.clone()) {
            db.insert_provider_request(&ProviderRequest {
                id: request_id.clone(),
                run_id: options.run_id.clone(),
                provider: record.provenance.provider.clone(),
                request_json: serde_json::to_string(&options.request).unwrap_or_default(),
                cursor: options.request.cursor.clone(),
                status: "ok".into(),
            })
            .map_err(|e| crate::AppError::Message(e.to_string()))?;
        }
        db.insert_provider_record(&ProviderRecord {
            id: record_id.clone(),
            run_id: options.run_id.clone(),
            request_id,
            provider: record.provenance.provider.clone(),
            provider_record_id: record.provider_record_id.clone(),
            raw_payload: record.raw_payload.clone(),
            raw_payload_hash: record.provenance.payload_sha256.clone(),
            status: "ok".into(),
        })
        .map_err(|e| crate::AppError::Message(e.to_string()))?;
        if !seen.insert(identity.key.clone()) {
            continue;
        }
        let work_id = stable_id("work", &identity.key);
        let version_id = stable_id(
            "version",
            &format!(
                "{}:{}:{}",
                identity.key,
                record.work_type.as_deref().unwrap_or("unknown"),
                record.year.map_or_else(|| "".into(), |y| y.to_string())
            ),
        );
        let candidate_id = stable_id("candidate", &format!("{}:{}", options.run_id, version_id));
        db.upsert_work(&Work {
            id: work_id.clone(),
            title: record.title.clone().unwrap_or_default(),
            abstract_text: record.abstract_text.clone(),
            authors_json: serde_json::to_string(&record.authors).ok(),
            year: record.year,
        })
        .map_err(|e| crate::AppError::Message(e.to_string()))?;
        for (namespace, value) in &record.identifiers {
            db.insert_work_identifier(&WorkIdentifier {
                work_id: work_id.clone(),
                namespace: namespace.clone(),
                value: normalize_id(value),
                observed_by: Some(record.provenance.provider.clone()),
            })
            .map_err(|e| crate::AppError::Message(e.to_string()))?;
        }
        db.insert_work_version(&WorkVersion {
            id: version_id.clone(),
            work_id,
            version_kind: record.work_type.clone().unwrap_or_else(|| "unknown".into()),
            title: record.title.clone(),
            published_at: record.year.map(|y| y.to_string()),
            url: Some(record.provenance.request_url.clone()),
            open_access: None,
        })
        .map_err(|e| crate::AppError::Message(e.to_string()))?;
        if let Some(raw_venue) = &record.raw_venue {
            let resolution = catalogue.resolve(raw_venue, record.year, &[]);
            db.insert_work_venue(&WorkVenue {
                id: stable_id("venue", &format!("{version_id}:{raw_venue}")),
                version_id: version_id.clone(),
                venue_id: (resolution.status == ResolutionStatus::Resolved)
                    .then(|| resolution.candidate_ids[0].as_str().to_string()),
                raw_venue: raw_venue.clone(),
                normalized_venue: Some(resolution.normalized),
                resolution_status: match resolution.status {
                    ResolutionStatus::Resolved => "resolved",
                    ResolutionStatus::Ambiguous => "ambiguous",
                    ResolutionStatus::Unknown => "unknown",
                }
                .into(),
                evidence_json: serde_json::to_string(&resolution.evidence).ok(),
                catalogue_version: Some(resolution.catalogue_version),
                normalizer_version: Some(Normalizer.version() as i64),
            })
            .map_err(|e| crate::AppError::Message(e.to_string()))?;
        }
        db.insert_candidate(&Candidate {
            id: candidate_id.clone(),
            run_id: options.run_id.clone(),
            version_id,
            provider_record_id: Some(record_id),
            resolution: CandidateState::New,
            disposition: CandidateState::New,
            acquisition: CandidateState::New,
        })
        .map_err(|e| crate::AppError::Message(e.to_string()))?;
        candidate_ids.push(candidate_id);
        let _ = index;
    }
    let status = if errors.is_empty() {
        "complete"
    } else if all.is_empty() {
        "failed"
    } else {
        "partial"
    };
    db.update_discovery_run(&options.run_id, status, options.request.cursor.as_deref())
        .map_err(|e| crate::AppError::Message(e.to_string()))?;
    Ok(DiscoveryResult {
        status: status.into(),
        errors,
        candidate_ids,
        identities,
    })
}

fn stable_id(prefix: &str, value: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(value.as_bytes());
    format!("{prefix}:{}", format_args!("{:x}", hash.finalize()))
}
fn normalize_id(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(['.', ',', ';'])
        .to_ascii_lowercase()
}
fn normalize_doi(value: &str) -> String {
    normalize_id(value)
        .trim_start_matches("https://doi.org/")
        .trim_start_matches("doi:")
        .to_string()
}
fn normalize_arxiv(value: &str) -> String {
    let value = normalize_id(value)
        .trim_start_matches("arxiv:")
        .trim_start_matches("https://arxiv.org/abs/")
        .to_string();
    value
        .split_once('v')
        .map_or(value.clone(), |(base, version)| {
            format!("{base}:{}", version.trim_start_matches('v'))
        })
}
fn normalize_text(value: &str) -> String {
    value
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sil_api::{NeverCancel, RecordProvenance};
    use std::collections::BTreeMap;

    fn record(id: &str, doi: Option<&str>) -> RawRecord {
        RawRecord {
            provider_record_id: id.into(),
            title: Some("A Stable Paper".into()),
            authors: vec!["A Author".into()],
            abstract_text: None,
            year: Some(2025),
            raw_venue: None,
            work_type: Some("journal-article".into()),
            identifiers: doi
                .map(|v| BTreeMap::from([(String::from("doi"), v.into())]))
                .unwrap_or_default(),
            citation_count: None,
            source_external_ids: BTreeMap::new(),
            raw_payload: "{}".into(),
            provenance: RecordProvenance {
                provider: "fixture".into(),
                request_url: "offline".into(),
                response_status: 200,
                retrieved_at: 0,
                payload_sha256: "hash".into(),
            },
            evidence: Vec::new(),
            acceptance: None,
        }
    }

    #[test]
    fn identity_is_conservative() {
        assert_eq!(
            identify(&record("one", Some("https://doi.org/10.X/Y"))).key,
            "doi:10.x/y"
        );
        assert_ne!(
            identify(&record("one", None)).key,
            identify(&record("two", None)).key
        );
    }
    #[test]
    fn ranking_is_permutation_invariant_and_explained() {
        let a = CandidateForRanking {
            candidate_id: "a".into(),
            work_id: "w-a".into(),
            title: "Attention models".into(),
            year: None,
            venue_collection_match: false,
            provider_identifier_consensus: 1,
            citation_count: None,
            open_access: None,
        };
        let b = CandidateForRanking {
            candidate_id: "b".into(),
            work_id: "w-b".into(),
            ..a.clone()
        };
        let x = rank_candidates("attention", &[a.clone(), b.clone()]);
        let y = rank_candidates("attention", &[b, a]);
        assert_eq!(x, y);
        assert_eq!(x[0].components.total(), x[0].score);
    }
    #[test]
    fn arxiv_versions_share_base_but_keep_version() {
        assert_eq!(identify(&record("a", None)).basis, "provider_record_id");
        assert_eq!(normalize_arxiv("arXiv:1234.5678v2"), "1234.5678:2");
    }
    #[test]
    fn cancellation_type_is_available_offline() {
        let _ = NeverCancel;
    }
}
