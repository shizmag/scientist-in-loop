//! Global and local settings data structures and cache management.

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::SilError;

/// Author details (used for primary author and co-authors).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AuthorDetails {
    /// Full name of the author.
    pub name: String,
    /// Contact email address.
    #[serde(default)]
    pub email: String,
    /// Academic or organizational affiliation.
    #[serde(default)]
    pub affiliation: String,
    /// ORCID persistent digital identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orcid: Option<String>,
}

/// Grant requisites details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GrantDetails {
    /// Funding organization or agency name.
    pub funder: String,
    /// Grant reference number or identifier.
    #[serde(default)]
    pub grant_number: String,
    /// Standard acknowledgment prose fragment.
    #[serde(default)]
    pub acknowledgment: String,
}

/// Global settings stored in user's config directory (`~/.config/sil/settings.yaml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalSettings {
    /// Primary author details.
    #[serde(default)]
    pub author: AuthorDetails,
    /// Default grant requisites applied to new articles.
    #[serde(default)]
    pub default_grant: GrantDetails,
    /// Default LaTeX compilation engine (e.g. tectonic).
    #[serde(default = "default_engine")]
    pub default_latex_engine: String,
    /// Default target publication template (e.g. standard, neurips).
    #[serde(default = "default_template")]
    pub default_template: String,
    /// Custom metadata key-value pairs needed across articles.
    #[serde(default)]
    pub custom_fields: BTreeMap<String, String>,
    /// Global RAG settings.
    #[serde(default)]
    pub rag: RagSettings,
    /// Recently opened sil project paths (up to 20).
    #[serde(default)]
    pub recent_projects: Vec<Utf8PathBuf>,
}

fn default_engine() -> String {
    "tectonic".to_string()
}

fn default_template() -> String {
    "standard".to_string()
}

fn default_onnx_embedder_model() -> String {
    "bge-small-en-v1.5".to_string()
}

fn default_onnx_reranker_model() -> String {
    "ms-marco-MiniLM-L-6-v2".to_string()
}

fn default_model_cache_dir() -> Utf8PathBuf {
    dirs::home_dir()
        .and_then(|h| Utf8PathBuf::from_path_buf(h.join(".cache").join("sil").join("models")).ok())
        .unwrap_or_else(|| Utf8PathBuf::from("~/.cache/sil/models"))
}

fn default_xberg_model_cache_dir() -> Utf8PathBuf {
    dirs::home_dir()
        .and_then(|h| Utf8PathBuf::from_path_buf(h.join(".cache").join("sil").join("xberg")).ok())
        .unwrap_or_else(|| Utf8PathBuf::from("~/.cache/sil/xberg"))
}

fn default_execution_provider() -> String {
    "cpu".to_string()
}

fn default_num_threads() -> usize {
    4
}

fn default_parent_chunk_size() -> usize {
    1200
}

fn default_child_chunk_size() -> usize {
    300
}

/// RAG (Retrieval-Augmented Generation) settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RagSettings {
    /// ONNX embedding model name/identifier.
    #[serde(default = "default_onnx_embedder_model")]
    pub onnx_embedder_model: String,
    /// ONNX reranker model name/identifier.
    #[serde(default = "default_onnx_reranker_model")]
    pub onnx_reranker_model: String,
    /// Cache directory for storing model artifacts.
    #[serde(default = "default_model_cache_dir")]
    pub model_cache_dir: Utf8PathBuf,
    /// Cache directory for storing xberg model artifacts.
    #[serde(default = "default_xberg_model_cache_dir")]
    pub xberg_model_cache_dir: Utf8PathBuf,
    /// Custom directory path containing *.onnx models
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub onnx_models_dir: Option<Utf8PathBuf>,
    /// Explicit file path to specific embedder *.onnx model file
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub onnx_embedder_path: Option<Utf8PathBuf>,
    /// Explicit file path to specific reranker *.onnx model file
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub onnx_reranker_path: Option<Utf8PathBuf>,
    /// Execution provider for ONNX runtime (e.g. cpu, cuda).
    #[serde(default = "default_execution_provider")]
    pub execution_provider: String,
    /// Number of threads for ONNX inference execution.
    #[serde(default = "default_num_threads")]
    pub num_threads: usize,
    /// Parent chunk size for text chunking.
    #[serde(default = "default_parent_chunk_size")]
    pub parent_chunk_size: usize,
    /// Child chunk size for text chunking.
    #[serde(default = "default_child_chunk_size")]
    pub child_chunk_size: usize,
}

impl Default for RagSettings {
    fn default() -> Self {
        Self {
            onnx_embedder_model: default_onnx_embedder_model(),
            onnx_reranker_model: default_onnx_reranker_model(),
            model_cache_dir: default_model_cache_dir(),
            xberg_model_cache_dir: default_xberg_model_cache_dir(),
            onnx_models_dir: None,
            onnx_embedder_path: None,
            onnx_reranker_path: None,
            execution_provider: default_execution_provider(),
            num_threads: default_num_threads(),
            parent_chunk_size: default_parent_chunk_size(),
            child_chunk_size: default_child_chunk_size(),
        }
    }
}

fn find_onnx_in_path(path: &Utf8Path) -> Option<Utf8PathBuf> {
    if !path.exists() {
        return None;
    }
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    if path.is_dir()
        && let Ok(entries) = std::fs::read_dir(path)
    {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.extension().and_then(|s| s.to_str()) == Some("onnx")
                && let Ok(utf8) = Utf8PathBuf::from_path_buf(entry_path)
            {
                return Some(utf8);
            }
        }
    }
    None
}

impl RagSettings {
    /// Resolve exact embedder *.onnx model file path based on config precedence.
    pub fn resolve_embedder_path(&self) -> Option<Utf8PathBuf> {
        if let Some(ref path) = self.onnx_embedder_path
            && let Some(found) = find_onnx_in_path(path)
        {
            return Some(found);
        }
        if let Some(ref dir) = self.onnx_models_dir {
            let candidate1 = dir.join(format!("{}.onnx", self.onnx_embedder_model));
            if candidate1.is_file() && candidate1.exists() {
                return Some(candidate1);
            }
            let candidate2 = dir.join("embedder.onnx");
            if candidate2.is_file() && candidate2.exists() {
                return Some(candidate2);
            }
            if let Some(found) = find_onnx_in_path(dir) {
                return Some(found);
            }
        }
        let cache_candidate = self
            .model_cache_dir
            .join(format!("{}.onnx", self.onnx_embedder_model));
        if cache_candidate.is_file() && cache_candidate.exists() {
            return Some(cache_candidate);
        }
        find_onnx_in_path(&self.model_cache_dir)
    }

    /// Resolve exact reranker *.onnx model file path based on config precedence.
    pub fn resolve_reranker_path(&self) -> Option<Utf8PathBuf> {
        if let Some(ref path) = self.onnx_reranker_path
            && let Some(found) = find_onnx_in_path(path)
        {
            return Some(found);
        }
        if let Some(ref dir) = self.onnx_models_dir {
            let candidate1 = dir.join(format!("{}.onnx", self.onnx_reranker_model));
            if candidate1.is_file() && candidate1.exists() {
                return Some(candidate1);
            }
            let candidate2 = dir.join("reranker.onnx");
            if candidate2.is_file() && candidate2.exists() {
                return Some(candidate2);
            }
            if let Some(found) = find_onnx_in_path(dir) {
                return Some(found);
            }
        }
        let cache_candidate = self
            .model_cache_dir
            .join(format!("{}.onnx", self.onnx_reranker_model));
        if cache_candidate.is_file() && cache_candidate.exists() {
            return Some(cache_candidate);
        }
        find_onnx_in_path(&self.model_cache_dir)
    }
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            author: AuthorDetails::default(),
            default_grant: GrantDetails::default(),
            default_latex_engine: default_engine(),
            default_template: default_template(),
            custom_fields: BTreeMap::new(),
            rag: RagSettings::default(),
            recent_projects: Vec::new(),
        }
    }
}

/// Local settings embedded in project `config.yaml` or `.sil/config.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LocalSettings {
    /// Article title.
    #[serde(default)]
    pub title: String,
    /// List of co-authors for this specific work.
    #[serde(default)]
    pub co_authors: Vec<AuthorDetails>,
    /// List of grants associated with this specific work.
    #[serde(default)]
    pub grants: Vec<GrantDetails>,
    /// Project notes or specific work details.
    #[serde(default)]
    pub notes: String,
}

/// Global cache for remembering previously used co-authors and grant requisites.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SettingsCache {
    /// History of co-authors added across previous works.
    #[serde(default)]
    pub co_authors: Vec<AuthorDetails>,
    /// History of grants added across previous works.
    #[serde(default)]
    pub grants: Vec<GrantDetails>,
}

impl SettingsCache {
    /// Add or update a co-author in cache (deduplicating by name/email).
    pub fn remember_co_author(&mut self, author: AuthorDetails) {
        if author.name.trim().is_empty() {
            return;
        }
        if let Some(pos) = self.co_authors.iter().position(|a| {
            (!a.email.is_empty() && a.email.eq_ignore_ascii_case(&author.email))
                || a.name.eq_ignore_ascii_case(&author.name)
        }) {
            self.co_authors[pos] = author;
        } else {
            self.co_authors.push(author);
        }
    }

    /// Add or update a grant in cache (deduplicating by funder/grant_number).
    pub fn remember_grant(&mut self, grant: GrantDetails) {
        if grant.funder.trim().is_empty() && grant.grant_number.trim().is_empty() {
            return;
        }
        if let Some(pos) = self.grants.iter().position(|g| {
            (!g.grant_number.is_empty() && g.grant_number.eq_ignore_ascii_case(&grant.grant_number))
                || (!g.funder.is_empty() && g.funder.eq_ignore_ascii_case(&grant.funder))
        }) {
            self.grants[pos] = grant;
        } else {
            self.grants.push(grant);
        }
    }
}

/// Return standard path for global settings file (`~/.config/sil/settings.yaml`).
pub fn default_global_settings_path() -> Option<Utf8PathBuf> {
    dirs::config_dir()
        .and_then(|p| Utf8PathBuf::from_path_buf(p.join("sil").join("settings.yaml")).ok())
}

/// Return standard path for settings cache file (`~/.config/sil/cache.yaml`).
pub fn default_settings_cache_path() -> Option<Utf8PathBuf> {
    dirs::config_dir()
        .and_then(|p| Utf8PathBuf::from_path_buf(p.join("sil").join("cache.yaml")).ok())
}

impl GlobalSettings {
    /// Load global settings from path or default path, returning default if non-existent.
    pub fn load_or_default(path: Option<&Utf8Path>) -> Self {
        let p = path
            .map(|p| p.to_path_buf())
            .or_else(default_global_settings_path);
        p.filter(|path| path.exists())
            .and_then(|path| std::fs::read_to_string(path.as_std_path()).ok())
            .and_then(|text| serde_yaml::from_str::<GlobalSettings>(&text).ok())
            .unwrap_or_default()
    }

    /// Save global settings to specified path or default user config path.
    pub fn save(&self, path: Option<&Utf8Path>) -> Result<(), SilError> {
        let target_path = path
            .map(|p| p.to_path_buf())
            .or_else(default_global_settings_path)
            .ok_or_else(|| {
                SilError::Message("cannot resolve global config directory".to_string())
            })?;

        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent.as_std_path())?;
        }

        let yaml = serde_yaml::to_string(self)
            .map_err(|e| SilError::Message(format!("failed to serialize global settings: {e}")))?;

        crate::atomic::write_atomic_str(&target_path, &yaml)?;
        Ok(())
    }

    /// Add a project path to recent projects list (deduplicating and capping at 20).
    pub fn touch_recent_project(&mut self, path: Utf8PathBuf) {
        self.recent_projects.retain(|p| p != &path);
        self.recent_projects.insert(0, path);
        if self.recent_projects.len() > 20 {
            self.recent_projects.truncate(20);
        }
    }
}

impl SettingsCache {
    /// Load cache from path or default path, returning default if non-existent.
    pub fn load_or_default(path: Option<&Utf8Path>) -> Self {
        let p = path
            .map(|p| p.to_path_buf())
            .or_else(default_settings_cache_path);
        p.filter(|path| path.exists())
            .and_then(|path| std::fs::read_to_string(path.as_std_path()).ok())
            .and_then(|text| serde_yaml::from_str::<SettingsCache>(&text).ok())
            .unwrap_or_default()
    }

    /// Save cache to specified path or default user cache path.
    pub fn save(&self, path: Option<&Utf8Path>) -> Result<(), SilError> {
        let target_path = path
            .map(|p| p.to_path_buf())
            .or_else(default_settings_cache_path)
            .ok_or_else(|| {
                SilError::Message("cannot resolve global config directory".to_string())
            })?;

        let yaml = serde_yaml::to_string(self)
            .map_err(|e| SilError::Message(format!("failed to serialize settings cache: {e}")))?;

        crate::atomic::write_atomic_str(&target_path, &yaml)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn global_settings_roundtrip() {
        let dir = tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(dir.path().join("settings.yaml")).unwrap();

        let mut gs = GlobalSettings::default();
        gs.author.name = "Alice".to_string();
        gs.author.email = "alice@example.com".to_string();
        gs.default_grant.funder = "NSF".to_string();
        gs.default_grant.grant_number = "12345".to_string();

        gs.save(Some(&path)).unwrap();
        let loaded = GlobalSettings::load_or_default(Some(&path));

        assert_eq!(loaded.author.name, "Alice");
        assert_eq!(loaded.default_grant.funder, "NSF");
    }

    #[test]
    fn cache_deduplication() {
        let mut cache = SettingsCache::default();
        cache.remember_co_author(AuthorDetails {
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
            affiliation: "MIT".to_string(),
            orcid: None,
        });

        assert_eq!(cache.co_authors.len(), 1);

        // Updating Bob with new affiliation should replace, not duplicate
        cache.remember_co_author(AuthorDetails {
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
            affiliation: "Stanford".to_string(),
            orcid: Some("0000-0001".to_string()),
        });

        assert_eq!(cache.co_authors.len(), 1);
        assert_eq!(cache.co_authors[0].affiliation, "Stanford");
        assert_eq!(cache.co_authors[0].orcid, Some("0000-0001".to_string()));

        // Adding Carol adds a second entry
        cache.remember_co_author(AuthorDetails {
            name: "Carol".to_string(),
            email: "carol@example.com".to_string(),
            affiliation: "Harvard".to_string(),
            orcid: None,
        });

        assert_eq!(cache.co_authors.len(), 2);
    }

    #[test]
    fn rag_settings_defaults() {
        let rag = RagSettings::default();
        assert_eq!(rag.onnx_embedder_model, "bge-small-en-v1.5");
        assert_eq!(rag.onnx_reranker_model, "ms-marco-MiniLM-L-6-v2");
        assert!(rag.model_cache_dir.as_str().ends_with("sil/models"));
        assert_eq!(rag.execution_provider, "cpu");
        assert_eq!(rag.num_threads, 4);
        assert_eq!(rag.parent_chunk_size, 1200);
        assert_eq!(rag.child_chunk_size, 300);
    }

    #[test]
    fn global_settings_rag_roundtrip() {
        let dir = tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(dir.path().join("settings.yaml")).unwrap();

        let mut gs = GlobalSettings::default();
        gs.rag.onnx_embedder_model = "custom-embedder".to_string();
        gs.rag.num_threads = 8;

        gs.save(Some(&path)).unwrap();
        let loaded = GlobalSettings::load_or_default(Some(&path));

        assert_eq!(loaded.rag.onnx_embedder_model, "custom-embedder");
        assert_eq!(loaded.rag.num_threads, 8);
    }

    #[test]
    fn global_settings_deserializes_missing_rag() {
        let yaml = "default_latex_engine: tectonic\n";
        let gs: GlobalSettings = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(gs.rag, RagSettings::default());
    }

    #[test]
    fn test_custom_onnx_path_resolution() {
        let dir = tempdir().unwrap();
        let dir_path = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let model_file = dir_path.join("custom-embedder.onnx");
        std::fs::write(model_file.as_std_path(), b"fake onnx").unwrap();

        let rag = RagSettings {
            onnx_models_dir: Some(dir_path),
            onnx_embedder_model: "custom-embedder".to_string(),
            ..Default::default()
        };

        let resolved = rag.resolve_embedder_path();
        assert_eq!(resolved, Some(model_file));
    }

    #[test]
    fn test_dir_onnx_path_resolution() {
        let dir = tempdir().unwrap();
        let dir_path = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let model_file = dir_path.join("model.onnx");
        std::fs::write(model_file.as_std_path(), b"fake onnx").unwrap();

        let rag = RagSettings {
            onnx_embedder_path: Some(dir_path.clone()),
            ..Default::default()
        };

        let resolved = rag.resolve_embedder_path();
        assert_eq!(resolved, Some(model_file));
    }

    #[test]
    fn grant_cache_deduplication() {
        let mut cache = SettingsCache::default();

        // Empty grant should be ignored
        cache.remember_grant(GrantDetails::default());
        assert!(cache.grants.is_empty());

        let g1 = GrantDetails {
            funder: "NSF".to_string(),
            grant_number: "123".to_string(),
            acknowledgment: "Ack 1".to_string(),
        };
        cache.remember_grant(g1);
        assert_eq!(cache.grants.len(), 1);

        // Update grant matching grant number
        let g2 = GrantDetails {
            funder: "National Science Foundation".to_string(),
            grant_number: "123".to_string(),
            acknowledgment: "Ack 2".to_string(),
        };
        cache.remember_grant(g2);
        assert_eq!(cache.grants.len(), 1);
        assert_eq!(cache.grants[0].funder, "National Science Foundation");

        // Update grant matching funder
        let g3 = GrantDetails {
            funder: "national science foundation".to_string(),
            grant_number: "456".to_string(),
            acknowledgment: "Ack 3".to_string(),
        };
        cache.remember_grant(g3);
        assert_eq!(cache.grants.len(), 1);
        assert_eq!(cache.grants[0].grant_number, "456");

        // Add new grant
        let g4 = GrantDetails {
            funder: "NIH".to_string(),
            grant_number: "789".to_string(),
            acknowledgment: "Ack 4".to_string(),
        };
        cache.remember_grant(g4);
        assert_eq!(cache.grants.len(), 2);
    }

    #[test]
    fn author_cache_edge_cases() {
        let mut cache = SettingsCache::default();
        // Empty author name ignored
        cache.remember_co_author(AuthorDetails::default());
        assert!(cache.co_authors.is_empty());

        let a1 = AuthorDetails {
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            affiliation: "Uni A".to_string(),
            orcid: None,
        };
        cache.remember_co_author(a1);

        // Update author matching name case-insensitively
        let a2 = AuthorDetails {
            name: "ALICE".to_string(),
            email: "new_email@example.com".to_string(),
            affiliation: "Uni B".to_string(),
            orcid: Some("0000".to_string()),
        };
        cache.remember_co_author(a2);
        assert_eq!(cache.co_authors.len(), 1);
        assert_eq!(cache.co_authors[0].email, "new_email@example.com");
    }

    #[test]
    fn local_settings_and_author_grant_defaults() {
        let author = AuthorDetails::default();
        assert_eq!(author.name, "");
        assert_eq!(author.email, "");

        let grant = GrantDetails::default();
        assert_eq!(grant.funder, "");

        let local = LocalSettings::default();
        assert_eq!(local.title, "");
        assert!(local.co_authors.is_empty());
        assert!(local.grants.is_empty());

        let yaml = serde_yaml::to_string(&local).unwrap();
        let de: LocalSettings = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(local, de);
    }

    #[test]
    fn settings_cache_save_load() {
        let dir = tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(dir.path().join("cache.yaml")).unwrap();

        let mut cache = SettingsCache::default();
        cache.remember_co_author(AuthorDetails {
            name: "Bob".to_string(),
            ..Default::default()
        });

        cache.save(Some(&path)).unwrap();
        let loaded = SettingsCache::load_or_default(Some(&path));
        assert_eq!(loaded.co_authors.len(), 1);
        assert_eq!(loaded.co_authors[0].name, "Bob");

        // Non-existent path returns default
        let empty_path = Utf8PathBuf::from_path_buf(dir.path().join("non_existent.yaml")).unwrap();
        let loaded_empty = SettingsCache::load_or_default(Some(&empty_path));
        assert!(loaded_empty.co_authors.is_empty());
    }

    #[test]
    fn global_settings_load_or_default_none() {
        let gs = GlobalSettings::load_or_default(None);
        assert_eq!(gs.default_latex_engine, "tectonic");
        assert_eq!(gs.default_template, "standard");

        let cache = SettingsCache::load_or_default(None);
        let _ = cache;
    }

    #[test]
    fn default_paths_resolution() {
        let _p1 = default_global_settings_path();
        let _p2 = default_settings_cache_path();
    }

    #[test]
    fn resolve_reranker_path_precedence() {
        let dir = tempdir().unwrap();
        let dir_path = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();

        // 1. Direct path override
        let explicit_file = dir_path.join("explicit.onnx");
        std::fs::write(explicit_file.as_std_path(), b"onnx").unwrap();
        let mut rag = RagSettings {
            onnx_reranker_path: Some(explicit_file.clone()),
            ..Default::default()
        };
        assert_eq!(rag.resolve_reranker_path(), Some(explicit_file));

        // 2. Non-existent explicit path falls back
        rag.onnx_reranker_path = Some(dir_path.join("non_existent.onnx"));

        // Models dir with reranker.onnx
        let models_dir = dir_path.join("models_dir");
        std::fs::create_dir_all(models_dir.as_std_path()).unwrap();
        let reranker_file = models_dir.join("reranker.onnx");
        std::fs::write(reranker_file.as_std_path(), b"onnx").unwrap();

        rag.onnx_models_dir = Some(models_dir);
        assert_eq!(rag.resolve_reranker_path(), Some(reranker_file));

        // 3. Fallback to None when nothing found
        let empty_rag = RagSettings {
            onnx_reranker_path: None,
            onnx_models_dir: None,
            model_cache_dir: Utf8PathBuf::from("/non/existent/cache/dir"),
            ..Default::default()
        };
        assert_eq!(empty_rag.resolve_reranker_path(), None);
        assert_eq!(empty_rag.resolve_embedder_path(), None);
    }
}
