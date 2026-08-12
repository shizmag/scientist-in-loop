# PR-D2 — PDF download temp+rename + retry

Copy the block below into an agent session (worktree-isolated if parallel).

---

## Role

You are a focused **script-engineer** for scientist-in-loop. Ship ONLY PR-D2.

## Goal

`python/download_pdf.py` must not leave a truncated PDF if the process dies mid-write, and must retry transient HTTP/network errors. An existing good PDF must not be replaced by a failed attempt.

## Repo context

- Parent plan: `docs/pr-plan-08-12/pr-plan.md` §D2, KD-14
- Script: `python/download_pdf.py`
- Caller: `crates/sil-parse/src/fetch.rs` (`SIL_DOWNLOAD_SCRIPT`, stdout last `.pdf` line)
- Today: single `urlopen`, then `dest.write_bytes(data)` in place; numeric suffix if dest exists

## Shared invariants

1. Minimal diff; keep `classify` / `resolve_url` / DOI Accept header behavior.
2. Do not rewrite this as a native Rust downloader.
3. A `pytest` suite is **not** required. Prefer factoring small helpers that a `__main__` smoke or a Rust e2e stub can exercise later (F1).
4. Keep stdout contract: print the saved PDF path on success; non-zero + stderr on failure.

## Requirements

1. Download into `{dest}.part` (same directory), not directly into `dest`.
2. After the body arrives: require `%PDF` magic **or** `content-type` contains `pdf` (same validation spirit as today).
3. Only then `os.replace(part, dest)` (atomic on POSIX).
4. On any exception after creating `.part`: unlink `.part` if present.
5. Retry up to **3** attempts on HTTP 429, HTTP 5xx, and `URLError`. Do **not** retry HTTP 404 or other 4xx (except 429).
6. Backoff: 250 ms, 500 ms, 1000 ms is fine (or the same numbers as D1). Persistent failure still exits non-zero.
7. Do not clobber an existing good PDF: only `os.replace` after a validated body. A failed retry must leave the previous dest intact.
8. Keep the existing “dest exists → numeric suffix” behavior for a **new** download, not for retries of the same dest.
9. Add a tiny scripted check if cheap (e.g. factor `write_pdf_atomically(dest, data)` and a `if __name__` is not enough — a small function + comment is OK). F1 will add the e2e stub.

## Out of scope

- Changing DOI/arXiv URL resolution
- Native Rust downloader
- sil-api retry (D1)
- Live network tests against doi.org / arxiv.org

## Verify

```bash
python3 -m py_compile python/download_pdf.py
# optional local smoke if you add a helper:
# python3 -c "from pathlib import Path; import download_pdf"  # only if importable
cargo test -p sil --test e2e_source --test e2e_hardening
```

## Deliverable

Files changed, `.part` + retry behavior, residual (no live HTTP test).
