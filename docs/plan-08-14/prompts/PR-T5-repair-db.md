# PR-T5 — `sil project doctor --repair-db`

Copy the block below into an agent session. **After O3.**

---

## Role

You are the **doctor engineer** for scientist-in-loop. Ship ONLY PR-T5.

## Goal

When SQLite is corrupt or integrity fails, `sil project doctor --repair-db` copies `db.sqlite` aside, creates a fresh DB, and re-parses on-disk sources (best effort). **Never delete `sources/`.** `--fix` still means bib repair only.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.12, KD-16
- Today: `PRAGMA integrity_check` is reported. O3 added hints. No rebuild.
- Parse pipeline: `sil-parse` + `SilDb`. Reuse `allow_reparse` / upsert paths. Do not drop PDFs.
- Palette command `RepairDb` if D1 exists — call the same use-case (prefer `sil-app` wrapper to avoid TUI depending on the binary).

## Shared invariants

1. Minimal diff. Refuse if `sources/` is missing.
2. Never auto-commit. Never `remove_dir_all` on `sources/`.
3. Backup `db.sqlite` → `db.sqlite.corrupt-<ts>` (same directory).
4. Clippy clean. E2E required.

## Requirements

1. CLI flag `--repair-db` on `sil project doctor` (orthogonal to `--fix`).
2. Algorithm:
   1. Abort if no project / no `sources/` directory.
   2. If db exists, copy to `db.sqlite.corrupt-<ts>`.
   3. Remove (or replace) the live db file **only after** backup succeeds.
   4. Open fresh `SilDb`.
   5. For each file in `sources/` that looks like a source (pdf/md/txt/html), parse best-effort; collect per-file ok/fail.
   6. Print a report. Integrity on the new db should be `ok`.
3. TUI: palette `Repair database` runs the same function (confirm modal — this is destructive to the **index**, not to PDFs).
4. Tests:
   1. e2e: create a project, trash/corrupt the db (write garbage), `--repair-db`, db opens, `sources/` file count unchanged.
   2. Missing `sources/` → error, no backup dance that deletes things.
   3. Grep/test: implementation must not call `remove_dir_all` on the sources path.

## Out of scope

- Repairing bib (`--fix` already does)
- Changing Marker
- Network fetch during repair

## Verify

```bash
cargo test -p sil --test e2e_doctor
cargo test -p sil-db
cargo clippy -p sil -p sil-db -p sil-app --all-targets -- -D warnings
```

## Deliverable

Flag wiring, backup name, per-source report, TUI confirm, proof `sources/` is untouched.
