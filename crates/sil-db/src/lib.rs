//! SQLite + FTS5 storage for parsed scientific sources.

#![deny(missing_docs)]

mod error;
mod schema;
mod search;
mod sources;

pub use error::DbError;
pub use search::SearchHit;

use camino::Utf8Path;
use rusqlite::Connection;
use sil_core::{SourceDocument, SourceId};

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
        schema::migrate(&self.conn)
    }

    /// Number of sources in the database.
    pub fn source_count(&self) -> Result<usize, DbError> {
        sources::source_count(&self.conn)
    }

    /// Number of parsed sources.
    pub fn parsed_count(&self) -> Result<usize, DbError> {
        sources::parsed_count(&self.conn)
    }

    /// Whether a source id already exists and is parsed.
    pub fn is_parsed(&self, id: &SourceId) -> Result<bool, DbError> {
        sources::is_parsed(&self.conn, id)
    }

    /// Insert or replace a parsed source document with full text content.
    pub fn upsert_parsed(&self, doc: &SourceDocument, content: &str) -> Result<(), DbError> {
        sources::upsert_parsed(&self.conn, doc, content)
    }

    /// List all source documents (metadata only).
    pub fn list_sources(&self) -> Result<Vec<SourceDocument>, DbError> {
        sources::list_sources(&self.conn)
    }

    /// Full-text search over parsed sources.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, DbError> {
        search::search(&self.conn, query, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sil_core::{DocumentStatus, SourceDocument, SourceId};

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
        assert!(
            hits[0].snippet.to_lowercase().contains("transformer")
                || !hits[0].snippet.is_empty()
        );
    }

    #[test]
    fn search_no_match() {
        let db = SilDb::open_in_memory().unwrap();
        let hits = db.search("nonexistenttokenxyz", 10).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn list_sources_and_parsed_count() {
        let db = SilDb::open_in_memory().unwrap();
        assert_eq!(db.parsed_count().unwrap(), 0);
        let mut a = SourceDocument::new("a.pdf".into());
        a.status = Some(DocumentStatus::ValidPdf);
        let mut b = SourceDocument::new("b.pdf".into());
        b.status = Some(DocumentStatus::ValidPdf);
        db.upsert_parsed(&a, "alpha content about graphs").unwrap();
        db.upsert_parsed(&b, "beta content about trees").unwrap();
        assert_eq!(db.source_count().unwrap(), 2);
        assert_eq!(db.parsed_count().unwrap(), 2);
        let list = db.list_sources().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|d| d.parsed));
        // filenames sorted
        assert_eq!(list[0].filename, "a.pdf");
        assert_eq!(list[1].filename, "b.pdf");
    }

    #[test]
    fn upsert_replaces_content_and_fts() {
        let db = SilDb::open_in_memory().unwrap();
        let mut doc = SourceDocument::new("paper.pdf".into());
        doc.title = Some("V1".into());
        db.upsert_parsed(&doc, "original unique_token_aaa").unwrap();
        assert_eq!(db.search("unique_token_aaa", 5).unwrap().len(), 1);
        doc.title = Some("V2".into());
        db.upsert_parsed(&doc, "updated unique_token_bbb").unwrap();
        assert_eq!(db.source_count().unwrap(), 1);
        assert!(db.search("unique_token_aaa", 5).unwrap().is_empty());
        let hits = db.search("unique_token_bbb", 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title.as_deref(), Some("V2"));
    }

    #[test]
    fn is_parsed_false_for_unknown() {
        let db = SilDb::open_in_memory().unwrap();
        assert!(!db.is_parsed(&SourceId::new("missing.pdf")).unwrap());
    }

    #[test]
    fn open_on_disk_persists() {
        let dir = tempfile::tempdir().unwrap();
        let path = camino::Utf8PathBuf::from_path_buf(dir.path().join("db.sqlite")).unwrap();
        {
            let db = SilDb::open(&path).unwrap();
            let mut doc = SourceDocument::new("x.pdf".into());
            doc.status = Some(DocumentStatus::ValidPdf);
            db.upsert_parsed(&doc, "persisted content token_persist").unwrap();
        }
        let db2 = SilDb::open(&path).unwrap();
        assert_eq!(db2.source_count().unwrap(), 1);
        assert!(!db2.search("token_persist", 5).unwrap().is_empty());
    }

    #[test]
    fn search_respects_limit() {
        let db = SilDb::open_in_memory().unwrap();
        for i in 0..5 {
            let mut doc = SourceDocument::new(format!("p{i}.pdf").into());
            doc.status = Some(DocumentStatus::ValidPdf);
            db.upsert_parsed(&doc, "sharedkeyword document body").unwrap();
        }
        let hits = db.search("sharedkeyword", 2).unwrap();
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn search_empty_index() {
        let db = SilDb::open_in_memory().unwrap();
        assert!(db.search("anything", 10).unwrap().is_empty());
    }

    #[test]
    fn search_unicode_content_stored_and_ascii_queryable() {
        // Store mixed unicode; query with an ASCII token the default FTS5
        // tokenizer reliably indexes (CJK segmentation is not guaranteed).
        let db = SilDb::open_in_memory().unwrap();
        let mut doc = SourceDocument::new("zh.pdf".into());
        doc.status = Some(DocumentStatus::ValidPdf);
        db.upsert_parsed(
            &doc,
            "注意力机制 uniquetokenxyz 自注意力 café αβγ",
        )
        .unwrap();
        let hits = db.search("uniquetokenxyz", 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].filename, "zh.pdf");
        // Must not panic on unicode query even if zero hits
        let _ = db.search("注意力", 5);
    }

    #[test]
    fn search_short_query() {
        let db = SilDb::open_in_memory().unwrap();
        let mut doc = SourceDocument::new("s.pdf".into());
        doc.status = Some(DocumentStatus::ValidPdf);
        db.upsert_parsed(&doc, "a short token xy").unwrap();
        // single-character may or may not match depending on tokenizer; must not panic
        let _ = db.search("a", 5);
        let hits = db.search("xy", 5).unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn search_very_long_query_no_panic() {
        let db = SilDb::open_in_memory().unwrap();
        let mut doc = SourceDocument::new("l.pdf".into());
        doc.status = Some(DocumentStatus::ValidPdf);
        db.upsert_parsed(&doc, "needle in haystack").unwrap();
        let long = "needle ".repeat(2000);
        // FTS may error on some pathological queries; never panic
        let _ = db.search(&long, 5);
        let hits = db.search("needle", 5).unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn search_special_characters_no_panic() {
        let db = SilDb::open_in_memory().unwrap();
        let mut doc = SourceDocument::new("sp.pdf".into());
        doc.status = Some(DocumentStatus::ValidPdf);
        db.upsert_parsed(&doc, "foo bar baz").unwrap();
        // FTS5 special chars: must not panic (may return error or empty)
        for q in ["foo*", "\"bar\"", "foo AND bar", "!!!", "@@@"] {
            let _ = db.search(q, 5);
        }
        assert_eq!(db.search("baz", 5).unwrap().len(), 1);
    }

    #[test]
    fn upsert_empty_content() {
        let db = SilDb::open_in_memory().unwrap();
        let mut doc = SourceDocument::new("empty.pdf".into());
        doc.status = Some(DocumentStatus::ValidPdf);
        db.upsert_parsed(&doc, "").unwrap();
        assert_eq!(db.source_count().unwrap(), 1);
        assert!(db.is_parsed(&doc.id).unwrap());
    }

    #[test]
    fn search_empty_query_no_panic() {
        let db = SilDb::open_in_memory().unwrap();
        let mut doc = SourceDocument::new("e.pdf".into());
        doc.status = Some(DocumentStatus::ValidPdf);
        db.upsert_parsed(&doc, "content here").unwrap();
        let _ = db.search("", 5);
        let _ = db.search("   ", 5);
    }

    #[test]
    fn filename_with_spaces_and_unicode() {
        let db = SilDb::open_in_memory().unwrap();
        let mut doc = SourceDocument::new("my paper (final) 论文.pdf".into());
        doc.status = Some(DocumentStatus::ValidPdf);
        doc.title = Some("标题 Title".into());
        db.upsert_parsed(&doc, "body token_spacefile").unwrap();
        let list = db.list_sources().unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].filename.contains("paper") || list[0].filename.contains('论'));
        assert_eq!(db.search("token_spacefile", 5).unwrap().len(), 1);
    }

    #[test]
    fn zero_limit_search() {
        let db = SilDb::open_in_memory().unwrap();
        let mut doc = SourceDocument::new("z.pdf".into());
        doc.status = Some(DocumentStatus::ValidPdf);
        db.upsert_parsed(&doc, "token_zerolimit").unwrap();
        let hits = db.search("token_zerolimit", 0).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn multiple_docs_same_token_all_returned() {
        let db = SilDb::open_in_memory().unwrap();
        for name in ["a.pdf", "b.pdf", "c.pdf"] {
            let mut doc = SourceDocument::new(name.into());
            doc.status = Some(DocumentStatus::ValidPdf);
            db.upsert_parsed(&doc, "sharededge token").unwrap();
        }
        let hits = db.search("sharededge", 10).unwrap();
        assert_eq!(hits.len(), 3);
    }
}
