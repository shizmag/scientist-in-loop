# PR-F3 — Non-blocking draft–ref similarity

## Role

Ship ONLY PR-F3. **Depends on F2** (required) and D2 (honest status).

## Goal

`X` recompute enqueues background similarity job; UI stays responsive. **`m`/`c` remain sort-only**.

## Requirements

1. Grep: today only `X` calls `recompute_draft_ref_similarities`.
2. Single `enqueue_similarity_job` used **only** by `X` handler.
3. Draft-hash staleness: discard results if draft changed mid-job.
4. Settings-based embedder; show fallback badge if D2 present.
5. Help: honest RAG wording on `X`; `m/c` still "sort by score".
6. Outcomes in unified history from F2.

## Out of scope

- Changing sort keys; module split H1

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Files, proof m/c do not enqueue, residual risk.
