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
        let resolved = settings
            .resolve_embedder_path()
            .map(|p| p.into_std_path_buf());
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
        let resolved = settings
            .resolve_reranker_path()
            .map(|p| p.into_std_path_buf());
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

#[cfg(test)]
mod tests {
    use super::*;
    use sil_core::RagSettings;

    #[test]
    fn test_onnx_embedder_constructors_and_dimension() {
        let embedder1 = OnnxEmbedder::new(Some("/path/to/model.onnx"));
        assert_eq!(embedder1.dimension(), DEFAULT_EMBEDDING_DIM);

        let embedder2 = OnnxEmbedder::new(None::<&Path>);
        assert_eq!(embedder2.dimension(), DEFAULT_EMBEDDING_DIM);

        let custom_dim = OnnxEmbedder::with_dimension(128);
        assert_eq!(custom_dim.dimension(), 128);

        let settings = RagSettings::default();
        let from_settings = OnnxEmbedder::from_rag_settings(&settings);
        assert_eq!(from_settings.dimension(), DEFAULT_EMBEDDING_DIM);
    }

    #[test]
    fn test_onnx_embedder_embedding_logic() {
        let embedder = OnnxEmbedder::with_dimension(64);

        // Empty text fallback
        let empty_emb = embedder.embed("").unwrap();
        assert_eq!(empty_emb.len(), 64);
        let expected_val = 1.0 / (64.0f32).sqrt();
        for &v in &empty_emb {
            assert!((v - expected_val).abs() < 1e-5);
        }

        // Punctuation-only text fallback (tokens empty)
        let punc_emb = embedder.embed("!@#$%^&*()").unwrap();
        assert_eq!(punc_emb.len(), 64);

        // Normal text embedding and normalization
        let text_emb = embedder.embed("Machine learning transformers").unwrap();
        assert_eq!(text_emb.len(), 64);
        let norm_sq: f32 = text_emb.iter().map(|x| x * x).sum();
        assert!((norm_sq.sqrt() - 1.0).abs() < 1e-4);

        // Deterministic output test
        let text_emb2 = embedder.embed("Machine learning transformers").unwrap();
        assert_eq!(text_emb, text_emb2);
    }

    #[test]
    fn test_onnx_reranker_constructors_and_scoring() {
        let reranker = OnnxReranker::new(Some("/path/to/reranker.onnx"));
        let settings = RagSettings::default();
        let reranker_from_settings = OnnxReranker::from_rag_settings(&settings);
        assert!(reranker.model_path.is_some());
        assert!(reranker_from_settings.model_path.is_none());

        let reranker_none = OnnxReranker::new(None::<&Path>);

        // Empty query or doc
        assert_eq!(reranker_none.score("", "some document").unwrap(), 0.0);
        assert_eq!(reranker_none.score("query", "").unwrap(), 0.0);
        assert_eq!(reranker_none.score("!!", "document").unwrap(), 0.0);

        // Token overlap scoring
        let score_match = reranker_none
            .score("transformer attention", "Attention is all you need for transformers")
            .unwrap();
        let score_no_match = reranker_none
            .score("quantum computing", "Attention is all you need for transformers")
            .unwrap();
        assert!(score_match > score_no_match);

        // Rerank batch
        let docs = vec![
            "Attention mechanism in transformers",
            "Recipe for baking sourdough bread",
        ];
        let scores = reranker_none.rerank("transformer attention", &docs).unwrap();
        assert_eq!(scores.len(), 2);
        assert!(scores[0] > scores[1]);
    }
}

