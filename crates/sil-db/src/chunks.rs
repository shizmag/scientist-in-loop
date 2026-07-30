//! Source chunking, parent-child hierarchy, and hybrid search (BM25 + Dense RRF).

use std::collections::HashMap;

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use sil_core::SourceId;

use crate::error::DbError;
use crate::onnx::OnnxEmbedder;

/// Type of chunk in parent-child hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkType {
    /// Parent section chunk (header section).
    Parent,
    /// Child paragraph chunk.
    Child,
}

impl ChunkType {
    /// Convert enum to database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            ChunkType::Parent => "parent",
            ChunkType::Child => "child",
        }
    }

    /// Parse enum from database string representation.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "parent" => Some(ChunkType::Parent),
            "child" => Some(ChunkType::Child),
            _ => None,
        }
    }
}

/// A parsed document chunk stored in source_chunks table.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceChunk {
    /// Unique chunk identifier.
    pub id: String,
    /// Parent source document ID.
    pub source_id: SourceId,
    /// Parent chunk ID if this is a child chunk.
    pub parent_chunk_id: Option<String>,
    /// Chunk type (parent section vs child paragraph).
    pub chunk_type: ChunkType,
    /// Heading title for section parent chunks.
    pub heading_title: Option<String>,
    /// Raw textual content.
    pub content: String,
    /// Start char/byte offset in original document.
    pub start_offset: usize,
    /// End char/byte offset in original document.
    pub end_offset: usize,
    /// Optional binary embedding vector blob.
    pub embedding_blob: Option<Vec<u8>>,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
}

/// Search hit returned by chunk hybrid / HyDE search.
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkSearchHit {
    /// Matched or expanded chunk.
    pub chunk: SourceChunk,
    /// Search relevance score (RRF or BM25).
    pub score: f32,
    /// Highlighted content snippet.
    pub snippet: String,
}

/// Calculate cosine similarity between two embedding vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for (&x, &y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    if norm_a <= 1e-8 || norm_b <= 1e-8 {
        0.0
    } else {
        dot / (norm_a.sqrt() * norm_b.sqrt())
    }
}

/// Convert float slice into byte blob for SQLite BLOB storage.
pub fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(embedding.len() * 4);
    for &val in embedding {
        blob.extend_from_slice(&val.to_le_bytes());
    }
    blob
}

/// Convert SQLite byte blob into vector of f32 floats.
pub fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap_or([0; 4])))
        .collect()
}

/// Parse Markdown text into parent section chunks and child paragraph chunks.
pub fn chunk_markdown(source_id: &SourceId, markdown: &str) -> Vec<SourceChunk> {
    let mut chunks = Vec::new();

    if markdown.trim().is_empty() {
        return chunks;
    }

    let lines: Vec<&str> = markdown.lines().collect();
    let mut parent_counter = 0;
    let mut child_counter = 0;

    let mut current_parent_id: Option<String> = None;
    let mut current_parent_title: Option<String> = None;
    let mut current_parent_content = String::new();
    let mut current_parent_start = 0;

    let mut current_para = String::new();
    let mut current_para_start = 0;

    let mut byte_offset = 0;

    let flush_child = |
        chunks: &mut Vec<SourceChunk>,
        para: &mut String,
        para_start: usize,
        para_end: usize,
        parent_id: Option<&str>,
        child_counter: &mut usize
    | {
        let content = para.trim().to_string();
        if !content.is_empty() {
            *child_counter += 1;
            let child_id = format!("{}-c{}", source_id.as_str(), child_counter);
            chunks.push(SourceChunk {
                id: child_id,
                source_id: source_id.clone(),
                parent_chunk_id: parent_id.map(|s| s.to_string()),
                chunk_type: ChunkType::Child,
                heading_title: None,
                content,
                start_offset: para_start,
                end_offset: para_end,
                embedding_blob: None,
                created_at: String::new(),
            });
        }
        para.clear();
    };

    let flush_parent = |
        chunks: &mut Vec<SourceChunk>,
        parent_id: &mut Option<String>,
        parent_title: &mut Option<String>,
        parent_content: &mut String,
        parent_start: usize,
        parent_end: usize,
    | {
        if let Some(id) = parent_id.take() {
            let content = parent_content.trim().to_string();
            if !content.is_empty() {
                chunks.push(SourceChunk {
                    id,
                    source_id: source_id.clone(),
                    parent_chunk_id: None,
                    chunk_type: ChunkType::Parent,
                    heading_title: parent_title.take(),
                    content,
                    start_offset: parent_start,
                    end_offset: parent_end,
                    embedding_blob: None,
                    created_at: String::new(),
                });
            }
        }
        parent_content.clear();
    };

    for line in &lines {
        let line_len = line.len() + 1;
        let line_start = byte_offset;
        let line_end = byte_offset + line.len();
        let trimmed = line.trim();

        let is_header = trimmed.starts_with('#');
        let header_text = if is_header {
            let h = trimmed.trim_start_matches('#').trim();
            if !h.is_empty() { Some(h.to_string()) } else { None }
        } else {
            None
        };

        if is_header && header_text.is_some() {
            flush_child(&mut chunks, &mut current_para, current_para_start, line_start, current_parent_id.as_deref(), &mut child_counter);
            flush_parent(&mut chunks, &mut current_parent_id, &mut current_parent_title, &mut current_parent_content, current_parent_start, line_start);

            parent_counter += 1;
            let new_parent_id = format!("{}-p{}", source_id.as_str(), parent_counter);
            current_parent_id = Some(new_parent_id);
            current_parent_title = header_text;
            current_parent_content = format!("{}\n", trimmed);
            current_parent_start = line_start;
        } else if trimmed.is_empty() {
            flush_child(&mut chunks, &mut current_para, current_para_start, line_end, current_parent_id.as_deref(), &mut child_counter);
            if !current_parent_content.is_empty() {
                current_parent_content.push('\n');
            }
        } else {
            if current_parent_id.is_none() {
                parent_counter += 1;
                current_parent_id = Some(format!("{}-p{}", source_id.as_str(), parent_counter));
                current_parent_title = None;
                current_parent_start = line_start;
            }

            if current_para.is_empty() {
                current_para_start = line_start;
            } else {
                current_para.push('\n');
            }
            current_para.push_str(trimmed);

            current_parent_content.push_str(trimmed);
            current_parent_content.push('\n');
        }

        byte_offset += line_len;
    }

    flush_child(&mut chunks, &mut current_para, current_para_start, byte_offset, current_parent_id.as_deref(), &mut child_counter);
    flush_parent(&mut chunks, &mut current_parent_id, &mut current_parent_title, &mut current_parent_content, current_parent_start, byte_offset);

    chunks
}

/// Insert list of source chunks into source_chunks table (inserting parent chunks first).
pub fn insert_chunks(conn: &Connection, chunks: &[SourceChunk]) -> Result<(), DbError> {
    let tx = conn.unchecked_transaction()?;

    let mut stmt = tx.prepare(
        "INSERT INTO source_chunks (id, source_id, parent_chunk_id, chunk_type, heading_title, content, start_offset, end_offset, embedding_blob)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
    )?;

    // 1. Insert parent chunks
    for chunk in chunks.iter().filter(|c| c.chunk_type == ChunkType::Parent) {
        stmt.execute(params![
            chunk.id,
            chunk.source_id.as_str(),
            chunk.parent_chunk_id,
            chunk.chunk_type.as_str(),
            chunk.heading_title,
            chunk.content,
            chunk.start_offset as i64,
            chunk.end_offset as i64,
            chunk.embedding_blob,
        ])?;
    }

    // 2. Insert child chunks
    for chunk in chunks.iter().filter(|c| c.chunk_type == ChunkType::Child) {
        stmt.execute(params![
            chunk.id,
            chunk.source_id.as_str(),
            chunk.parent_chunk_id,
            chunk.chunk_type.as_str(),
            chunk.heading_title,
            chunk.content,
            chunk.start_offset as i64,
            chunk.end_offset as i64,
            chunk.embedding_blob,
        ])?;
    }

    drop(stmt);
    tx.commit()?;
    Ok(())
}

/// Get all chunks belonging to a specific source ID.
pub fn get_chunks_for_source(conn: &Connection, source_id: &SourceId) -> Result<Vec<SourceChunk>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, source_id, parent_chunk_id, chunk_type, heading_title, content, start_offset, end_offset, embedding_blob, created_at
         FROM source_chunks
         WHERE source_id = ?1
         ORDER BY start_offset ASC"
    )?;

    let rows = stmt.query_map(params![source_id.as_str()], |row| {
        let chunk_type_str: String = row.get(3)?;
        let chunk_type = ChunkType::from_str(&chunk_type_str)
            .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(DbError::Message("invalid chunk type".into()))))?;

        Ok(SourceChunk {
            id: row.get(0)?,
            source_id: SourceId::new(row.get::<_, String>(1)?),
            parent_chunk_id: row.get(2)?,
            chunk_type,
            heading_title: row.get(4)?,
            content: row.get(5)?,
            start_offset: row.get::<_, i64>(6)? as usize,
            end_offset: row.get::<_, i64>(7)? as usize,
            embedding_blob: row.get(8)?,
            created_at: row.get(9)?,
        })
    })?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Get a specific chunk by chunk ID.
pub fn get_chunk_by_id(conn: &Connection, chunk_id: &str) -> Result<Option<SourceChunk>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, source_id, parent_chunk_id, chunk_type, heading_title, content, start_offset, end_offset, embedding_blob, created_at
         FROM source_chunks
         WHERE id = ?1"
    )?;

    let mut rows = stmt.query_map(params![chunk_id], |row| {
        let chunk_type_str: String = row.get(3)?;
        let chunk_type = ChunkType::from_str(&chunk_type_str)
            .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(DbError::Message("invalid chunk type".into()))))?;

        Ok(SourceChunk {
            id: row.get(0)?,
            source_id: SourceId::new(row.get::<_, String>(1)?),
            parent_chunk_id: row.get(2)?,
            chunk_type,
            heading_title: row.get(4)?,
            content: row.get(5)?,
            start_offset: row.get::<_, i64>(6)? as usize,
            end_offset: row.get::<_, i64>(7)? as usize,
            embedding_blob: row.get(8)?,
            created_at: row.get(9)?,
        })
    })?;

    if let Some(res) = rows.next() {
        Ok(Some(res?))
    } else {
        Ok(None)
    }
}

/// Delete all chunks for a source.
pub fn delete_chunks_for_source(conn: &Connection, source_id: &SourceId) -> Result<(), DbError> {
    conn.execute("DELETE FROM source_chunks WHERE source_id = ?1", params![source_id.as_str()])?;
    Ok(())
}

/// Expand a child chunk's parent chunk ID to return the full parent section chunk.
pub fn expand_parent(conn: &Connection, parent_id: &str) -> Result<Option<SourceChunk>, DbError> {
    get_chunk_by_id(conn, parent_id)
}

/// Perform Hybrid BM25 FTS5 + Dense ONNX Reciprocal Rank Fusion (RRF) search.
///
/// RRF score formula: RRF_score(d) = 1 / (60 + r_bm25(d)) + 1 / (60 + r_dense(d))
pub fn search_hybrid(
    conn: &Connection,
    embedder: &OnnxEmbedder,
    query: &str,
    limit: usize,
    expand_to_parent: bool,
) -> Result<Vec<ChunkSearchHit>, DbError> {
    search_hybrid_dual(conn, embedder, query, query, limit, expand_to_parent)
}

/// Hybrid search with distinct queries for BM25 keyword search and dense embedding search.
pub fn search_hybrid_dual(
    conn: &Connection,
    embedder: &OnnxEmbedder,
    bm25_query: &str,
    dense_text: &str,
    limit: usize,
    expand_to_parent: bool,
) -> Result<Vec<ChunkSearchHit>, DbError> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    // 1. BM25 Search via chunks_fts
    let mut bm25_ranks: HashMap<String, (usize, SourceChunk, String)> = HashMap::new();
    let sanitized_query = bm25_query
        .chars()
        .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect::<String>();
    let clean_q = sanitized_query.split_whitespace().collect::<Vec<_>>().join(" ");

    if !clean_q.is_empty() {
        let mut stmt = conn.prepare(
            r#"
            SELECT c.id, c.source_id, c.parent_chunk_id, c.chunk_type, c.heading_title,
                   c.content, c.start_offset, c.end_offset, c.embedding_blob, c.created_at,
                   snippet(chunks_fts, 2, '>>>', '<<<', '…', 32) AS snip
            FROM chunks_fts
            JOIN source_chunks c ON c.id = chunks_fts.id
            WHERE chunks_fts MATCH ?1
            ORDER BY rank
            LIMIT ?2
            "#
        )?;

        let fetch_limit = (limit * 3).max(50) as i64;
        if let Ok(rows) = stmt.query_map(params![clean_q, fetch_limit], |row| {
            let chunk_type_str: String = row.get(3)?;
            let chunk_type = ChunkType::from_str(&chunk_type_str).unwrap_or(ChunkType::Child);
            let chunk = SourceChunk {
                id: row.get(0)?,
                source_id: SourceId::new(row.get::<_, String>(1)?),
                parent_chunk_id: row.get(2)?,
                chunk_type,
                heading_title: row.get(4)?,
                content: row.get(5)?,
                start_offset: row.get::<_, i64>(6)? as usize,
                end_offset: row.get::<_, i64>(7)? as usize,
                embedding_blob: row.get(8)?,
                created_at: row.get(9)?,
            };
            let snippet: String = row.get(10)?;
            Ok((chunk, snippet))
        }) {
            for (rank, (chunk, snip)) in (1..).zip(rows.flatten()) {
                bm25_ranks.insert(chunk.id.clone(), (rank, chunk, snip));
            }
        }
    }

    // 2. Dense ONNX Search
    let mut dense_ranks: HashMap<String, (usize, SourceChunk)> = HashMap::new();
    if !dense_text.trim().is_empty()
        && let Ok(q_emb) = embedder.embed(dense_text)
    {
        let mut dense_candidates: Vec<(String, f32, SourceChunk)> = Vec::new();

            let mut stmt = conn.prepare(
                "SELECT id, source_id, parent_chunk_id, chunk_type, heading_title, content, start_offset, end_offset, embedding_blob, created_at
                 FROM source_chunks
                 WHERE embedding_blob IS NOT NULL"
            )?;

            if let Ok(rows) = stmt.query_map([], |row| {
                let chunk_type_str: String = row.get(3)?;
                let chunk_type = ChunkType::from_str(&chunk_type_str).unwrap_or(ChunkType::Child);
                Ok(SourceChunk {
                    id: row.get(0)?,
                    source_id: SourceId::new(row.get::<_, String>(1)?),
                    parent_chunk_id: row.get(2)?,
                    chunk_type,
                    heading_title: row.get(4)?,
                    content: row.get(5)?,
                    start_offset: row.get::<_, i64>(6)? as usize,
                    end_offset: row.get::<_, i64>(7)? as usize,
                    embedding_blob: row.get(8)?,
                    created_at: row.get(9)?,
                })
            }) {
                for chunk in rows.flatten() {
                    if let Some(ref blob) = chunk.embedding_blob {
                        let chunk_emb = blob_to_embedding(blob);
                        let sim = cosine_similarity(&q_emb, &chunk_emb);
                        dense_candidates.push((chunk.id.clone(), sim, chunk));
                    }
                }
            }

            dense_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            for (rank_0, (id, _sim, chunk)) in dense_candidates.into_iter().enumerate() {
                dense_ranks.insert(id, (rank_0 + 1, chunk));
            }
        }

    // 3. Reciprocal Rank Fusion (RRF)
    let k = 60.0f32;
    let mut candidate_ids: Vec<String> = bm25_ranks.keys().cloned().collect();
    for id in dense_ranks.keys() {
        if !candidate_ids.contains(id) {
            candidate_ids.push(id.clone());
        }
    }

    let mut scored_chunks: Vec<(SourceChunk, f32, String)> = Vec::new();

    for id in candidate_ids {
        let r_bm25 = bm25_ranks.get(&id).map(|(r, _, _)| *r);
        let r_dense = dense_ranks.get(&id).map(|(r, _)| *r);

        let rrf_bm25 = r_bm25.map_or(0.0, |r| 1.0 / (k + r as f32));
        let rrf_dense = r_dense.map_or(0.0, |r| 1.0 / (k + r as f32));
        let rrf_score = rrf_bm25 + rrf_dense;

        let (chunk, snippet) = if let Some((_, c, snip)) = bm25_ranks.get(&id) {
            (c.clone(), snip.clone())
        } else if let Some((_, c)) = dense_ranks.get(&id) {
            (c.clone(), c.content.chars().take(100).collect())
        } else {
            continue;
        };

        scored_chunks.push((chunk, rrf_score, snippet));
    }

    // 4. Parent Expansion if requested
    let mut final_hits_map: HashMap<String, ChunkSearchHit> = HashMap::new();

    for (chunk, score, snippet) in scored_chunks {
        let (target_chunk, target_snippet) = if expand_to_parent && chunk.chunk_type == ChunkType::Child {
            if let Some(ref p_id) = chunk.parent_chunk_id {
                if let Ok(Some(parent_chunk)) = expand_parent(conn, p_id) {
                    let p_snip = format!("Section: {}", parent_chunk.heading_title.as_deref().unwrap_or(""));
                    (parent_chunk, p_snip)
                } else {
                    (chunk, snippet)
                }
            } else {
                (chunk, snippet)
            }
        } else {
            (chunk, snippet)
        };

        let entry = final_hits_map.entry(target_chunk.id.clone()).or_insert_with(|| ChunkSearchHit {
            chunk: target_chunk.clone(),
            score,
            snippet: target_snippet,
        });

        if score > entry.score {
            entry.score = score;
        }
    }

    let mut hits: Vec<ChunkSearchHit> = final_hits_map.into_values().collect();
    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    hits.truncate(limit);

    Ok(hits)
}
