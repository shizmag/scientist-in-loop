# PR-F1 — E2E crash / recovery / SQLITE_BUSY gates

Copy the block below into an agent session. **Depends on A2, B1, C1, D1, D2** (E2 soft).

---

## Role

You are a focused **test-engineer** for scientist-in-loop. Ship ONLY PR-F1.

## Goal

Lock the Wave 08-12 durability invariants behind tests so they stay green. Prefer extending `crates/sil/tests/e2e_hardening.rs` and filling gaps only where unit tests from A–E did not already cover the gate.

## Repo context

- Parent plan: `docs/pr-plan-08-12/pr-plan.md` §F1, verification V1–V6
- Existing: `crates/sil/tests/e2e_hardening.rs` (`reparse_same_pdf_fails_idempotently` must remain)
- Fetch stub hook: `SIL_DOWNLOAD_SCRIPT` in `crates/sil-parse/src/fetch.rs`
- Marker stub hook: `SIL_MARKER_STUB` / `SIL_PARSE_SCRIPT`
- Do **not** invent a CLI `--force` parse flag. Use `sil-parse` library `ParseOptions { allow_reparse: true }` for the preserve case (unit test in sil-parse is enough if C1 already added it — then F1 only asserts it still exists / add e2e helper).

## Shared invariants

1. Tests only — no product behavior changes unless a previous PR left an untestable hole (then the smallest test hook, not a feature).
2. No live Crossref / arXiv / doi.org.
3. Never auto-commit.
4. Keep default re-parse-without-force failing.

## Requirements

Add or confirm **all** of the following. Skip a case only if an earlier PR already has an equivalent test — then cite the test name in the deliverable.

1. **Re-parse preserve:** parse with stub token `first`; force re-parse via `ParseOptions { allow_reparse: true }` with a failing runner; previous content still readable / searchable. Default second `sil source parse` still errors (`reparse_same_pdf_fails_idempotently`).
2. **Force re-parse success:** allow_reparse + succeeding stub with a new token; search finds the new token.
3. **SQLITE_BUSY:** file-backed DB, two connections, overlapping write; no immediate `database is locked` (B1 unit test is sufficient — add here only if missing).
4. **Atomic write:** sil-core A1 units are sufficient; add one CLI-level check only if cheap (e.g. a command that saves structure/settings still loads).
5. **Download stub:** `SIL_DOWNLOAD_SCRIPT` pointing at a local script that writes a `.part` then a valid tiny PDF (or invokes the D2 helper). Dest exists, starts with `%PDF`, no leftover `.part`. Use `sil-parse::minimal_pdf_bytes()` or a 5-byte `%PDF-` stub if the fetch path only checks the name.

## Out of scope

- Live network
- New CLI flags
- Golden fixtures
- Doctor rebuild
- Product refactors

## Verify

```bash
cargo test -p sil --test e2e_hardening --test e2e_source
cargo test -p sil-core -p sil-db -p sil-parse -p sil-api
```

## Deliverable

New/confirmed test names mapped to V1–V6 gates, files changed, residual gaps.
