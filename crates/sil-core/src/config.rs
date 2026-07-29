//! Typed project configuration (`config.yaml`).

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::error::ConfigError;
use crate::stage::Stage;
use crate::types::LatexEngine;

/// Full project configuration matching templates/config.yaml.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Project metadata.
    pub project: ProjectConfig,
    /// Path layout relative to project root.
    pub paths: PathsConfig,
    /// LaTeX settings.
    pub latex: LatexConfig,
    /// PDF parsing settings.
    pub parsing: ParsingConfig,
}

/// `project:` section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Human-readable title.
    #[serde(default)]
    pub title: String,
    /// Lifecycle stage.
    #[serde(default)]
    pub stage: Stage,
}

/// `paths:` section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathsConfig {
    /// Original PDF sources.
    #[serde(default = "default_sources")]
    pub sources: Utf8PathBuf,
    /// Experimental data.
    #[serde(default = "default_data")]
    pub data: Utf8PathBuf,
    /// Figures root.
    #[serde(default = "default_figures")]
    pub figures: Utf8PathBuf,
    /// Agent-written code.
    #[serde(default = "default_agent")]
    pub agent: Utf8PathBuf,
}

fn default_sources() -> Utf8PathBuf {
    Utf8PathBuf::from("./sources")
}
fn default_data() -> Utf8PathBuf {
    Utf8PathBuf::from("./data")
}
fn default_figures() -> Utf8PathBuf {
    Utf8PathBuf::from("./figures")
}
fn default_agent() -> Utf8PathBuf {
    Utf8PathBuf::from("./agent")
}

/// `latex:` section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatexConfig {
    /// Compilation engine.
    #[serde(default)]
    pub engine: LatexEngine,
    /// Main `.tex` file relative to project root.
    #[serde(default = "default_main_tex")]
    pub main: Utf8PathBuf,
}

fn default_main_tex() -> Utf8PathBuf {
    Utf8PathBuf::from("paper_draft.tex")
}

/// `parsing:` section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsingConfig {
    /// Parsing engine name (MVP: marker).
    #[serde(default = "default_parse_engine")]
    pub engine: String,
}

fn default_parse_engine() -> String {
    "marker".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            project: ProjectConfig {
                title: String::new(),
                stage: Stage::Draft,
            },
            paths: PathsConfig {
                sources: default_sources(),
                data: default_data(),
                figures: default_figures(),
                agent: default_agent(),
            },
            latex: LatexConfig {
                engine: LatexEngine::Tectonic,
                main: default_main_tex(),
            },
            parsing: ParsingConfig {
                engine: default_parse_engine(),
            },
        }
    }
}

impl Config {
    /// Parse config from YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, ConfigError> {
        let cfg: Config = serde_yaml::from_str(yaml).map_err(|source| ConfigError::Parse {
            path: "<memory>".to_string(),
            source,
        })?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Load config from a file path.
    pub fn load(path: &Utf8Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::NotFound(path.to_string()));
        }
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.to_string(),
            source,
        })?;
        let cfg: Config = serde_yaml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_string(),
            source,
        })?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Serialize to YAML string.
    pub fn to_yaml(&self) -> Result<String, ConfigError> {
        serde_yaml::to_string(self).map_err(|source| ConfigError::Parse {
            path: "<memory>".to_string(),
            source,
        })
    }

    /// Semantic validation after parse.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.parsing.engine != "marker" {
            return Err(ConfigError::Validation(
                crate::error::ValidationError::Message(format!(
                    "unsupported parsing engine '{}'; only 'marker' is supported in MVP",
                    self.parsing.engine
                )),
            ));
        }
        if self.latex.main.as_str().is_empty() {
            return Err(ConfigError::Validation(
                crate::error::ValidationError::Message(
                    "latex.main must not be empty".to_string(),
                ),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
project:
  title: "Test Paper"
  stage: draft
paths:
  sources: ./sources
  data: ./data
  figures: ./figures
  agent: ./agent
latex:
  engine: tectonic
  main: paper_draft.tex
parsing:
  engine: marker
"#;

    #[test]
    fn parse_sample_config() {
        let cfg = Config::from_yaml(SAMPLE).unwrap();
        assert_eq!(cfg.project.title, "Test Paper");
        assert_eq!(cfg.project.stage, Stage::Draft);
        assert_eq!(cfg.latex.engine, LatexEngine::Tectonic);
        assert_eq!(cfg.parsing.engine, "marker");
    }

    #[test]
    fn reject_unknown_engine() {
        let bad = SAMPLE.replace("marker", "grobid");
        assert!(Config::from_yaml(&bad).is_err());
    }

    #[test]
    fn default_config_valid() {
        Config::default().validate().unwrap();
    }

    #[test]
    fn roundtrip_yaml() {
        let cfg = Config::from_yaml(SAMPLE).unwrap();
        let yaml = cfg.to_yaml().unwrap();
        let again = Config::from_yaml(&yaml).unwrap();
        assert_eq!(cfg, again);
    }
}
