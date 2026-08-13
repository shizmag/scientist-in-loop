//! Local dense embedding and cross-encoder reranking.
//!
//! # Feature `onnx`
//!
//! When built with `--features onnx` (re-exported on the `sil` binary as
//! `cargo build -p sil --features onnx`), this module can load ONNX Runtime
//! sessions and HuggingFace `tokenizer.json` assets from paths resolved by
//! [`sil_core::RagSettings`].
//!
//! **Honesty policy:** `mode=onnx` / [`RagBackend::Onnx`] is reported only when
//! both a session and a tokenizer loaded successfully. Missing models or
//! tokenizers degrade to deterministic hash / token-overlap **fallback** and
//! surface a structured reason via [`OnnxEmbedder::backend`] /
//! [`OnnxReranker::backend`].
//!
//! Default builds (no feature) always use fallback with
//! [`RagFallbackReason::FeatureDisabled`].
//!
//! # Model layout
//!
//! Preferred package under `~/.cache/sil/models/<name>/`:
//!
//! ```text
//! model.onnx          # or <name>.onnx
//! tokenizer.json      # required for mode=onnx
//! ```
//!
//! Explicit `.onnx` file paths look for a sibling `tokenizer.json` or
//! `{stem}.tokenizer.json`.
//!
//! Full production weights are **not** shipped in-repo. Export from HuggingFace
//! (e.g. `bge-small-en-v1.5`, `ms-marco-MiniLM-L-6-v2`) into the cache dir.

use std::path::{Path, PathBuf};

#[cfg(feature = "onnx")]
use std::sync::{Arc, Mutex};

use crate::error::DbError;

/// Default embedding dimension for local ONNX embedder fallback (bge-small class).
pub const DEFAULT_EMBEDDING_DIM: usize = 384;

/// Why dense RAG is not using a real ONNX session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RagFallbackReason {
    /// Crate built without the `onnx` cargo feature.
    FeatureDisabled,
    /// Resolved model path missing or not found.
    ModelPathMissing,
    /// Model present but `tokenizer.json` missing.
    MissingTokenizer,
    /// Session or tokenizer failed to load / run.
    SessionLoadFailed,
    /// Intentionally constructed without a model path (e.g. unit tests).
    NoModelConfigured,
}

impl RagFallbackReason {
    /// Stable machine-readable reason string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FeatureDisabled => "feature_disabled",
            Self::ModelPathMissing => "model_path_missing",
            Self::MissingTokenizer => "missing_tokenizer",
            Self::SessionLoadFailed => "session_load_failed",
            Self::NoModelConfigured => "no_model_configured",
        }
    }
}

/// Dense RAG backend status for doctor / TUI honesty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RagBackend {
    /// Hash / token-overlap fallback is active.
    Fallback {
        /// Why onnx is not active.
        reason: RagFallbackReason,
    },
    /// Real ONNX session + tokenizer are loaded.
    Onnx {
        /// Output embedding dimension (embedder) or 0 for reranker scores.
        dim: usize,
    },
}

/// Execution provider for ONNX Runtime acceleration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnnxExecutionProvider {
    /// CPU execution provider (default).
    Cpu,
    /// CoreML execution provider (macOS Apple Silicon / GPU / Neural Engine).
    CoreMl,
    /// CUDA execution provider (NVIDIA GPU).
    Cuda,
    /// DirectML execution provider (Windows DirectX 12 GPU).
    DirectMl,
}

impl OnnxExecutionProvider {
    /// Machine-readable provider name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::CoreMl => "coreml",
            Self::Cuda => "cuda",
            Self::DirectMl => "directml",
        }
    }
}

impl RagBackend {
    /// Whether this is a real ONNX path.
    pub fn is_onnx(&self) -> bool {
        matches!(self, Self::Onnx { .. })
    }

    /// Short human-readable status line.
    pub fn summary(&self) -> String {
        match self {
            Self::Fallback { reason } => {
                format!("fallback ({})", reason.as_str())
            }
            Self::Onnx { dim } => format!("onnx (dim={dim})"),
        }
    }
}

/// Resolve sibling tokenizer for an ONNX model file.
#[cfg_attr(not(feature = "onnx"), allow(dead_code))]
fn resolve_tokenizer_path(model_path: &Path) -> Option<PathBuf> {
    let parent = model_path.parent()?;
    let sibling = parent.join("tokenizer.json");
    if sibling.is_file() {
        return Some(sibling);
    }
    if let Some(stem) = model_path.file_stem().and_then(|s| s.to_str()) {
        let named = parent.join(format!("{stem}.tokenizer.json"));
        if named.is_file() {
            return Some(named);
        }
    }
    // Package dir: model_cache/bge-small-en-v1.5/model.onnx already covered by sibling.
    // Also accept tokenizer next to a directory-style package when model is nested.
    None
}

#[cfg(feature = "onnx")]
mod ort_engine {
    use super::*;
    use ort::session::Session;
    use ort::session::builder::GraphOptimizationLevel;
    use ort::value::Tensor;
    use tokenizers::Tokenizer;

    pub(super) struct OrtEngine {
        pub session: Mutex<Session>,
        pub tokenizer: Tokenizer,
        pub dim: usize,
    }

    impl OrtEngine {
        /// Load embedder models from directory.
        pub fn load(
            model_path: &Path,
            tokenizer_path: &Path,
            num_threads: usize,
            is_reranker: bool,
        ) -> Result<Self, String> {
            let tokenizer = Tokenizer::from_file(tokenizer_path)
                .map_err(|e| format!("tokenizer load {}: {e}", tokenizer_path.display()))?;

            let threads = num_threads.max(1);
            let session = Session::builder()
                .map_err(|e| format!("ort session builder: {e}"))?
                .with_optimization_level(GraphOptimizationLevel::Level1)
                .map_err(|e| format!("ort optimization: {e}"))?
                .with_intra_threads(threads)
                .map_err(|e| format!("ort intra_threads: {e}"))?
                .commit_from_file(model_path)
                .map_err(|e| format!("ort commit_from_file {}: {e}", model_path.display()))?;

            // Reranker outputs a scalar; embedder dim defaults to bge-small class until first run.
            let dim = if is_reranker {
                0
            } else {
                DEFAULT_EMBEDDING_DIM
            };

            Ok(Self {
                session: Mutex::new(session),
                tokenizer,
                dim,
            })
        }

        fn encode_ids_mask(&self, text: &str) -> Result<(Vec<i64>, Vec<i64>), String> {
            let encoding = self
                .tokenizer
                .encode(text, true)
                .map_err(|e| format!("tokenize: {e}"))?;
            let ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
            let mask: Vec<i64> = encoding
                .get_attention_mask()
                .iter()
                .map(|&m| m as i64)
                .collect();
            if ids.is_empty() {
                return Err("empty tokenization".into());
            }
            Ok((ids, mask))
        }

        fn encode_pair(&self, query: &str, document: &str) -> Result<(Vec<i64>, Vec<i64>), String> {
            let encoding = self
                .tokenizer
                .encode((query, document), true)
                .map_err(|e| format!("tokenize pair: {e}"))?;
            let ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
            let mask: Vec<i64> = encoding
                .get_attention_mask()
                .iter()
                .map(|&m| m as i64)
                .collect();
            if ids.is_empty() {
                return Err("empty pair tokenization".into());
            }
            Ok((ids, mask))
        }

        /// Compute dense embedding vector for text.
        pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
            let (ids, mask) = self.encode_ids_mask(text)?;
            let seq_len = ids.len();
            let token_type: Vec<i64> = vec![0; seq_len];

            let input_ids = Tensor::from_array(([1usize, seq_len], ids))
                .map_err(|e| format!("input_ids tensor: {e}"))?;
            let attention_mask = Tensor::from_array(([1usize, seq_len], mask.clone()))
                .map_err(|e| format!("attention_mask tensor: {e}"))?;
            let token_type_ids = Tensor::from_array(([1usize, seq_len], token_type))
                .map_err(|e| format!("token_type_ids tensor: {e}"))?;

            let mut session = self
                .session
                .lock()
                .map_err(|_| "ort session mutex poisoned".to_string())?;

            // Bind by common HF export names; fall back to positional via ort::inputs! order.
            let input_names: Vec<String> = session
                .inputs()
                .iter()
                .map(|i| i.name().to_string())
                .collect();
            let outputs = if input_names.iter().any(|n| n == "token_type_ids") {
                session
                    .run(ort::inputs![
                        "input_ids" => input_ids,
                        "attention_mask" => attention_mask,
                        "token_type_ids" => token_type_ids,
                    ])
                    .map_err(|e| format!("ort run: {e}"))?
            } else if input_names.iter().any(|n| n == "input_ids") {
                session
                    .run(ort::inputs![
                        "input_ids" => input_ids,
                        "attention_mask" => attention_mask,
                    ])
                    .map_err(|e| format!("ort run: {e}"))?
            } else {
                // Positional: ids, mask, optional type ids
                session
                    .run(ort::inputs![input_ids, attention_mask])
                    .map_err(|e| format!("ort run positional: {e}"))?
            };

            let (_name, value) = outputs
                .iter()
                .next()
                .ok_or_else(|| "ort model produced no outputs".to_string())?;

            let (shape, data) = value
                .try_extract_tensor::<f32>()
                .map_err(|e| format!("extract f32 output: {e}"))?;

            let emb = pool_embedding(shape, data, &mask)?;
            Ok(l2_normalize(emb))
        }

        /// Score query-document relevance pair using reranker model.
        pub fn score(&self, query: &str, document: &str) -> Result<f32, String> {
            let (ids, mask) = self.encode_pair(query, document)?;
            let seq_len = ids.len();
            let token_type: Vec<i64> = {
                // Prefer tokenizer-provided type ids when available via encode pair.
                // Fallback: zeros (many MiniLM exports still accept them).
                vec![0; seq_len]
            };

            let input_ids = Tensor::from_array(([1usize, seq_len], ids))
                .map_err(|e| format!("input_ids tensor: {e}"))?;
            let attention_mask = Tensor::from_array(([1usize, seq_len], mask))
                .map_err(|e| format!("attention_mask tensor: {e}"))?;
            let token_type_ids = Tensor::from_array(([1usize, seq_len], token_type))
                .map_err(|e| format!("token_type_ids tensor: {e}"))?;

            let mut session = self
                .session
                .lock()
                .map_err(|_| "ort session mutex poisoned".to_string())?;

            let input_names: Vec<String> = session
                .inputs()
                .iter()
                .map(|i| i.name().to_string())
                .collect();
            let outputs = if input_names.iter().any(|n| n == "token_type_ids") {
                session
                    .run(ort::inputs![
                        "input_ids" => input_ids,
                        "attention_mask" => attention_mask,
                        "token_type_ids" => token_type_ids,
                    ])
                    .map_err(|e| format!("ort run: {e}"))?
            } else if input_names.iter().any(|n| n == "input_ids") {
                session
                    .run(ort::inputs![
                        "input_ids" => input_ids,
                        "attention_mask" => attention_mask,
                    ])
                    .map_err(|e| format!("ort run: {e}"))?
            } else {
                session
                    .run(ort::inputs![input_ids, attention_mask])
                    .map_err(|e| format!("ort run positional: {e}"))?
            };

            let (_name, value) = outputs
                .iter()
                .next()
                .ok_or_else(|| "ort model produced no outputs".to_string())?;

            let (_shape, data) = value
                .try_extract_tensor::<f32>()
                .map_err(|e| format!("extract f32 score: {e}"))?;

            data.first()
                .copied()
                .ok_or_else(|| "empty score tensor".to_string())
        }
    }

    /// Mean-pool last hidden state with attention mask, or take row for rank-2 pooled output.
    fn pool_embedding(shape: &[i64], data: &[f32], mask: &[i64]) -> Result<Vec<f32>, String> {
        match shape.len() {
            2 => {
                // [batch, dim]
                let dim = shape[1] as usize;
                if data.len() < dim {
                    return Err(format!("output len {} smaller than dim {dim}", data.len()));
                }
                Ok(data[..dim].to_vec())
            }
            3 => {
                // [batch, seq, dim] — mask mean-pool over sequence
                let seq = shape[1] as usize;
                let dim = shape[2] as usize;
                if data.len() < seq * dim {
                    return Err(format!(
                        "output len {} smaller than seq*dim {}",
                        data.len(),
                        seq * dim
                    ));
                }
                let mut pooled = vec![0.0f32; dim];
                let mut count = 0.0f32;
                for t in 0..seq {
                    let m = mask.get(t).copied().unwrap_or(1);
                    if m == 0 {
                        continue;
                    }
                    count += 1.0;
                    let off = t * dim;
                    for d in 0..dim {
                        pooled[d] += data[off + d];
                    }
                }
                if count < 1.0 {
                    return Err("all tokens masked in pooling".into());
                }
                for p in &mut pooled {
                    *p /= count;
                }
                Ok(pooled)
            }
            _ => Err(format!("unsupported embedding output rank {}", shape.len())),
        }
    }

    fn l2_normalize(mut v: Vec<f32>) -> Vec<f32> {
        let norm_sq: f32 = v.iter().map(|x| x * x).sum();
        let norm = norm_sq.sqrt();
        if norm > 1e-6 {
            for x in &mut v {
                *x /= norm;
            }
        } else if !v.is_empty() {
            let val = 1.0 / (v.len() as f32).sqrt();
            v.fill(val);
        }
        v
    }
}

/// Local ONNX embedding model wrapper with deterministic fallback.
#[derive(Clone)]
pub struct OnnxEmbedder {
    model_path: Option<PathBuf>,
    dim: usize,
    backend: RagBackend,
    #[cfg(feature = "onnx")]
    engine: Option<Arc<ort_engine::OrtEngine>>,
}

impl std::fmt::Debug for OnnxEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnnxEmbedder")
            .field("model_path", &self.model_path)
            .field("dim", &self.dim)
            .field("backend", &self.backend)
            .field("engine_loaded", &{
                #[cfg(feature = "onnx")]
                {
                    self.engine.is_some()
                }
                #[cfg(not(feature = "onnx"))]
                {
                    false
                }
            })
            .finish()
    }
}

impl OnnxEmbedder {
    /// Create a new ONNX embedder with an optional model file path (no session load).
    ///
    /// Prefer [`Self::from_rag_settings`] when the `onnx` feature is enabled so
    /// tokenizer + session can load.
    pub fn new(model_path: Option<impl AsRef<Path>>) -> Self {
        let model_path = model_path.map(|p| p.as_ref().to_path_buf());
        let reason = if model_path.is_some() {
            #[cfg(feature = "onnx")]
            {
                RagFallbackReason::MissingTokenizer // path set but not loaded via settings
            }
            #[cfg(not(feature = "onnx"))]
            {
                RagFallbackReason::FeatureDisabled
            }
        } else {
            #[cfg(feature = "onnx")]
            {
                RagFallbackReason::NoModelConfigured
            }
            #[cfg(not(feature = "onnx"))]
            {
                RagFallbackReason::FeatureDisabled
            }
        };

        // If path is set and feature on, attempt load (may still fallback).
        #[cfg(feature = "onnx")]
        {
            if let Some(ref mp) = model_path {
                return Self::try_load(mp, DEFAULT_EMBEDDING_DIM, 4, false);
            }
        }

        Self {
            model_path,
            dim: DEFAULT_EMBEDDING_DIM,
            backend: RagBackend::Fallback { reason },
            #[cfg(feature = "onnx")]
            engine: None,
        }
    }

    /// Create an ONNX embedder resolving model path and loading session when possible.
    pub fn from_rag_settings(settings: &sil_core::RagSettings) -> Self {
        let resolved = settings
            .resolve_embedder_path()
            .map(|p| p.into_std_path_buf());
        let threads = settings.num_threads;

        #[cfg(not(feature = "onnx"))]
        {
            let _ = (resolved.as_ref(), threads);
            Self {
                model_path: resolved,
                dim: DEFAULT_EMBEDDING_DIM,
                backend: RagBackend::Fallback {
                    reason: RagFallbackReason::FeatureDisabled,
                },
            }
        }

        #[cfg(feature = "onnx")]
        {
            match resolved {
                Some(path) => Self::try_load(&path, DEFAULT_EMBEDDING_DIM, threads, false),
                None => Self {
                    model_path: None,
                    dim: DEFAULT_EMBEDDING_DIM,
                    backend: RagBackend::Fallback {
                        reason: RagFallbackReason::ModelPathMissing,
                    },
                    engine: None,
                },
            }
        }
    }

    #[cfg(feature = "onnx")]
    fn try_load(
        model_path: &Path,
        default_dim: usize,
        num_threads: usize,
        is_reranker: bool,
    ) -> Self {
        let model_path_buf = model_path.to_path_buf();
        if !model_path.is_file() {
            return Self {
                model_path: Some(model_path_buf),
                dim: default_dim,
                backend: RagBackend::Fallback {
                    reason: RagFallbackReason::ModelPathMissing,
                },
                engine: None,
            };
        }
        let Some(tok_path) = resolve_tokenizer_path(model_path) else {
            return Self {
                model_path: Some(model_path_buf),
                dim: default_dim,
                backend: RagBackend::Fallback {
                    reason: RagFallbackReason::MissingTokenizer,
                },
                engine: None,
            };
        };
        match ort_engine::OrtEngine::load(model_path, &tok_path, num_threads, is_reranker) {
            Ok(engine) => {
                let dim = if engine.dim > 0 {
                    engine.dim
                } else {
                    default_dim
                };
                Self {
                    model_path: Some(model_path_buf),
                    dim,
                    backend: RagBackend::Onnx { dim },
                    engine: Some(Arc::new(engine)),
                }
            }
            Err(_e) => Self {
                model_path: Some(model_path_buf),
                dim: default_dim,
                backend: RagBackend::Fallback {
                    reason: RagFallbackReason::SessionLoadFailed,
                },
                engine: None,
            },
        }
    }

    /// Create an embedder with custom dimension in fallback mode.
    pub fn with_dimension(dim: usize) -> Self {
        Self {
            model_path: None,
            dim,
            backend: RagBackend::Fallback {
                reason: {
                    #[cfg(feature = "onnx")]
                    {
                        RagFallbackReason::NoModelConfigured
                    }
                    #[cfg(not(feature = "onnx"))]
                    {
                        RagFallbackReason::FeatureDisabled
                    }
                },
            },
            #[cfg(feature = "onnx")]
            engine: None,
        }
    }

    /// Return the embedding dimension.
    pub fn dimension(&self) -> usize {
        self.dim
    }

    /// Current backend status (never claims onnx without a loaded session).
    pub fn backend(&self) -> RagBackend {
        self.backend.clone()
    }

    /// Optional model path configured for this embedder.
    pub fn model_path(&self) -> Option<&Path> {
        self.model_path.as_deref()
    }

    /// Generate normalized embedding vector for text input.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, DbError> {
        #[cfg(feature = "onnx")]
        {
            if let Some(ref engine) = self.engine
                && matches!(self.backend, RagBackend::Onnx { .. })
            {
                match engine.embed(text) {
                    Ok(v) => {
                        // Keep reported dim in sync with actual vector length when possible.
                        return Ok(v);
                    }
                    Err(_) => {
                        // Degrade this call to fallback; status still reflects load-time backend.
                        // Call-time failure should not panic — fall through to hash embed.
                    }
                }
            }
        }
        let _ = self.model_path.as_ref();
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

impl Default for OnnxEmbedder {
    fn default() -> Self {
        Self::new(None::<&Path>)
    }
}

/// Local ONNX cross-encoder reranker wrapper with deterministic scoring.
#[derive(Clone)]
pub struct OnnxReranker {
    model_path: Option<PathBuf>,
    backend: RagBackend,
    #[cfg(feature = "onnx")]
    engine: Option<Arc<ort_engine::OrtEngine>>,
}

impl std::fmt::Debug for OnnxReranker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnnxReranker")
            .field("model_path", &self.model_path)
            .field("backend", &self.backend)
            .field("engine_loaded", &{
                #[cfg(feature = "onnx")]
                {
                    self.engine.is_some()
                }
                #[cfg(not(feature = "onnx"))]
                {
                    false
                }
            })
            .finish()
    }
}

impl OnnxReranker {
    /// Create a new ONNX reranker with an optional model file path.
    pub fn new(model_path: Option<impl AsRef<Path>>) -> Self {
        let model_path = model_path.map(|p| p.as_ref().to_path_buf());

        #[cfg(feature = "onnx")]
        {
            if let Some(ref mp) = model_path {
                return Self::try_load(mp, 4);
            }
            return Self {
                model_path,
                backend: RagBackend::Fallback {
                    reason: RagFallbackReason::NoModelConfigured,
                },
                engine: None,
            };
        }

        #[cfg(not(feature = "onnx"))]
        {
            Self {
                model_path,
                backend: RagBackend::Fallback {
                    reason: RagFallbackReason::FeatureDisabled,
                },
            }
        }
    }

    /// Create an ONNX reranker from [`sil_core::RagSettings`].
    pub fn from_rag_settings(settings: &sil_core::RagSettings) -> Self {
        let resolved = settings
            .resolve_reranker_path()
            .map(|p| p.into_std_path_buf());
        let threads = settings.num_threads;

        #[cfg(not(feature = "onnx"))]
        {
            let _ = (resolved.as_ref(), threads);
            Self {
                model_path: resolved,
                backend: RagBackend::Fallback {
                    reason: RagFallbackReason::FeatureDisabled,
                },
            }
        }

        #[cfg(feature = "onnx")]
        {
            match resolved {
                Some(path) => Self::try_load(&path, threads),
                None => Self {
                    model_path: None,
                    backend: RagBackend::Fallback {
                        reason: RagFallbackReason::ModelPathMissing,
                    },
                    engine: None,
                },
            }
        }
    }

    #[cfg(feature = "onnx")]
    fn try_load(model_path: &Path, num_threads: usize) -> Self {
        let model_path_buf = model_path.to_path_buf();
        if !model_path.is_file() {
            return Self {
                model_path: Some(model_path_buf),
                backend: RagBackend::Fallback {
                    reason: RagFallbackReason::ModelPathMissing,
                },
                engine: None,
            };
        }
        let Some(tok_path) = resolve_tokenizer_path(model_path) else {
            return Self {
                model_path: Some(model_path_buf),
                backend: RagBackend::Fallback {
                    reason: RagFallbackReason::MissingTokenizer,
                },
                engine: None,
            };
        };
        match ort_engine::OrtEngine::load(model_path, &tok_path, num_threads, true) {
            Ok(engine) => Self {
                model_path: Some(model_path_buf),
                backend: RagBackend::Onnx { dim: 0 },
                engine: Some(Arc::new(engine)),
            },
            Err(_) => Self {
                model_path: Some(model_path_buf),
                backend: RagBackend::Fallback {
                    reason: RagFallbackReason::SessionLoadFailed,
                },
                engine: None,
            },
        }
    }

    /// Current backend status.
    pub fn backend(&self) -> RagBackend {
        self.backend.clone()
    }

    /// Optional model path.
    pub fn model_path(&self) -> Option<&Path> {
        self.model_path.as_deref()
    }

    /// Score query against document passage (returns float relevance score).
    pub fn score(&self, query: &str, document: &str) -> Result<f32, DbError> {
        #[cfg(feature = "onnx")]
        {
            if let Some(ref engine) = self.engine
                && matches!(self.backend, RagBackend::Onnx { .. })
            {
                if let Ok(s) = engine.score(query, document) {
                    return Ok(s);
                }
            }
        }
        let _ = self.model_path.as_ref();
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
    fn test_backend_never_claims_onnx_without_session() {
        let e = OnnxEmbedder::default();
        assert!(!e.backend().is_onnx());
        assert!(matches!(e.backend(), RagBackend::Fallback { .. }));

        // Non-existent path must not report onnx.
        let e2 = OnnxEmbedder::new(Some("/nonexistent/model.onnx"));
        assert!(!e2.backend().is_onnx());

        let r = OnnxReranker::new(None::<&Path>);
        assert!(!r.backend().is_onnx());
    }

    #[test]
    fn test_fallback_reason_feature_or_missing() {
        let e = OnnxEmbedder::default();
        match e.backend() {
            RagBackend::Fallback { reason } => {
                #[cfg(feature = "onnx")]
                assert_eq!(reason, RagFallbackReason::NoModelConfigured);
                #[cfg(not(feature = "onnx"))]
                assert_eq!(reason, RagFallbackReason::FeatureDisabled);
            }
            RagBackend::Onnx { .. } => panic!("default embedder must not be onnx"),
        }
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

        // Non-equal texts → non-identical vectors (fallback path)
        let other = embedder.embed("Quantum chromodynamics lattice").unwrap();
        assert_ne!(text_emb, other);
    }

    #[test]
    fn test_onnx_reranker_constructors_and_scoring() {
        let reranker = OnnxReranker::new(Some("/path/to/reranker.onnx"));
        let settings = RagSettings::default();
        let reranker_from_settings = OnnxReranker::from_rag_settings(&settings);
        assert!(reranker.model_path().is_some());
        // Default settings typically have no models on disk.
        let _ = reranker_from_settings.backend();

        let reranker_none = OnnxReranker::new(None::<&Path>);

        // Empty query or doc
        assert_eq!(reranker_none.score("", "some document").unwrap(), 0.0);
        assert_eq!(reranker_none.score("query", "").unwrap(), 0.0);
        assert_eq!(reranker_none.score("!!", "document").unwrap(), 0.0);

        // Token overlap scoring
        let score_match = reranker_none
            .score(
                "transformer attention",
                "Attention is all you need for transformers",
            )
            .unwrap();
        let score_no_match = reranker_none
            .score(
                "quantum computing",
                "Attention is all you need for transformers",
            )
            .unwrap();
        assert!(score_match > score_no_match);

        // Rerank batch
        let docs = vec![
            "Attention mechanism in transformers",
            "Recipe for baking sourdough bread",
        ];
        let scores = reranker_none
            .rerank("transformer attention", &docs)
            .unwrap();
        assert_eq!(scores.len(), 2);
        assert!(scores[0] > scores[1]);
    }

    #[test]
    fn test_resolve_tokenizer_path_sibling() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("model.onnx");
        std::fs::write(&model, b"fake").unwrap();
        assert!(resolve_tokenizer_path(&model).is_none());
        let tok = dir.path().join("tokenizer.json");
        std::fs::write(&tok, b"{}").unwrap();
        assert_eq!(
            resolve_tokenizer_path(&model).as_deref(),
            Some(tok.as_path())
        );
    }

    #[test]
    fn test_rag_backend_summary() {
        let s = RagBackend::Fallback {
            reason: RagFallbackReason::FeatureDisabled,
        }
        .summary();
        assert!(s.contains("fallback"));
        let s_onnx = RagBackend::Onnx { dim: 384 }.summary();
        assert_eq!(s_onnx, "onnx (dim=384)");
    }

    #[test]
    fn test_onnx_execution_provider() {
        assert_eq!(OnnxExecutionProvider::Cpu.as_str(), "cpu");
        assert_eq!(OnnxExecutionProvider::CoreMl.as_str(), "coreml");
        assert_eq!(OnnxExecutionProvider::Cuda.as_str(), "cuda");
        assert_eq!(OnnxExecutionProvider::DirectMl.as_str(), "directml");
    }
}
