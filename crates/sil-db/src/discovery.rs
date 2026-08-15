//! Persistence for offline literature discovery.

use rusqlite::{Connection, OptionalExtension, params};

use crate::DbError;

/// A durable, immutable discovery query/run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryRun {
    /// Stable run identifier.
    pub id: String,
    /// User query snapshot.
    pub query: String,
    /// Run status, such as `running`, `complete`, or `partial`.
    pub status: String,
    /// Provider cursor snapshot.
    pub cursor_json: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
}

/// A provider request made during a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRequest {
    /// Stable request identifier.
    pub id: String,
    /// Owning run.
    pub run_id: String,
    /// Provider name.
    pub provider: String,
    /// Request metadata, serialized as JSON.
    pub request_json: String,
    /// Cursor sent to the provider.
    pub cursor: Option<String>,
    /// Request status.
    pub status: String,
}

/// An immutable raw provider response record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRecord {
    /// Stable record identifier.
    pub id: String,
    /// Owning run.
    pub run_id: String,
    /// Request that produced the record.
    pub request_id: String,
    /// Provider name and provider-local identifier.
    pub provider: String,
    /// Provider-local record identifier.
    pub provider_record_id: String,
    /// Raw payload, retained for audit/export.
    pub raw_payload: String,
    /// Hash of `raw_payload`.
    pub raw_payload_hash: String,
    /// Record status.
    pub status: String,
}

/// Canonical work metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Work {
    /// Stable canonical work identifier.
    pub id: String,
    /// Work title.
    pub title: String,
    /// Optional abstract.
    pub abstract_text: Option<String>,
    /// Authors serialized as JSON.
    pub authors_json: Option<String>,
    /// Publication year when known.
    pub year: Option<i32>,
}

/// An identifier belonging to a work, separate from its publication versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkIdentifier {
    /// Work identifier.
    pub work_id: String,
    /// Identifier namespace, for example `doi` or `arxiv`.
    pub namespace: String,
    /// Normalized identifier value.
    pub value: String,
    /// Provider/source that observed this identifier.
    pub observed_by: Option<String>,
}

/// A publication version of a canonical work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkVersion {
    /// Stable version identifier.
    pub id: String,
    /// Canonical work.
    pub work_id: String,
    /// Version kind, for example `preprint`, `conference`, or `journal`.
    pub version_kind: String,
    /// Version-specific title.
    pub title: Option<String>,
    /// Publication date/year snapshot.
    pub published_at: Option<String>,
    /// Landing page URL.
    pub url: Option<String>,
    /// Whether open access was observed.
    pub open_access: Option<bool>,
}

/// Venue evidence attached to a publication version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkVenue {
    /// Stable row identifier.
    pub id: String,
    /// Publication version.
    pub version_id: String,
    /// B1 canonical venue ID, when resolved.
    pub venue_id: Option<String>,
    /// Provider/raw venue string, retained forever.
    pub raw_venue: String,
    /// Normalized venue input.
    pub normalized_venue: Option<String>,
    /// `resolved`, `ambiguous`, or `unknown`.
    pub resolution_status: String,
    /// Resolver confidence/evidence JSON.
    pub evidence_json: Option<String>,
    /// B1 catalogue version.
    pub catalogue_version: Option<String>,
    /// B1 normalizer version.
    pub normalizer_version: Option<i64>,
}

/// Orthogonal candidate lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Stable candidate identifier.
    pub id: String,
    /// Owning discovery run.
    pub run_id: String,
    /// Candidate work version.
    pub version_id: String,
    /// Source provider record, when available.
    pub provider_record_id: Option<String>,
    /// Resolution state.
    pub resolution: CandidateState,
    /// User disposition.
    pub disposition: CandidateState,
    /// Acquisition state.
    pub acquisition: CandidateState,
}

/// Append-only candidate transition event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateEvent {
    /// Event identifier.
    pub id: i64,
    /// Candidate identifier.
    pub candidate_id: String,
    /// State dimension changed.
    pub dimension: String,
    /// Previous state.
    pub from_state: String,
    /// New state.
    pub to_state: String,
    /// Actor responsible for the transition.
    pub actor: String,
    /// Human-readable reason.
    pub reason: String,
    /// Event timestamp.
    pub created_at: String,
}

/// Allowed values for a candidate state dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateState {
    /// Initial state.
    New,
    /// State is being processed/requested.
    Pending,
    /// Positive terminal or selected state.
    Accepted,
    /// Negative terminal state.
    Rejected,
    /// Explicitly unresolved/failed state.
    Unknown,
}

impl CandidateState {
    fn as_str(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Unknown => "unknown",
        }
    }
}

/// Create a run and return it.
pub fn create_run(conn: &Connection, run: &DiscoveryRun) -> Result<(), DbError> {
    conn.execute("INSERT INTO discovery_runs (id, query, status, cursor_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)", params![run.id, run.query, run.status, run.cursor_json, run.created_at])?;
    Ok(())
}

/// Update run progress/status while retaining the latest resumable cursor.
pub fn update_run(
    conn: &Connection,
    id: &str,
    status: &str,
    cursor_json: Option<&str>,
) -> Result<(), DbError> {
    let changed = conn.execute(
        "UPDATE discovery_runs SET status=?1, cursor_json=?2, finished_at=CASE WHEN ?1 IN ('complete','partial','failed') THEN datetime('now') ELSE finished_at END WHERE id=?3",
        params![status, cursor_json, id],
    )?;
    if changed == 0 {
        return Err(DbError::Message(format!("discovery run '{id}' not found")));
    }
    Ok(())
}

/// Store a ranking explanation without overwriting another algorithm version.
pub fn insert_candidate_ranking(
    conn: &Connection,
    run_id: &str,
    candidate_id: &str,
    algorithm_version: &str,
    score: i64,
    components_json: &str,
) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO candidate_rankings (run_id, candidate_id, algorithm_version, score, components_json) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![run_id, candidate_id, algorithm_version, score, components_json],
    )?;
    Ok(())
}

/// Insert an immutable provider request.
pub fn insert_provider_request(
    conn: &Connection,
    request: &ProviderRequest,
) -> Result<(), DbError> {
    conn.execute("INSERT INTO provider_requests (id, run_id, provider, request_json, cursor, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![request.id, request.run_id, request.provider, request.request_json, request.cursor, request.status])?;
    Ok(())
}

/// Insert an immutable provider record.
pub fn insert_provider_record(conn: &Connection, record: &ProviderRecord) -> Result<(), DbError> {
    conn.execute("INSERT INTO provider_records (id, run_id, request_id, provider, provider_record_id, raw_payload, raw_payload_hash, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", params![record.id, record.run_id, record.request_id, record.provider, record.provider_record_id, record.raw_payload, record.raw_payload_hash, record.status])?;
    Ok(())
}

/// Insert or update canonical work metadata.
pub fn upsert_work(conn: &Connection, work: &Work) -> Result<(), DbError> {
    conn.execute("INSERT INTO works (id, title, abstract_text, authors_json, year) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(id) DO UPDATE SET title=excluded.title, abstract_text=excluded.abstract_text, authors_json=excluded.authors_json, year=excluded.year", params![work.id, work.title, work.abstract_text, work.authors_json, work.year])?;
    Ok(())
}

/// Insert a work identifier without conflating it with a publication version.
pub fn insert_work_identifier(
    conn: &Connection,
    identifier: &WorkIdentifier,
) -> Result<(), DbError> {
    conn.execute("INSERT INTO work_identifiers (work_id, namespace, value, observed_by) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(namespace, value) DO UPDATE SET observed_by=COALESCE(excluded.observed_by, observed_by)", params![identifier.work_id, identifier.namespace, identifier.value, identifier.observed_by])?;
    Ok(())
}

/// Insert a publication version.
pub fn insert_work_version(conn: &Connection, version: &WorkVersion) -> Result<(), DbError> {
    conn.execute("INSERT INTO work_versions (id, work_id, version_kind, title, published_at, url, open_access) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![version.id, version.work_id, version.version_kind, version.title, version.published_at, version.url, version.open_access.map(i32::from)])?;
    Ok(())
}

/// Insert venue evidence, including unresolved raw values.
pub fn insert_work_venue(conn: &Connection, venue: &WorkVenue) -> Result<(), DbError> {
    conn.execute("INSERT INTO work_venues (id, version_id, venue_id, raw_venue, normalized_venue, resolution_status, evidence_json, catalogue_version, normalizer_version) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)", params![venue.id, venue.version_id, venue.venue_id, venue.raw_venue, venue.normalized_venue, venue.resolution_status, venue.evidence_json, venue.catalogue_version, venue.normalizer_version])?;
    Ok(())
}

/// Insert a candidate in its initial lifecycle state.
pub fn insert_candidate(conn: &Connection, candidate: &Candidate) -> Result<(), DbError> {
    conn.execute("INSERT INTO candidates (id, run_id, version_id, provider_record_id, resolution, disposition, acquisition) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![candidate.id, candidate.run_id, candidate.version_id, candidate.provider_record_id, candidate.resolution.as_str(), candidate.disposition.as_str(), candidate.acquisition.as_str()])?;
    Ok(())
}

/// Transition one candidate dimension and append its audit event atomically.
pub fn transition_candidate(
    conn: &Connection,
    id: &str,
    dimension: &str,
    to: CandidateState,
    actor: &str,
    reason: &str,
) -> Result<(), DbError> {
    if !matches!(dimension, "resolution" | "disposition" | "acquisition") {
        return Err(DbError::Message(format!(
            "invalid candidate dimension '{dimension}'"
        )));
    }
    let tx = conn.unchecked_transaction()?;
    let column = dimension;
    let from: String = tx
        .query_row(
            &format!("SELECT {column} FROM candidates WHERE id=?1"),
            [id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| DbError::Message(format!("candidate '{id}' not found")))?;
    if !valid_transition(dimension, &from, to.as_str()) {
        return Err(DbError::Message(format!(
            "invalid {dimension} transition '{from}' -> '{}'",
            to.as_str()
        )));
    }
    tx.execute(
        &format!("UPDATE candidates SET {column}=?1, updated_at=datetime('now') WHERE id=?2"),
        params![to.as_str(), id],
    )?;
    tx.execute("INSERT INTO candidate_events (candidate_id, dimension, from_state, to_state, actor, reason) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![id, dimension, from, to.as_str(), actor, reason])?;
    tx.commit()?;
    Ok(())
}

fn valid_transition(dimension: &str, from: &str, to: &str) -> bool {
    if from == to {
        return false;
    }
    match dimension {
        "resolution" => matches!(
            (from, to),
            ("new", "pending")
                | ("pending", "accepted")
                | ("pending", "rejected")
                | ("pending", "unknown")
        ),
        "disposition" => matches!(
            (from, to),
            ("new", "accepted") | ("new", "rejected") | ("rejected", "new")
        ),
        "acquisition" => matches!(
            (from, to),
            ("new", "pending") | ("pending", "accepted") | ("pending", "rejected")
        ),
        _ => false,
    }
}

/// List candidate events in append order.
pub fn candidate_events(
    conn: &Connection,
    candidate_id: &str,
) -> Result<Vec<CandidateEvent>, DbError> {
    let mut stmt = conn.prepare("SELECT id, candidate_id, dimension, from_state, to_state, actor, reason, created_at FROM candidate_events WHERE candidate_id=?1 ORDER BY id")?;
    let rows = stmt.query_map([candidate_id], |r| {
        Ok(CandidateEvent {
            id: r.get(0)?,
            candidate_id: r.get(1)?,
            dimension: r.get(2)?,
            from_state: r.get(3)?,
            to_state: r.get(4)?,
            actor: r.get(5)?,
            reason: r.get(6)?,
            created_at: r.get(7)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}
