//! Schema migrations and FTS5 setup.

use rusqlite::Connection;

use crate::error::DbError;

/// Apply schema migrations (sources table + FTS5 + triggers).
pub fn migrate(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(
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

        CREATE TABLE IF NOT EXISTS todo_ideas (
            id          TEXT PRIMARY KEY NOT NULL,
            content     TEXT NOT NULL,
            section_id  TEXT,
            line_start  INTEGER NOT NULL,
            line_end    INTEGER NOT NULL,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS journal_digest (
            doi            TEXT PRIMARY KEY NOT NULL,
            title          TEXT NOT NULL,
            authors        TEXT NOT NULL,
            journal        TEXT NOT NULL,
            year           INTEGER,
            abstract_text  TEXT NOT NULL,
            citation_count INTEGER,
            url            TEXT NOT NULL,
            fetched_at     TEXT NOT NULL DEFAULT (datetime('now'))
        );
        "#,
    )?;
    Ok(())
}

