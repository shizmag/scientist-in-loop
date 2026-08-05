# PR-C2 — Official BibTeX resolve reliability

Copy the block below into an agent session. Base on PR-A1. Can parallel A2 carefully.

---

## Role

Focused implementer. Ship ONLY PR-C2. Base on PR-A1 (pretty_format available). Can parallel A2 if merge conflicts avoided in journal_digest.rs.

## Goal

Make official resolution resilient when DOI/arXiv lookups fail, and reduce wrong Crossref matches.

## Repo context

- `crates/sil-parse/src/journal_digest.rs`:
  - `fetch_bibtex_by_doi`, `fetch_bibtex_by_arxiv_id`, `lookup_doi_by_title`
  - `resolve_official_bibtex_entry`, `resolve_official_bibtex_for_source`
- Current bug-class: if DOI is set and fetch fails, resolution returns Failed immediately (no arXiv/title fallback).
- Crossref title path takes rows=1 with no similarity gate.
- Rate limit: `enforce_api_ratelimit` ~250ms.

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. Prefer unit tests co-located with modules; keep clippy clean on touched crates.
3. Do not invent MCP bib write paths.

## Requirements

1. Fallback chain always continues on miss/error:
   - DOI attempt → on fail, try arXiv if id present → on fail, try title (+ authors if available)
2. Title acceptance: compute simple normalized similarity (token Jaccard or containment) between query title and Crossref title; reject below ~0.6 with Failed reason mentioning low confidence
3. Pretty-format every successful bib string (DOI and arXiv)
4. Optional small improvement: basic 429 retry once with backoff (keep tiny)
5. Unit tests with mocked/stubbed HTTP if existing test harness allows; otherwise pure tests for similarity helper + chain decision logic extracted from network

## Out of scope

- TUI job UI
- Changing local stub format beyond pretty
- Journal digest search CLI (PR-C3)

## Verify

```bash
cargo test -p sil-parse --lib
cargo clippy -p sil-parse --all-targets -- -D warnings
```

## Deliverable

New precedence documented in code comments; failure reason examples.
