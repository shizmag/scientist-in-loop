# PR-R1 — Fetch + parse composite

Copy the block below into an agent session. **After D1.**

---

## Role

You are the **ingest engineer** for scientist-in-loop. Ship ONLY PR-R1.

## Goal

Digest `Enter` and Sources add-link (`a`) download **and parse** (`sil_app::fetch_source` with `parse=true`). Do **not** auto-open the reader. Status tells the scientist the paper is ready to Open on tab 2.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.3, KD-6, KD-7
- Today: TUI fetch jobs use `parse=false` (ADR-015 residual). `sil_app::fetch_source` already accepts a parse flag.
- Dashboard digest Enter is C3 (`docs/plan-08-13`). Sources `a` queues fetch.
- Sources `e` / `E` stay parse-only for on-disk files.

## Shared invariants

1. Minimal diff. Reuse `sil_app::fetch_source`. Do not fork download.
2. Never auto-commit. Never auto-open reader. `open_after_parse` default false if you add the setting — prefer **do not add the setting** unless already specified; plan says optional, default false. Skip the setting; just don't auto-open.
3. Register `FetchParse` / `OpenSource` on the palette if D1 is present.
4. Clippy clean.

## Requirements

1. Change TUI fetch queue used by digest Enter and add-source modal to `parse: true`.
2. On success: reload sources list; status like “parsed — Open from Sources or palette”.
3. On parse failure after a successful download: keep the file on disk; surface `UserError` (`parse.failed`) if T2 landed; still no auto-open.
4. Sources `e` remains parse-only (no re-fetch).
5. Unit tests:
   1. Digest Enter / queue_fetch uses `parse: true` (assert on the call args via a test hook or by inspecting job payload).
   2. Success path does not set `reading_md_content`.
   3. Existing fetch job chrome (`J`, in-flight set) still works.

## Out of scope

- Auto-open reader
- Badges (R3) — R3 depends on this PR but is separate
- Changing CLI `sil source fetch` defaults unless it already has a parse flag you merely pass through

## Verify

```bash
cargo test -p sil-tui -p sil-app
cargo clippy -p sil-tui -p sil-app --all-targets -- -D warnings
```

## Deliverable

Which TUI entry points now parse, how success/failure status reads, residual CLI default.
