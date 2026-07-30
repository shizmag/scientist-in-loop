//! SQLite + FTS5 storage for parsed scientific sources.

#![deny(missing_docs)]

pub mod chunks;
pub mod error;
pub mod onnx;
pub mod schema;
pub mod search;
pub mod sources;

pub use chunks::{
    ChunkSearchHit, ChunkType, SourceChunk, blob_to_embedding, chunk_markdown, cosine_similarity,
    embedding_to_blob,
};
pub use error::DbError;
pub use onnx::{DEFAULT_EMBEDDING_DIM, OnnxEmbedder, OnnxReranker};
pub use search::{SearchHit, search_hyde};

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

    /// Remove a source by id. Returns true if a row was deleted.
    pub fn remove_source(&self, id: &SourceId) -> Result<bool, DbError> {
        sources::remove_source(&self.conn, id)
    }

    /// Full-text search over parsed sources.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, DbError> {
        search::search(&self.conn, query, limit)
    }

    /// Insert source chunks into the database.
    pub fn insert_source_chunks(&self, chunks: &[SourceChunk]) -> Result<(), DbError> {
        chunks::insert_chunks(&self.conn, chunks)
    }

    /// Get all chunks for a source ID.
    pub fn get_chunks_for_source(&self, source_id: &SourceId) -> Result<Vec<SourceChunk>, DbError> {
        chunks::get_chunks_for_source(&self.conn, source_id)
    }

    /// Get a specific chunk by ID.
    pub fn get_chunk_by_id(&self, chunk_id: &str) -> Result<Option<SourceChunk>, DbError> {
        chunks::get_chunk_by_id(&self.conn, chunk_id)
    }

    /// Delete all chunks for a source ID.
    pub fn delete_chunks_for_source(&self, source_id: &SourceId) -> Result<(), DbError> {
        chunks::delete_chunks_for_source(&self.conn, source_id)
    }

    /// Perform hybrid BM25 + Dense ONNX Reciprocal Rank Fusion (RRF) search over source chunks.
    pub fn search_hybrid(
        &self,
        embedder: &OnnxEmbedder,
        query: &str,
        limit: usize,
        expand_to_parent: bool,
    ) -> Result<Vec<ChunkSearchHit>, DbError> {
        chunks::search_hybrid(&self.conn, embedder, query, limit, expand_to_parent)
    }

    /// Perform HyDE (Hypothetical Document Expansion) search over source chunks.
    pub fn search_hyde(
        &self,
        embedder: &OnnxEmbedder,
        hypothetical_passage: &str,
        keyword_query: &str,
        limit: usize,
        expand_to_parent: bool,
    ) -> Result<Vec<ChunkSearchHit>, DbError> {
        search::search_hyde(
            &self.conn,
            embedder,
            hypothetical_passage,
            keyword_query,
            limit,
            expand_to_parent,
        )
    }

    /// Parse markdown into chunks, compute embeddings with optional embedder, and insert into DB.
    pub fn parse_and_index_chunks(
        &self,
        doc: &SourceDocument,
        markdown: &str,
        embedder: Option<&OnnxEmbedder>,
    ) -> Result<Vec<SourceChunk>, DbError> {
        let mut chunks = chunks::chunk_markdown(&doc.id, markdown);

        if let Some(emb) = embedder {
            for chunk in &mut chunks {
                if let Ok(vec) = emb.embed(&chunk.content) {
                    chunk.embedding_blob = Some(chunks::embedding_to_blob(&vec));
                }
            }
        }

        self.insert_source_chunks(&chunks)?;
        Ok(chunks)
    }

    /// Replace all idea/TODO blocks in database with fresh set from parser.
    pub fn replace_todo_ideas(&self, ideas: &[sil_core::IdeaBlock]) -> Result<(), DbError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM todo_ideas", [])?;
        let mut stmt = tx.prepare(
            "INSERT INTO todo_ideas (id, content, section_id, line_start, line_end, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for idea in ideas {
            stmt.execute(rusqlite::params![
                idea.id,
                idea.content,
                idea.section_id,
                idea.line_start,
                idea.line_end,
                idea.created_at
            ])?;
        }
        drop(stmt);
        tx.commit()?;
        Ok(())
    }

    /// List all parsed idea/TODO blocks.
    pub fn list_todo_ideas(&self) -> Result<Vec<sil_core::IdeaBlock>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, section_id, line_start, line_end, created_at FROM todo_ideas ORDER BY line_start ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(sil_core::IdeaBlock {
                id: row.get(0)?,
                content: row.get(1)?,
                section_id: row.get(2)?,
                line_start: row.get(3)?,
                line_end: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    /// Upsert a journal publication entry into the database.
    pub fn save_journal_publication(&self, item: &sil_core::JournalPublication) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO journal_digest (doi, title, authors, journal, year, abstract_text, citation_count, url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                item.doi.as_deref().unwrap_or(&item.title),
                item.title,
                item.authors,
                item.journal,
                item.year,
                item.abstract_text,
                item.citation_count,
                item.url
            ],
        )?;
        Ok(())
    }

    /// List top journal publications.
    pub fn list_journal_publications(&self) -> Result<Vec<sil_core::JournalPublication>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT doi, title, authors, journal, year, abstract_text, citation_count, url FROM journal_digest ORDER BY year DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let doi: Option<String> = row.get(0)?;
            Ok(sil_core::JournalPublication {
                doi,
                title: row.get(1)?,
                authors: row.get(2)?,
                journal: row.get(3)?,
                year: row.get(4)?,
                abstract_text: row.get(5)?,
                citation_count: row.get(6)?,
                url: row.get(7)?,
                pdf_url: None,
            })
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
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
        let _ = db.search("注意力", 5);
    }

    #[test]
    fn search_short_query() {
        let db = SilDb::open_in_memory().unwrap();
        let mut doc = SourceDocument::new("s.pdf".into());
        doc.status = Some(DocumentStatus::ValidPdf);
        db.upsert_parsed(&doc, "a short token xy").unwrap();
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

    #[test]
    fn remove_source_deletes_row_and_fts() {
        let db = SilDb::open_in_memory().unwrap();
        let mut doc = SourceDocument::new("gone.pdf".into());
        doc.status = Some(DocumentStatus::ValidPdf);
        db.upsert_parsed(&doc, "remove_token_xyz content").unwrap();
        assert_eq!(db.source_count().unwrap(), 1);
        assert!(!db.search("remove_token_xyz", 5).unwrap().is_empty());
        assert!(db.remove_source(&doc.id).unwrap());
        assert_eq!(db.source_count().unwrap(), 0);
        assert!(db.search("remove_token_xyz", 5).unwrap().is_empty());
        assert!(!db.remove_source(&doc.id).unwrap());
    }

    #[test]
    fn todo_ideas_crud() {
        let db = SilDb::open_in_memory().unwrap();
        assert!(db.list_todo_ideas().unwrap().is_empty());
        let idea1 = sil_core::IdeaBlock::new("id1", "Check ablation study", Some("Section 3".into()), 10, 15);
        let idea2 = sil_core::IdeaBlock::new("id2", "Add baseline comparison graph", Some("Section 4".into()), 40, 45);
        db.replace_todo_ideas(&[idea1.clone(), idea2.clone()]).unwrap();
        let list = db.list_todo_ideas().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].content, "Check ablation study");
        assert_eq!(list[1].content, "Add baseline comparison graph");
    }

    #[test]
    fn journal_publication_crud() {
        let db = SilDb::open_in_memory().unwrap();
        assert!(db.list_journal_publications().unwrap().is_empty());
        let item = sil_core::JournalPublication {
            doi: Some("10.1038/s41586-023-00000-0".into()),
            title: "Quantum Advantage in Scientific Discovery".into(),
            authors: "A. Einstein, N. Bohr".into(),
            journal: "Nature".into(),
            year: Some(2024),
            abstract_text: "We demonstrate quantum speedup for molecular simulation.".into(),
            citation_count: Some(42),
            url: "https://doi.org/10.1038/s41586-023-00000-0".into(),
            pdf_url: None,
        };
        db.save_journal_publication(&item).unwrap();
        let list = db.list_journal_publications().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "Quantum Advantage in Scientific Discovery");
        assert_eq!(list[0].journal, "Nature");
    }

    #[test]
    fn schema_migration_new_tables_and_columns() {
        let db = SilDb::open_in_memory().unwrap();
        let mut stmt = db.conn.prepare("PRAGMA table_info(todo_ideas)").unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(cols.contains(&"status".to_string()));
        assert!(cols.contains(&"priority".to_string()));
        assert!(cols.contains(&"author_type".to_string()));
        assert!(cols.contains(&"tags".to_string()));

        let mut stmt_chunks = db.conn.prepare("PRAGMA table_info(source_chunks)").unwrap();
        let chunk_cols: Vec<String> = stmt_chunks
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(chunk_cols.contains(&"parent_chunk_id".to_string()));
        assert!(chunk_cols.contains(&"chunk_type".to_string()));
        assert!(chunk_cols.contains(&"embedding_blob".to_string()));
    }

    #[test]
    fn schema_migration_pre_existing_db() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE todo_ideas (
                id TEXT PRIMARY KEY NOT NULL,
                content TEXT NOT NULL,
                section_id TEXT,
                line_start INTEGER NOT NULL,
                line_end INTEGER NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .unwrap();

        schema::migrate(&conn).unwrap();

        let mut stmt = conn.prepare("PRAGMA table_info(todo_ideas)").unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(cols.contains(&"status".to_string()));
        assert!(cols.contains(&"priority".to_string()));
        assert!(cols.contains(&"author_type".to_string()));
        assert!(cols.contains(&"tags".to_string()));
    }

    #[test]
    fn parent_child_chunk_splitting_and_insertion() {
        let db = SilDb::open_in_memory().unwrap();
        let mut doc = SourceDocument::new("paper.md".into());
        doc.status = Some(DocumentStatus::ValidPdf);
        db.upsert_parsed(&doc, "Full text").unwrap();

        let md = r#"
# Introduction
Deep learning models have transformed NLP.

Transformers use self-attention.

## Methodology
We propose a new architecture with cross-encoder reranking.

The cross-encoder scores candidate pairs.
"#;
        let chunks = db.parse_and_index_chunks(&doc, md, None).unwrap();
        assert!(chunks.len() >= 4);

        let parent_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.chunk_type == ChunkType::Parent)
            .collect();
        let child_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.chunk_type == ChunkType::Child)
            .collect();

        assert_eq!(parent_chunks.len(), 2);
        assert_eq!(
            parent_chunks[0].heading_title.as_deref(),
            Some("Introduction")
        );
        assert_eq!(
            parent_chunks[1].heading_title.as_deref(),
            Some("Methodology")
        );

        for child in &child_chunks {
            assert!(child.parent_chunk_id.is_some());
        }

        let retrieved = db.get_chunks_for_source(&doc.id).unwrap();
        assert_eq!(retrieved.len(), chunks.len());
    }

    #[test]
    fn bm25_dense_rrf_hybrid_search_and_parent_expansion() {
        let db = SilDb::open_in_memory().unwrap();
        let mut doc = SourceDocument::new("transformer.md".into());
        doc.status = Some(DocumentStatus::ValidPdf);
        db.upsert_parsed(&doc, "Full text").unwrap();

        let embedder = OnnxEmbedder::new(None::<&std::path::Path>);

        let md = r#"
# Transformer Architecture
The Transformer model relies entirely on self-attention mechanisms without recurrent layers.

Self-attention allows parallel sequence computation across GPU clusters.

# Cross-Encoder Reranker
Cross-encoder models process query and document pairs simultaneously to calculate fine-grained relevance scores.
"#;
        db.parse_and_index_chunks(&doc, md, Some(&embedder))
            .unwrap();

        let hits = db
            .search_hybrid(
                &embedder,
                "self-attention sequence computation",
                5,
                false,
            )
            .unwrap();
        assert!(!hits.is_empty());
        assert!(
            hits.iter().any(|h| h
                .chunk
                .content
                .contains("parallel sequence computation")
                || h.chunk.content.contains("self-attention"))
        );

        let expanded_hits = db
            .search_hybrid(&embedder, "parallel sequence computation", 5, true)
            .unwrap();
        assert!(!expanded_hits.is_empty());
        assert_eq!(expanded_hits[0].chunk.chunk_type, ChunkType::Parent);
        assert_eq!(
            expanded_hits[0].chunk.heading_title.as_deref(),
            Some("Transformer Architecture")
        );
    }

    #[test]
    fn hyde_search_execution() {
        let db = SilDb::open_in_memory().unwrap();
        let mut doc = SourceDocument::new("rag_paper.md".into());
        doc.status = Some(DocumentStatus::ValidPdf);
        db.upsert_parsed(&doc, "Full text").unwrap();

        let embedder = OnnxEmbedder::new(None::<&std::path::Path>);

        let md = r#"
# Dense Retrieval Methods
Reciprocal Rank Fusion combines BM25 keyword rankings with dense vector embeddings to achieve state of the art search accuracy.
"#;
        db.parse_and_index_chunks(&doc, md, Some(&embedder))
            .unwrap();

        let hypothetical = "Dense retrieval uses vector representations and rank fusion algorithms like RRF to improve search accuracy.";
        let keyword_q = "Reciprocal Rank Fusion BM25";

        let hyde_hits = db
            .search_hyde(&embedder, hypothetical, keyword_q, 5, false)
            .unwrap();
        assert!(!hyde_hits.is_empty());
        assert!(hyde_hits[0].score > 0.0);
    }
}
