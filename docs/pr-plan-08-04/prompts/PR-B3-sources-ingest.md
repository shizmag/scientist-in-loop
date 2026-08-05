# PR-B3 — Sources tab honest ingest + reload

Copy the block below into an agent session. Prefer base with PR-B2 job chrome.

---

## Role

Focused implementer. Ship ONLY PR-B3. Prefer base with PR-B2 job chrome.

## Goal

Make Sources tab ingest honest and support refresh without restarting the TUI.

## Repo context

- Sources keys in `app.rs`: `a` add link modal, `b` bib, `r` rename, `d` delete, `v` refs, Enter read
- Current `a`: modal says fetch-ish language but creates stub MD + placeholder DB upsert — no real PDF/download
- Existing fetch machinery: `crates/sil-parse/src/fetch.rs`, CLI `sil source fetch`, python `download_pdf.py`
- Parse is CLI-only today for Marker/xberg

## Default product decision (unless user overrides)

Prefer **real fetch** when input is DOI / arXiv / URL, using existing parse/fetch helpers, with local-first UX:

- Validate input → start background job (reuse B2 chrome if present) → on success reload sources list → status
- If offline/failure: keep clear error; do not claim success

If real fetch is too large for one PR, acceptable fallback:

- Retitle modal + status to “Register link stub (no download)”
- Still implement `R` reload sources from disk+DB
- Document which path you took in the summary.

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. TUI bib add stays non-blocking (local first + background hydrate).
3. Prefer unit tests co-located with modules; keep clippy clean on touched crates.

## Requirements

1. `R` (Sources list normal mode): reload sources from DB/disk; status confirmation
2. Fix `a` path honesty OR real fetch (see decision above)
3. Status strings must not say “fetched” if only stub registered
4. Do not break `b` hydration path
5. Tests: pure input classification (doi vs arxiv vs url vs garbage) if you add a helper

## Out of scope

- Full parse-from-TUI (PR-B4 stretch)
- Settings/RAG changes
- Aggressive key remap

## Verify

```bash
cargo test -p sil-tui --lib
cargo test -p sil-parse --lib
cargo clippy -p sil-tui -p sil-parse --all-targets -- -D warnings
```

## Deliverable

Exact behavior of `a` and `R`; residual gaps for B4.
