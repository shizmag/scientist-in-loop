# PR-C3 — Digest Enter → existing fetch

Copy the block below into an agent session. **Depends on B2** (live digest rows + job chrome).

---

## Role

You are the **digest-inbox engineer** for scientist-in-loop. Ship ONLY PR-C3.

## Goal

On the Dashboard digest pane, let the user highlight a row and press Enter to queue the **existing** TUI fetch job (DOI or URL). Do not invent a new ingest path and do not auto-open the reader.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-13/pr-plan.md` §5.5, KD-13, KD-14
- Dashboard: `crates/sil-tui/src/ui/dashboard.rs` + A1 `DashboardModel`
- Digest rows: `JournalPublication` (`doi`, `url`, `title`, …)
- Existing fetch: TUI `a` / `queue_fetch` → `sil_app::fetch_source` with **`parse=false`** (Stage 12 / ADR-014)
- Classify: `classify_source_input` in `crates/sil-tui/src/app/types.rs`
- Dashboard keys today: tab switch / `?` / `R` — no in-pane cursor

## Shared invariants

1. Minimal diff. Call the existing fetch queue. Do not reimplement download.
2. Never auto-commit. `parse=false` (match current TUI fetch). Do **not** auto-parse. Do **not** switch to Sources or open the reader.
3. One selected digest index. j/k only move that index when `ActiveTab::Dashboard`.
4. Prefer unit tests; clippy clean on touched crates.

## Requirements

1. `App` holds `selected_digest_index` (clamp on list reload).
2. Dashboard digest pane shows a selection marker on the current row (same `►` language as Sources).
3. Keys when `ActiveTab::Dashboard` and no modal:
   - `j` / Down, `k` / Up: move digest selection
   - `Enter`: queue fetch
4. Fetch target resolution:
   - Prefer DOI (`10.…` or `doi` field)
   - Else `url` if it looks like http(s)
   - Else status `cannot fetch (no DOI or URL)` and do not queue
5. After queue: stay on Dashboard; status like `Fetching {title}…` using existing job chrome / `J`.
6. Help overlay `HelpMode::Dashboard`: document j/k + Enter.
7. Tests:
   1. Publication with DOI → target string is that DOI.
   2. Publication with only URL → URL.
   3. Publication with neither → no queue.
   4. Selection clamps when the list shrinks.

## Out of scope

- Auto-parse after fetch
- Auto-open reader
- Multi-select digest
- Changing `sil source digest` CLI
- Watch-list / multiple queries

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Selection state, target resolution helper, fetch-queue call site, residual “user still parses on tab 2”.
