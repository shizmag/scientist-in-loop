# ADR-001: LaTeX Templator Crate (`sil-template`)

## Context

Manuscripts in `scientist-in-loop` start as a clean working draft in `paper_draft.tex`.
To submit or publish a paper to major machine learning and artificial intelligence conferences (such as NeurIPS, ICML, ICLR, IEEE/CVPR, or arXiv), authors must reformat the draft prose, title, authors, abstract, and bibliography into specific target conference/journal LaTeX packages and document styles.

## Decision

We introduce a dedicated library crate `crates/sil-template` (`sil-template`) responsible for:
1. Extracting structured components (title, author, abstract, prose body, bibliography) from a manuscript (`paper_draft.tex` or `paper.tex`).
2. Target template rendering into popular ML/AI conference and journal layouts (`neurips`, `icml`, `iclr`, `ieee`, `arxiv`, `standard`).
3. Declarative configuration in `.sil/config.yaml` under `latex.template` with CLI commands `sil template list`, `sil template apply`, and release build integration (`sil build --release`).

## Consequences

- Authors can write content in standard `paper_draft.tex` without locking into a specific conference template upfront.
- `sil template apply` or `sil build --release` deterministically produces publication-ready manuscript files.
- The `sil-template` crate stays pure and independent of SQLite/git operations for easy unit testing.
