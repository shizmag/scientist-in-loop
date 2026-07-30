//! FTS5 full-text search and HyDE search helper.

use rusqlite::{Connection, params};
use sil_core::SourceId;

use crate::chunks::{ChunkSearchHit, search_hybrid_dual};
use crate::error::DbError;
use crate::onnx::OnnxEmbedder;

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

/// Hypothetical Document Expansion (HyDE) search helper.
///
/// Accepts a hypothetical document/passage (embedded via `OnnxEmbedder` for dense search)
/// and a keyword query (used for BM25 FTS5 search), fusing results using Reciprocal Rank Fusion (RRF).
pub fn search_hyde(
    conn: &Connection,
    embedder: &OnnxEmbedder,
    hypothetical_passage: &str,
    keyword_query: &str,
    limit: usize,
    expand_to_parent: bool,
) -> Result<Vec<ChunkSearchHit>, DbError> {
    search_hybrid_dual(
        conn,
        embedder,
        keyword_query,
        hypothetical_passage,
        limit,
        expand_to_parent,
    )
}
