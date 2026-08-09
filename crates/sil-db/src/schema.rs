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
            authors     TEXT,
            abstract_text TEXT,
            doi         TEXT,
            year        INTEGER,
            venue       TEXT,
            kind        TEXT,
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

        CREATE TABLE IF NOT EXISTS source_references (
            id          TEXT PRIMARY KEY NOT NULL,
            source_id   TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
            ref_index   INTEGER NOT NULL,
            raw_text    TEXT NOT NULL,
            title       TEXT,
            authors     TEXT,
            year        INTEGER,
            venue       TEXT,
            doi         TEXT,
            arxiv_id    TEXT,
            url         TEXT,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS source_references_fts USING fts5(
            id UNINDEXED,
            source_id UNINDEXED,
            raw_text,
            title,
            authors,
            content='source_references',
            content_rowid='rowid'
        );

        CREATE TRIGGER IF NOT EXISTS source_references_ai AFTER INSERT ON source_references BEGIN
            INSERT INTO source_references_fts(rowid, id, source_id, raw_text, title, authors)
            VALUES (new.rowid, new.id, new.source_id, new.raw_text, new.title, new.authors);
        END;

        CREATE TRIGGER IF NOT EXISTS source_references_ad AFTER DELETE ON source_references BEGIN
            INSERT INTO source_references_fts(source_references_fts, rowid, id, source_id, raw_text, title, authors)
            VALUES ('delete', old.rowid, old.id, old.source_id, old.raw_text, old.title, old.authors);
        END;

        CREATE TRIGGER IF NOT EXISTS source_references_au AFTER UPDATE ON source_references BEGIN
            INSERT INTO source_references_fts(source_references_fts, rowid, id, source_id, raw_text, title, authors)
            VALUES ('delete', old.rowid, old.id, old.source_id, old.raw_text, old.title, old.authors);
            INSERT INTO source_references_fts(rowid, id, source_id, raw_text, title, authors)
            VALUES (new.rowid, new.id, new.source_id, new.raw_text, new.title, new.authors);
        END;

        CREATE TABLE IF NOT EXISTS draft_ref_similarity (
            ref_id      TEXT PRIMARY KEY NOT NULL REFERENCES source_references(id) ON DELETE CASCADE,
            score       REAL NOT NULL,
            draft_hash  TEXT NOT NULL,
            model_dim   INTEGER NOT NULL,
            updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS embedding_cache (
            content_hash TEXT PRIMARY KEY NOT NULL,
            model_name   TEXT NOT NULL,
            dimension    INTEGER NOT NULL,
            embedding    BLOB NOT NULL,
            created_at   TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS bib_references (
            cite_key   TEXT PRIMARY KEY NOT NULL,
            doi        TEXT,
            doi_exists INTEGER,
            raw_bibtex TEXT NOT NULL,
            checked_at TEXT,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS doi_verifications (
            doi         TEXT PRIMARY KEY NOT NULL,
            exists_flag INTEGER NOT NULL,
            error_cat   TEXT,
            checked_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );
        "#,
    )?;

    migrate_todo_ideas_columns(conn)?;
    migrate_sources_columns(conn)?;
    migrate_source_references_columns(conn)?;

    Ok(())
}

fn migrate_sources_columns(conn: &Connection) -> Result<(), DbError> {
    let mut stmt = conn.prepare("PRAGMA table_info(sources)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get(1))?
        .collect::<Result<_, _>>()?;

    let new_cols = [
        ("references_text", "TEXT"),
        ("authors", "TEXT"),
        ("abstract_text", "TEXT"),
        ("doi", "TEXT"),
        ("year", "INTEGER"),
        ("venue", "TEXT"),
        ("kind", "TEXT"),
    ];

    for (col_name, col_type) in new_cols {
        if !columns.iter().any(|c| c == col_name) {
            conn.execute(
                &format!("ALTER TABLE sources ADD COLUMN {col_name} {col_type}"),
                [],
            )?;
        }
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
        conn.execute("ALTER TABLE todo_ideas ADD COLUMN tags TEXT DEFAULT ''", [])?;
    }

    Ok(())
}

fn migrate_source_references_columns(conn: &Connection) -> Result<(), DbError> {
    let mut stmt = conn.prepare("PRAGMA table_info(source_references)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get(1))?
        .collect::<Result<_, _>>()?;

    if !columns.iter().any(|c| c == "venue") {
        conn.execute("ALTER TABLE source_references ADD COLUMN venue TEXT", [])?;
    }
    if !columns.iter().any(|c| c == "arxiv_id") {
        conn.execute("ALTER TABLE source_references ADD COLUMN arxiv_id TEXT", [])?;
    }
    if !columns.iter().any(|c| c == "url") {
        conn.execute("ALTER TABLE source_references ADD COLUMN url TEXT", [])?;
    }

    Ok(())
}
