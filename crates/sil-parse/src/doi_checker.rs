//! Incremental DOI checking and background orchestrator for BibTeX files.

use std::path::PathBuf;
use std::thread::JoinHandle;

use serde::{Deserialize, Serialize};
use sil_db::SilDb;

use crate::error::ParseError;

/// Outcome category for checking an individual BibTeX item's DOI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DoiCheckCategory {
    /// DOI was checked online and verified to exist.
    Valid,
    /// DOI was checked online, exists, but paper title differs from Crossref official metadata (< 60% similarity).
    TitleMismatch {
        /// Local title extracted from BibTeX block.
        local_title: String,
        /// Official title returned by Crossref metadata.
        official_title: String,
        /// Jaccard title similarity score (0.0 .. 1.0).
        similarity: f64,
    },
    /// DOI was checked online and returned 404 Not Found.
    NotFound,
    /// Checking DOI failed with a network error or rate limit error.
    NetworkError(String),
    /// BibTeX entry lacks a valid DOI format or string.
    InvalidFormat,
    /// DOI check was skipped because it was already verified in the local DB cache.
    SkippedCached,
}

/// Report for a single BibTeX entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BibDoiItemReport {
    /// Citation key of the BibTeX entry.
    pub cite_key: String,
    /// Extracted DOI string, if available.
    pub doi: Option<String>,
    /// Result category of the DOI check.
    pub category: DoiCheckCategory,
}

/// Comprehensive report for an incremental BibTeX DOI check batch.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DoiCheckReport {
    /// Total number of BibTeX entries parsed from content.
    pub total_entries: usize,
    /// Number of entries with a valid extracted DOI.
    pub entries_with_doi: usize,
    /// Number of DOIs checked via network requests.
    pub checked_online: usize,
    /// Total number of valid DOIs (online and cached).
    pub valid_dois: usize,
    /// List of broken DOIs (cite_key, doi) returning 404.
    pub broken_dois: Vec<(String, String)>,
    /// List of DOIs that failed with network errors (cite_key, doi, err_msg).
    pub network_errors: Vec<(String, String, String)>,
    /// List of title mismatched DOIs (cite_key, local_title, official_title, similarity).
    pub mismatched_dois: Vec<(String, String, String, f64)>,
    /// Number of DOIs skipped due to incremental caching.
    pub skipped_cached: usize,
    /// Number of BibTeX entries automatically updated with official BibTeX.
    pub autofixed_count: usize,
    /// Updated BibTeX content string if autofix modified any entries.
    pub updated_bib_content: Option<String>,
    /// Individual reports for each parsed BibTeX item.
    pub items: Vec<BibDoiItemReport>,
}

/// Incrementally check DOIs for BibTeX entries in `bib_content` using stored cache in `db`.
///
/// 1. Parses `bib_content` into blocks using [`sil_core::bib::parse_bib_blocks`].
/// 2. Retrieves stored [`BibReferenceRecord`]s and [`DoiVerificationRecord`]s from `db`.
/// 3. For each block:
///    - Extracts cite key, local title, and DOI.
///    - If cached and unchanged: marks as [`DoiCheckCategory::SkippedCached`], performs update surgery
///      via [`SilDb::upsert_bib_reference`] (skipping DB mutation if identical), and avoids network calls.
///    - If uncached: queries Crossref via [`sil_api::verify_doi_with_metadata`], records result in DB via
///      [`SilDb::upsert_doi_verification`] and [`SilDb::upsert_bib_reference`].
///    - If similarity between local title and official title < 0.60: records title mismatch.
///    - If `autofix` is true: fetches official BibTeX via [`sil_api::fetch_bibtex_by_doi`] and updates `bib_content`.
/// 4. Wraps all operations in panic safety to guarantee invalid BibTeX or network errors never crash.
pub fn check_bib_dois_incremental(
    db: &SilDb,
    bib_content: &str,
    autofix: bool,
) -> Result<DoiCheckReport, ParseError> {
    let unified_report = crate::checkers::run_all_checkers_incremental(db, bib_content, autofix)?;

    let mut report = DoiCheckReport {
        total_entries: unified_report.total_entries,
        autofixed_count: unified_report.autofixed_count,
        updated_bib_content: unified_report.updated_bib_content,
        ..Default::default()
    };

    report.mismatched_dois = unified_report
        .mismatched_identifiers
        .into_iter()
        .filter(|(_, id_type, _, _, _)| id_type == "DOI")
        .map(|(k, _, loc, off, sim)| (k, loc, off, sim))
        .collect();

    report.broken_dois = unified_report
        .broken_identifiers
        .into_iter()
        .filter(|(_, id_type, _)| id_type == "DOI")
        .map(|(k, _, id)| (k, id))
        .collect();

    report.network_errors = unified_report
        .network_errors
        .into_iter()
        .filter(|(_, id_type, _, _)| id_type == "DOI")
        .map(|(k, _, id, msg)| (k, id, msg))
        .collect();

    for item in unified_report.items {
        if item.identifier_type == "DOI" {
            let doi = item.identifier.clone();
            report.entries_with_doi += 1;

            let doi_category = match item.category {
                crate::checkers::ReferenceCheckCategory::Valid => {
                    report.valid_dois += 1;
                    DoiCheckCategory::Valid
                }
                crate::checkers::ReferenceCheckCategory::TitleMismatch {
                    local_title,
                    official_title,
                    similarity,
                } => DoiCheckCategory::TitleMismatch {
                    local_title,
                    official_title,
                    similarity,
                },
                crate::checkers::ReferenceCheckCategory::NotFound => DoiCheckCategory::NotFound,
                crate::checkers::ReferenceCheckCategory::NetworkError(ref msg) => {
                    DoiCheckCategory::NetworkError(msg.clone())
                }
                crate::checkers::ReferenceCheckCategory::InvalidFormat => {
                    DoiCheckCategory::InvalidFormat
                }
                crate::checkers::ReferenceCheckCategory::SkippedCached => {
                    report.skipped_cached += 1;
                    let doi_str = doi.as_deref().unwrap_or("");
                    let is_mismatched = report
                        .mismatched_dois
                        .iter()
                        .any(|(k, _, _, _)| k == &item.cite_key);
                    let is_broken = report.broken_dois.iter().any(|(k, _)| k == &item.cite_key);
                    let is_net_err = report
                        .network_errors
                        .iter()
                        .any(|(k, _, _)| k == &item.cite_key);
                    if !is_mismatched && !is_broken && !is_net_err && !doi_str.is_empty() {
                        report.valid_dois += 1;
                    }
                    DoiCheckCategory::SkippedCached
                }
            };

            report.items.push(BibDoiItemReport {
                cite_key: item.cite_key,
                doi,
                category: doi_category,
            });
        } else if item.identifier_type == "None" {
            report.items.push(BibDoiItemReport {
                cite_key: item.cite_key,
                doi: None,
                category: DoiCheckCategory::InvalidFormat,
            });
        }
    }

    report.checked_online = report
        .items
        .iter()
        .filter(|i| i.doi.is_some() && i.category != DoiCheckCategory::SkippedCached)
        .count();

    Ok(report)
}

/// Spawns a background thread that opens `SilDb` at `db_path`, reads `bib_path`, and executes [`check_bib_dois_incremental`].
pub fn spawn_background_bib_doi_check(
    db_path: PathBuf,
    bib_path: PathBuf,
    autofix: bool,
) -> JoinHandle<Result<DoiCheckReport, ParseError>> {
    std::thread::spawn(move || {
        let utf8_db_path = camino::Utf8PathBuf::from_path_buf(db_path)
            .map_err(|p| ParseError::Message(format!("invalid utf-8 path: {p:?}")))?;
        let db = SilDb::open(&utf8_db_path)?;
        let bib_content = std::fs::read_to_string(&bib_path).map_err(|e| {
            ParseError::Message(format!("failed to read bib file {bib_path:?}: {e}"))
        })?;
        check_bib_dois_incremental(&db, &bib_content, autofix)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incremental_doi_check_skips_cached_dois() {
        let db = SilDb::open_in_memory().unwrap();
        let bib =
            "@article{paper1,\n  title={Attention Is All You Need},\n  doi={10.5555/cached_1}\n}\n";

        db.upsert_doi_verification("10.5555/cached_1", true, None)
            .unwrap();
        db.upsert_bib_reference("paper1", Some("10.5555/cached_1"), Some(true), bib)
            .unwrap();

        let report = check_bib_dois_incremental(&db, bib, false).unwrap();
        assert_eq!(report.total_entries, 1);
        assert_eq!(report.entries_with_doi, 1);
        assert_eq!(
            report.checked_online, 0,
            "Cached DOI should perform 0 network checks"
        );
        assert_eq!(report.skipped_cached, 1);
        assert_eq!(report.valid_dois, 1);
        assert_eq!(report.items.len(), 1);
        assert_eq!(report.items[0].category, DoiCheckCategory::SkippedCached);
    }

    #[test]
    fn test_incremental_doi_check_text_changed_same_doi() {
        let db = SilDb::open_in_memory().unwrap();
        let old_bib = "@article{paper1,\n  title={Old Title},\n  doi={10.5555/cached_2}\n}\n";
        let new_bib = "@article{paper1,\n  title={New Updated Title},\n  author={Vaswani, A.},\n  doi={10.5555/cached_2}\n}\n";

        db.upsert_doi_verification("10.5555/cached_2", true, None)
            .unwrap();
        db.upsert_bib_reference("paper1", Some("10.5555/cached_2"), Some(true), old_bib)
            .unwrap();

        let report = check_bib_dois_incremental(&db, new_bib, false).unwrap();
        assert_eq!(
            report.checked_online, 0,
            "Editing text with same DOI must skip network check"
        );
        assert_eq!(report.skipped_cached, 1);

        let refs = db.get_bib_references().unwrap();
        assert_eq!(refs.len(), 1);
        assert!(refs[0].raw_bibtex.contains("New Updated Title"));
    }

    #[test]
    fn test_incremental_doi_check_detects_new_doi() {
        let db = SilDb::open_in_memory().unwrap();
        let bib = "@article{cached_paper,\n  title={Cached Paper},\n  doi={10.5555/cached_3}\n}\n\n@article{new_paper,\n  title={New Paper},\n  doi={10.0000/nonexistent_new_doi_12345}\n}\n";

        db.upsert_doi_verification("10.5555/cached_3", true, None)
            .unwrap();
        db.upsert_bib_reference(
            "cached_paper",
            Some("10.5555/cached_3"),
            Some(true),
            "@article{cached_paper, doi={10.5555/cached_3}}",
        )
        .unwrap();

        let report = check_bib_dois_incremental(&db, bib, false).unwrap();
        assert_eq!(report.total_entries, 2);
        assert_eq!(report.entries_with_doi, 2);
        assert_eq!(report.skipped_cached, 1, "Cached paper must be skipped");
        assert_eq!(
            report.checked_online, 1,
            "Only new paper should be checked online"
        );
        assert_eq!(report.items[0].category, DoiCheckCategory::SkippedCached);
        assert!(
            matches!(
                report.items[1].category,
                DoiCheckCategory::NotFound | DoiCheckCategory::NetworkError(_)
            ),
            "Expected NotFound or NetworkError, got {:?}",
            report.items[1].category
        );
    }

    #[test]
    fn test_incremental_doi_check_updated_doi() {
        let db = SilDb::open_in_memory().unwrap();
        let old_bib = "@article{paper1,\n  title={Paper 1},\n  doi={10.5555/old_doi}\n}\n";
        let updated_bib = "@article{paper1,\n  title={Paper 1},\n  doi={10.0000/nonexistent_updated_doi_67890}\n}\n";

        db.upsert_doi_verification("10.5555/old_doi", true, None)
            .unwrap();
        db.upsert_bib_reference("paper1", Some("10.5555/old_doi"), Some(true), old_bib)
            .unwrap();

        let report = check_bib_dois_incremental(&db, updated_bib, false).unwrap();
        assert_eq!(report.skipped_cached, 0, "Changed DOI must not be skipped");
        assert_eq!(
            report.checked_online, 1,
            "Updated DOI must be checked online"
        );
        assert!(
            matches!(
                report.items[0].category,
                DoiCheckCategory::NotFound | DoiCheckCategory::NetworkError(_)
            ),
            "Expected NotFound or NetworkError, got {:?}",
            report.items[0].category
        );

        let refs = db.get_bib_references().unwrap();
        assert_eq!(
            refs[0].doi.as_deref(),
            Some("10.0000/nonexistent_updated_doi_67890")
        );
    }

    #[test]
    fn test_categorized_errors() {
        let db = SilDb::open_in_memory().unwrap();

        db.upsert_doi_verification("10.5555/valid_1", true, None)
            .unwrap();
        db.upsert_bib_reference(
            "k1",
            Some("10.5555/valid_1"),
            Some(true),
            "@article{k1, doi={10.5555/valid_1}}",
        )
        .unwrap();

        db.upsert_doi_verification("10.5555/404_1", false, Some("http_404"))
            .unwrap();
        db.upsert_bib_reference(
            "k2",
            Some("10.5555/404_1"),
            Some(false),
            "@article{k2, doi={10.5555/404_1}}",
        )
        .unwrap();

        db.upsert_doi_verification("10.5555/net_1", false, Some("network_error"))
            .unwrap();
        db.upsert_bib_reference(
            "k3",
            Some("10.5555/net_1"),
            Some(false),
            "@article{k3, doi={10.5555/net_1}}",
        )
        .unwrap();

        let bib = "@article{k1, doi={10.5555/valid_1}}\n\n@article{k2, doi={10.5555/404_1}}\n\n@article{k3, doi={10.5555/net_1}}\n\n@article{k4, title={No DOI Entry}}\n";

        let report = check_bib_dois_incremental(&db, bib, false).unwrap();
        assert_eq!(report.total_entries, 4);
        assert_eq!(report.entries_with_doi, 3);
        assert_eq!(report.skipped_cached, 3);
        assert_eq!(report.checked_online, 0);

        assert_eq!(report.valid_dois, 1);
        assert_eq!(
            report.broken_dois,
            vec![("k2".to_string(), "10.5555/404_1".to_string())]
        );
        assert_eq!(report.network_errors.len(), 1);
        assert_eq!(report.network_errors[0].0, "k3");
        assert_eq!(report.network_errors[0].1, "10.5555/net_1");

        assert_eq!(report.items[0].category, DoiCheckCategory::SkippedCached);
        assert_eq!(report.items[1].category, DoiCheckCategory::SkippedCached);
        assert_eq!(report.items[2].category, DoiCheckCategory::SkippedCached);
        assert_eq!(report.items[3].category, DoiCheckCategory::InvalidFormat);
    }

    #[test]
    fn test_spawn_background_bib_doi_check() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test_bg.sqlite");
        let bib_path = dir.path().join("test_bg.bib");

        let bib_content = "@article{bg1, title={Background Test}, doi={10.5555/bg_cached}}\n";
        std::fs::write(&bib_path, bib_content).unwrap();

        {
            let utf8_db_path = camino::Utf8PathBuf::from_path_buf(db_path.clone()).unwrap();
            let db = SilDb::open(&utf8_db_path).unwrap();
            db.upsert_doi_verification("10.5555/bg_cached", true, None)
                .unwrap();
            db.upsert_bib_reference("bg1", Some("10.5555/bg_cached"), Some(true), bib_content)
                .unwrap();
        }

        let handle = spawn_background_bib_doi_check(db_path, bib_path, false);
        let report = handle.join().unwrap().unwrap();
        assert_eq!(report.total_entries, 1);
        assert_eq!(report.skipped_cached, 1);
    }

    #[test]
    fn test_title_mismatch_detection() {
        let db = SilDb::open_in_memory().unwrap();
        let bib = "@article{paper1,\n  title={Totally Incorrect Local Title},\n  doi={10.1038/s41586-020-2649-2}\n}\n";

        let report = check_bib_dois_incremental(&db, bib, false).unwrap();
        assert_eq!(report.total_entries, 1);
        assert_eq!(report.checked_online, 1);

        match &report.items[0].category {
            DoiCheckCategory::TitleMismatch {
                local_title,
                official_title,
                similarity,
            } => {
                assert_eq!(local_title, "Totally Incorrect Local Title");
                assert!(!official_title.is_empty());
                assert!(*similarity < 0.60);
                assert_eq!(report.mismatched_dois.len(), 1);
                assert_eq!(report.mismatched_dois[0].0, "paper1");
                assert_eq!(report.mismatched_dois[0].1, "Totally Incorrect Local Title");

                let ver = db.get_doi_verifications().unwrap();
                let record = &ver["10.1038/s41586-020-2649-2"];
                assert!(record.exists_flag);
                assert_eq!(record.error_cat.as_deref(), Some("title_mismatch"));
            }
            DoiCheckCategory::NetworkError(err) => {
                assert!(!err.is_empty());
            }
            other => panic!("Unexpected category: {:?}", other),
        }
    }

    #[test]
    fn test_autofix_mismatched_bibtex_entry() {
        let db = SilDb::open_in_memory().unwrap();
        let bib = "@article{paper1,\n  title={Wrong Local Title},\n  doi={10.1038/s41586-020-2649-2}\n}\n";

        let report = check_bib_dois_incremental(&db, bib, true).unwrap();
        assert_eq!(report.total_entries, 1);

        match &report.items[0].category {
            DoiCheckCategory::TitleMismatch { .. } => {
                assert_eq!(report.autofixed_count, 1);
                assert!(report.updated_bib_content.is_some());
                let updated = report.updated_bib_content.as_ref().unwrap();
                assert!(
                    updated.contains("@article{paper1,")
                        || updated.contains("10.1038/s41586-020-2649-2")
                );
            }
            DoiCheckCategory::NetworkError(_) => {
                assert_eq!(report.autofixed_count, 0);
                assert!(report.updated_bib_content.is_none());
            }
            other => panic!("Unexpected category: {:?}", other),
        }
    }

    #[test]
    fn test_panic_safety_malformed_bibtex() {
        let db = SilDb::open_in_memory().unwrap();

        let malformed_inputs = [
            "@article{unclosed_key, title={unclosed brace, doi={10.5555/1234}\n",
            "\0\0\0\u{00FF}\u{00FE}@@@@@{{{}}}@article{,,,==",
            "@article{,,,,,, doi=,,, title=,,,}",
            "random garbage without at symbol \0\0\x1f",
            "{}",
            "@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@",
        ];

        for input in malformed_inputs {
            let res = check_bib_dois_incremental(&db, input, false);
            assert!(
                res.is_ok(),
                "Expected safe Result return, got error for input: {:?}",
                input
            );
        }
    }
}
