# PR-A1 — Pretty BibTeX foundation

Copy the block below into an agent session (worktree-isolated if parallel).

---

## Role

You are a focused Rust implementer for scientist-in-loop. Ship ONLY PR-A1.

## Goal

Make all newly written BibTeX entries consistently multiline via `pretty_format_bibtex`, and ensure network-fetched entries are pretty-formatted at the source.

## Repo context

- Workspace root: scientist-in-loop (Rust 2024 edition monorepo).
- Core bib module: `crates/sil-core/src/bib.rs` (re-exported from `crates/sil-core/src/lib.rs`).
- DOI fetch: `crates/sil-parse/src/journal_digest.rs` → `fetch_bibtex_by_doi`.
- arXiv fetch: same file → `fetch_bibtex_by_arxiv_id`.
- Uncommitted WIP may already add `pretty_format_bibtex` and wire it into `upsert_bib_entry` / `mark_tui_added_bib_entry` / DOI fetch. Inspect git status/diff first; complete and polish — do not duplicate.

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. TUI bib add stays non-blocking (local first + background hydrate).
3. Release strip only removes `% [sil: tui-added]` blocks from packages.
4. Do not invent MCP bib write paths or full keybinding remaps.
5. Prefer unit tests co-located with modules; keep clippy clean on touched crates.

## Requirements

1. `pretty_format_bibtex(bibtex: &str) -> String`:
   - Preserve leading comment lines (`%` / `#`) before the `@` entry.
   - Output `@type{key,` then fields as `  key = value,` (2-space indent), last field without trailing comma issues, closing `}`.
   - Handle single-line Crossref-style entries (mixed braces, `month=Feb` without braces).
   - On unparseable input, return trimmed original (no panic).
2. Call pretty-format from:
   - `upsert_bib_entry` (incoming new entry)
   - `mark_tui_added_bib_entry`
   - `fetch_bibtex_by_doi` and `fetch_bibtex_by_arxiv_id` when body starts with `@`
3. Unit tests in `bib.rs` for single-line → multiline (include a realistic DOI-style sample with multi-author names).
4. Update any tests that assumed single-line `author={...}` substrings if formatting changes spacing.

## Out of scope

- Completeness-aware upsert policy (PR-A2)
- Cite-key preservation (PR-A3)
- TUI hydration races (PR-A4)
- Docs-only PRs

## Verify

```bash
cargo test -p sil-core
cargo test -p sil-parse --lib
cargo clippy -p sil-core -p sil-parse --all-targets -- -D warnings
```

## Deliverable

Summary of files changed, behavior before/after, and any edge cases left for A2+.
