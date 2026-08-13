//! BibTeX reference tracking and DOI verification persistence.

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::DbError;

/// Represents a row in the `bib_references` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibReferenceRecord {
    /// Citation key (primary key, e.g. "Vaswani2017").
    pub cite_key: String,
    /// Extracted or associated DOI if available.
    pub doi: Option<String>,
    /// Flag indicating whether the DOI has been verified to exist (`None` if unchecked).
    pub doi_exists: Option<bool>,
    /// Raw BibTeX text entry.
    pub raw_bibtex: String,
    /// ISO timestamp of last DOI check.
    pub checked_at: Option<String>,
    /// ISO timestamp of last update in DB.
    pub updated_at: String,
}

/// Represents a row in the `doi_verifications` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoiVerificationRecord {
    /// DOI string (primary key).
    pub doi: String,
    /// Whether the DOI exists/resolves.
    pub exists_flag: bool,
    /// Optional error category if check failed (e.g., "http_404", "timeout", "network_error").
    pub error_cat: Option<String>,
    /// ISO timestamp when check was performed.
    pub checked_at: String,
}

/// Represents a row in the `arxiv_verifications` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArxivVerificationRecord {
    /// arXiv ID string (primary key, e.g. "2106.09685").
    pub arxiv_id: String,
    /// Whether the arXiv ID exists/resolves.
    pub exists_flag: bool,
    /// Optional error category if check failed (e.g., "http_404", "timeout").
    pub error_cat: Option<String>,
    /// ISO timestamp when check was performed.
    pub checked_at: String,
}

/// Represents a row in the `openreview_verifications` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenreviewVerificationRecord {
    /// OpenReview ID string (primary key, e.g. "forum?id=...").
    pub openreview_id: String,
    /// Whether the OpenReview ID exists/resolves.
    pub exists_flag: bool,
    /// Optional error category if check failed (e.g., "http_404", "timeout").
    pub error_cat: Option<String>,
    /// ISO timestamp when check was performed.
    pub checked_at: String,
}

/// Get all records from `bib_references` table.
pub fn get_bib_references(conn: &Connection) -> Result<Vec<BibReferenceRecord>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT cite_key, doi, doi_exists, raw_bibtex, checked_at, updated_at
         FROM bib_references
         ORDER BY cite_key ASC",
    )?;

    let rows = stmt.query_map([], |row| {
        let doi_exists_raw: Option<i32> = row.get(2)?;
        Ok(BibReferenceRecord {
            cite_key: row.get(0)?,
            doi: row.get(1)?,
            doi_exists: doi_exists_raw.map(|v| v != 0),
            raw_bibtex: row.get(3)?,
            checked_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Get a single DOI verification record by DOI.
pub fn get_doi_verification(
    conn: &Connection,
    doi: &str,
) -> Result<Option<DoiVerificationRecord>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT doi, exists_flag, error_cat, checked_at
         FROM doi_verifications
         WHERE doi = ?1",
    )?;

    let mut rows = stmt.query(params![doi])?;
    if let Some(row) = rows.next()? {
        let exists_raw: i32 = row.get(1)?;
        Ok(Some(DoiVerificationRecord {
            doi: row.get(0)?,
            exists_flag: exists_raw != 0,
            error_cat: row.get(2)?,
            checked_at: row.get(3)?,
        }))
    } else {
        Ok(None)
    }
}

/// Get all DOI verification records as a HashMap keyed by DOI string.
pub fn get_doi_verifications(
    conn: &Connection,
) -> Result<HashMap<String, DoiVerificationRecord>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT doi, exists_flag, error_cat, checked_at
         FROM doi_verifications",
    )?;

    let rows = stmt.query_map([], |row| {
        let exists_raw: i32 = row.get(1)?;
        let doi: String = row.get(0)?;
        let record = DoiVerificationRecord {
            doi: doi.clone(),
            exists_flag: exists_raw != 0,
            error_cat: row.get(2)?,
            checked_at: row.get(3)?,
        };
        Ok((doi, record))
    })?;

    let mut map = HashMap::new();
    for r in rows {
        let (doi, record) = r?;
        map.insert(doi, record);
    }
    Ok(map)
}

/// Upsert a DOI verification record.
pub fn upsert_doi_verification(
    conn: &Connection,
    doi: &str,
    exists: bool,
    error_cat: Option<&str>,
) -> Result<(), DbError> {
    conn.execute(
        "INSERT OR REPLACE INTO doi_verifications (doi, exists_flag, error_cat, checked_at)
         VALUES (?1, ?2, ?3, datetime('now'))",
        params![doi, if exists { 1 } else { 0 }, error_cat],
    )?;
    Ok(())
}

/// Get a single arXiv verification record by arXiv ID.
pub fn get_arxiv_verification(
    conn: &Connection,
    arxiv_id: &str,
) -> Result<Option<ArxivVerificationRecord>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT arxiv_id, exists_flag, error_cat, checked_at
         FROM arxiv_verifications
         WHERE arxiv_id = ?1",
    )?;

    let mut rows = stmt.query(params![arxiv_id])?;
    if let Some(row) = rows.next()? {
        let exists_raw: i32 = row.get(1)?;
        Ok(Some(ArxivVerificationRecord {
            arxiv_id: row.get(0)?,
            exists_flag: exists_raw != 0,
            error_cat: row.get(2)?,
            checked_at: row.get(3)?,
        }))
    } else {
        Ok(None)
    }
}

/// Get all arXiv verification records as a HashMap keyed by arXiv ID string.
pub fn get_arxiv_verifications(
    conn: &Connection,
) -> Result<HashMap<String, ArxivVerificationRecord>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT arxiv_id, exists_flag, error_cat, checked_at
         FROM arxiv_verifications",
    )?;

    let rows = stmt.query_map([], |row| {
        let exists_raw: i32 = row.get(1)?;
        let arxiv_id: String = row.get(0)?;
        let record = ArxivVerificationRecord {
            arxiv_id: arxiv_id.clone(),
            exists_flag: exists_raw != 0,
            error_cat: row.get(2)?,
            checked_at: row.get(3)?,
        };
        Ok((arxiv_id, record))
    })?;

    let mut map = HashMap::new();
    for r in rows {
        let (arxiv_id, record) = r?;
        map.insert(arxiv_id, record);
    }
    Ok(map)
}

/// Upsert an arXiv verification record.
pub fn upsert_arxiv_verification(
    conn: &Connection,
    arxiv_id: &str,
    exists: bool,
    error_cat: Option<&str>,
) -> Result<(), DbError> {
    conn.execute(
        "INSERT OR REPLACE INTO arxiv_verifications (arxiv_id, exists_flag, error_cat, checked_at)
         VALUES (?1, ?2, ?3, datetime('now'))",
        params![arxiv_id, if exists { 1 } else { 0 }, error_cat],
    )?;
    Ok(())
}

/// Get a single OpenReview verification record by OpenReview ID.
pub fn get_openreview_verification(
    conn: &Connection,
    openreview_id: &str,
) -> Result<Option<OpenreviewVerificationRecord>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT openreview_id, exists_flag, error_cat, checked_at
         FROM openreview_verifications
         WHERE openreview_id = ?1",
    )?;

    let mut rows = stmt.query(params![openreview_id])?;
    if let Some(row) = rows.next()? {
        let exists_raw: i32 = row.get(1)?;
        Ok(Some(OpenreviewVerificationRecord {
            openreview_id: row.get(0)?,
            exists_flag: exists_raw != 0,
            error_cat: row.get(2)?,
            checked_at: row.get(3)?,
        }))
    } else {
        Ok(None)
    }
}

/// Get all OpenReview verification records as a HashMap keyed by OpenReview ID string.
pub fn get_openreview_verifications(
    conn: &Connection,
) -> Result<HashMap<String, OpenreviewVerificationRecord>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT openreview_id, exists_flag, error_cat, checked_at
         FROM openreview_verifications",
    )?;

    let rows = stmt.query_map([], |row| {
        let exists_raw: i32 = row.get(1)?;
        let openreview_id: String = row.get(0)?;
        let record = OpenreviewVerificationRecord {
            openreview_id: openreview_id.clone(),
            exists_flag: exists_raw != 0,
            error_cat: row.get(2)?,
            checked_at: row.get(3)?,
        };
        Ok((openreview_id, record))
    })?;

    let mut map = HashMap::new();
    for r in rows {
        let (openreview_id, record) = r?;
        map.insert(openreview_id, record);
    }
    Ok(map)
}

/// Upsert an OpenReview verification record.
pub fn upsert_openreview_verification(
    conn: &Connection,
    openreview_id: &str,
    exists: bool,
    error_cat: Option<&str>,
) -> Result<(), DbError> {
    conn.execute(
        "INSERT OR REPLACE INTO openreview_verifications (openreview_id, exists_flag, error_cat, checked_at)
         VALUES (?1, ?2, ?3, datetime('now'))",
        params![openreview_id, if exists { 1 } else { 0 }, error_cat],
    )?;
    Ok(())
}

/// Upsert a bib reference record using UPDATE SURGERY logic.
///
/// First checks if a row with `cite_key` exists. If it exists AND
/// `(doi == existing.doi && doi_exists == existing.doi_exists && raw_bibtex == existing.raw_bibtex)`,
/// returns `Ok(false)` (skipped/no mutation).
/// If it's new or any field changed, executes `INSERT OR REPLACE INTO bib_references ...` and returns `Ok(true)` (mutated).
pub fn upsert_bib_reference(
    conn: &Connection,
    cite_key: &str,
    doi: Option<&str>,
    doi_exists: Option<bool>,
    raw_bibtex: &str,
) -> Result<bool, DbError> {
    let mut stmt = conn.prepare(
        "SELECT doi, doi_exists, raw_bibtex
         FROM bib_references
         WHERE cite_key = ?1",
    )?;

    let existing: Option<(Option<String>, Option<bool>, String)> = stmt
        .query_row(params![cite_key], |row| {
            let doi_exists_raw: Option<i32> = row.get(1)?;
            Ok((row.get(0)?, doi_exists_raw.map(|v| v != 0), row.get(2)?))
        })
        .optional()?;

    if let Some((ex_doi, ex_doi_exists, ex_raw_bibtex)) = existing {
        let is_identical = doi == ex_doi.as_deref()
            && doi_exists == ex_doi_exists
            && raw_bibtex == ex_raw_bibtex.as_str();
        if is_identical {
            return Ok(false);
        }
    }

    let doi_exists_int: Option<i32> = doi_exists.map(|b| if b { 1 } else { 0 });

    conn.execute(
        "INSERT OR REPLACE INTO bib_references (cite_key, doi, doi_exists, raw_bibtex, checked_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))",
        params![cite_key, doi, doi_exists_int, raw_bibtex],
    )?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bib_references_initial_insert_and_surgery() {
        let db = crate::SilDb::open_in_memory().unwrap();

        // 1. Initial insertion
        let mutated = upsert_bib_reference(
            &db.conn,
            "vaswani2017",
            Some("10.5555/3295222.3295349"),
            Some(true),
            "@article{vaswani2017, title={Attention is all you need}}",
        )
        .unwrap();
        assert!(mutated, "Initial insertion should return true (mutated)");

        let refs = get_bib_references(&db.conn).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].cite_key, "vaswani2017");
        assert_eq!(refs[0].doi.as_deref(), Some("10.5555/3295222.3295349"));
        assert_eq!(refs[0].doi_exists, Some(true));
        assert_eq!(
            refs[0].raw_bibtex,
            "@article{vaswani2017, title={Attention is all you need}}"
        );

        let initial_updated_at = refs[0].updated_at.clone();

        // 2. Re-upserting identical entry -> assert returns false (no mutation)
        let mutated_again = upsert_bib_reference(
            &db.conn,
            "vaswani2017",
            Some("10.5555/3295222.3295349"),
            Some(true),
            "@article{vaswani2017, title={Attention is all you need}}",
        )
        .unwrap();
        assert!(!mutated_again, "Identical re-upsert should return false");

        let refs_after_identical = get_bib_references(&db.conn).unwrap();
        assert_eq!(refs_after_identical.len(), 1);
        assert_eq!(
            refs_after_identical[0].updated_at, initial_updated_at,
            "updated_at timestamp should remain unchanged on skipped upsert"
        );

        // 3. Modifying entry field (doi_exists) -> assert returns true (surgery update)
        let mutated_field = upsert_bib_reference(
            &db.conn,
            "vaswani2017",
            Some("10.5555/3295222.3295349"),
            Some(false),
            "@article{vaswani2017, title={Attention is all you need}}",
        )
        .unwrap();
        assert!(
            mutated_field,
            "Modifying doi_exists should mutate and return true"
        );

        let refs_after_modified = get_bib_references(&db.conn).unwrap();
        assert_eq!(refs_after_modified.len(), 1);
        assert_eq!(refs_after_modified[0].doi_exists, Some(false));

        // 4. Modifying raw_bibtex -> assert returns true
        let mutated_bibtex = upsert_bib_reference(
            &db.conn,
            "vaswani2017",
            Some("10.5555/3295222.3295349"),
            Some(false),
            "@article{vaswani2017, title={Attention is all you need - Updated}}",
        )
        .unwrap();
        assert!(
            mutated_bibtex,
            "Modifying raw_bibtex should mutate and return true"
        );

        // 5. Modifying doi -> assert returns true
        let mutated_doi = upsert_bib_reference(
            &db.conn,
            "vaswani2017",
            Some("10.1000/new_doi"),
            Some(false),
            "@article{vaswani2017, title={Attention is all you need - Updated}}",
        )
        .unwrap();
        assert!(mutated_doi, "Modifying doi should mutate and return true");

        let final_refs = get_bib_references(&db.conn).unwrap();
        assert_eq!(final_refs[0].doi.as_deref(), Some("10.1000/new_doi"));
    }

    #[test]
    fn test_bib_references_none_fields() {
        let db = crate::SilDb::open_in_memory().unwrap();

        let mutated = upsert_bib_reference(
            &db.conn,
            "key_none",
            None,
            None,
            "@misc{key_none, title={No DOI}}",
        )
        .unwrap();
        assert!(mutated);

        let refs = get_bib_references(&db.conn).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].cite_key, "key_none");
        assert_eq!(refs[0].doi, None);
        assert_eq!(refs[0].doi_exists, None);

        // Re-upserting identical entry with None fields
        let re_up = upsert_bib_reference(
            &db.conn,
            "key_none",
            None,
            None,
            "@misc{key_none, title={No DOI}}",
        )
        .unwrap();
        assert!(
            !re_up,
            "Identical re-upsert with None fields should return false"
        );
    }

    #[test]
    fn test_doi_verifications_storing_and_retrieving() {
        let db = crate::SilDb::open_in_memory().unwrap();

        // Initially empty
        let initial_map = get_doi_verifications(&db.conn).unwrap();
        assert!(initial_map.is_empty());
        assert!(
            get_doi_verification(&db.conn, "10.1000/1")
                .unwrap()
                .is_none()
        );

        // Store valid DOI verification
        upsert_doi_verification(&db.conn, "10.1000/1", true, None).unwrap();

        let single = get_doi_verification(&db.conn, "10.1000/1").unwrap();
        assert!(single.is_some());
        let rec = single.unwrap();
        assert_eq!(rec.doi, "10.1000/1");
        assert!(rec.exists_flag);
        assert_eq!(rec.error_cat, None);

        // Store failing DOI verification with error category
        upsert_doi_verification(&db.conn, "10.1000/404", false, Some("http_404")).unwrap();

        let map = get_doi_verifications(&db.conn).unwrap();
        assert_eq!(map.len(), 2);
        let rec_404 = map.get("10.1000/404").unwrap();
        assert_eq!(rec_404.doi, "10.1000/404");
        assert!(!rec_404.exists_flag);
        assert_eq!(rec_404.error_cat.as_deref(), Some("http_404"));
    }

    #[test]
    fn test_arxiv_verifications_crud_and_surgery() {
        let db = crate::SilDb::open_in_memory().unwrap();

        // Initially empty
        let initial_map = get_arxiv_verifications(&db.conn).unwrap();
        assert!(initial_map.is_empty());
        assert!(
            get_arxiv_verification(&db.conn, "2106.09685")
                .unwrap()
                .is_none()
        );

        // Initial insert (Create)
        upsert_arxiv_verification(&db.conn, "2106.09685", true, None).unwrap();

        // Read single
        let single = get_arxiv_verification(&db.conn, "2106.09685").unwrap();
        assert!(single.is_some());
        let rec = single.unwrap();
        assert_eq!(rec.arxiv_id, "2106.09685");
        assert!(rec.exists_flag);
        assert_eq!(rec.error_cat, None);

        // Read map
        let map = get_arxiv_verifications(&db.conn).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("2106.09685").unwrap().arxiv_id, "2106.09685");

        // Update surgery (Update existing entry with new status/error_cat)
        upsert_arxiv_verification(&db.conn, "2106.09685", false, Some("http_404")).unwrap();

        let updated = get_arxiv_verification(&db.conn, "2106.09685")
            .unwrap()
            .unwrap();
        assert_eq!(updated.arxiv_id, "2106.09685");
        assert!(!updated.exists_flag);
        assert_eq!(updated.error_cat.as_deref(), Some("http_404"));
    }

    #[test]
    fn test_openreview_verifications_crud_and_surgery() {
        let db = crate::SilDb::open_in_memory().unwrap();

        // Initially empty
        let initial_map = get_openreview_verifications(&db.conn).unwrap();
        assert!(initial_map.is_empty());
        assert!(
            get_openreview_verification(&db.conn, "forum?id=abc12345")
                .unwrap()
                .is_none()
        );

        // Initial insert (Create)
        upsert_openreview_verification(&db.conn, "forum?id=abc12345", true, None).unwrap();

        // Read single
        let single = get_openreview_verification(&db.conn, "forum?id=abc12345").unwrap();
        assert!(single.is_some());
        let rec = single.unwrap();
        assert_eq!(rec.openreview_id, "forum?id=abc12345");
        assert!(rec.exists_flag);
        assert_eq!(rec.error_cat, None);

        // Read map
        let map = get_openreview_verifications(&db.conn).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(
            map.get("forum?id=abc12345").unwrap().openreview_id,
            "forum?id=abc12345"
        );

        // Update surgery (Update existing entry with new status/error_cat)
        upsert_openreview_verification(
            &db.conn,
            "forum?id=abc12345",
            false,
            Some("network_timeout"),
        )
        .unwrap();

        let updated = get_openreview_verification(&db.conn, "forum?id=abc12345")
            .unwrap()
            .unwrap();
        assert_eq!(updated.openreview_id, "forum?id=abc12345");
        assert!(!updated.exists_flag);
        assert_eq!(updated.error_cat.as_deref(), Some("network_timeout"));
    }
}
