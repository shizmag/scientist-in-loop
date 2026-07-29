//! FTS5 full-text search.

use rusqlite::{Connection, params};
use sil_core::SourceId;

use crate::error::DbError;

/// One full-text search result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    /// Source id.
    pub id: SourceId,
    /// Filename.
    pub filename: String,
    /// Optional title.
    pub title: Option<String>,
    /// Highlighted snippet.
    pub snippet: String,
}

/// Full-text search over parsed sources.
pub fn search(conn: &Connection, query: &str, limit: usize) -> Result<Vec<SearchHit>, DbError> {
    let mut stmt = conn.prepare(
        r#"
        SELECT s.id, s.filename, s.title,
               snippet(sources_fts, 3, '>>>', '<<<', '…', 32) AS snip
        FROM sources_fts
        JOIN sources s ON s.id = sources_fts.id
        WHERE sources_fts MATCH ?1
        ORDER BY rank
        LIMIT ?2
        "#,
    )?;
    let rows = stmt.query_map(params![query, limit as i64], |row| {
        Ok(SearchHit {
            id: SourceId::new(row.get::<_, String>(0)?),
            filename: row.get(1)?,
            title: row.get(2)?,
            snippet: row.get(3)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}
