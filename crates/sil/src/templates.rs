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

/// Markers for the sil-managed block inside `.gitignore`.
/// `sil init --update` refreshes only this block so local rules survive.
pub const GITIGNORE_MANAGED_START: &str = "# >>> sil-managed";
/// End marker for the sil-managed `.gitignore` block.
pub const GITIGNORE_MANAGED_END: &str = "# <<< sil-managed";

/// Default project `.gitignore` (sil-managed block + room for local rules).
///
/// Large / rebuildable artifacts are ignored by default:
/// - SQLite FTS database (rebuild with `sil parse`)
/// - figure binaries under `figures/plots/` and `figures/images/`
/// - experiment data under `data/` (except README)
/// - common result / cache / checkpoint trees
///
/// Literature PDFs in `sources/` stay trackable. Folder README.md files stay tracked.
pub const GITIGNORE: &str = r#"# >>> sil-managed
# Refreshed by `sil init --update`. Put custom rules below the end marker.

# --- sil local state (rebuildable) ---
.sil/db.sqlite
.sil/db.sqlite-*
.sil/*.sqlite
.sil/*.sqlite-*

# --- LaTeX build artifacts ---
*.aux
*.bbl
*.blg
*.fdb_latexmk
*.fls
*.log
*.out
*.synctex.gz
*.synctex(busy)
*.toc
*.lof
*.lot
*.nav
*.snm
*.vrb
*.bcf
*.run.xml
*-blx.bib
_minted*/

# Root-only build PDFs (paper_draft.pdf, paper.pdf, …).
# Literature under sources/ remains trackable.
/*.pdf

# --- large binaries: figures (keep README.md tracked) ---
figures/plots/**
!figures/plots/
!figures/plots/README.md
figures/images/**
!figures/images/
!figures/images/README.md

# --- experiment data (keep README.md tracked) ---
data/**
!data/
!data/README.md

# --- common experiment / ML outputs ---
results/
outputs/
output/
runs/
checkpoints/
wandb/
mlruns/
lightning_logs/
.cache/
tmp/
temp/
*.ckpt
*.pt
*.pth
*.onnx
*.h5
*.hdf5
*.parquet
*.feather
*.npz
*.pkl
*.pickle
*.npy

# --- OS / editor ---
.DS_Store
Thumbs.db
.idea/
.vscode/
*.swp
*~

# --- Python ---
__pycache__/
*.py[cod]
.venv/
venv/
.env
.ipynb_checkpoints/

# --- Rust (if agent builds crates under agent/) ---
**/target/
# <<< sil-managed

# Custom rules below this line are preserved by `sil init --update`.
"#;
