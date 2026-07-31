//! Schema migrations and FTS5 setup.

use rusqlite::Connection;

use crate::error::DbError;

/// Apply schema migrations (sources, source_chunks, todo_ideas, journal_digest, FTS5 + triggers).
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
            references_text TEXT,
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

        CREATE TABLE IF NOT EXISTS source_chunks (
            id              TEXT PRIMARY KEY NOT NULL,
            source_id       TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
            parent_chunk_id TEXT REFERENCES source_chunks(id) ON DELETE CASCADE,
            chunk_type      TEXT NOT NULL,
            heading_title   TEXT,
            content         TEXT NOT NULL,
            start_offset    INTEGER NOT NULL DEFAULT 0,
            end_offset      INTEGER NOT NULL DEFAULT 0,
            embedding_blob  BLOB,
            created_at      TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
            id UNINDEXED,
            source_id UNINDEXED,
            content,
            content='source_chunks',
            content_rowid='rowid'
        );

        CREATE TRIGGER IF NOT EXISTS source_chunks_ai AFTER INSERT ON source_chunks BEGIN
            INSERT INTO chunks_fts(rowid, id, source_id, content)
            VALUES (new.rowid, new.id, new.source_id, new.content);
        END;

        CREATE TRIGGER IF NOT EXISTS source_chunks_ad AFTER DELETE ON source_chunks BEGIN
            INSERT INTO chunks_fts(chunks_fts, rowid, id, source_id, content)
            VALUES ('delete', old.rowid, old.id, old.source_id, old.content);
        END;

        CREATE TRIGGER IF NOT EXISTS source_chunks_au AFTER UPDATE ON source_chunks BEGIN
            INSERT INTO chunks_fts(chunks_fts, rowid, id, source_id, content)
            VALUES ('delete', old.rowid, old.id, old.source_id, old.content);
            INSERT INTO chunks_fts(rowid, id, source_id, content)
            VALUES (new.rowid, new.id, new.source_id, new.content);
        END;

        CREATE TABLE IF NOT EXISTS todo_ideas (
            id          TEXT PRIMARY KEY NOT NULL,
            content     TEXT NOT NULL,
            section_id  TEXT,
            line_start  INTEGER NOT NULL,
            line_end    INTEGER NOT NULL,
            status      TEXT DEFAULT 'open',
            priority    TEXT DEFAULT 'medium',
            author_type TEXT DEFAULT 'human',
            tags        TEXT DEFAULT '',
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

    migrate_todo_ideas_columns(conn)?;
    migrate_sources_columns(conn)?;

    Ok(())
}

fn migrate_sources_columns(conn: &Connection) -> Result<(), DbError> {
    let mut stmt = conn.prepare("PRAGMA table_info(sources)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get(1))?
        .collect::<Result<_, _>>()?;

    if !columns.iter().any(|c| c == "references_text") {
        conn.execute(
            "ALTER TABLE sources ADD COLUMN references_text TEXT",
            [],
        )?;
    }

    Ok(())
}

fn migrate_todo_ideas_columns(conn: &Connection) -> Result<(), DbError> {
    let mut stmt = conn.prepare("PRAGMA table_info(todo_ideas)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get(1))?
        .collect::<Result<_, _>>()?;

    if !columns.iter().any(|c| c == "status") {
        conn.execute(
            "ALTER TABLE todo_ideas ADD COLUMN status TEXT DEFAULT 'open'",
            [],
        )?;
    }
    if !columns.iter().any(|c| c == "priority") {
        conn.execute(
            "ALTER TABLE todo_ideas ADD COLUMN priority TEXT DEFAULT 'medium'",
            [],
        )?;
    }
    if !columns.iter().any(|c| c == "author_type") {
        conn.execute(
            "ALTER TABLE todo_ideas ADD COLUMN author_type TEXT DEFAULT 'human'",
            [],
        )?;
    }
    if !columns.iter().any(|c| c == "tags") {
        conn.execute(
            "ALTER TABLE todo_ideas ADD COLUMN tags TEXT DEFAULT ''",
            [],
        )?;
    }

    Ok(())
}


