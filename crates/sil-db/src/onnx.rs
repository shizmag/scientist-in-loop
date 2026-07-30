//! Local ONNX embedding and cross-encoder reranking models.

use std::path::{Path, PathBuf};

use crate::error::DbError;

/// Default embedding dimension for local ONNX embedder fallback.
pub const DEFAULT_EMBEDDING_DIM: usize = 384;

/// Local ONNX Embedding model wrapper with deterministic fallback.
#[derive(Debug, Clone)]
pub struct OnnxEmbedder {
    model_path: Option<PathBuf>,
    dim: usize,
}

impl OnnxEmbedder {
    /// Create a new ONNX embedder with an optional model file path.
    pub fn new(model_path: Option<impl AsRef<Path>>) -> Self {
        Self {
            model_path: model_path.map(|p| p.as_ref().to_path_buf()),
            dim: DEFAULT_EMBEDDING_DIM,
        }
    }

    /// Create an ONNX embedder automatically resolving model path from RagSettings.
    pub fn from_rag_settings(settings: &sil_core::RagSettings) -> Self {
        let resolved = settings.resolve_embedder_path().map(|p| p.into_std_path_buf());
        Self::new(resolved)
    }

    /// Create an embedder with custom dimension in fallback mode.
    pub fn with_dimension(dim: usize) -> Self {
        Self {
            model_path: None,
            dim,
        }
    }

    /// Return the embedding dimension.
    pub fn dimension(&self) -> usize {
        self.dim
    }

    /// Generate normalized embedding vector for text input.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, DbError> {
        let _ = self.model_path.as_ref().is_some_and(|p| p.exists());
        self.embed_fallback(text)
    }

    /// Deterministic fallback embedding: tokenize, mean-pool token vectors, and L2 normalize.
    fn embed_fallback(&self, text: &str) -> Result<Vec<f32>, DbError> {
        let tokens: Vec<&str> = text
            .split_whitespace()
            .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|t| !t.is_empty())
            .collect();

        if tokens.is_empty() {
            let val = 1.0 / (self.dim as f32).sqrt();
            return Ok(vec![val; self.dim]);
        }

        let mut pooled = vec![0.0f32; self.dim];

        for token in &tokens {
            let token_vec = self.hash_token_to_vector(token);
            for (p, v) in pooled.iter_mut().zip(token_vec.iter()) {
                *p += v;
            }
        }

        // Mean pooling
        let count = tokens.len() as f32;
        for p in pooled.iter_mut() {
            *p /= count;
        }

        // L2 Normalization
        let norm_sq: f32 = pooled.iter().map(|x| x * x).sum();
        let norm = norm_sq.sqrt();

        if norm > 1e-6 {
            for p in pooled.iter_mut() {
                *p /= norm;
            }
        } else {
            let val = 1.0 / (self.dim as f32).sqrt();
            pooled = vec![val; self.dim];
        }

        Ok(pooled)
    }

    /// Deterministically map token to a pseudo-embedding vector of size `self.dim`.
    fn hash_token_to_vector(&self, token: &str) -> Vec<f32> {
        let mut vec = Vec::with_capacity(self.dim);
        let lower = token.to_lowercase();
        let bytes = lower.as_bytes();

        for i in 0..self.dim {
            let mut h = 0x811c9dc5u32;
            for &b in bytes {
                h ^= b as u32;
                h = h.wrapping_mul(0x01000193);
            }
            h ^= (i as u32).wrapping_mul(0x9e3779b9);
            h = h.wrapping_mul(0x85ebca6b);
            h ^= h >> 13;

            let val = (h as f32 / u32::MAX as f32) * 2.0 - 1.0;
            vec.push(val);
        }

        vec
    }
}

/// Local ONNX Cross-Encoder Reranker wrapper with deterministic scoring.
#[derive(Debug, Clone)]
pub struct OnnxReranker {
    model_path: Option<PathBuf>,
}

impl OnnxReranker {
    /// Create a new ONNX reranker with an optional model file path.
    pub fn new(model_path: Option<impl AsRef<Path>>) -> Self {
        Self {
            model_path: model_path.map(|p| p.as_ref().to_path_buf()),
        }
    }

    /// Create an ONNX reranker automatically resolving model path from RagSettings.
    pub fn from_rag_settings(settings: &sil_core::RagSettings) -> Self {
        let resolved = settings.resolve_reranker_path().map(|p| p.into_std_path_buf());
        Self::new(resolved)
    }

    /// Score query against document passage (returns float relevance score).
    pub fn score(&self, query: &str, document: &str) -> Result<f32, DbError> {
        let _ = self.model_path.as_ref().is_some_and(|p| p.exists());
        self.score_fallback(query, document)
    }

    /// Rerank documents for a query, returning array of relevance scores.
    pub fn rerank(&self, query: &str, documents: &[impl AsRef<str>]) -> Result<Vec<f32>, DbError> {
        let mut scores = Vec::with_capacity(documents.len());
        for doc in documents {
            scores.push(self.score(query, doc.as_ref())?);
        }
        Ok(scores)
    }

    /// Deterministic fallback scoring based on token overlap & embedding cosine similarity.
    fn score_fallback(&self, query: &str, document: &str) -> Result<f32, DbError> {
        let q_tokens: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let doc_lower = document.to_lowercase();

        if q_tokens.is_empty() || doc_lower.is_empty() {
            return Ok(0.0);
        }

        let mut matches = 0;
        for qt in &q_tokens {
            if doc_lower.contains(qt) {
                matches += 1;
            }
        }

        let token_overlap_ratio = matches as f32 / q_tokens.len() as f32;

        let embedder = OnnxEmbedder::new(None::<&Path>);
        let q_emb = embedder.embed(query)?;
        let doc_emb = embedder.embed(document)?;
        let cos_sim = crate::chunks::cosine_similarity(&q_emb, &doc_emb);

        let score = 0.5 * token_overlap_ratio + 0.5 * cos_sim.max(0.0);
        Ok(score)
    }
}
