//! SQLite + FTS5 storage for parsed scientific sources.
//!
//! Stage 0: crate skeleton. Stage 3: full schema and query API.

#![deny(missing_docs)]

use camino::Utf8Path;
use rusqlite::{Connection, params};
use sil_core::{DocumentStatus, SilError, SourceDocument, SourceId};
use thiserror::Error;

/// Database-specific errors.
#[derive(Debug, Error)]
pub enum DbError {
    /// SQLite error.
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Domain message.
    #[error("{0}")]
    Message(String),
}

impl From<DbError> for SilError {
    fn from(value: DbError) -> Self {
        SilError::Database(value.to_string())
    }
}

/// Open or create the project database and ensure schema.
pub struct SilDb {
    conn: Connection,
}

impl SilDb {
    /// Open database at `path`, creating parent dirs and schema as needed.
    pub fn open(path: &Utf8Path) -> Result<Self, DbError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DbError::Message(format!("failed to create db directory {parent}: {e}"))
            })?;
        }
        let conn = Connection::open(path.as_str())?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Open an in-memory database (tests).
    pub fn open_in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Apply schema migrations.
    pub fn migrate(&self) -> Result<(), DbError> {
        self.conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS sources (
                id          TEXT PRIMARY KEY NOT NULL,
                path        TEXT NOT NULL,
                filename    TEXT NOT NULL,
                title       TEXT,
                parsed      INTEGER NOT NULL DEFAULT 0,
                status      TEXT,
                content     TEXT,
                created_at  TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS sources_fts USING fts5(
                id UNINDEXED,
                filename,
                title,
                content,
                content='sources',
                content_rowid='rowid'
            );

            CREATE TRIGGER IF NOT EXISTS sources_ai AFTER INSERT ON sources BEGIN
                INSERT INTO sources_fts(rowid, id, filename, title, content)
                VALUES (new.rowid, new.id, new.filename, new.title, new.content);
            END;

            CREATE TRIGGER IF NOT EXISTS sources_ad AFTER DELETE ON sources BEGIN
                INSERT INTO sources_fts(sources_fts, rowid, id, filename, title, content)
                VALUES ('delete', old.rowid, old.id, old.filename, old.title, old.content);
            END;

            CREATE TRIGGER IF NOT EXISTS sources_au AFTER UPDATE ON sources BEGIN
                INSERT INTO sources_fts(sources_fts, rowid, id, filename, title, content)
                VALUES ('delete', old.rowid, old.id, old.filename, old.title, old.content);
                INSERT INTO sources_fts(rowid, id, filename, title, content)
                VALUES (new.rowid, new.id, new.filename, new.title, new.content);
            END;
            "#,
        )?;
        Ok(())
    }

    /// Number of sources in the database.
    pub fn source_count(&self) -> Result<usize, DbError> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM sources", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    /// Number of parsed sources.
    pub fn parsed_count(&self) -> Result<usize, DbError> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sources WHERE parsed = 1",
            [],
            |r| r.get(0),
        )?;
        Ok(n as usize)
    }

    /// Whether a source id already exists and is parsed.
    pub fn is_parsed(&self, id: &SourceId) -> Result<bool, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT parsed FROM sources WHERE id = ?1")?;
        let result = stmt.query_row(params![id.as_str()], |r| r.get::<_, i64>(0));
        match result {
            Ok(v) => Ok(v != 0),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert or replace a parsed source document with full text content.
    pub fn upsert_parsed(
        &self,
        doc: &SourceDocument,
        content: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            r#"
            INSERT INTO sources (id, path, filename, title, parsed, status, content, updated_at)
            VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, datetime('now'))
            ON CONFLICT(id) DO UPDATE SET
                path = excluded.path,
                filename = excluded.filename,
                title = excluded.title,
                parsed = 1,
                status = excluded.status,
                content = excluded.content,
                updated_at = datetime('now')
            "#,
            params![
                doc.id.as_str(),
                doc.path.as_str(),
                doc.filename,
                doc.title,
                doc.status.map(|s| format!("{s:?}")),
                content,
            ],
        )?;
        Ok(())
    }

    /// List all source documents (metadata only).
    pub fn list_sources(&self) -> Result<Vec<SourceDocument>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, filename, title, parsed, status FROM sources ORDER BY filename",
        )?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let path: String = row.get(1)?;
            let filename: String = row.get(2)?;
            let title: Option<String> = row.get(3)?;
            let parsed: i64 = row.get(4)?;
            let status: Option<String> = row.get(5)?;
            Ok(SourceDocument {
                id: SourceId::new(id),
                path: path.into(),
                filename,
                parsed: parsed != 0,
                status: status.and_then(|s| parse_status_debug(&s)),
                title,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Full-text search over parsed sources.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, DbError> {
        let mut stmt = self.conn.prepare(
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
}

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

fn parse_status_debug(s: &str) -> Option<DocumentStatus> {
    match s {
        "ValidPdf" => Some(DocumentStatus::ValidPdf),
        "NotFound" => Some(DocumentStatus::NotFound),
        "NotPdf" => Some(DocumentStatus::NotPdf),
        "AlreadyParsed" => Some(DocumentStatus::AlreadyParsed),
        "Corrupted" => Some(DocumentStatus::Corrupted),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sil_core::SourceDocument;

    #[test]
    fn migrate_and_count() {
        let db = SilDb::open_in_memory().unwrap();
        assert_eq!(db.source_count().unwrap(), 0);
    }

    #[test]
    fn upsert_and_search() {
        let db = SilDb::open_in_memory().unwrap();
        let mut doc = SourceDocument::new("sources/attention.pdf".into());
        doc.title = Some("Attention Is All You Need".into());
        doc.status = Some(DocumentStatus::ValidPdf);
        db.upsert_parsed(
            &doc,
            "The transformer architecture uses multi-head self-attention.",
        )
        .unwrap();
        assert_eq!(db.source_count().unwrap(), 1);
        assert!(db.is_parsed(&doc.id).unwrap());
        let hits = db.search("transformer", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].snippet.to_lowercase().contains("transformer") || !hits[0].snippet.is_empty());
    }

    #[test]
    fn search_no_match() {
        let db = SilDb::open_in_memory().unwrap();
        let hits = db.search("nonexistenttokenxyz", 10).unwrap();
        assert!(hits.is_empty());
    }
}
