//! DOI reference checker.

use sil_db::{DbError, SilDb};

use super::ReferenceChecker;

/// Reference checker for Digital Object Identifiers (DOIs).
pub struct DoiChecker;

impl ReferenceChecker for DoiChecker {
    fn identifier_name(&self) -> &'static str {
        "DOI"
    }

    fn extract_identifier(&self, block: &str) -> Option<String> {
        sil_regex::extract_doi(block)
            .map(|d| sil_api::clean_doi_str(&d))
            .filter(|d| !d.is_empty())
    }

    fn verify_online(&self, id: &str) -> Result<sil_api::DoiMetadataResult, sil_api::ApiError> {
        sil_api::verify_doi_with_metadata(id)
    }

    fn fetch_official_bibtex(&self, id: &str) -> Result<Option<String>, sil_api::ApiError> {
        sil_api::fetch_bibtex_by_doi(id)
    }

    fn get_cached_verification(
        &self,
        db: &SilDb,
        id: &str,
    ) -> Result<Option<(bool, Option<String>)>, DbError> {
        let opt = db.get_doi_verification(id)?;
        Ok(opt.map(|rec| (rec.exists_flag, rec.error_cat)))
    }

    fn save_verification(
        &self,
        db: &SilDb,
        id: &str,
        exists: bool,
        error_cat: Option<&str>,
    ) -> Result<(), DbError> {
        db.upsert_doi_verification(id, exists, error_cat)
    }
}
