//! Offline-testable discovery providers.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

mod crossref;
mod dblp;
mod openalex;
mod openreview;

pub use crossref::CrossrefProvider;
pub use dblp::DblpProvider;
pub use openalex::OpenAlexProvider;
pub use openreview::OpenReviewProvider;

/// A provider-neutral discovery request. Providers preserve these values in request provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryRequest {
    /// User query, interpreted by each provider.
    pub query: String,
    /// Maximum records requested per page.
    pub page_size: usize,
    /// Provider cursor or page token from a previous page.
    pub cursor: Option<String>,
    /// Provider-supported filters, such as `type:journal-article`.
    #[serde(default)]
    pub filters: Vec<String>,
    /// Optional raw venue/container query.
    pub venue: Option<String>,
}

impl DiscoveryRequest {
    /// Construct a request with a bounded, non-empty page size.
    pub fn new(query: impl Into<String>, page_size: usize) -> Self {
        Self {
            query: query.into(),
            page_size: page_size.max(1),
            cursor: None,
            filters: Vec::new(),
            venue: None,
        }
    }
}

/// HTTP request passed to an injectable transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    /// HTTP method.
    pub method: String,
    /// Absolute URL, including encoded query parameters.
    pub url: String,
    /// Request headers in deterministic order.
    pub headers: BTreeMap<String, String>,
}

/// HTTP response returned by a transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response headers in deterministic order.
    pub headers: BTreeMap<String, String>,
    /// UTF-8 response body.
    pub body: String,
}

/// Transport failure. Implementations may use `timeout` for cancellation/retry reporting.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// The request exceeded the transport timeout.
    #[error("request timed out: {0}")]
    Timeout(String),
    /// An offline or network transport failed.
    #[error("transport failure: {0}")]
    Failed(String),
}

/// Injectable synchronous HTTP transport used by providers and fixtures.
pub trait HttpTransport: Send + Sync {
    /// Send one request without applying provider policy.
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, TransportError>;
}

/// Production `ureq` transport. Tests should provide their own implementation.
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl UreqTransport {
    /// Create a transport with a finite request timeout.
    pub fn new(timeout: Duration) -> Self {
        Self {
            agent: ureq::AgentBuilder::new().timeout(timeout).build(),
        }
    }
}

impl Default for UreqTransport {
    fn default() -> Self {
        Self::new(Duration::from_secs(15))
    }
}

impl HttpTransport for UreqTransport {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, TransportError> {
        let mut call = self.agent.request(&request.method, &request.url);
        for (key, value) in &request.headers {
            call = call.set(key, value);
        }
        match call.call() {
            Ok(response) => Ok(HttpResponse {
                status: response.status(),
                headers: BTreeMap::new(),
                body: response
                    .into_string()
                    .map_err(|e| TransportError::Failed(e.to_string()))?,
            }),
            Err(ureq::Error::Status(status, response)) => Ok(HttpResponse {
                status,
                headers: BTreeMap::new(),
                body: response.into_string().unwrap_or_default(),
            }),
            Err(ureq::Error::Transport(error)) => {
                let message = error.to_string();
                if message.to_lowercase().contains("timeout") {
                    Err(TransportError::Timeout(message))
                } else {
                    Err(TransportError::Failed(message))
                }
            }
        }
    }
}

/// Cooperative cancellation hook checked before every request and page.
pub trait Cancellation: Send + Sync {
    /// Return true when the caller no longer wants discovery to continue.
    fn is_cancelled(&self) -> bool;
}

/// A cancellation hook that never cancels.
pub struct NeverCancel;
impl Cancellation for NeverCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Provider-specific retry and rate-limit settings.
#[derive(Debug, Clone)]
pub struct ProviderPolicy {
    /// Maximum attempts including the initial request.
    pub max_attempts: usize,
    /// Minimum interval between requests for this provider instance.
    pub min_interval: Duration,
    /// Initial retry delay for 5xx/429/transport failures.
    pub retry_delay: Duration,
    /// Maximum retry delay.
    pub retry_cap: Duration,
}

impl Default for ProviderPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            min_interval: Duration::from_millis(250),
            retry_delay: Duration::from_millis(250),
            retry_cap: Duration::from_secs(2),
        }
    }
}

/// Immutable request/response provenance retained with every raw record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordProvenance {
    /// Provider name.
    pub provider: String,
    /// Exact request URL.
    pub request_url: String,
    /// HTTP status observed.
    pub response_status: u16,
    /// Retrieval timestamp in Unix seconds.
    pub retrieved_at: u64,
    /// SHA-256 of the exact raw payload.
    pub payload_sha256: String,
}

/// Provider-normalized record. Venue fields are observations, never canonical IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawRecord {
    /// Provider-local stable identifier.
    pub provider_record_id: String,
    /// Observed title.
    pub title: Option<String>,
    /// Observed author display names.
    #[serde(default)]
    pub authors: Vec<String>,
    /// Observed abstract, when available.
    pub abstract_text: Option<String>,
    /// Observed publication year.
    pub year: Option<i32>,
    /// Raw venue/container string. It is intentionally unresolved.
    pub raw_venue: Option<String>,
    /// Provider work type, for example `journal-article` or `proceedings-article`.
    pub work_type: Option<String>,
    /// DOI, arXiv, and other observed identifiers.
    #[serde(default)]
    pub identifiers: BTreeMap<String, String>,
    /// Provider citation count if supplied.
    pub citation_count: Option<u64>,
    /// Source/venue external IDs, preserved verbatim by namespace.
    #[serde(default)]
    pub source_external_ids: BTreeMap<String, String>,
    /// Exact provider item JSON.
    pub raw_payload: String,
    /// Request and payload evidence.
    pub provenance: RecordProvenance,
    /// Provider-specific facts used to explain venue and acceptance decisions.
    #[serde(default)]
    pub evidence: Vec<ProviderEvidence>,
    /// Acceptance observation. `None` means the provider did not expose an acceptance state.
    #[serde(default)]
    pub acceptance: Option<AcceptanceEvidence>,
}

/// One provider fact retained for audit and later policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderEvidence {
    /// Evidence category, such as `forum`, `invitation`, `group`, or `proceedings`.
    pub kind: String,
    /// Exact provider value, serialized when the source value is structured.
    pub value: String,
}

/// Acceptance observation and the evidence that supports or limits it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceEvidence {
    /// `accepted`, `rejected`, `withdrawn`, `unknown`, or `ambiguous`.
    pub state: String,
    /// Facts used to derive the state. Hosting facts alone are never sufficient.
    #[serde(default)]
    pub evidence: Vec<ProviderEvidence>,
}

/// One provider page, including a cursor for the next page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryPage {
    /// Provider name.
    pub provider: String,
    /// Records in stable provider order.
    pub records: Vec<RawRecord>,
    /// Cursor for the next page, if any.
    pub next_cursor: Option<String>,
    /// Exact request metadata.
    pub request: DiscoveryRequest,
}

/// Structured provider failure attached to a partial run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartialError {
    /// Provider that failed.
    pub provider: String,
    /// Request URL, where available.
    pub request_url: Option<String>,
    /// HTTP status, where available.
    pub status: Option<u16>,
    /// Stable error category.
    pub category: String,
    /// Human-readable detail.
    pub message: String,
}

/// Aggregate result from independent providers. A failure never becomes empty success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryRunResult {
    /// Records returned by successful providers, in provider invocation order.
    pub records: Vec<RawRecord>,
    /// Explicit failures from providers that could not complete.
    pub errors: Vec<PartialError>,
    /// `complete`, `partial`, or `failed`.
    pub status: String,
}

/// Provider contract for one paged discovery source.
pub trait DiscoveryProvider: Send + Sync {
    /// Stable provider name.
    fn name(&self) -> &'static str;
    /// Fetch one page. Pagination is driven by `DiscoveryProvider::discover`.
    fn discover_page(
        &self,
        request: &DiscoveryRequest,
        cancellation: &dyn Cancellation,
    ) -> Result<DiscoveryPage, PartialError>;
    /// Fetch all pages until the provider is exhausted or cancelled.
    fn discover(
        &self,
        request: &DiscoveryRequest,
        cancellation: &dyn Cancellation,
    ) -> Result<Vec<RawRecord>, PartialError> {
        let mut next = request.clone();
        let mut records = Vec::new();
        loop {
            if cancellation.is_cancelled() {
                return Err(PartialError {
                    provider: self.name().into(),
                    request_url: None,
                    status: None,
                    category: "cancelled".into(),
                    message: "discovery cancelled".into(),
                });
            }
            let page = self.discover_page(&next, cancellation)?;
            records.extend(page.records);
            match page.next_cursor {
                Some(cursor) => next.cursor = Some(cursor),
                None => break,
            }
        }
        Ok(records)
    }
}

/// Run providers independently and represent one-provider failure explicitly.
pub fn discover_with_providers(
    providers: &[&dyn DiscoveryProvider],
    request: &DiscoveryRequest,
    cancellation: &dyn Cancellation,
) -> DiscoveryRunResult {
    let mut result = DiscoveryRunResult {
        records: Vec::new(),
        errors: Vec::new(),
        status: "complete".into(),
    };
    for provider in providers {
        match provider.discover(request, cancellation) {
            Ok(mut records) => result.records.append(&mut records),
            Err(error) => result.errors.push(error),
        }
    }
    if !result.errors.is_empty() {
        result.status = if result.records.is_empty() {
            "failed"
        } else {
            "partial"
        }
        .into();
    }
    result
}

pub(crate) fn payload_hash(payload: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(payload.as_bytes());
    format!("{:x}", hash.finalize())
}

pub(crate) fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(crate) struct ProviderClient {
    pub transport: Arc<dyn HttpTransport>,
    pub policy: ProviderPolicy,
    last_request: Mutex<Option<std::time::Instant>>,
}

impl ProviderClient {
    pub fn new(transport: Arc<dyn HttpTransport>, policy: ProviderPolicy) -> Self {
        Self {
            transport,
            policy,
            last_request: Mutex::new(None),
        }
    }

    pub fn send(
        &self,
        request: HttpRequest,
        provider: &'static str,
        cancellation: &dyn Cancellation,
    ) -> Result<HttpResponse, PartialError> {
        let mut delay = self.policy.retry_delay;
        for attempt in 0..self.policy.max_attempts.max(1) {
            if cancellation.is_cancelled() {
                return Err(PartialError {
                    provider: provider.into(),
                    request_url: Some(request.url.clone()),
                    status: None,
                    category: "cancelled".into(),
                    message: "discovery cancelled".into(),
                });
            }
            if let Ok(mut last) = self.last_request.lock() {
                if let Some(at) = *last
                    && let Some(wait) = self.policy.min_interval.checked_sub(at.elapsed())
                {
                    thread::sleep(wait);
                }
                *last = Some(std::time::Instant::now());
            }
            let response = match self.transport.send(&request) {
                Ok(response) => response,
                Err(_error) if attempt + 1 < self.policy.max_attempts.max(1) => {
                    thread::sleep(delay);
                    delay = (delay * 2).min(self.policy.retry_cap);
                    continue;
                }
                Err(error) => {
                    return Err(PartialError {
                        provider: provider.into(),
                        request_url: Some(request.url.clone()),
                        status: None,
                        category: "transport".into(),
                        message: error.to_string(),
                    });
                }
            };
            if response.status == 429 || (500..=599).contains(&response.status) {
                if attempt + 1 < self.policy.max_attempts.max(1) {
                    let retry_after = response
                        .headers
                        .get("Retry-After")
                        .and_then(|v| v.parse::<u64>().ok())
                        .map(Duration::from_secs);
                    thread::sleep(retry_after.unwrap_or(delay).min(self.policy.retry_cap));
                    delay = (delay * 2).min(self.policy.retry_cap);
                    continue;
                }
                return Err(PartialError {
                    provider: provider.into(),
                    request_url: Some(request.url.clone()),
                    status: Some(response.status),
                    category: if response.status == 429 {
                        "rate_limited"
                    } else {
                        "server"
                    }
                    .into(),
                    message: format!("HTTP {} after retries", response.status),
                });
            }
            if response.status == 404 {
                return Err(PartialError {
                    provider: provider.into(),
                    request_url: Some(request.url.clone()),
                    status: Some(response.status),
                    category: "not_found".into(),
                    message: "provider endpoint returned HTTP 404".into(),
                });
            }
            return Ok(response);
        }
        unreachable!()
    }
}

pub(crate) fn base_headers() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("Accept".into(), "application/json".into()),
        (
            "User-Agent".into(),
            "scientist-in-loop/1.0 (mailto:info@scientist-in-loop.org)".into(),
        ),
    ])
}

pub(crate) fn url_encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "%20".into(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub(crate) fn parse_error(
    provider: &'static str,
    url: &str,
    message: impl Into<String>,
) -> PartialError {
    PartialError {
        provider: provider.into(),
        request_url: Some(url.into()),
        status: None,
        category: "parse".into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct FixtureTransport {
        responses: Mutex<VecDeque<Result<HttpResponse, TransportError>>>,
        calls: AtomicUsize,
    }

    impl FixtureTransport {
        fn new(responses: Vec<Result<HttpResponse, TransportError>>) -> Self {
            Self {
                responses: Mutex::new(responses.into()),
                calls: AtomicUsize::new(0),
            }
        }
    }

    impl HttpTransport for FixtureTransport {
        fn send(&self, _request: &HttpRequest) -> Result<HttpResponse, TransportError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Err(TransportError::Failed("fixture exhausted".into())))
        }
    }

    struct FlagCancel(AtomicBool);
    impl Cancellation for FlagCancel {
        fn is_cancelled(&self) -> bool {
            self.0.load(Ordering::SeqCst)
        }
    }

    fn response(status: u16, body: &str) -> Result<HttpResponse, TransportError> {
        Ok(HttpResponse {
            status,
            headers: BTreeMap::new(),
            body: body.into(),
        })
    }

    fn policy() -> ProviderPolicy {
        ProviderPolicy {
            max_attempts: 3,
            min_interval: Duration::ZERO,
            retry_delay: Duration::ZERO,
            retry_cap: Duration::ZERO,
        }
    }

    #[test]
    fn crossref_fixture_paginates_and_preserves_provenance() {
        let transport = Arc::new(FixtureTransport::new(vec![
            response(200, include_str!("../fixtures/crossref_page_1.json")),
            response(200, include_str!("../fixtures/crossref_page_2.json")),
        ]));
        let provider = CrossrefProvider::new(transport.clone(), policy());
        let records = provider
            .discover(&DiscoveryRequest::new("attention", 1), &NeverCancel)
            .unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].provider_record_id, "10.1000/proceedings");
        assert_eq!(
            records[0].raw_venue.as_deref(),
            Some("Proceedings of Testconf")
        );
        assert_eq!(records[0].provenance.provider, "crossref");
        assert_eq!(
            records[0].provenance.payload_sha256,
            payload_hash(&records[0].raw_payload)
        );
        assert_eq!(transport.calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn openalex_fixture_keeps_source_ids_and_empty_pages() {
        let transport = Arc::new(FixtureTransport::new(vec![response(
            200,
            include_str!("../fixtures/openalex_empty.json"),
        )]));
        let provider = OpenAlexProvider::new(transport, policy());
        let page = provider
            .discover_page(&DiscoveryRequest::new("nothing", 10), &NeverCancel)
            .unwrap();
        assert!(page.records.is_empty());
        assert!(page.next_cursor.is_none());
    }

    #[test]
    fn retry_after_and_server_errors_are_retried_without_global_bucket() {
        let limited = HttpResponse {
            status: 429,
            headers: BTreeMap::from([("Retry-After".into(), "0".into())]),
            body: String::new(),
        };
        let transport = Arc::new(FixtureTransport::new(vec![
            Ok(limited.clone()),
            response(500, "{}"),
            response(200, include_str!("../fixtures/openalex_page.json")),
        ]));
        let provider = OpenAlexProvider::new(transport.clone(), policy());
        let result = provider
            .discover_page(&DiscoveryRequest::new("x", 1), &NeverCancel)
            .unwrap();
        assert_eq!(result.records.len(), 1);
        assert_eq!(transport.calls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn malformed_payload_and_provider_failure_are_structured_partial_results() {
        let transport = Arc::new(FixtureTransport::new(vec![response(200, "{bad")]));
        let provider = CrossrefProvider::new(transport, policy());
        let result =
            discover_with_providers(&[&provider], &DiscoveryRequest::new("x", 1), &NeverCancel);
        assert_eq!(result.status, "failed");
        assert_eq!(result.errors[0].category, "parse");
        assert!(result.records.is_empty());
    }

    #[test]
    fn cancellation_is_checked_before_transport() {
        let transport = Arc::new(FixtureTransport::new(vec![response(200, "{}")]));
        let provider = OpenAlexProvider::new(transport.clone(), policy());
        let cancel = FlagCancel(AtomicBool::new(true));
        let error = provider
            .discover_page(&DiscoveryRequest::new("x", 1), &cancel)
            .unwrap_err();
        assert_eq!(error.category, "cancelled");
        assert_eq!(transport.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn conference_fixtures_preserve_acceptance_and_proceedings_evidence() {
        let transport = Arc::new(FixtureTransport::new(vec![
            response(200, include_str!("../fixtures/openreview_page.json")),
            response(200, include_str!("../fixtures/dblp_page.json")),
        ]));
        let openreview = OpenReviewProvider::new(transport.clone(), policy());
        let accepted = openreview
            .discover(&DiscoveryRequest::new("paper", 10), &NeverCancel)
            .unwrap();
        assert_eq!(accepted[0].acceptance.as_ref().unwrap().state, "accepted");
        assert_eq!(accepted[1].acceptance.as_ref().unwrap().state, "rejected");
        assert_eq!(accepted[2].acceptance.as_ref().unwrap().state, "withdrawn");
        assert_eq!(accepted[3].acceptance.as_ref().unwrap().state, "unknown");
        let dblp = DblpProvider::new(transport, policy());
        let records = dblp
            .discover(&DiscoveryRequest::new("test", 10), &NeverCancel)
            .unwrap();
        assert_eq!(records[0].year, Some(2024));
        assert!(records[0].evidence.iter().any(|e| e.kind == "proceedings"));
        assert_eq!(
            records[0].provenance.payload_sha256,
            payload_hash(&records[0].raw_payload)
        );
    }

    #[test]
    fn dblp_xml_fixture_is_supported() {
        let transport = Arc::new(FixtureTransport::new(vec![response(
            200,
            include_str!("../fixtures/dblp_page.xml"),
        )]));
        let provider = DblpProvider::new(transport, policy());
        let records = provider
            .discover(&DiscoveryRequest::new("workshop", 10), &NeverCancel)
            .unwrap();
        assert_eq!(records[0].provider_record_id, "conf/test/Workshop2023");
        assert_eq!(records[0].year, Some(2023));
        assert_eq!(
            records[0].raw_venue.as_deref(),
            Some("Test Conference Workshops")
        );
    }

    #[test]
    fn not_found_and_timeout_are_not_silently_successful() {
        let not_found_transport = Arc::new(FixtureTransport::new(vec![response(404, "")]));
        let not_found = OpenAlexProvider::new(not_found_transport, policy());
        let error = not_found
            .discover_page(&DiscoveryRequest::new("x", 1), &NeverCancel)
            .unwrap_err();
        assert_eq!(error.category, "not_found");
        assert_eq!(error.status, Some(404));

        let timeout_transport = Arc::new(FixtureTransport::new(vec![
            Err(TransportError::Timeout("slow".into())),
            Err(TransportError::Timeout("slow".into())),
            Err(TransportError::Timeout("slow".into())),
        ]));
        let timeout = CrossrefProvider::new(timeout_transport, policy());
        let error = timeout
            .discover_page(&DiscoveryRequest::new("x", 1), &NeverCancel)
            .unwrap_err();
        assert_eq!(error.category, "transport");
        assert!(error.message.contains("timed out"));
    }
}
