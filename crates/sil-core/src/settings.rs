//! Global and local settings data structures and cache management.

use std::collections::BTreeMap;
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

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
}

fn default_engine() -> String {
    "tectonic".to_string()
}

fn default_template() -> String {
    "standard".to_string()
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            author: AuthorDetails::default(),
            default_grant: GrantDetails::default(),
            default_latex_engine: default_engine(),
            default_template: default_template(),
            custom_fields: BTreeMap::new(),
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
    dirs::config_dir().map(|p| Utf8PathBuf::from_path_buf(p.join("sil").join("settings.yaml")).ok()).flatten()
}

/// Return standard path for settings cache file (`~/.config/sil/cache.yaml`).
pub fn default_settings_cache_path() -> Option<Utf8PathBuf> {
    dirs::config_dir().map(|p| Utf8PathBuf::from_path_buf(p.join("sil").join("cache.yaml")).ok()).flatten()
}

impl GlobalSettings {
    /// Load global settings from path or default path, returning default if non-existent.
    pub fn load_or_default(path: Option<&Utf8Path>) -> Self {
        let p = path.map(|p| p.to_path_buf()).or_else(default_global_settings_path);
        if let Some(p) = p {
            if p.exists() {
                if let Ok(text) = std::fs::read_to_string(p.as_std_path()) {
                    if let Ok(cfg) = serde_yaml::from_str::<GlobalSettings>(&text) {
                        return cfg;
                    }
                }
            }
        }
        Self::default()
    }

    /// Save global settings to specified path or default user config path.
    pub fn save(&self, path: Option<&Utf8Path>) -> Result<(), SilError> {
        let target_path = path.map(|p| p.to_path_buf()).or_else(default_global_settings_path)
            .ok_or_else(|| SilError::Message("cannot resolve global config directory".to_string()))?;

        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent.as_std_path())?;
        }

        let yaml = serde_yaml::to_string(self)
            .map_err(|e| SilError::Message(format!("failed to serialize global settings: {e}")))?;

        std::fs::write(target_path.as_std_path(), yaml)?;
        Ok(())
    }
}

impl SettingsCache {
    /// Load cache from path or default path, returning default if non-existent.
    pub fn load_or_default(path: Option<&Utf8Path>) -> Self {
        let p = path.map(|p| p.to_path_buf()).or_else(default_settings_cache_path);
        if let Some(p) = p {
            if p.exists() {
                if let Ok(text) = std::fs::read_to_string(p.as_std_path()) {
                    if let Ok(cache) = serde_yaml::from_str::<SettingsCache>(&text) {
                        return cache;
                    }
                }
            }
        }
        Self::default()
    }

    /// Save cache to specified path or default user cache path.
    pub fn save(&self, path: Option<&Utf8Path>) -> Result<(), SilError> {
        let target_path = path.map(|p| p.to_path_buf()).or_else(default_settings_cache_path)
            .ok_or_else(|| SilError::Message("cannot resolve global config directory".to_string()))?;

        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent.as_std_path())?;
        }

        let yaml = serde_yaml::to_string(self)
            .map_err(|e| SilError::Message(format!("failed to serialize settings cache: {e}")))?;

        std::fs::write(target_path.as_std_path(), yaml)?;
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
}
