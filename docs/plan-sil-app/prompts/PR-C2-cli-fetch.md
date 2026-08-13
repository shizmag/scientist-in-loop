# PR-C2 — CLI source fetch via sil-app

Copy the block below into an agent session (worktree-isolated if parallel).

---

## Role

You are a focused Rust **CLI engineer** for scientist-in-loop. Ship ONLY PR-C2.

## Goal

Replace the orchestration in `sil source fetch` with `sil_app::fetch_source`. Keep spinner, ✓ messages, and **existing** fetch/parse proposal prints (KD-8 is cite-only).

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-sil-app/pr-plan.md` §5.5, §6 C2, KD-9
- Prerequisite: **PR-C1 merged**
- Today: `crates/sil/src/commands/source.rs` `fetch` (~35–114)
  - `fetch_source_target` + optional `parse_one` + DOI/arXiv-only bib upsert
  - prints `CommitProposal` via `print_proposal` for fetch and parse
- E2E: `crates/sil/tests/e2e_source.rs` `source_fetch_surfaces_download_failure`
- `sil` may already depend on `sil-app` if B1 landed; add the dep if missing

## Shared invariants

1. Match existing Rust style; minimal diff.
2. Never auto-commit.
3. `parse = !no_parse` (CLI default still parses).
4. Keep user-visible fetch/parse proposal blocks (this is **not** the cite-quiet rule).
5. Official bib now uses the richest resolver (C1) — URL fetches may write bib after parse. Intended.

## Requirements

1. `fetch(target, no_parse, ui)`:
   - Load project → `AppContext::from_root`
   - Spinner around `fetch_source` (whole call is sync and includes parse/bib — acceptable; do not split unless spinner UX becomes misleading). Prefer: spinner “Fetching {target}” for the whole use-case, then report outcomes.
   - On use-case `Err`: finish_error + return anyhow (download failed).
   - On ok:
     - ✓ Downloaded → path
     - `print_proposal` for `fetch_proposal`
     - If `parsed` Some: ✓ Parsed filename; `print_proposal` for `parse_proposal` if Some
     - If `parse_error` Some: `ui.warn` (do not fail the command — download succeeded)
     - If `bib` Some: ✓ Replaced / Added official metadata (same wording as today)
2. Delete the duplicated DOI/arXiv + `upsert_bib_entry` block from this function.
3. `e2e_source::source_fetch_surfaces_download_failure` stays green.
4. Do not add new flags.

## Out of scope

- MCP / TUI
- Changing download Python
- Cite command
- STAGES / ADR

## Verify

```bash
cargo test -p sil --test e2e_source
cargo clippy -p sil --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Files changed, how parse_error is shown, confirmation download-failure e2e still fails the command.
