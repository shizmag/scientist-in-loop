# Architectural Decision Record: Sources BibTeX Resolution and Draft Reference Cosine Similarity

## Context & Motivation

To enhance scientific paper workspace capabilities in `scientist-in-loop`, two new features are introduced:
1. **Feature A**: Metadata-first addition of selected source documents (`SourceDocument`) into `references.bib`.
2. **Feature B**: Cosine similarity calculation of the paper draft (`paper_draft.tex`) against extracted references (`source_references`), persisted in SQLite and surfaced in the TUI and CLI.

## Architectural Design

### Feature A: Metadata-First Source BibTeX Resolution

- **Resolution Pipeline (`sil-parse`)**:
  - `resolve_official_bibtex_for_source(doc: &SourceDocument) -> SourceBibResolution`
  - Order of precedence:
    1. Direct DOI fetch (`fetch_bibtex_by_doi`) via DOI content negotiation if DOI is available.
    2. arXiv API BibTeX fetch (`fetch_bibtex_by_arxiv_id`) if arXiv ID is present or extracted from DOI/filename.
    3. Title & author bibliographic lookup via Crossref (`lookup_doi_by_title`) to resolve a DOI, followed by DOI content negotiation.
    4. If resolution fails: returns `SourceBibResolution::Failed(reason)` without silently creating high-confidence fake entries.
- **BibTeX Upsert (`sil-core`)**:
  - Reuses `upsert_bib_entry` to perform smart deduplication based on DOI, arXiv ID, or title normalization.
  - Prefers official complete BibTeX entries over existing local stubs.
- **TUI & CLI Integration**:
  - Key `b` in TUI Sources tab triggers source BibTeX resolution and upsert into `references.bib`.
  - Warns user clearly in the status line upon resolution failure.
  - `sil cite` command incorporates metadata-first resolution.

### Feature B: Paper Draft vs Reference Cosine Similarity

- **LaTeX Stripping & Text Extraction**:
  - `strip_latex_for_embed(tex: &str)` strips LaTeX comments and formatting commands, isolating prose for embedding.
  - `ref_text_for_embed(entry: &ReferenceEntry)` combines title, authors, venue, and year into a single dense text query.
- **Embedding & Scoring**:
  - Reuses `OnnxEmbedder` (with token-hashing fallback for offline/test environments) and `cosine_similarity`.
- **Database Persistence (`sil-db`)**:
  - Dedicated table `draft_ref_similarity` with foreign key cascade on `source_references(id)`:
    ```sql
    CREATE TABLE IF NOT EXISTS draft_ref_similarity (
        ref_id TEXT PRIMARY KEY NOT NULL REFERENCES source_references(id) ON DELETE CASCADE,
        score REAL NOT NULL,
        draft_hash TEXT NOT NULL,
        model_dim INTEGER NOT NULL,
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    ```
- **Stale Handling & Hash Tracking**:
  - Stores `draft_hash` (`std::collections::hash_map::DefaultHasher`) to check staleness against `paper_draft.tex`.
  - TUI flags hash mismatches with a prompt to recompute (key `X` or `m`).
- **TUI & CLI Features**:
  - Added `RefSortKey::Similarity` in `sil-tui` for ranking references by draft similarity.
  - Render similarity score `[0.XX]` in extracted reference list UI.
  - `sil source rank-draft` CLI subcommand to recompute and display scores outside TUI.

### Feature C: TUI-Added BibTeX Comment Marking & Release Stripping

- **Comment Marking (`% [sil: tui-added]`)**:
  - Canonical marker comment `% [sil: tui-added]` pre-pended to BibTeX blocks added via TUI.
  - Pre-parsed comment association via `parse_bib_blocks` keeps preceding comments attached to the corresponding BibTeX entry block.
  - Pure helpers in `sil-core`: `mark_tui_added_bib_entry`, `unmark_tui_added_bib_entry`, `is_tui_added_bib_block`, `strip_tui_added_bib_entries`.
- **TUI Write Paths & Promotion**:
  - All TUI write paths (Sources tab `b` key, References right pane `p` key, Viewing-refs modal `a`/`b`/`p`) apply `mark_tui_added_bib_entry` prior to upserting entries into `references.bib`.
  - TUI promote key `P` on `references.bib` pane and CLI `sil cite <target> --promote` unmarks `% [sil: tui-added]` from selected entries, promoting them into permanent bibliography entries.
- **Publication Release Packaging & Compilation**:
  - `sil build release` and submission zip creation (`create_submission_archive`) strip `% [sil: tui-added]` entries from `references.bib` when generating publication artifacts.
  - Workspace `references.bib` on disk is kept intact for normal draft workflows.

