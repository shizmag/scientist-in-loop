//! Available LaTeX paper target templates.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Target ML/AI conference or journal LaTeX template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PaperTemplate {
    /// Standard clean LaTeX article layout (default).
    #[default]
    Standard,
    /// NeurIPS conference format.
    Neurips,
    /// ICML conference format.
    Icml,
    /// ICLR conference format.
    Iclr,
    /// IEEE / CVPR conference format.
    Ieee,
    /// Clean modern arXiv preprint format.
    Arxiv,
}

impl PaperTemplate {
    /// All supported template target names.
    pub const ALL: &'static [&'static str] =
        &["standard", "neurips", "icml", "iclr", "ieee", "arxiv"];

    /// String name of the template.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Neurips => "neurips",
            Self::Icml => "icml",
            Self::Iclr => "iclr",
            Self::Ieee => "ieee",
            Self::Arxiv => "arxiv",
        }
    }

    /// Human-readable title of the conference/journal template.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Standard => "Standard Clean Article Format",
            Self::Neurips => "NeurIPS (Neural Information Processing Systems)",
            Self::Icml => "ICML (International Conference on Machine Learning)",
            Self::Iclr => "ICLR (International Conference on Learning Representations)",
            Self::Ieee => "IEEE / CVPR Conference Format",
            Self::Arxiv => "arXiv Clean Preprint Format",
        }
    }
}

impl fmt::Display for PaperTemplate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for PaperTemplate {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "standard" | "default" | "article" => Ok(Self::Standard),
            "neurips" => Ok(Self::Neurips),
            "icml" => Ok(Self::Icml),
            "iclr" => Ok(Self::Iclr),
            "ieee" | "cvpr" => Ok(Self::Ieee),
            "arxiv" => Ok(Self::Arxiv),
            other => Err(format!(
                "unsupported template '{other}'; supported options are: {}",
                Self::ALL.join(", ")
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_templates() {
        assert_eq!(
            "neurips".parse::<PaperTemplate>(),
            Ok(PaperTemplate::Neurips)
        );
        assert_eq!("icml".parse::<PaperTemplate>(), Ok(PaperTemplate::Icml));
        assert_eq!("iclr".parse::<PaperTemplate>(), Ok(PaperTemplate::Iclr));
        assert_eq!("ieee".parse::<PaperTemplate>(), Ok(PaperTemplate::Ieee));
        assert_eq!("cvpr".parse::<PaperTemplate>(), Ok(PaperTemplate::Ieee));
        assert_eq!("arxiv".parse::<PaperTemplate>(), Ok(PaperTemplate::Arxiv));
        assert_eq!(
            "standard".parse::<PaperTemplate>(),
            Ok(PaperTemplate::Standard)
        );
        assert!("unknown".parse::<PaperTemplate>().is_err());
    }

    #[test]
    fn roundtrip_string() {
        for name in PaperTemplate::ALL {
            let t = name.parse::<PaperTemplate>().unwrap();
            assert_eq!(t.as_str(), *name);
            assert_eq!(t.to_string(), *name);
        }
    }
}
