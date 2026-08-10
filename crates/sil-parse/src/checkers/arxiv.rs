//! arXiv reference checker.

use sil_db::{DbError, SilDb};

use super::ReferenceChecker;

/// Reference checker for arXiv identifiers.
pub struct ArxivChecker;

impl ReferenceChecker for ArxivChecker {
    fn identifier_name(&self) -> &'static str {
        "arXiv"
    }

    fn extract_identifier(&self, block: &str) -> Option<String> {
        sil_regex::extract_arxiv_id(block)
            .map(|a| sil_api::clean_arxiv_id_str(&a))
            .filter(|a| !a.is_empty())
    }

    fn verify_online(&self, id: &str) -> Result<sil_api::DoiMetadataResult, sil_api::ApiError> {
        sil_api::verify_arxiv_with_metadata(id)
    }

    fn fetch_official_bibtex(&self, id: &str) -> Result<Option<String>, sil_api::ApiError> {
        sil_api::fetch_bibtex_by_arxiv_id(id)
    }

    fn get_cached_verification(
        &self,
        db: &SilDb,
        id: &str,
    ) -> Result<Option<(bool, Option<String>)>, DbError> {
        let opt = db.get_arxiv_verification(id)?;
        Ok(opt.map(|rec| (rec.exists_flag, rec.error_cat)))
    }

    fn save_verification(
        &self,
        db: &SilDb,
        id: &str,
        exists: bool,
        error_cat: Option<&str>,
    ) -> Result<(), DbError> {
        db.upsert_arxiv_verification(id, exists, error_cat)
    }
}
