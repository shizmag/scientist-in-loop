# PR-D1 — API retry / backoff + arXiv HTTPS

Copy the block below into an agent session (worktree-isolated if parallel).

---

## Role

You are a focused Rust **api-engineer** for scientist-in-loop. Ship ONLY PR-D1.

## Goal

Transient Crossref / arXiv / DOI / OpenReview failures retry with a bounded backoff. arXiv export API is HTTPS. 404s and parse errors still fail immediately.

## Repo context

- Parent plan: `docs/pr-plan-08-12/pr-plan.md` §D1, KD-11, KD-12, KD-13
- Crate: `crates/sil-api/` (`arxiv.rs`, `crossref.rs`, `doi.rs`, `openreview.rs`, `ratelimit.rs`, `error.rs`, `tests.rs`)
- Today: `ApiError::RateLimited` is returned immediately; `enforce_api_ratelimit` is a 250 ms global gap only
- arXiv query URL is `http://export.arxiv.org/api/query` — must become `https://export.arxiv.org/api/query`
- arXiv BibTeX path is already HTTPS — do not regress

## Shared invariants

1. Match existing Rust style; minimal diff.
2. Never auto-commit.
3. Keep `enforce_api_ratelimit()` in front of **each** attempt.
4. Tests must not sleep for real multi-second backoffs (inject a sleeper / `#[cfg(test)]` instant sleeper).
5. No mock HTTP server required.

## Requirements

1. Add `crates/sil-api/src/retry.rs` with:
   - `RetryPolicy { max_attempts: 3, base: 250ms, factor: 2, cap: 2s, jitter: ±20% }`
   - `should_retry(&ApiError) -> bool`: true for `RateLimited` and `NetworkError` that indicate HTTP 5xx or transport; false for `NotFound`, `ParseError`, `InvalidIdentifier`, and 4xx other than 429
   - `with_retry(policy, f) -> Result<T, ApiError>` that retries only when `should_retry` is true
2. Wire `with_retry` around the ureq calls in arxiv / crossref / doi / openreview (or around the existing public fetch functions — pick the smaller, consistent layer).
3. Change `http://export.arxiv.org` → `https://export.arxiv.org`.
4. Tests:
   1. Classifier table (RateLimited / 5xx / transport retry; NotFound / Parse / Invalid do not).
   2. Closure that fails twice then succeeds: counter == 3, result Ok (use instant sleeper).
   3. NotFound: counter == 1.
5. Do not change User-Agent strings, timeouts (except retry sleeps), or response parsing.

## Out of scope

- `download_pdf.py` (D2)
- New bibliographic APIs
- Honoring `Retry-After` is optional stretch only if it is a few lines; do not block the PR on it
- Mock HTTP
- Changing sil-parse resolve/Jaccard gates

## Verify

```bash
cargo test -p sil-api
cargo clippy -p sil-api --all-targets -- -D warnings
rg -n 'http://export.arxiv.org' crates/sil-api
# expect: no hits (https only)
```

## Deliverable

Files changed, retry policy numbers, HTTPS confirmation, residual (no live 429 integration test).
