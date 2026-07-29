//! Source document insert / get / list.

use rusqlite::{Connection, params};
use sil_core::{DocumentStatus, SourceDocument, SourceId};

use crate::error::DbError;

/// Number of sources in the database.
pub fn source_count(conn: &Connection) -> Result<usize, DbError> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM sources", [], |r| r.get(0))?;
    Ok(n as usize)
}

/// Number of parsed sources.
pub fn parsed_count(conn: &Connection) -> Result<usize, DbError> {
    let n: i64 =
        conn.query_row("SELECT COUNT(*) FROM sources WHERE parsed = 1", [], |r| {
            r.get(0)
        })?;
    Ok(n as usize)
}

/// Whether a source id already exists and is parsed.
pub fn is_parsed(conn: &Connection, id: &SourceId) -> Result<bool, DbError> {
    let mut stmt = conn.prepare("SELECT parsed FROM sources WHERE id = ?1")?;
    let result = stmt.query_row(params![id.as_str()], |r| r.get::<_, i64>(0));
    match result {
        Ok(v) => Ok(v != 0),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Insert or replace a parsed source document with full text content.
pub fn upsert_parsed(
    conn: &Connection,
    doc: &SourceDocument,
    content: &str,
) -> Result<(), DbError> {
    conn.execute(
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
pub fn list_sources(conn: &Connection) -> Result<Vec<SourceDocument>, DbError> {
    let mut stmt = conn.prepare(
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

/// Remove a source by id from the sources table (FTS updated via delete trigger).
pub fn remove_source(conn: &Connection, id: &SourceId) -> Result<bool, DbError> {
    let n = conn.execute("DELETE FROM sources WHERE id = ?1", params![id.as_str()])?;
    Ok(n > 0)
}
