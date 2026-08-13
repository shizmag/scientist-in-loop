# ADR-014: Three-Surface Use-Case Layer (`sil-app`)

## Status
Accepted (Wave sil-app / Stage 12)

## Context
CLI (`crates/sil`), TUI (`sil-tui`), and MCP (`sil-mcp`) each orchestrated literature fetch and bibliography operations independently. Because `crates/sil` is a binary crate depending on `sil-tui` and `sil-mcp`, neither library crate could depend on `sil` without introducing dependency cycles. Consequently, I/O logic, atomic file writes, commit proposal generation, and policy decisions (such as cite-key preservation, draft markers, and error swallowing) diverged across the three interfaces:
- CLI `cite --append` did not preserve existing cite keys or add draft markers; CLI `fetch` only resolved BibTeX if the target string was a DOI or arXiv ID.
- MCP `sil_cite` supported `preserve_cite_key` toggles and draft flags; MCP `fetch` swallowed parse errors into silent `parsed=false` responses and never wrote to `references.bib`.
- TUI explicit bib actions appended draft markers (`% [sil: tui-added]`) and promoted by `is_same_paper` only; TUI fetch downloaded files but performed no BibTeX resolution.

## Decision

1. **`sil-app` Crate**: Introduced a dedicated sync use-case library crate `crates/sil-app` positioned between domain libraries (`sil-core`, `sil-parse`, `sil-git`, `sil-db`) and user surfaces (`sil`, `sil-tui`, `sil-mcp`). Use-case functions take `&AppContext` and request structs, returning structured results with `CommitProposal` instances. Surfaces act as thin presentation adapters.
2. **Richest Policy Unification**:
   - `upsert_bib`: Enforces `preserve_cite_key: true` unconditionally. Accepts a `draft: bool` role flag to add or omit `% [sil: tui-added]`. Re-reads `references.bib`, performs atomic write via `sil_core::write_atomic_str`, and builds an `UpdateBibliography` proposal.
   - `promote_bib`: Matches targets against cite keys (case-insensitive) or paper identity (`is_same_paper`), unmarks draft entries, performs atomic write, and builds a `PromoteBibliography` proposal.
   - `fetch_source`: Orchestrates hard-error source download, optional parsing via `sil_parse::parse_one`, official BibTeX resolution (target DOI/arXiv -> document DOI/arXiv/title resolver -> `upsert_bib(draft=false)`), and generates `FetchSource` / `ParsePdf` proposals. Parse errors surface via `FetchSourceResult.parse_error` rather than swallowing.
3. **Surface Adaptation**:
   - **CLI**: `source cite --append` / `--promote` and `source fetch` call `sil-app`. CLI `cite` keeps quiet output (no proposal stdout). CLI `fetch` displays download/parse proposals as before.
   - **MCP**: `sil_cite` (upsert/promote) and `sil_sources` (fetch) call `sil-app`. MCP `preserve_cite_key: false` argument is ignored (always preserved). MCP `fetch` includes resolved `bib` object and `parse_error` in JSON response.
   - **TUI**: Explicit bib append/promote and background fetch job call `sil-app`. TUI fetch specifies `parse=false` and reloads bibliography if an official BibTeX entry was written.

## Consequences

- **Eliminated Surface Drift**: BibTeX upserts, promotions, and source fetches follow identical rules, validation, and proposal generation regardless of entry point.
- **Official BibTeX on Fetch**: MCP and CLI source fetches now resolve and write official BibTeX entries into `references.bib` when metadata is found.
- **Cite Key Preservation**: Same-paper replaces always preserve existing cite keys across CLI, MCP, and TUI.
- **Never Auto-Commit**: Use-cases produce `CommitProposal` objects; adapters decide presentation.

## Residuals

- **Search & Rank Parity**: Search on CLI remains FTS-only while MCP supports RAG hybrid search; rank embedder configuration still differs across surfaces.
- **Checker `--fix` Autofix**: `sil-parse` checker `--fix` (autofix) is aligned with richest policy (`preserve_cite_key: true`), but remains in `sil-parse` to avoid introducing a dependency on `sil-app` (preventing cycle `sil-app` → `sil-parse`). Note that TUI `p` handler and hydration apply now write `references.bib` via `sil_app::upsert_bib`.
- **URL + TUI `parse=false`**: Fetching a raw URL in TUI with `parse=false` yields no title or extracted DOI, resulting in `bib = None` until the source is parsed and hydrated.
