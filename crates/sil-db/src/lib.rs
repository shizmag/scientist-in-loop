//! SQLite + FTS5 storage for parsed scientific sources.

#![deny(missing_docs)]

/// BibTeX references and DOI verifications storage.
pub mod bib_references;
pub mod chunks;
/// Discovery runs, provider evidence, canonical works, and candidates.
pub mod discovery;
pub mod embed_cache;
/// Database and embedding error types.
pub mod error;
/// ONNX Runtime embedding & reranking model integration.
pub mod onnx;
pub(crate) mod references;
pub(crate) mod schema;
pub(crate) mod search;
pub(crate) mod sources;
pub(crate) mod todo;

pub use bib_references::{
    ArxivVerificationRecord, BibReferenceRecord, DoiVerificationRecord,
    OpenreviewVerificationRecord,
};
pub use chunks::{
    ChunkSearchHit, ChunkType, SourceChunk, blob_to_embedding, chunk_markdown, cosine_similarity,
    embedding_to_blob,
};
pub use discovery::{
    Candidate, CandidateEvent, CandidateState, DiscoveryRun, ProviderRecord, ProviderRequest, Work,
    WorkIdentifier, WorkVenue, WorkVersion,
};
pub use error::DbError;
pub use onnx::{DEFAULT_EMBEDDING_DIM, OnnxEmbedder, OnnxReranker, RagBackend, RagFallbackReason};
pub use search::{SearchHit, search_hyde};

use camino::Utf8Path;
use rusqlite::Connection;
use sil_core::{SourceDocument, SourceId};

/// Open or create the project database and ensure schema.
pub struct SilDb {
    conn: Connection,
}

fn apply_pragmas(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(
        r#"
        PRAGMA busy_timeout = 5000;
        PRAGMA foreign_keys = ON;
        PRAGMA synchronous = NORMAL;
        "#,
    )?;
    let journal_mode =
        match conn.query_row::<String, _, _>("PRAGMA journal_mode", [], |row| row.get(0)) {
            Ok(mode) => mode,
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::DatabaseBusy =>
            {
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        };
    if !journal_mode.eq_ignore_ascii_case("wal") {
        conn.query_row::<String, _, _>("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
    }
    Ok(())
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
        apply_pragmas(&conn)?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Open an in-memory database (tests).
    pub fn open_in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        apply_pragmas(&conn)?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Check SQLite database integrity using `PRAGMA integrity_check`.
    pub fn integrity_check(&self) -> Result<String, DbError> {
        let res: String = self
            .conn
            .query_row("PRAGMA integrity_check;", [], |row| row.get(0))?;
        Ok(res)
    }

    /// Apply schema migrations.
    pub fn migrate(&self) -> Result<(), DbError> {
        schema::migrate(&self.conn)
    }

    /// Store a discovery run.
    pub fn create_discovery_run(&self, run: &DiscoveryRun) -> Result<(), DbError> {
        discovery::create_run(&self.conn, run)
    }

    /// Update the durable status and cursor of a discovery run.
    pub fn update_discovery_run(
        &self,
        id: &str,
        status: &str,
        cursor_json: Option<&str>,
    ) -> Result<(), DbError> {
        discovery::update_run(&self.conn, id, status, cursor_json)
    }

    /// Store one versioned, inspectable candidate ranking.
    pub fn insert_candidate_ranking(
        &self,
        run_id: &str,
        candidate_id: &str,
        algorithm_version: &str,
        score: i64,
        components_json: &str,
    ) -> Result<(), DbError> {
        discovery::insert_candidate_ranking(
            &self.conn,
            run_id,
            candidate_id,
            algorithm_version,
            score,
            components_json,
        )
    }

    /// Store immutable provider request metadata.
    pub fn insert_provider_request(&self, request: &ProviderRequest) -> Result<(), DbError> {
        discovery::insert_provider_request(&self.conn, request)
    }

    /// Store an immutable provider response record and its raw evidence.
    pub fn insert_provider_record(&self, record: &ProviderRecord) -> Result<(), DbError> {
        discovery::insert_provider_record(&self.conn, record)
    }

    /// Upsert canonical work metadata.
    pub fn upsert_work(&self, work: &Work) -> Result<(), DbError> {
        discovery::upsert_work(&self.conn, work)
    }

    /// Store a work identifier.
    pub fn insert_work_identifier(&self, identifier: &WorkIdentifier) -> Result<(), DbError> {
        discovery::insert_work_identifier(&self.conn, identifier)
    }

    /// Store a distinct publication version.
    pub fn insert_work_version(&self, version: &WorkVersion) -> Result<(), DbError> {
        discovery::insert_work_version(&self.conn, version)
    }

    /// Store raw and resolved venue evidence for a version.
    pub fn insert_work_venue(&self, venue: &WorkVenue) -> Result<(), DbError> {
        discovery::insert_work_venue(&self.conn, venue)
    }

    /// Store a candidate with its three independent initial states.
    pub fn insert_candidate(&self, candidate: &Candidate) -> Result<(), DbError> {
        discovery::insert_candidate(&self.conn, candidate)
    }

    /// Apply and audit one candidate state transition.
    pub fn transition_candidate(
        &self,
        id: &str,
        dimension: &str,
        to: CandidateState,
        actor: &str,
        reason: &str,
    ) -> Result<(), DbError> {
        discovery::transition_candidate(&self.conn, id, dimension, to, actor, reason)
    }

    /// Read append-only candidate events.
    pub fn candidate_events(&self, candidate_id: &str) -> Result<Vec<CandidateEvent>, DbError> {
        discovery::candidate_events(&self.conn, candidate_id)
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

    /// Upsert a parsed source document and its extracted references in a single SQLite transaction.
    pub fn upsert_parsed_with_references(
        &self,
        doc: &SourceDocument,
        content: &str,
        entries: &[sil_core::ReferenceEntry],
    ) -> Result<(), DbError> {
        self.conn.execute_batch("BEGIN TRANSACTION;")?;
        let res = (|| -> Result<(), DbError> {
            sources::upsert_parsed(&self.conn, doc, content)?;
            if !entries.is_empty() {
                references::save_source_references(&self.conn, entries)?;
            }
            Ok(())
        })();

        if res.is_ok() {
            self.conn.execute_batch("COMMIT;")?;
        } else {
            let _ = self.conn.execute_batch("ROLLBACK;");
        }
        res
    }

    /// List all source documents (metadata only).
    pub fn list_sources(&self) -> Result<Vec<SourceDocument>, DbError> {
        sources::list_sources(&self.conn)
    }

    /// Get the full parsed document and markdown content of a source by id or filename.
    pub fn get_source_content(
        &self,
        id_or_filename: &str,
    ) -> Result<Option<(SourceDocument, String)>, DbError> {
        sources::get_source_content(&self.conn, id_or_filename)
    }

    /// Remove a source by id. Returns true if a row was deleted.
    pub fn remove_source(&self, id: &SourceId) -> Result<bool, DbError> {
        sources::remove_source(&self.conn, id)
    }

    /// Update title of a source document by id. Returns true if a row was updated.
    pub fn update_source_title(&self, id: &SourceId, new_title: &str) -> Result<bool, DbError> {
        sources::update_source_title(&self.conn, id, new_title)
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

    /// Insert a single TodoIdea.
    pub fn insert_todo_idea(&self, idea: &sil_core::IdeaBlock) -> Result<(), DbError> {
        todo::insert_todo_idea(&self.conn, idea)
    }

    /// Update an existing TodoIdea.
    pub fn update_todo_idea(&self, idea: &sil_core::IdeaBlock) -> Result<(), DbError> {
        todo::update_todo_idea(&self.conn, idea)
    }

    /// Upsert a TodoIdea (insert or update on conflict).
    pub fn upsert_todo_idea(&self, idea: &sil_core::IdeaBlock) -> Result<(), DbError> {
        todo::upsert_todo_idea(&self.conn, idea)
    }

    /// Delete a TodoIdea by id.
    pub fn delete_todo_idea(&self, id: &str) -> Result<bool, DbError> {
        todo::delete_todo_idea(&self.conn, id)
    }

    /// Get a TodoIdea by id.
    pub fn get_todo_idea_by_id(&self, id: &str) -> Result<Option<sil_core::IdeaBlock>, DbError> {
        todo::get_todo_idea_by_id(&self.conn, id)
    }

    /// Replace all idea/TODO blocks in database with fresh set from parser.
    pub fn replace_todo_ideas(&self, ideas: &[sil_core::IdeaBlock]) -> Result<(), DbError> {
        todo::replace_todo_ideas(&self.conn, ideas)
    }

    /// List all parsed idea/TODO blocks.
    pub fn list_todo_ideas(&self) -> Result<Vec<sil_core::IdeaBlock>, DbError> {
        todo::list_todo_ideas_filtered(&self.conn, None, None, None, None)
    }

    /// List todo_ideas with optional filters for status, priority, section_id, and sorting.
    pub fn list_todo_ideas_filtered(
        &self,
        status: Option<&str>,
        priority: Option<&str>,
        section_id: Option<&str>,
        sort_by: Option<&str>,
    ) -> Result<Vec<sil_core::IdeaBlock>, DbError> {
        todo::list_todo_ideas_filtered(&self.conn, status, priority, section_id, sort_by)
    }

    /// Upsert a journal publication entry into the database.
    pub fn save_journal_publication(
        &self,
        item: &sil_core::JournalPublication,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO journal_digest (doi, title, authors, journal, year, abstract_text, citation_count, url, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'))",
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

    /// Return the timestamp (ISO format) of the most recently fetched journal publication in the digest cache.
    pub fn digest_last_fetched_at(&self) -> Result<Option<String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT MAX(fetched_at) FROM journal_digest")?;
        let res: Option<String> = stmt.query_row([], |row| row.get(0))?;
        Ok(res)
    }

    /// List top journal publications.
    pub fn list_journal_publications(&self) -> Result<Vec<sil_core::JournalPublication>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT doi, title, authors, journal, year, abstract_text, citation_count, url FROM journal_digest ORDER BY year DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let doi: Option<String> = row
                .get::<_, Option<String>>(0)?
                .filter(|value| value.starts_with("10.") && value.contains('/'));
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

    /// Retrieve cached embedding for text content hash.
    pub fn get_cached_embedding(
        &self,
        content_hash: &str,
        model_name: &str,
    ) -> Result<Option<Vec<f32>>, DbError> {
        embed_cache::get_cached_embedding(&self.conn, content_hash, model_name)
    }

    /// Store calculated embedding in vector cache.
    pub fn put_cached_embedding(
        &self,
        content_hash: &str,
        model_name: &str,
        dimension: usize,
        embedding: &[f32],
    ) -> Result<(), DbError> {
        embed_cache::put_cached_embedding(
            &self.conn,
            content_hash,
            model_name,
            dimension,
            embedding,
        )
    }

    /// Clear vector embedding cache.
    pub fn clear_embedding_cache(&self) -> Result<usize, DbError> {
        embed_cache::clear_embedding_cache(&self.conn)
    }

    /// Get vector embedding cache row count.
    pub fn embedding_cache_stats(&self) -> Result<usize, DbError> {
        embed_cache::embedding_cache_stats(&self.conn)
    }

    /// Save reference entries for a source document.
    pub fn save_source_references(&self, refs: &[sil_core::ReferenceEntry]) -> Result<(), DbError> {
        references::save_source_references(&self.conn, refs)
    }

    /// Get all reference entries for a source ID.
    pub fn get_references_for_source(
        &self,
        source_id: &sil_core::SourceId,
    ) -> Result<Vec<sil_core::ReferenceEntry>, DbError> {
        references::get_references_for_source(&self.conn, source_id)
    }

    /// Get all reference entries across all source documents in the project.
    pub fn get_all_references(&self) -> Result<Vec<sil_core::ReferenceEntry>, DbError> {
        references::get_all_references(&self.conn)
    }

    /// Full-text search over extracted source references.
    pub fn search_references(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<sil_core::ReferenceEntry>, DbError> {
        references::search_references(&self.conn, query, limit)
    }

    /// Delete all reference entries for a source ID.
    pub fn delete_references_for_source(
        &self,
        source_id: &sil_core::SourceId,
    ) -> Result<(), DbError> {
        references::delete_references_for_source(&self.conn, source_id)
    }

    /// Recompute cosine similarity scores between paper draft text and all extracted source references.
    pub fn recompute_draft_ref_similarities(
        &self,
        draft_text: &str,
        embedder: &crate::onnx::OnnxEmbedder,
    ) -> Result<usize, DbError> {
        references::recompute_draft_ref_similarities(&self.conn, draft_text, embedder)
    }

    /// Retrieve all persisted draft-reference similarity scores keyed by `ref_id`.
    pub fn get_draft_ref_similarities(
        &self,
    ) -> Result<std::collections::HashMap<String, f32>, DbError> {
        references::get_draft_ref_similarities(&self.conn)
    }

    /// Retrieve stored draft content hash to verify staleness.
    pub fn get_draft_similarity_hash(&self) -> Result<Option<String>, DbError> {
        references::get_draft_similarity_hash(&self.conn)
    }

    /// Get all records from `bib_references` table.
    pub fn get_bib_references(&self) -> Result<Vec<BibReferenceRecord>, DbError> {
        bib_references::get_bib_references(&self.conn)
    }

    /// Get a single DOI verification record by DOI.
    pub fn get_doi_verification(
        &self,
        doi: &str,
    ) -> Result<Option<DoiVerificationRecord>, DbError> {
        bib_references::get_doi_verification(&self.conn, doi)
    }

    /// Get all DOI verification records as a HashMap keyed by DOI string.
    pub fn get_doi_verifications(
        &self,
    ) -> Result<std::collections::HashMap<String, DoiVerificationRecord>, DbError> {
        bib_references::get_doi_verifications(&self.conn)
    }

    /// Upsert a DOI verification record.
    pub fn upsert_doi_verification(
        &self,
        doi: &str,
        exists: bool,
        error_cat: Option<&str>,
    ) -> Result<(), DbError> {
        bib_references::upsert_doi_verification(&self.conn, doi, exists, error_cat)
    }

    /// Get a single arXiv verification record by arXiv ID.
    pub fn get_arxiv_verification(
        &self,
        arxiv_id: &str,
    ) -> Result<Option<ArxivVerificationRecord>, DbError> {
        bib_references::get_arxiv_verification(&self.conn, arxiv_id)
    }

    /// Get all arXiv verification records as a HashMap keyed by arXiv ID string.
    pub fn get_arxiv_verifications(
        &self,
    ) -> Result<std::collections::HashMap<String, ArxivVerificationRecord>, DbError> {
        bib_references::get_arxiv_verifications(&self.conn)
    }

    /// Upsert an arXiv verification record.
    pub fn upsert_arxiv_verification(
        &self,
        arxiv_id: &str,
        exists: bool,
        error_cat: Option<&str>,
    ) -> Result<(), DbError> {
        bib_references::upsert_arxiv_verification(&self.conn, arxiv_id, exists, error_cat)
    }

    /// Get a single OpenReview verification record by OpenReview ID.
    pub fn get_openreview_verification(
        &self,
        openreview_id: &str,
    ) -> Result<Option<OpenreviewVerificationRecord>, DbError> {
        bib_references::get_openreview_verification(&self.conn, openreview_id)
    }

    /// Get all OpenReview verification records as a HashMap keyed by OpenReview ID string.
    pub fn get_openreview_verifications(
        &self,
    ) -> Result<std::collections::HashMap<String, OpenreviewVerificationRecord>, DbError> {
        bib_references::get_openreview_verifications(&self.conn)
    }

    /// Upsert an OpenReview verification record.
    pub fn upsert_openreview_verification(
        &self,
        openreview_id: &str,
        exists: bool,
        error_cat: Option<&str>,
    ) -> Result<(), DbError> {
        bib_references::upsert_openreview_verification(&self.conn, openreview_id, exists, error_cat)
    }

    /// Upsert a bib reference record using UPDATE SURGERY logic.
    pub fn upsert_bib_reference(
        &self,
        cite_key: &str,
        doi: Option<&str>,
        doi_exists: Option<bool>,
        raw_bibtex: &str,
    ) -> Result<bool, DbError> {
        bib_references::upsert_bib_reference(&self.conn, cite_key, doi, doi_exists, raw_bibtex)
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
            hits[0].snippet.to_lowercase().contains("transformer") || !hits[0].snippet.is_empty()
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
            db.upsert_parsed(&doc, "persisted content token_persist")
                .unwrap();
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
            db.upsert_parsed(&doc, "sharedkeyword document body")
                .unwrap();
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
        db.upsert_parsed(&doc, "注意力机制 uniquetokenxyz 自注意力 café αβγ")
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
    fn update_source_title_on_sildb() {
        let db = SilDb::open_in_memory().unwrap();
        let mut doc = SourceDocument::new("sildb_paper.pdf".into());
        doc.status = Some(DocumentStatus::ValidPdf);
        db.upsert_parsed(&doc, "sildb update title test content")
            .unwrap();

        assert!(
            db.update_source_title(&doc.id, "Updated SilDb Title")
                .unwrap()
        );
        let list = db.list_sources().unwrap();
        assert_eq!(list[0].title.as_deref(), Some("Updated SilDb Title"));
    }

    #[test]
    fn todo_ideas_crud() {
        let db = SilDb::open_in_memory().unwrap();
        assert!(db.list_todo_ideas().unwrap().is_empty());

        let mut idea1 = sil_core::IdeaBlock::new(
            "id1",
            "Check ablation study",
            Some("Section 3".into()),
            10,
            15,
        );
        idea1.status = "open".into();
        idea1.priority = "high".into();
        idea1.author_type = "human".into();
        idea1.tags = vec!["ablation".into()];

        let mut idea2 = sil_core::IdeaBlock::new(
            "id2",
            "Add baseline comparison graph",
            Some("Section 4".into()),
            40,
            45,
        );
        idea2.status = "in_progress".into();
        idea2.priority = "critical".into();
        idea2.author_type = "agent".into();
        idea2.tags = vec!["baseline".into(), "figure".into()];

        // 1. insert & replace
        db.replace_todo_ideas(&[idea1.clone(), idea2.clone()])
            .unwrap();
        let list = db.list_todo_ideas().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "id1");
        assert_eq!(list[0].priority, "high");
        assert_eq!(list[1].id, "id2");
        assert_eq!(list[1].author_type, "agent");
        assert_eq!(list[1].tags, vec!["baseline", "figure"]);

        // 2. update & get_by_id
        let mut idea1_updated = idea1.clone();
        idea1_updated.status = "resolved".into();
        db.update_todo_idea(&idea1_updated).unwrap();
        let fetched = db.get_todo_idea_by_id("id1").unwrap().unwrap();
        assert_eq!(fetched.status, "resolved");

        // 3. upsert
        let mut idea3 =
            sil_core::IdeaBlock::new("id3", "New upserted idea", Some("Section 1".into()), 5, 8);
        idea3.priority = "low".into();
        db.upsert_todo_idea(&idea3).unwrap();
        assert_eq!(db.list_todo_ideas().unwrap().len(), 3);

        // 4. delete
        assert!(db.delete_todo_idea("id1").unwrap());
        assert_eq!(db.list_todo_ideas().unwrap().len(), 2);
        assert!(db.get_todo_idea_by_id("id1").unwrap().is_none());
    }

    #[test]
    fn todo_ideas_filtering_and_sorting() {
        let db = SilDb::open_in_memory().unwrap();

        let mut t1 = sil_core::IdeaBlock::new("t1", "Task 1", Some("Intro".into()), 10, 15);
        t1.status = "open".into();
        t1.priority = "medium".into();

        let mut t2 = sil_core::IdeaBlock::new("t2", "Task 2", Some("Intro".into()), 20, 25);
        t2.status = "in_progress".into();
        t2.priority = "critical".into();

        let mut t3 = sil_core::IdeaBlock::new("t3", "Task 3", Some("Methods".into()), 30, 35);
        t3.status = "open".into();
        t3.priority = "high".into();

        db.replace_todo_ideas(&[t1, t2, t3]).unwrap();

        // Filter status=open
        let open_tasks = db
            .list_todo_ideas_filtered(Some("open"), None, None, None)
            .unwrap();
        assert_eq!(open_tasks.len(), 2);

        // Filter priority=critical
        let crit_tasks = db
            .list_todo_ideas_filtered(None, Some("critical"), None, None)
            .unwrap();
        assert_eq!(crit_tasks.len(), 1);
        assert_eq!(crit_tasks[0].id, "t2");

        // Filter section_id=Intro
        let intro_tasks = db
            .list_todo_ideas_filtered(None, None, Some("Intro"), None)
            .unwrap();
        assert_eq!(intro_tasks.len(), 2);

        // Sort by priority (critical > high > medium)
        let sorted_prio = db
            .list_todo_ideas_filtered(None, None, None, Some("priority"))
            .unwrap();
        assert_eq!(sorted_prio.len(), 3);
        assert_eq!(sorted_prio[0].id, "t2"); // critical
        assert_eq!(sorted_prio[1].id, "t3"); // high
        assert_eq!(sorted_prio[2].id, "t1"); // medium
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
    fn digest_last_fetched_at_roundtrip() {
        let db = SilDb::open_in_memory().unwrap();
        assert_eq!(db.digest_last_fetched_at().unwrap(), None);

        let item = sil_core::JournalPublication {
            doi: Some("10.1038/s41586-023-00000-0".into()),
            title: "Quantum Advantage".into(),
            authors: "A. Scientist".into(),
            journal: "Nature".into(),
            year: Some(2024),
            abstract_text: "Abstract".into(),
            citation_count: Some(10),
            url: "https://doi.org/10.1038/s41586-023-00000-0".into(),
            pdf_url: None,
        };
        db.save_journal_publication(&item).unwrap();

        let fetched_at = db.digest_last_fetched_at().unwrap();
        assert!(fetched_at.is_some());
        assert!(!fetched_at.unwrap().is_empty());
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
            .search_hybrid(&embedder, "self-attention sequence computation", 5, false)
            .unwrap();
        assert!(!hits.is_empty());
        assert!(hits.iter().any(
            |h| h.chunk.content.contains("parallel sequence computation")
                || h.chunk.content.contains("self-attention")
        ));

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

    #[test]
    fn test_sildb_facade_wrappers() {
        let db = SilDb::open_in_memory().unwrap();

        // 1. Source content wrapper
        let doc = SourceDocument::new("doc_facade.pdf".into());
        db.upsert_parsed(&doc, "Facade content test").unwrap();
        let fetched = db.get_source_content("doc_facade.pdf").unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().1, "Facade content test");

        // 2. Chunks wrappers on SilDb
        let chunk = SourceChunk {
            id: "facade_c1".into(),
            source_id: doc.id.clone(),
            parent_chunk_id: None,
            chunk_type: ChunkType::Child,
            heading_title: None,
            content: "Facade chunk text".into(),
            start_offset: 0,
            end_offset: 17,
            embedding_blob: None,
            created_at: String::new(),
        };
        db.insert_source_chunks(&[chunk]).unwrap();

        let chunks_by_src = db.get_chunks_for_source(&doc.id).unwrap();
        assert_eq!(chunks_by_src.len(), 1);

        let chunk_by_id = db.get_chunk_by_id("facade_c1").unwrap();
        assert!(chunk_by_id.is_some());

        db.delete_chunks_for_source(&doc.id).unwrap();
        assert!(db.get_chunks_for_source(&doc.id).unwrap().is_empty());

        // 3. Reference wrappers on SilDb
        let ref_entry = sil_core::ReferenceEntry {
            id: "ref_facade_1".into(),
            source_id: doc.id.clone(),
            ref_index: 1,
            raw_text: "Ref raw text".into(),
            title: Some("Ref title".into()),
            authors: None,
            year: None,
            venue: None,
            doi: None,
            arxiv_id: None,
            url: None,
        };
        db.save_source_references(&[ref_entry]).unwrap();

        let refs_for_src = db.get_references_for_source(&doc.id).unwrap();
        assert_eq!(refs_for_src.len(), 1);

        let all_refs = db.get_all_references().unwrap();
        assert_eq!(all_refs.len(), 1);

        let search_refs = db.search_references("Ref title", 5).unwrap();
        assert_eq!(search_refs.len(), 1);

        db.delete_references_for_source(&doc.id).unwrap();
        assert!(db.get_references_for_source(&doc.id).unwrap().is_empty());

        // 4. Todo idea insert wrapper on SilDb
        let idea = sil_core::IdeaBlock::new("idea_facade", "Facade idea content", None, 1, 5);
        db.insert_todo_idea(&idea).unwrap();
        assert!(db.get_todo_idea_by_id("idea_facade").unwrap().is_some());
    }

    #[test]
    fn test_source_references_automigration() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(r#"
            CREATE TABLE sources (id TEXT PRIMARY KEY NOT NULL, path TEXT NOT NULL, filename TEXT NOT NULL);
            CREATE TABLE source_references (
                id TEXT PRIMARY KEY NOT NULL,
                source_id TEXT NOT NULL REFERENCES sources(id),
                ref_index INTEGER NOT NULL,
                raw_text TEXT NOT NULL,
                title TEXT,
                authors TEXT,
                year INTEGER,
                venue TEXT,
                doi TEXT
            );
        "#).unwrap();

        schema::migrate(&conn).unwrap();

        let mut stmt = conn
            .prepare("PRAGMA table_info(source_references)")
            .unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        assert!(cols.contains(&"arxiv_id".to_string()));
        assert!(cols.contains(&"url".to_string()));
    }

    #[test]
    fn test_sildb_bib_references_facade() {
        let db = SilDb::open_in_memory().unwrap();

        // Bib references
        assert!(db.get_bib_references().unwrap().is_empty());
        let mut_ins = db
            .upsert_bib_reference(
                "key_facade",
                Some("10.1000/facade"),
                Some(true),
                "@article{key_facade}",
            )
            .unwrap();
        assert!(mut_ins);

        let refs = db.get_bib_references().unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].cite_key, "key_facade");

        let mut_re = db
            .upsert_bib_reference(
                "key_facade",
                Some("10.1000/facade"),
                Some(true),
                "@article{key_facade}",
            )
            .unwrap();
        assert!(!mut_re);

        // DOI verifications
        assert!(db.get_doi_verifications().unwrap().is_empty());
        db.upsert_doi_verification("10.1000/facade", true, None)
            .unwrap();

        let v = db.get_doi_verification("10.1000/facade").unwrap();
        assert!(v.is_some());
        assert!(v.unwrap().exists_flag);

        let map = db.get_doi_verifications().unwrap();
        assert_eq!(map.len(), 1);

        // arXiv verifications
        assert!(db.get_arxiv_verifications().unwrap().is_empty());
        db.upsert_arxiv_verification("2106.09685", true, None)
            .unwrap();

        let v_arxiv = db.get_arxiv_verification("2106.09685").unwrap();
        assert!(v_arxiv.is_some());
        assert!(v_arxiv.unwrap().exists_flag);

        let map_arxiv = db.get_arxiv_verifications().unwrap();
        assert_eq!(map_arxiv.len(), 1);

        // OpenReview verifications
        assert!(db.get_openreview_verifications().unwrap().is_empty());
        db.upsert_openreview_verification("forum?id=abc12345", true, None)
            .unwrap();

        let v_or = db.get_openreview_verification("forum?id=abc12345").unwrap();
        assert!(v_or.is_some());
        assert!(v_or.unwrap().exists_flag);

        let map_or = db.get_openreview_verifications().unwrap();
        assert_eq!(map_or.len(), 1);
    }

    #[test]
    fn test_wal_busy_timeout_and_integrity() {
        use camino::Utf8PathBuf;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let db_path = Utf8PathBuf::from_path_buf(dir.path().join("test_wal.db")).unwrap();

        let db1 = SilDb::open(&db_path).unwrap();
        let journal_mode: String = db1
            .conn
            .query_row("PRAGMA journal_mode;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(journal_mode.to_lowercase(), "wal");

        let db2 = SilDb::open(&db_path).unwrap();
        assert_eq!(db1.integrity_check().unwrap(), "ok");
        assert_eq!(db2.integrity_check().unwrap(), "ok");

        db1.conn.execute("BEGIN IMMEDIATE;", []).unwrap();

        let db2_clone = db_path.clone();
        let handle = std::thread::spawn(move || {
            let db2 = SilDb::open(&db2_clone).unwrap();
            db2.source_count()
        });

        std::thread::sleep(std::time::Duration::from_millis(50));
        db1.conn.execute("COMMIT;", []).unwrap();

        let res = handle.join().unwrap();
        assert!(res.is_ok());
    }

    #[test]
    fn test_in_memory_integrity_check() {
        let db = SilDb::open_in_memory().unwrap();
        assert_eq!(db.integrity_check().unwrap(), "ok");
        let journal_mode: String = db
            .conn
            .query_row("PRAGMA journal_mode;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(journal_mode.to_lowercase(), "memory");
    }

    #[test]
    fn discovery_schema_is_idempotent_and_legacy_digest_survives() {
        let db = SilDb::open_in_memory().unwrap();
        db.conn
            .execute(
                "INSERT INTO journal_digest (doi, title, authors, journal, abstract_text, url) VALUES ('title-only-key', 'A title', 'An author', 'A journal', 'An abstract', 'https://example.test')",
                [],
            )
            .unwrap();
        db.migrate().unwrap();

        let version: i64 = db
            .conn
            .query_row(
                "SELECT version FROM schema_versions WHERE name='discovery'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 1);
        assert_eq!(db.list_journal_publications().unwrap()[0].doi, None);
        db.migrate().unwrap();
        assert_eq!(db.integrity_check().unwrap(), "ok");
    }

    #[test]
    fn discovery_preserves_provider_and_venue_evidence_and_is_run_scoped() {
        let db = SilDb::open_in_memory().unwrap();
        let run = DiscoveryRun {
            id: "run-a".into(),
            query: "attention".into(),
            status: "complete".into(),
            cursor_json: Some(r#"{"next":"c2"}"#.into()),
            created_at: "2026-08-15T00:00:00Z".into(),
        };
        db.create_discovery_run(&run).unwrap();
        db.create_discovery_run(&DiscoveryRun {
            id: "run-b".into(),
            ..run.clone()
        })
        .unwrap();
        db.insert_provider_request(&ProviderRequest {
            id: "req-a".into(),
            run_id: run.id.clone(),
            provider: "fixture".into(),
            request_json: r#"{"url":"https://example.test"}"#.into(),
            cursor: Some("c1".into()),
            status: "ok".into(),
        })
        .unwrap();
        db.insert_provider_record(&ProviderRecord {
            id: "record-a".into(),
            run_id: run.id.clone(),
            request_id: "req-a".into(),
            provider: "fixture".into(),
            provider_record_id: "paper-1".into(),
            raw_payload: r#"{"title":"Raw"}"#.into(),
            raw_payload_hash: "sha256:abc".into(),
            status: "ok".into(),
        })
        .unwrap();
        db.upsert_work(&Work {
            id: "work-a".into(),
            title: "Canonical title".into(),
            abstract_text: None,
            authors_json: None,
            year: Some(2026),
        })
        .unwrap();
        db.insert_work_identifier(&WorkIdentifier {
            work_id: "work-a".into(),
            namespace: "doi".into(),
            value: "10.1000/example".into(),
            observed_by: Some("fixture".into()),
        })
        .unwrap();
        db.insert_work_version(&WorkVersion {
            id: "version-a".into(),
            work_id: "work-a".into(),
            version_kind: "conference".into(),
            title: Some("Version title".into()),
            published_at: Some("2026-06-01".into()),
            url: None,
            open_access: Some(true),
        })
        .unwrap();
        db.insert_work_venue(&WorkVenue {
            id: "venue-a".into(),
            version_id: "version-a".into(),
            venue_id: None,
            raw_venue: "NeurIPS".into(),
            normalized_venue: Some("neurips".into()),
            resolution_status: "ambiguous".into(),
            evidence_json: Some(r#"{"candidates":["conf.nips"]}"#.into()),
            catalogue_version: Some("2026.08.15-seed".into()),
            normalizer_version: Some(1),
        })
        .unwrap();

        let raw: String = db
            .conn
            .query_row(
                "SELECT raw_payload FROM provider_records WHERE id='record-a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let evidence: String = db
            .conn
            .query_row(
                "SELECT evidence_json FROM work_venues WHERE id='venue-a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(raw.contains("Raw"));
        assert!(evidence.contains("conf.nips"));
        assert_eq!(
            db.conn
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM provider_records WHERE run_id='run-b'",
                    [],
                    |r| r.get(0)
                )
                .unwrap(),
            0
        );
    }

    #[test]
    fn candidate_transitions_are_orthogonal_and_append_only() {
        let db = SilDb::open_in_memory().unwrap();
        db.create_discovery_run(&DiscoveryRun {
            id: "run".into(),
            query: "q".into(),
            status: "running".into(),
            cursor_json: None,
            created_at: "now".into(),
        })
        .unwrap();
        db.upsert_work(&Work {
            id: "work".into(),
            title: "t".into(),
            abstract_text: None,
            authors_json: None,
            year: None,
        })
        .unwrap();
        db.insert_work_version(&WorkVersion {
            id: "version".into(),
            work_id: "work".into(),
            version_kind: "preprint".into(),
            title: None,
            published_at: None,
            url: None,
            open_access: None,
        })
        .unwrap();
        db.insert_candidate(&Candidate {
            id: "candidate".into(),
            run_id: "run".into(),
            version_id: "version".into(),
            provider_record_id: None,
            resolution: CandidateState::New,
            disposition: CandidateState::New,
            acquisition: CandidateState::New,
        })
        .unwrap();

        db.transition_candidate(
            "candidate",
            "resolution",
            CandidateState::Pending,
            "system",
            "queued",
        )
        .unwrap();
        db.transition_candidate(
            "candidate",
            "resolution",
            CandidateState::Accepted,
            "resolver",
            "exact DOI",
        )
        .unwrap();
        db.transition_candidate(
            "candidate",
            "disposition",
            CandidateState::Accepted,
            "human",
            "shortlist",
        )
        .unwrap();
        assert!(
            db.transition_candidate(
                "candidate",
                "resolution",
                CandidateState::Rejected,
                "human",
                "invalid reversal"
            )
            .is_err()
        );
        let events = db.candidate_events("candidate").unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].dimension, "resolution");
        assert_eq!(events[2].dimension, "disposition");
    }

    #[test]
    fn failed_discovery_migration_rolls_back() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute("CREATE TABLE schema_versions (wrong TEXT PRIMARY KEY)", [])
            .unwrap();
        assert!(schema::migrate(&conn).is_err());
        assert!(
            conn.query_row::<i64, _, _>(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='discovery_runs'",
                [],
                |r| r.get(0)
            )
            .unwrap()
                == 0
        );
    }
}
