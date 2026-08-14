//! Embedded templates copied by `sil init` and the TUI first-run wizard.
//!
//! Content mirrors the `templates/` directory in the repository so the binary
//! is self-contained; the on-disk templates remain the source of truth for docs.

/// Default `.sil/config.yaml`.
pub const CONFIG_YAML: &str = include_str!("../../../templates/config.yaml");
/// Default `.sil/structure.yaml`.
pub const STRUCTURE_YAML: &str = include_str!("../../../templates/structure.yaml");
/// Example structure document.
pub const STRUCTURE_EXAMPLE_YAML: &str = include_str!("../../../templates/structure.example.yaml");
/// SYSTEM skill.
pub const SKILL_SYSTEM: &str = include_str!("../../../templates/agent/skills/SYSTEM.md");
/// paper skill.
pub const SKILL_PAPER: &str = include_str!("../../../templates/agent/skills/paper.md");
/// agent-code skill.
pub const SKILL_AGENT_CODE: &str = include_str!("../../../templates/agent/skills/agent-code.md");
/// Manuscript estimate / review skill.
pub const SKILL_REVIEW: &str = include_str!("../../../templates/agent/skills/review.md");
/// Review rubrics.
pub const SKILL_REVIEW_RUBRICS: &str =
    include_str!("../../../templates/agent/skills/review/rubrics.md");
/// Review personas.
pub const SKILL_REVIEW_PERSONAS: &str =
    include_str!("../../../templates/agent/skills/review/personas.md");
/// Review report template.
pub const SKILL_REVIEW_REPORT: &str =
    include_str!("../../../templates/agent/skills/review/report_template.md");
/// `.sil/reviews/README.md`
pub const REVIEWS_README: &str = include_str!("../../../templates/reviews.README.md");
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
/// sources/README.md
pub const SOURCES_README: &str = include_str!("../../../templates/sources.README.md");
/// `.sil/improvement/README.md`
pub const IMPROVEMENT_README: &str = include_str!("../../../templates/improvement.README.md");

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
% # -- X -- #
% TODO: write abstract summarizing main research contributions.
% # -- X -- #
\end{abstract}

\section{Introduction}
% # -- X -- #
% TODO: write introduction — see .sil/structure.yaml
% # -- X -- #

\section{Related Work}
% # -- X -- #
% TODO: survey related work and baseline methods.
% # -- X -- #

\section{Methods}
% # -- X -- #
% TODO: describe methodological framework and formulation.
% # -- X -- #

\section{Experiments}
% # -- X -- #
% TODO: design experiments, benchmarks, and ablation studies.
% # -- X -- #

\section{Conclusion}
% # -- X -- #
% TODO: summarize findings and future directions.
% # -- X -- #

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

/// Default bibliography file.
pub const REFERENCES_BIB: &str = r#"% references.bib — bibtex entries managed by sil
"#;

/// Managed `.gitignore` block marker — start.
pub const GITIGNORE_MANAGED_START: &str = "# >>> sil-managed";
/// Managed `.gitignore` block marker — end.
pub const GITIGNORE_MANAGED_END: &str = "# <<< sil-managed";

/// Default `.gitignore` content.
pub const GITIGNORE: &str = r#"# >>> sil-managed
# SQLite working database (can be re-derived from sources/)
.sil/db.sqlite
.sil/db.sqlite-wal
.sil/db.sqlite-shm
.sil/jobs.json

# Ephemeral build artifacts and caches
.sil/cache/
.sil/models/
.sil/undo/
target/

# Generated PDF outputs (manuscript drafts are source)
*.pdf
*.PDF
*.aux
*.log
*.out
*.bbl
*.blg
*.fls
*.fdb_latexmk
*.synctex.gz

# Data files and figures can be large; tracked explicitly if needed
figures/images/**
figures/plots/**
data/**
agent/**
*.jpg
*.JPG
*.jpeg
*.JPEG
*.png
*.PNG
*.webp
*.WEBP
*.svg
*.SVG
*.csv
*.CSV
*.tsv
*.TSV
*.parquet
*.h5
*.hdf5
*.pkl
*.pickle
*.npy
*.npz

# Keep READMEs and tracked text files
!**/README.md
!README.md
!.sil/config.yaml
!.sil/structure.yaml
!.sil/improvement/
!.sil/improvement/**
!.sil/draft_sections/
!.sil/draft_sections/**
!paper_draft.tex
!paper.tex
!references.bib
# <<< sil-managed

# Custom rules below this line are preserved by `sil init --update`.
"#;
