# PR-C3 — Journal digest native/Python parity

Copy the block below into an agent session. Parallel-safe with most other PRs.

---

## Role

Focused implementer. Ship ONLY PR-C3. Parallel-safe with most other PRs (digest CLI + journal_digest).

## Goal

Remove dual-stack drift: native Crossref digest should match Python filters, and CLI should not force Python-only.

## Repo context

- CLI: `crates/sil/src/commands/digest.rs` — currently always passes Python script path
- Native: `fetch_journal_publications_native` in `crates/sil-parse/src/journal_digest.rs`
- Python: `python/fetch_journal_digest.py` uses `filter=type:journal-article`, relevance sort
- Glue: `fetch_journal_publications` — if script_path Some → Python only

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. Prefer unit tests co-located with modules; keep clippy clean on touched crates.
3. Avoid live network in CI unit tests.

## Requirements

1. Native path: add `type:journal-article` filter (and sort/relevance parity as close as Crossref allows)
2. CLI: call with native-first + Python fallback (e.g. `script_path: None` first, or explicit fallback on native error)
3. Preserve DB save behavior (`journal_digest` table) if CLI saves today
4. Tests: pure URL/query builder tests if extracted; avoid live network in CI unit tests
5. Brief comment in CLI help text if behavior changes

## Out of scope

- Deleting Python helper entirely (keep fallback)
- PDF download pipeline changes

## Verify

```bash
cargo test -p sil-parse --lib
# cargo test -p sil --test e2e_*  # only if digest e2e exists; otherwise skip
cargo clippy -p sil-parse -p sil --all-targets -- -D warnings
```

## Deliverable

CLI call order, filter parity notes, residual differences vs Python.
