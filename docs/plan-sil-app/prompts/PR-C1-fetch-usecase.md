# PR-C1 — fetch_source use-case

Copy the block below into an agent session (worktree-isolated if parallel).

---

## Role

You are a focused Rust **library engineer** for scientist-in-loop. Ship ONLY PR-C1.

## Goal

Add `sil_app::fetch_source`: download + optional parse + richest official-bib upsert. No CLI / TUI / MCP wiring.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-sil-app/pr-plan.md` §5.4, KD-9–KD-12, KD-14
- Prerequisite: **PR-A1 merged** (`upsert_bib` exists)
- Download: `sil_parse::fetch_source_target`
- Parse: `sil_parse::parse_one` + `discover_marker_runner` + `sil_core::NullUi`
- Official bib:
  - `sil_regex::extract_doi` / `extract_arxiv_id` on the **target string**
  - `sil_parse::journal_digest::fetch_bibtex_by_doi` / `fetch_bibtex_by_arxiv_id`
  - then `resolve_official_bibtex_for_source` on a `SourceDocument` built from the downloaded path + parsed metadata if any
- Existing CLI fetch (do **not** edit): `crates/sil/src/commands/source.rs` `fetch`
- Existing MCP fetch (do **not** edit): `handle_fetch_source`
- `sil-parse` already has `fetch_source_target_mock_script` tests — reuse `SIL_DOWNLOAD_SCRIPT` if useful

## Shared invariants

1. Match existing Rust style; minimal diff.
2. Never auto-commit.
3. Download failure is a **hard error**. Missing official bib is **not**.
4. `parse=true` but runner missing / `parse_one` fails → set `parse_error`, continue to bib resolve. **Do not** invent stub markdown in `sil-app`.
5. Official bib goes through `upsert_bib(..., draft=false)` — do not call `upsert_bib_entry` directly.
6. No `SilUi` / JSON / Ratatui in this crate.

## Requirements

1. Add `sil-parse` and `sil-regex` (if extractors are used here) to `sil-app` deps. Add `sil-db` only if `parse_one` requires a `SilDb` you open here (`parse_one` needs `&SilDb` — open `SilDb::open(&ctx.paths.db())` when `parse` is true **or** when you need the db; opening for parse only is fine).
2. New `src/fetch.rs`; export from `lib.rs`.
3. Types (names may be bikesheded slightly; fields are normative):

```rust
pub struct FetchSource { pub target: String, pub parse: bool }

pub struct ParseSummary {
    pub filename: String,
    pub title: Option<String>,
    pub source_id: String,
    pub reference_count: usize,
}

pub struct FetchSourceResult {
    pub downloaded_path: Utf8PathBuf,
    pub parsed: Option<ParseSummary>,
    pub parse_error: Option<String>,
    pub bib: Option<UpsertBibResult>,
    pub fetch_proposal: CommitProposal,
    pub parse_proposal: Option<CommitProposal>,
}

pub fn fetch_source(ctx: &AppContext, req: FetchSource) -> Result<FetchSourceResult, AppError>;
```

4. Algorithm:
   1. `sources_dir = ctx.paths.sources(&ctx.config)`
   2. `saved = fetch_source_target(&req.target, &sources_dir)?`
   3. Resolve on-disk path: absolute if `saved` is absolute; else `sources_dir.join(file_name)` if that exists; else `ctx.root.join(&saved)` (same as CLI/MCP).
   4. If `req.parse` && path exists:
      - `discover_marker_runner()` — on err, `parse_error = Some(...)`, skip parse_one
      - else `parse_one(path, &db, runner, &NullUi)` — on err, `parse_error = Some(...)`; on ok, fill `parsed` + `parse_proposal = proposal_for_action(ParsePdf, ...)`
   5. Official bib:
      - If DOI on target → `fetch_bibtex_by_doi`
      - Else if arXiv on target → `fetch_bibtex_by_arxiv_id`
      - If still none: `SourceDocument` from resolved path; if `parsed` is Some, copy title / id / metadata onto the doc when those fields exist
      - `resolve_official_bibtex_for_source`
      - `Resolved(bib)` → `upsert_bib(draft=false)` → `result.bib = Some(...)`
      - else `bib = None`
   6. `fetch_proposal = proposal_for_action(FetchSource, Some("Fetch source: {target}"), Some("Saved to {path}"))`
5. Tests (no live network):
   1. **Download failure is hard:** point `SIL_DOWNLOAD_SCRIPT` at a failing script (see `sil-parse` tests) → `fetch_source` errors; `references.bib` unchanged / absent.
   2. If you can use a succeeding mock script that writes a tiny PDF into `sources/`:
      - `parse=false` → `parsed.is_none()`, `parse_proposal.is_none()`, download path set, fetch proposal has `Sci-Action: fetch-source`
      - Optional: if mock target is a DOI-looking string but bib APIs will fail offline, `bib` may be None — that is OK; assert it does not panic.
   3. Do **not** hit Crossref/arXiv in CI. Do not add network tests.
6. Extend `AppError` with a parse/fetch variant if needed (`#[from]` `ParseError` or a message variant).

## Out of scope

- Wiring CLI / MCP / TUI (C2–C4)
- Stub Marker content inside sil-app
- TUI empty `upsert_parsed("",…)`
- Changing `sil-parse` download implementation
- Search / rank / hydration
- STAGES / ADR

## Verify

```bash
cargo test -p sil-app
cargo clippy -p sil-app --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Signatures, how download is mocked in tests, residual “offline bib is None” note.
