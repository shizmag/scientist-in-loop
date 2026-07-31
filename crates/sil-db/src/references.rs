//! Source reference item persistence and full-text search.

use rusqlite::{Connection, params};
use sil_core::{ReferenceEntry, SourceId};

use crate::error::DbError;

/// Save a batch of reference entries for a source document.
pub fn save_source_references(conn: &Connection, refs: &[ReferenceEntry]) -> Result<(), DbError> {
    let mut stmt = conn.prepare(
        "INSERT OR REPLACE INTO source_references (id, source_id, ref_index, raw_text, title, authors, year, venue, doi)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;

    for entry in refs {
        stmt.execute(params![
            entry.id,
            entry.source_id.as_str(),
            entry.ref_index as i64,
            entry.raw_text,
            entry.title,
            entry.authors,
            entry.year,
            entry.venue,
            entry.doi,
        ])?;
    }

    Ok(())
}

/// Get all reference entries for a source document.
pub fn get_references_for_source(
    conn: &Connection,
    source_id: &SourceId,
) -> Result<Vec<ReferenceEntry>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, source_id, ref_index, raw_text, title, authors, year, venue, doi
         FROM source_references
         WHERE source_id = ?1
         ORDER BY ref_index ASC",
    )?;

    let rows = stmt.query_map(params![source_id.as_str()], |row| {
        let sid: String = row.get(1)?;
        Ok(ReferenceEntry {
            id: row.get(0)?,
            source_id: SourceId::new(sid),
            ref_index: row.get::<_, i64>(2)? as usize,
            raw_text: row.get(3)?,
            title: row.get(4)?,
            authors: row.get(5)?,
            year: row.get(6)?,
            venue: row.get(7)?,
            doi: row.get(8)?,
        })
    })?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Get all reference entries across all source documents.
pub fn get_all_references(conn: &Connection) -> Result<Vec<ReferenceEntry>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, source_id, ref_index, raw_text, title, authors, year, venue, doi
         FROM source_references
         ORDER BY source_id ASC, ref_index ASC",
    )?;

    let rows = stmt.query_map([], |row| {
        let sid: String = row.get(1)?;
        Ok(ReferenceEntry {
            id: row.get(0)?,
            source_id: SourceId::new(sid),
            ref_index: row.get::<_, i64>(2)? as usize,
            raw_text: row.get(3)?,
            title: row.get(4)?,
            authors: row.get(5)?,
            year: row.get(6)?,
            venue: row.get(7)?,
            doi: row.get(8)?,
        })
    })?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Full-text search over extracted source references.
pub fn search_references(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<ReferenceEntry>, DbError> {
    let clean = query.trim();
    if clean.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    // Escape FTS query
    let fts_query = clean.replace(['"', '\''], "");
    let mut stmt = conn.prepare(
        "SELECT r.id, r.source_id, r.ref_index, r.raw_text, r.title, r.authors, r.year, r.venue, r.doi
         FROM source_references r
         JOIN source_references_fts f ON r.rowid = f.rowid
         WHERE source_references_fts MATCH ?1
         LIMIT ?2",
    )?;

    let rows = stmt.query_map(params![fts_query, limit as i64], |row| {
        let sid: String = row.get(1)?;
        Ok(ReferenceEntry {
            id: row.get(0)?,
            source_id: SourceId::new(sid),
            ref_index: row.get::<_, i64>(2)? as usize,
            raw_text: row.get(3)?,
            title: row.get(4)?,
            authors: row.get(5)?,
            year: row.get(6)?,
            venue: row.get(7)?,
            doi: row.get(8)?,
        })
    })?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Delete all references for a source document.
pub fn delete_references_for_source(
    conn: &Connection,
    source_id: &SourceId,
) -> Result<(), DbError> {
    conn.execute(
        "DELETE FROM source_references WHERE source_id = ?1",
        params![source_id.as_str()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sil_core::{DocumentStatus, SourceDocument};

    #[test]
    fn test_references_crud_and_fts() {
        let db = crate::SilDb::open_in_memory().unwrap();
        let sid = SourceId::new("transformer.pdf");
        let mut doc = SourceDocument::new("transformer.pdf".into());
        doc.status = Some(DocumentStatus::ValidPdf);
        db.upsert_parsed(&doc, "Body text").unwrap();

        let entries = vec![
            ReferenceEntry {
                id: "transformer.pdf_ref_1".into(),
                source_id: sid.clone(),
                ref_index: 1,
                raw_text: "[1] Vaswani et al. \"Attention is all you need.\" 2017.".into(),
                title: Some("Attention is all you need.".into()),
                authors: Some("Vaswani et al.".into()),
                year: Some(2017),
                venue: Some("NeurIPS".into()),
                doi: None,
            },
            ReferenceEntry {
                id: "transformer.pdf_ref_2".into(),
                source_id: sid.clone(),
                ref_index: 2,
                raw_text: "[2] Devlin et al. \"BERT: Pre-training of Deep Bidirectional Transformers.\" 2019.".into(),
                title: Some("BERT: Pre-training of Deep Bidirectional Transformers.".into()),
                authors: Some("Devlin et al.".into()),
                year: Some(2019),
                venue: None,
                doi: None,
            },
        ];

        // Save
        db.save_source_references(&entries).unwrap();

        // Get by source_id
        let fetched = db.get_references_for_source(&sid).unwrap();
        assert_eq!(fetched.len(), 2);
        assert_eq!(fetched[0].ref_index, 1);
        assert_eq!(fetched[1].year, Some(2019));

        // FTS Search
        let hits = db.search_references("Attention", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "transformer.pdf_ref_1");

        let hits_bert = db.search_references("Transformers", 10).unwrap();
        assert_eq!(hits_bert.len(), 1);
        assert_eq!(hits_bert[0].id, "transformer.pdf_ref_2");

        // Delete
        db.delete_references_for_source(&sid).unwrap();
        let fetched_after = db.get_references_for_source(&sid).unwrap();
        assert!(fetched_after.is_empty());
    }

    #[test]
    fn test_get_all_references_and_search_edge_cases() {
        let db = crate::SilDb::open_in_memory().unwrap();
        let s1 = SourceId::new("source_a.pdf");
        let s2 = SourceId::new("source_b.pdf");

        let doc1 = SourceDocument::new("source_a.pdf".into());
        let doc2 = SourceDocument::new("source_b.pdf".into());
        db.upsert_parsed(&doc1, "Content A").unwrap();
        db.upsert_parsed(&doc2, "Content B").unwrap();

        let ref1 = ReferenceEntry {
            id: "ref_a1".into(),
            source_id: s1.clone(),
            ref_index: 1,
            raw_text: "Ref A1 text".into(),
            title: Some("Title A1".into()),
            authors: None,
            year: None,
            venue: None,
            doi: None,
        };

        let ref2 = ReferenceEntry {
            id: "ref_b1".into(),
            source_id: s2.clone(),
            ref_index: 1,
            raw_text: "Ref B1 text".into(),
            title: Some("Title B1".into()),
            authors: None,
            year: None,
            venue: None,
            doi: None,
        };

        db.save_source_references(&[ref1, ref2]).unwrap();

        let all_refs = db.get_all_references().unwrap();
        assert_eq!(all_refs.len(), 2);
        assert_eq!(all_refs[0].source_id.as_str(), "source_a.pdf");
        assert_eq!(all_refs[1].source_id.as_str(), "source_b.pdf");

        // Empty search and zero limit
        assert!(db.search_references("", 10).unwrap().is_empty());
        assert!(db.search_references("   ", 10).unwrap().is_empty());
        assert!(db.search_references("Title", 0).unwrap().is_empty());

        // Escaped quotes in search query
        let hits_quotes = db.search_references("\"Title A1\"", 10).unwrap();
        assert_eq!(hits_quotes.len(), 1);
        assert_eq!(hits_quotes[0].id, "ref_a1");

        // Non-existent source id
        let missing = db
            .get_references_for_source(&SourceId::new("missing.pdf"))
            .unwrap();
        assert!(missing.is_empty());
    }
}
