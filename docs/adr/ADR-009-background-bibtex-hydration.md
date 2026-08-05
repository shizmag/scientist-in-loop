# ADR-009: Non-Blocking Background BibTeX Metadata Hydration in TUI

## Context

When users copy source citations into `references.bib` via the TUI (from the extracted references viewer `a`/`A`, the References pane `p`, or the Sources tab `b`), fetching official metadata via external network APIs (DOI content negotiation, Crossref title lookups, arXiv API) can take several seconds due to network latency and rate limits. Previously, these network lookups ran synchronously on the UI thread, freezing TUI input and rendering.

## Decision

We implement immediate local append with non-blocking background metadata hydration in `sil-tui`:

1. **Immediate Local Append (UI thread, non-blocking)**:
   - When a reference or source is added, local BibTeX is immediately generated (`to_bibtex()` / `suggest_from_source()`) and marked with `% [sil: tui-added]`.
   - The local entry is upserted into `references.bib` on disk right away, and the left BibTeX pane reloads immediately.
   - The user receives immediate TUI feedback without any input blocking.

2. **Background Hydration Manager & Write Serialization**:
   - `should_attempt_metadata_fetch` checks if resolvable identifiers (DOI, arXiv ID, or non-empty title) exist. If missing, network fetch is skipped with a clear status warning.
   - If identifiers exist, a background worker thread (`std::thread::spawn`) is dispatched to invoke `resolve_official_bibtex_entry` or `resolve_official_bibtex_for_source`.
   - In-flight jobs are deduplicated by `dedup_key` (`doi:...`, `arxiv:...`, `ref_id:...`, `source_id:...`) using `in_flight_hydration_keys`.
   - Results are sent back to the main TUI loop over an `mpsc` channel and processed during `app.poll_background_hydration()` on every event loop tick (~100ms polling).
   - **Write Serialization**: File mutations to `references.bib` are strictly serialized on the main event loop thread inside `poll_background_hydration()`. Before writing, the main thread reads the current on-disk `references.bib` content, parses BibTeX blocks, applies upsert updates, and writes back atomically, preventing background worker write races or file corruption.

3. **In-Flight Mutation Safety Policies**:
   - **Promote-During-Flight Preservation**: If a user promotes a local stub via key `P` (`promote_selected_bib_entry`) while background hydration is in flight for that entry, the `% [sil: tui-added]` marker comment is stripped immediately on disk. When background hydration completes, `poll_background_hydration` reads the existing block on disk and detects `is_tui_added_bib_block(matching_block) == false`. It preserves the user's promoted state (unmarked) on the official metadata entry and retains the existing cite-key.
   - **Delete-During-Flight Skipping**: If a user deletes an entry (`d`/`D`) from `references.bib` while background hydration is in flight, when the hydration response arrives, `poll_background_hydration` checks if a matching block still exists in `references.bib`. If no matching block is found, hydration is skipped with an informational status message (`ℹ Skipped hydration for '{label}': entry was deleted from references.bib`), preventing deleted entries from re-appearing.

4. **Job Status Chrome & Failure Handling**:
   - **Job Status Chrome**: Active background hydration job count is tracked via `in_flight_hydration_keys`. The TUI status line renders visual status chrome (`Hydrating [N]...`) whenever hydration tasks are active, clearing dynamically as jobs finish or fail.
   - **Failure Handling**: On background fetch failure (e.g. 404, rate limit, network error), the local stub entry remains on disk in `references.bib`.
   - The TUI status message displays a clear warning containing the failure reason string (e.g. `⚠ Metadata fetch failed for '...': {reason}`). Write errors trigger explicit error notifications (`Error writing references.bib: {e}`).

## Consequences

- The TUI never blocks or freezes user input when adding references.
- `references.bib` receives instant local entries and seamless background upgrades to official metadata.
- Rate limits (`enforce_api_ratelimit`) are honored in worker threads.
- All hydration paths are offline-testable with clear unit test coverage.
