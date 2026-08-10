//! OpenReview reference checker.

use sil_db::{DbError, SilDb};

use super::ReferenceChecker;

/// Reference checker for OpenReview note identifiers.
pub struct OpenReviewChecker;

impl ReferenceChecker for OpenReviewChecker {
    fn identifier_name(&self) -> &'static str {
        "OpenReview"
    }

    fn extract_identifier(&self, block: &str) -> Option<String> {
        sil_regex::extract_openreview_id(block)
            .map(|o| sil_api::clean_openreview_id_str(&o))
            .filter(|o| !o.is_empty())
    }

    fn verify_online(&self, id: &str) -> Result<sil_api::DoiMetadataResult, sil_api::ApiError> {
        sil_api::verify_openreview_with_metadata(id)
    }

    fn fetch_official_bibtex(&self, id: &str) -> Result<Option<String>, sil_api::ApiError> {
        sil_api::fetch_bibtex_by_openreview_id(id)
    }

    fn get_cached_verification(
        &self,
        db: &SilDb,
        id: &str,
    ) -> Result<Option<(bool, Option<String>)>, DbError> {
        let opt = db.get_openreview_verification(id)?;
        Ok(opt.map(|rec| (rec.exists_flag, rec.error_cat)))
    }

    fn save_verification(
        &self,
        db: &SilDb,
        id: &str,
        exists: bool,
        error_cat: Option<&str>,
    ) -> Result<(), DbError> {
        db.upsert_openreview_verification(id, exists, error_cat)
    }
}
