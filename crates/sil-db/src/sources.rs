//! Source document insert / get / list.

use camino::Utf8PathBuf;
use rusqlite::{Connection, params};
use sil_core::{DocumentStatus, SourceDocument, SourceId, SourceKind};

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
        INSERT INTO sources (id, path, filename, title, parsed, status, content, references_text, authors, abstract_text, doi, year, venue, kind, updated_at)
        VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, datetime('now'))
        ON CONFLICT(id) DO UPDATE SET
            path = excluded.path,
            filename = excluded.filename,
            title = excluded.title,
            parsed = 1,
            status = excluded.status,
            content = excluded.content,
            references_text = excluded.references_text,
            authors = excluded.authors,
            abstract_text = excluded.abstract_text,
            doi = excluded.doi,
            year = excluded.year,
            venue = excluded.venue,
            kind = excluded.kind,
            updated_at = datetime('now')
        "#,
        params![
            doc.id.as_str(),
            doc.path.as_str(),
            doc.filename,
            doc.title,
            doc.status.map(|s| format!("{s:?}")),
            content,
            doc.references_text,
            doc.authors,
            doc.abstract_text,
            doc.doi,
            doc.year,
            doc.venue,
            doc.kind.to_string(),
        ],
    )?;
    Ok(())
}

/// List all source documents (metadata only).
pub fn list_sources(conn: &Connection) -> Result<Vec<SourceDocument>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, path, filename, title, parsed, status, references_text, authors, abstract_text, doi, year, venue, kind FROM sources ORDER BY filename",
    )?;
    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let path: String = row.get(1)?;
        let filename: String = row.get(2)?;
        let title: Option<String> = row.get(3)?;
        let parsed: i64 = row.get(4)?;
        let status: Option<String> = row.get(5)?;
        let references_text: Option<String> = row.get(6)?;
        let authors: Option<String> = row.get(7)?;
        let abstract_text: Option<String> = row.get(8)?;
        let doi: Option<String> = row.get(9)?;
        let year: Option<i32> = row.get(10)?;
        let venue: Option<String> = row.get(11)?;
        let kind_str: Option<String> = row.get(12)?;

        let path_buf: Utf8PathBuf = path.into();
        let mut doc = SourceDocument::new(path_buf);
        doc.id = SourceId::new(id);
        doc.filename = filename;
        doc.parsed = parsed != 0;
        doc.status = status.and_then(|s| parse_status_debug(&s));
        doc.title = title;
        doc.references_text = references_text;
        doc.authors = authors;
        doc.abstract_text = abstract_text;
        doc.doi = doi;
        doc.year = year;
        doc.venue = venue;
        if let Some(ks) = kind_str
            && let Ok(k) = ks.parse::<SourceKind>()
        {
            doc.kind = k;
        }
        Ok(doc)
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

fn parse_status_debug(s: &str) -> Option<DocumentStatus> {
    if s.starts_with("Valid(") && s.ends_with(')') {
        let inner = &s[6..s.len() - 1];
        if let Ok(kind) = inner.parse::<SourceKind>() {
            return Some(DocumentStatus::Valid(kind));
        }
    }
    match s {
        "ValidPdf" => Some(DocumentStatus::ValidPdf),
        "NotFound" => Some(DocumentStatus::NotFound),
        "NotPdf" => Some(DocumentStatus::NotPdf),
        "UnsupportedFormat" => Some(DocumentStatus::UnsupportedFormat),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema;

    #[test]
    fn test_upsert_and_list_full_metadata() {
        let conn = Connection::open_in_memory().unwrap();
        schema::migrate(&conn).unwrap();

        let mut doc = SourceDocument::new("sources/notes.md".into());
        doc.title = Some("Deep Learning Advances".into());
        doc.authors = Some("Alice Smith, Bob Jones".into());
        doc.abstract_text = Some("A survey of deep learning.".into());
        doc.doi = Some("10.1234/5678".into());
        doc.year = Some(2024);
        doc.venue = Some("NeurIPS".into());
        doc.kind = SourceKind::Markdown;
        doc.status = Some(DocumentStatus::Valid(SourceKind::Markdown));
        doc.references_text = Some("1. Smith et al.".into());

        upsert_parsed(&conn, &doc, "# Deep Learning Advances\nContent body.").unwrap();

        let sources = list_sources(&conn).unwrap();
        assert_eq!(sources.len(), 1);
        let fetched = &sources[0];

        assert_eq!(fetched.filename, "notes.md");
        assert_eq!(fetched.kind, SourceKind::Markdown);
        assert_eq!(fetched.title.as_deref(), Some("Deep Learning Advances"));
        assert_eq!(fetched.authors.as_deref(), Some("Alice Smith, Bob Jones"));
        assert_eq!(fetched.abstract_text.as_deref(), Some("A survey of deep learning."));
        assert_eq!(fetched.doi.as_deref(), Some("10.1234/5678"));
        assert_eq!(fetched.year, Some(2024));
        assert_eq!(fetched.venue.as_deref(), Some("NeurIPS"));
        assert_eq!(fetched.references_text.as_deref(), Some("1. Smith et al."));
        assert!(fetched.parsed);
    }
}

