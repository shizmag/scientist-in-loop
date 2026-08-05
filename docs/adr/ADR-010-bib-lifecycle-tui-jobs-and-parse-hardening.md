# ADR-010: Bibliography Lifecycle, Non-Blocking TUI Background Jobs, and Parsing Hardening

## Status
Accepted

## Context

During the 2026-08-04 architecture consolidation cycle (PR-A1 to PR-A4, PR-B1 to PR-B4, PR-C1 to PR-C3), multiple features across `sil-core`, `sil-parse`, `sil-tui`, and `sil` CLI were implemented to harden bibliography management, user experience, reference extraction quality, and CLI operation reliability.

Prior to these consolidation PRs:
1. BibTeX blocks generated across TUI and CLI lacked consistent formatting and completeness scoring, causing potential field duplication or loss during upserts.
2. In-flight background hydration tasks could race with user actions in the TUI (such as entry promotion or deletion) or crash write operations on concurrent disk edits.
3. Reference extraction on complex document layouts (e.g. `BEE-RAG`) suffered from fragmented line-wrapped citation entries.
4. Metadata resolution occasionally accepted false-positive Crossref matches when direct DOIs were absent.
5. CLI journal digest commands relied on external script execution rather than native Rust query execution.

## Decision

We establish the following normative architectural policies for bibliography lifecycle management, TUI background job orchestration, and parsing hardening:

1. **Pretty BibTeX Formatting & Completeness-Aware Upsert (PR-A1, PR-A2)**:
   - **Pretty Formatting**: `format_bib_entry_pretty` in `sil-core::bib` enforces standard canonical field ordering (`title`, `author`, `journal`/`booktitle`, `year`, `volume`, `number`, `pages`, `doi`, `url`, `arxiv_id`, `abstract`, `eprint`, `archiveprefix`, `primaryclass`), 2-space indentation, lower-case keys, and clean brace wrapping.
   - **Completeness Scoring**: `compute_bib_entry_completeness` calculates a quantitative completeness score. Upsert operations (`upsert_bib_entry_with_options`) preserve non-conflicting user fields and only upgrade entries when incoming metadata improves completeness.
   - **arXiv ID Normalization**: `normalize_arxiv_id` standardizes arXiv identifiers into canonical format (`YYMM.NNNNN` or legacy `category/YYMMNNN`), stripping version tags (`v1`, `v2`) and URL scheme prefixes.

2. **Cite-Key Preservation & Marker Lifecycle (PR-A3, PR-C3)**:
   - **Cite-Key Preservation**: Background hydration and manual upserts preserve existing cite-keys (`UpsertOptions { preserve_cite_key: true }`) so manuscript `\cite{key}` references remain valid.
   - **Marker Lifecycle**: TUI-added entries are tagged with `% [sil: tui-added]`. The marker is retained until explicit user promotion (`P` key in TUI or `sil cite --promote` in CLI), and stripped automatically during release builds (`sil build --release`).

3. **Write Serialization & In-Flight Race Protection (PR-A4, ADR-009)**:
   - **Write Serialization**: All background hydration results returned via `mpsc` channels are applied sequentially on the main event loop thread inside `poll_background_hydration()`. Content is re-read from disk prior to writing, preventing file corruption and write races.
   - **Promote-During-Flight Preservation**: If an entry is promoted while background hydration is active, `poll_background_hydration()` detects the unmarked on-disk status and retains the promoted state without re-adding the marker comment.
   - **Delete-During-Flight Skipping**: If an entry is deleted while hydration is in flight, `poll_background_hydration()` checks for entry existence and skips the update if deleted, preventing deleted entries from reappearing.

4. **TUI Status Chrome, Help System, and Workflow Actions (PR-B1 to PR-B4)**:
   - **Help Overlay**: `?` / `F1` toggles a mode-aware keyboard help modal displaying context-specific shortcuts.
   - **Status Chrome**: Live hydration job counts (`Hydrating [N]...`) are displayed in the footer status line.
   - **Reload & Parse Actions**: `R` reloads project sources and references into memory; `e` / `E` triggers non-blocking inline PDF parsing for selected or all unparsed sources.

5. **Reference Parsing & Fallback Resolution Hardening (PR-C1, PR-C2)**:
   - **Line-Wrap Continuation Joining**: `sil-parse::references` detects line-wrapped citations without new entry boundaries and joins them into unified citation entries with normalized spacing and hyphenation repair.
   - **Fallback Chain & Jaccard Gating**: `resolve_official_bibtex_entry` follows the chain: direct DOI -> direct arXiv -> Crossref Title+Author search. Crossref candidate matches are gated by a mandatory token Jaccard title similarity check ($\\ge 0.60$), rejecting false-positive lookups.

6. **Native-First CLI Execution (PR-C3)**:
   - `sil source digest [query]` uses a native Rust Crossref query builder in `sil-parse::journal_digest`, eliminating mandatory Python script dependencies.

## Consequences

- End-to-end bibliography updates in `references.bib` are deterministic, non-destructive, and cite-key stable.
- TUI user interactions are completely responsive, zero-blocking, and race-free under active background network hydration.
- Extracted references maintain high precision and recall across complex benchmark documents.
- External API lookups fail safely without corrupting local bibliography entries or injecting incorrect publication metadata.
