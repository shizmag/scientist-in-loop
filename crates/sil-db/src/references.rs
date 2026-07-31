//! Source reference item persistence and full-text search.

use rusqlite::{Connection, params};
use sil_core::{ReferenceEntry, SourceId};

use crate::error::DbError;

/// Save a batch of reference entries for a source document.
pub fn save_source_references(conn: &Connection, refs: &[ReferenceEntry]) -> Result<(), DbError> {
    let mut stmt = conn.prepare(
        "INSERT OR REPLACE INTO source_references (id, source_id, ref_index, raw_text, title, authors, year, doi)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )?;

    for entry in refs {
        stmt.execute(params![
            entry.id,
            entry.source_id.as_str(),
            entry.ref_index,
            entry.raw_text,
            entry.title,
            entry.authors,
            entry.year,
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
        "SELECT id, source_id, ref_index, raw_text, title, authors, year, doi
         FROM source_references
         WHERE source_id = ?1
         ORDER BY ref_index ASC",
    )?;

    let rows = stmt.query_map(params![source_id.as_str()], |row| {
        let sid: String = row.get(1)?;
        Ok(ReferenceEntry {
            id: row.get(0)?,
            source_id: SourceId::new(sid),
            ref_index: row.get(2)?,
            raw_text: row.get(3)?,
            title: row.get(4)?,
            authors: row.get(5)?,
            year: row.get(6)?,
            doi: row.get(7)?,
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
    let fts_query = clean.replace('"', "").replace('\'', "");
    let mut stmt = conn.prepare(
        "SELECT r.id, r.source_id, r.ref_index, r.raw_text, r.title, r.authors, r.year, r.doi
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
            ref_index: row.get(2)?,
            raw_text: row.get(3)?,
            title: row.get(4)?,
            authors: row.get(5)?,
            year: row.get(6)?,
            doi: row.get(7)?,
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
