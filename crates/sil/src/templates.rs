//! Embedded templates copied by `sil init`.
//!
//! Content mirrors the `templates/` directory in the repository so the binary
//! is self-contained; the on-disk templates remain the source of truth for docs.

/// Default `.sil/config.yaml`.
pub const CONFIG_YAML: &str = include_str!("../../../templates/config.yaml");
/// Default `.sil/structure.yaml`.
pub const STRUCTURE_YAML: &str = include_str!("../../../templates/structure.yaml");
/// Example structure document.
pub const STRUCTURE_EXAMPLE_YAML: &str =
    include_str!("../../../templates/structure.example.yaml");
/// SYSTEM skill.
pub const SKILL_SYSTEM: &str = include_str!("../../../templates/skills/SYSTEM.md");
/// paper skill.
pub const SKILL_PAPER: &str = include_str!("../../../templates/skills/paper.md");
/// agent-code skill.
pub const SKILL_AGENT_CODE: &str = include_str!("../../../templates/skills/agent-code.md");
/// data/README.md
pub const DATA_README: &str = include_str!("../../../templates/data.README.md");
/// figures/plots/README.md
pub const FIGURES_PLOTS_README: &str = include_str!("../../../templates/figures.plots.README.md");
/// figures/images/README.md
pub const FIGURES_IMAGES_README: &str = include_str!("../../../templates/figures.images.README.md");
/// agent/README.md
pub const AGENT_README: &str = include_str!("../../../templates/agent.README.md");
/// Project README.md
pub const PROJECT_README: &str = include_str!("../../../templates/README.md");

/// Minimal draft manuscript.
pub const PAPER_DRAFT_TEX: &str = r#"\documentclass{article}
\usepackage[utf8]{inputenc}
\usepackage{hyperref}

\title{Working Title}
\author{}
\date{\today}

\begin{document}
\maketitle

\begin{abstract}
% TODO: abstract
\end{abstract}

\section{Introduction}
% TODO: write introduction — see .sil/structure.yaml

\section{Related Work}
% TODO

\section{Methods}
% TODO

\section{Experiments}
% TODO

\section{Conclusion}
% TODO

\bibliographystyle{plain}
\bibliography{references}

\end{document}
"#;

/// Placeholder final manuscript (promoted later).
pub const PAPER_FINAL_TEX: &str = r#"\documentclass{article}
% paper.tex — cleaned manuscript promoted from paper_draft.tex
% Do not edit until sections are at least `draft` in structure.yaml.
\begin{document}
% empty shell
\end{document}
"#;

/// Empty bibliography.
pub const REFERENCES_BIB: &str = r#"% Bibliography for the paper.
% Example:
% @article{vaswani2017attention,
%   title={Attention is all you need},
%   author={Vaswani, Ashish and others},
%   journal={NeurIPS},
%   year={2017}
% }
"#;

/// Project .gitignore
pub const GITIGNORE: &str = r#"# LaTeX build artifacts
*.aux
*.bbl
*.blg
*.fdb_latexmk
*.fls
*.log
*.out
*.synctex.gz
*.toc

# Ignore LaTeX/build PDFs at the project root only (e.g. paper_draft.pdf).
# Source literature and figure PDFs must remain trackable in git.
/*.pdf

# Track PDFs under sources/ and figures/ (literature + plots/images).
!sources/
!sources/**
!figures/
!figures/**

# OS / editor
.DS_Store
.idea/
.vscode/
*.swp

# Python
__pycache__/
*.pyc
.venv/

# sil local overrides (keep db tracked optionally — default ignore large db)
# .sil/db.sqlite
"#;
