//! Embedding vector cache in SQLite database keyed by content hash, model name, and dimension.

use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use rusqlite::{Connection, params};

use crate::chunks::{blob_to_embedding, embedding_to_blob};
use crate::error::DbError;

/// Calculate hex content hash of text content.
pub fn compute_content_hash(text: &str) -> String {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Retrieve cached embedding for text content hash + model name.
pub fn get_cached_embedding(
    conn: &Connection,
    content_hash: &str,
    model_name: &str,
) -> Result<Option<Vec<f32>>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT embedding FROM embedding_cache WHERE content_hash = ?1 AND model_name = ?2",
    )?;
    let mut rows = stmt.query(params![content_hash, model_name])?;

    if let Some(row) = rows.next()? {
        let blob: Vec<u8> = row.get(0)?;
        let vec = blob_to_embedding(&blob);
        Ok(Some(vec))
    } else {
        Ok(None)
    }
}

/// Put calculated embedding in SQLite vector cache.
pub fn put_cached_embedding(
    conn: &Connection,
    content_hash: &str,
    model_name: &str,
    dimension: usize,
    embedding: &[f32],
) -> Result<(), DbError> {
    let blob = embedding_to_blob(embedding);
    conn.execute(
        "INSERT OR REPLACE INTO embedding_cache (content_hash, model_name, dimension, embedding) VALUES (?1, ?2, ?3, ?4)",
        params![content_hash, model_name, dimension as i64, blob],
    )?;
    Ok(())
}

/// Clear embedding cache table. Returns number of rows deleted.
pub fn clear_embedding_cache(conn: &Connection) -> Result<usize, DbError> {
    let count = conn.execute("DELETE FROM embedding_cache", [])?;
    Ok(count)
}

/// Get embedding cache statistics (total entries).
pub fn embedding_cache_stats(conn: &Connection) -> Result<usize, DbError> {
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM embedding_cache")?;
    let count: i64 = stmt.query_row([], |row| row.get(0))?;
    Ok(count as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_cache_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let conn = Connection::open_in_memory()?;
        crate::schema::migrate(&conn)?;

        let hash = compute_content_hash("hello world");
        let vec = vec![0.1f32, 0.2, 0.3];
        put_cached_embedding(&conn, &hash, "test-model", 3, &vec)?;

        let fetched = get_cached_embedding(&conn, &hash, "test-model")?;
        assert_eq!(fetched, Some(vec));

        assert_eq!(embedding_cache_stats(&conn)?, 1);
        assert_eq!(clear_embedding_cache(&conn)?, 1);
        assert_eq!(embedding_cache_stats(&conn)?, 0);

        Ok(())
    }
}
