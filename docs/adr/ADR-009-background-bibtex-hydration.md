# ADR-009: Non-Blocking Background BibTeX Metadata Hydration in TUI

## Context

When users copy source citations into `references.bib` via the TUI (from the extracted references viewer `a`/`A`, the References pane `p`, or the Sources tab `b`), fetching official metadata via external network APIs (DOI content negotiation, Crossref title lookups, arXiv API) can take several seconds due to network latency and rate limits. Previously, these network lookups ran synchronously on the UI thread, freezing TUI input and rendering.

## Decision

We implement immediate local append with non-blocking background metadata hydration in `sil-tui`:

1. **Immediate Local Append (UI thread, non-blocking)**:
   - When a reference or source is added, local BibTeX is immediately generated (`to_bibtex()` / `suggest_from_source()`) and marked with `% [sil: tui-added]`.
   - The local entry is upserted into `references.bib` on disk right away, and the left BibTeX pane reloads immediately.
   - The user receives immediate TUI feedback without any input blocking.

2. **Background Hydration Manager**:
   - `should_attempt_metadata_fetch` checks if resolvable identifiers (DOI, arXiv ID, or non-empty title) exist. If missing, network fetch is skipped with a clear status warning.
   - If identifiers exist, a background worker thread (`std::thread::spawn`) is dispatched to invoke `resolve_official_bibtex_entry` or `resolve_official_bibtex_for_source`.
   - In-flight jobs are deduplicated by `dedup_key` (`doi:...`, `arxiv:...`, `ref_id:...`, `source_id:...`).
   - Results are sent back to the main TUI loop over an `mpsc` channel and processed during `app.poll_background_hydration()` on every event loop tick (~100ms polling).

3. **Marker Preservation Policy**:
   - On background fetch success, the official BibTeX entry replaces the local stub via `upsert_bib_entry`.
   - The `% [sil: tui-added]` comment marker is retained on the hydrated official entry until the user explicitly promotes it via `P` (`promote_selected_bib_entry`).

4. **Failure Handling**:
   - On background fetch failure (e.g. 404, rate limit, network error), the local stub entry remains on disk in `references.bib`.
   - The TUI status message displays a clear warning containing the failure reason string (e.g. `⚠ Metadata fetch failed for '...': {reason}`).

## Consequences

- The TUI never blocks or freezes user input when adding references.
- `references.bib` receives instant local entries and seamless background upgrades to official metadata.
- Rate limits (`enforce_api_ratelimit`) are honored in worker threads.
- All hydration paths are offline-testable with clear unit test coverage.
