//! Domain types for top-journal publication digest feeds.

use serde::{Deserialize, Serialize};

/// Represents a publication item retrieved from top academic journals (Crossref / PubMed / IEEE / etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalPublication {
    /// DOI identifier (if available).
    pub doi: Option<String>,
    /// Paper title.
    pub title: String,
    /// Author list string.
    pub authors: String,
    /// Journal or publication venue name (e.g. Nature, IEEE TPAMI, JMLR).
    pub journal: String,
    /// Publication year.
    pub year: Option<u32>,
    /// Abstract summary.
    pub abstract_text: String,
    /// Citation count if available.
    pub citation_count: Option<u32>,
    /// URL to publication landing page.
    pub url: String,
    /// PDF download URL if open access.
    pub pdf_url: Option<String>,
}
