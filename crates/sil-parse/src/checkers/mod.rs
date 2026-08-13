//! Abstract Factory & Trait-based Reference Checkers for DOIs, arXiv IDs, and OpenReview notes.

use serde::{Deserialize, Serialize};
use sil_db::{DbError, SilDb};

use crate::error::ParseError;

pub mod arxiv;
pub mod doi;
pub mod openreview;

pub use arxiv::ArxivChecker;
pub use doi::DoiChecker;
pub use openreview::OpenReviewChecker;

/// Trait defining the operations required for checking a scientific reference identifier online or from cache.
pub trait ReferenceChecker: Send + Sync {
    /// Return human-readable identifier type name (e.g. "DOI", "arXiv", "OpenReview").
    fn identifier_name(&self) -> &'static str;

    /// Extract clean identifier string from a BibTeX block, if present.
    fn extract_identifier(&self, block: &str) -> Option<String>;

    /// Verify identifier online against external API and retrieve paper title metadata.
    fn verify_online(&self, id: &str) -> Result<sil_api::DoiMetadataResult, sil_api::ApiError>;

    /// Fetch official BibTeX entry string from external API, if available.
    fn fetch_official_bibtex(&self, id: &str) -> Result<Option<String>, sil_api::ApiError>;

    /// Retrieve cached verification record from local SQLite database.
    fn get_cached_verification(
        &self,
        db: &SilDb,
        id: &str,
    ) -> Result<Option<(bool, Option<String>)>, DbError>;

    /// Save verification outcome to local SQLite database.
    fn save_verification(
        &self,
        db: &SilDb,
        id: &str,
        exists: bool,
        error_cat: Option<&str>,
    ) -> Result<(), DbError>;
}

/// Abstract Factory for creating reference checkers.
pub struct CheckerFactory;

impl CheckerFactory {
    /// Return all supported reference checkers in priority order: DOI, arXiv, OpenReview.
    pub fn all_checkers() -> Vec<Box<dyn ReferenceChecker>> {
        vec![
            Box::new(DoiChecker),
            Box::new(ArxivChecker),
            Box::new(OpenReviewChecker),
        ]
    }
}

/// Category of verification result for a single reference check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReferenceCheckCategory {
    /// Identifier checked online/cached and verified to exist with matching title.
    Valid,
    /// Identifier exists but local paper title differs from official metadata (< 60% similarity).
    TitleMismatch {
        /// Local title extracted from BibTeX block.
        local_title: String,
        /// Official title from metadata API.
        official_title: String,
        /// Jaccard title similarity score (0.0 .. 1.0).
        similarity: f64,
    },
    /// Identifier checked online and returned 404 / Not Found.
    NotFound,
    /// Verification failed with network error or rate limit error.
    NetworkError(String),
    /// BibTeX entry lacks a valid identifier format for this checker.
    InvalidFormat,
    /// Checking skipped because result was cached in local DB.
    SkippedCached,
}

/// Item report for a single reference entry check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BibItemCheckReport {
    /// Citation key of the BibTeX entry.
    pub cite_key: String,
    /// Type of identifier checked (e.g. "DOI", "arXiv", "OpenReview").
    pub identifier_type: String,
    /// Clean identifier string, if extracted.
    pub identifier: Option<String>,
    /// Verification category result.
    pub category: ReferenceCheckCategory,
}

/// Unified report summarizing incremental check results across all reference checkers.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UnifiedBibCheckReport {
    /// Total number of BibTeX blocks parsed.
    pub total_entries: usize,
    /// Number of entries with at least one extracted identifier.
    pub entries_with_identifier: usize,
    /// Number of online API network checks performed.
    pub checked_online: usize,
    /// Number of valid identifiers (online + cached).
    pub valid_identifiers: usize,
    /// List of broken identifiers (cite_key, identifier_type, identifier).
    pub broken_identifiers: Vec<(String, String, String)>,
    /// List of network errors (cite_key, identifier_type, identifier, err_msg).
    pub network_errors: Vec<(String, String, String, String)>,
    /// List of title mismatches (cite_key, identifier_type, local_title, official_title, similarity).
    pub mismatched_identifiers: Vec<(String, String, String, String, f64)>,
    /// Number of entries skipped due to local caching.
    pub skipped_cached: usize,
    /// Number of entries automatically updated with official BibTeX via autofix.
    pub autofixed_count: usize,
    /// Updated BibTeX file content string if autofix modified any entries.
    pub updated_bib_content: Option<String>,
    /// Itemized reports for each checked entry.
    pub items: Vec<BibItemCheckReport>,
}

fn extract_local_title(block: &str) -> String {
    let info = sil_core::bib::extract_bib_entry_info(block);
    if let Some(title) = info.title
        && !title.trim().is_empty()
    {
        return title.trim().to_string();
    }
    String::new()
}

/// Incrementally check references in `bib_content` using all registered checkers and local SQLite database cache.
pub fn run_all_checkers_incremental(
    db: &SilDb,
    bib_content: &str,
    autofix: bool,
) -> Result<UnifiedBibCheckReport, ParseError> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_all_checkers_incremental_inner(db, bib_content, autofix)
    }));

    match result {
        Ok(res) => res,
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic occurred during reference check".to_string()
            };
            Err(ParseError::Message(format!(
                "Reference check panicked: {msg}"
            )))
        }
    }
}

fn run_all_checkers_incremental_inner(
    db: &SilDb,
    bib_content: &str,
    autofix: bool,
) -> Result<UnifiedBibCheckReport, ParseError> {
    let blocks = sil_core::bib::parse_bib_blocks(bib_content);
    let checkers = CheckerFactory::all_checkers();

    let mut report = UnifiedBibCheckReport {
        total_entries: blocks.len(),
        ..Default::default()
    };

    let mut working_bib_content = bib_content.to_string();

    for block in &blocks {
        let entry_info = sil_core::bib::extract_bib_entry_info(block);
        let cite_key = entry_info.cite_key.unwrap_or_else(|| "unknown".to_string());
        let local_title = extract_local_title(block);

        let mut checked_any = false;

        for checker in &checkers {
            let Some(id) = checker.extract_identifier(block) else {
                continue;
            };

            checked_any = true;
            report.entries_with_identifier += 1;

            let cached = checker.get_cached_verification(db, &id)?;

            if let Some((exists, error_cat)) = cached {
                report.skipped_cached += 1;

                if exists {
                    if error_cat.as_deref() == Some("title_mismatch") {
                        report.mismatched_identifiers.push((
                            cite_key.clone(),
                            checker.identifier_name().to_string(),
                            local_title.clone(),
                            "Cached title mismatch".to_string(),
                            0.0,
                        ));
                    } else {
                        report.valid_identifiers += 1;
                    }
                } else if error_cat.as_deref() == Some("network_error") {
                    report.network_errors.push((
                        cite_key.clone(),
                        checker.identifier_name().to_string(),
                        id.clone(),
                        "network_error".to_string(),
                    ));
                } else {
                    report.broken_identifiers.push((
                        cite_key.clone(),
                        checker.identifier_name().to_string(),
                        id.clone(),
                    ));
                }

                db.upsert_bib_reference(&cite_key, Some(&id), Some(exists), block)?;

                report.items.push(BibItemCheckReport {
                    cite_key: cite_key.clone(),
                    identifier_type: checker.identifier_name().to_string(),
                    identifier: Some(id),
                    category: ReferenceCheckCategory::SkippedCached,
                });
                break;
            } else {
                report.checked_online += 1;

                match checker.verify_online(&id) {
                    Ok(meta) => {
                        if meta.exists {
                            let official_title = meta.title.unwrap_or_default();
                            let similarity = crate::journal_digest::title_similarity(
                                &local_title,
                                &official_title,
                            );

                            if similarity >= 0.60 {
                                report.valid_identifiers += 1;
                                checker.save_verification(db, &id, true, None)?;
                                db.upsert_bib_reference(&cite_key, Some(&id), Some(true), block)?;
                                report.items.push(BibItemCheckReport {
                                    cite_key: cite_key.clone(),
                                    identifier_type: checker.identifier_name().to_string(),
                                    identifier: Some(id),
                                    category: ReferenceCheckCategory::Valid,
                                });
                            } else {
                                report.mismatched_identifiers.push((
                                    cite_key.clone(),
                                    checker.identifier_name().to_string(),
                                    local_title.clone(),
                                    official_title.clone(),
                                    similarity,
                                ));
                                checker.save_verification(db, &id, true, Some("title_mismatch"))?;
                                db.upsert_bib_reference(&cite_key, Some(&id), Some(true), block)?;

                                if autofix
                                    && let Ok(Some(official_bib)) =
                                        checker.fetch_official_bibtex(&id)
                                {
                                    let (updated, _replaced) = sil_core::bib::upsert_bib_entry(
                                        &working_bib_content,
                                        &official_bib,
                                    );
                                    working_bib_content = updated;
                                    report.autofixed_count += 1;
                                }

                                report.items.push(BibItemCheckReport {
                                    cite_key: cite_key.clone(),
                                    identifier_type: checker.identifier_name().to_string(),
                                    identifier: Some(id),
                                    category: ReferenceCheckCategory::TitleMismatch {
                                        local_title: local_title.clone(),
                                        official_title,
                                        similarity,
                                    },
                                });
                            }
                        } else {
                            report.broken_identifiers.push((
                                cite_key.clone(),
                                checker.identifier_name().to_string(),
                                id.clone(),
                            ));
                            checker.save_verification(db, &id, false, Some("http_404"))?;
                            db.upsert_bib_reference(&cite_key, Some(&id), Some(false), block)?;
                            report.items.push(BibItemCheckReport {
                                cite_key: cite_key.clone(),
                                identifier_type: checker.identifier_name().to_string(),
                                identifier: Some(id),
                                category: ReferenceCheckCategory::NotFound,
                            });
                        }
                    }
                    Err(err) => {
                        let err_msg = err.to_string();
                        report.network_errors.push((
                            cite_key.clone(),
                            checker.identifier_name().to_string(),
                            id.clone(),
                            err_msg.clone(),
                        ));
                        checker.save_verification(db, &id, false, Some("network_error"))?;
                        db.upsert_bib_reference(&cite_key, Some(&id), Some(false), block)?;
                        report.items.push(BibItemCheckReport {
                            cite_key: cite_key.clone(),
                            identifier_type: checker.identifier_name().to_string(),
                            identifier: Some(id),
                            category: ReferenceCheckCategory::NetworkError(err_msg),
                        });
                    }
                }
                break;
            }
        }

        if !checked_any {
            report.items.push(BibItemCheckReport {
                cite_key,
                identifier_type: "None".to_string(),
                identifier: None,
                category: ReferenceCheckCategory::InvalidFormat,
            });
        }
    }

    if report.autofixed_count > 0 {
        report.updated_bib_content = Some(working_bib_content);
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checker_factory_polymorphism() {
        let checkers = CheckerFactory::all_checkers();
        assert_eq!(checkers.len(), 3);
        let names: Vec<&str> = checkers.iter().map(|c| c.identifier_name()).collect();
        assert_eq!(names, vec!["DOI", "arXiv", "OpenReview"]);
    }

    #[test]
    fn test_doi_extraction_and_caching() {
        let db = SilDb::open_in_memory().unwrap();
        let checker = DoiChecker;
        let block =
            "@article{paper1,\n  title={Attention Is All You Need},\n  doi={10.5555/doi12345}\n}\n";

        let id = checker.extract_identifier(block);
        assert_eq!(id.as_deref(), Some("10.5555/doi12345"));

        assert!(
            checker
                .get_cached_verification(&db, "10.5555/doi12345")
                .unwrap()
                .is_none()
        );

        checker
            .save_verification(&db, "10.5555/doi12345", true, None)
            .unwrap();

        let cached = checker
            .get_cached_verification(&db, "10.5555/doi12345")
            .unwrap();
        assert_eq!(cached, Some((true, None)));
    }

    #[test]
    fn test_arxiv_extraction_and_caching() {
        let db = SilDb::open_in_memory().unwrap();
        let checker = ArxivChecker;
        let block = "@article{paper2,\n  title={Deep Residual Learning},\n  eprint={1512.03385},\n  archivePrefix={arXiv}\n}\n";

        let id = checker.extract_identifier(block);
        assert_eq!(id.as_deref(), Some("1512.03385"));

        assert!(
            checker
                .get_cached_verification(&db, "1512.03385")
                .unwrap()
                .is_none()
        );

        checker
            .save_verification(&db, "1512.03385", true, None)
            .unwrap();

        let cached = checker
            .get_cached_verification(&db, "1512.03385")
            .unwrap()
            .unwrap();
        assert_eq!(cached, (true, None));
    }

    #[test]
    fn test_openreview_extraction_and_caching() {
        let db = SilDb::open_in_memory().unwrap();
        let checker = OpenReviewChecker;
        let block = "@inproceedings{paper3,\n  title={Masked Autoencoders},\n  url={https://openreview.net/forum?id=u5v2a7xyz99}\n}\n";

        let id = checker.extract_identifier(block);
        assert_eq!(id.as_deref(), Some("u5v2a7xyz99"));

        assert!(
            checker
                .get_cached_verification(&db, "u5v2a7xyz99")
                .unwrap()
                .is_none()
        );

        checker
            .save_verification(&db, "u5v2a7xyz99", true, Some("title_mismatch"))
            .unwrap();

        let cached = checker
            .get_cached_verification(&db, "u5v2a7xyz99")
            .unwrap()
            .unwrap();
        assert_eq!(cached, (true, Some("title_mismatch".to_string())));
    }

    #[test]
    fn test_run_all_checkers_incremental_cached() {
        let db = SilDb::open_in_memory().unwrap();
        let bib = r#"
@article{doi_paper,
  title={Paper 1},
  doi={10.5555/cached_doi_1}
}

@article{arxiv_paper,
  title={Paper 2},
  journal={arXiv:1706.03762}
}

@article{openreview_paper,
  title={Paper 3},
  url={https://openreview.net/forum?id=ab12cd34ef56}
}

@article{entryfour,
  title={Simple Title Four}
}
"#;

        // Pre-populate DB caches
        DoiChecker
            .save_verification(&db, "10.5555/cached_doi_1", true, None)
            .unwrap();
        ArxivChecker
            .save_verification(&db, "1706.03762", true, None)
            .unwrap();
        OpenReviewChecker
            .save_verification(&db, "ab12cd34ef56", false, Some("http_404"))
            .unwrap();

        let report = run_all_checkers_incremental(&db, bib, false).unwrap();
        assert_eq!(report.total_entries, 4);
        assert_eq!(report.entries_with_identifier, 3);
        assert_eq!(
            report.checked_online, 0,
            "All entries with identifiers should hit cache"
        );
        assert_eq!(report.skipped_cached, 3);
        assert_eq!(report.valid_identifiers, 2);
        assert_eq!(report.broken_identifiers.len(), 1);
        assert_eq!(report.broken_identifiers[0].0, "openreview_paper");
        assert_eq!(report.broken_identifiers[0].1, "OpenReview");
        assert_eq!(report.broken_identifiers[0].2, "ab12cd34ef56");

        // Verify item reports
        assert_eq!(report.items.len(), 4);
        assert_eq!(report.items[0].identifier_type, "DOI");
        assert_eq!(
            report.items[0].category,
            ReferenceCheckCategory::SkippedCached
        );

        assert_eq!(report.items[1].identifier_type, "arXiv");
        assert_eq!(
            report.items[1].category,
            ReferenceCheckCategory::SkippedCached
        );

        assert_eq!(report.items[2].identifier_type, "OpenReview");
        assert_eq!(
            report.items[2].category,
            ReferenceCheckCategory::SkippedCached
        );

        assert_eq!(report.items[3].identifier_type, "None");
        assert_eq!(
            report.items[3].category,
            ReferenceCheckCategory::InvalidFormat
        );
    }
}
