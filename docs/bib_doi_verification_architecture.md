# Architecture Decision Record: Incremental BibTeX DOI Verification & SQLite Update Surgery

## Context

Scientific papers managed by `scientist-in-loop` rely on `references.bib` for bibliography citations. Users and AI agents often add references with candidate or incomplete DOIs. Validating whether DOIs exist online (via Crossref or doi.org resolvers) is necessary to ensure citation accuracy, but network lookups must avoid redundant external API calls, obey rate limits, execute asynchronously, and operate without blocking main workspace operations.

## Decision

1. **Dedicated API Crate (`sil-api`)**:
   External API interactions (Crossref, arXiv, doi.org content negotiation) and API rate limiting are consolidated into `crates/sil-api`.
   - `enforce_api_ratelimit()` enforces a 250ms minimum gap between external HTTP requests using a thread-safe static.
   - `ApiError` provides categorized error types: `NotFound`, `RateLimited`, `NetworkError`, `ParseError`, `InvalidIdentifier`.

2. **SQLite Schema & Update Surgery (`sil-db`)**:
   Persistence is managed by `SilDb` via two tables:
   - `bib_references`: Stores parsed BibTeX entries, cite keys, DOIs, raw BibTeX, and verification statuses.
   - `doi_verifications`: Caches DOI existence flags (`exists_flag`) and error categories (`error_cat`).
   - **Update Surgery (`upsert_bib_reference`)**: Checks existing database records prior to mutation. If entry fields (`doi`, `doi_exists`, `raw_bibtex`) are unchanged, the database update is skipped (`returns false`), preserving timestamps and avoiding unnecessary writes.

3. **Incremental Execution & Background Orchestration (`sil-parse`)**:
   - `check_bib_dois_incremental`: Compares `references.bib` against SQLite cache. Identical, previously verified DOIs are marked `SkippedCached` with 0 network requests. Only new entries or updated DOIs trigger external API lookups.
   - `spawn_background_bib_doi_check`: Spawns non-blocking background threads for DOI verification.

4. **Integration Triggers (`sil`)**:
   - `sil project doctor`: Evaluates `references.bib` DOI health and reports categorized checks (Valid, Broken 404, Network Error, Skipped).
   - `sil paper build`: Triggers background DOI verification during LaTeX compilation and surfaces warnings if broken DOIs are detected in `references.bib`.

## Tradeoffs

- **Rate Limiting vs. Speed**: 250ms rate limit ensures compliance with Crossref API fair use guidelines while keeping background lookups efficient.
- **Background Execution**: Runs asynchronously without delaying user operations or LaTeX builds.
