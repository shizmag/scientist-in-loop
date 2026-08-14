# PR-O3 — Doctor-as-guide (human hints)

Copy the block below into an agent session. **After T2.**

---

## Role

You are the **doctor engineer** for scientist-in-loop. Ship ONLY PR-O3.

## Goal

Make `sil project doctor` speak English. Each check gets an optional `hint` (install / fix line). Human output shows the hint on failures. JSON adds `hint` backward-compatibly. Do **not** implement `--repair-db` (T5).

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.12, KD-17
- Today: `crates/sil/src/commands/doctor.rs` — `Check { name, ok, detail, extra }`. Integrity is reported, not repaired. `--fix` repairs bib entries. `--fix-rag` scaffolds ONNX dirs.
- E2E: `crates/sil/tests/e2e_doctor.rs`
- Use `UserError` / hint catalog from T2 where it fits (engine missing, python missing). Do not invent a second catalog.

## Shared invariants

1. Minimal diff.
2. Never auto-commit.
3. Do not delete or rebuild SQLite.
4. Host checks stay non-fatal where they already are (uv optional, etc.).
5. Clippy clean; e2e doctor still green.

## Requirements

1. Extend `Check` with `hint: Option<String>` (serde skip if none).
2. Failed/missing tools get a concrete hint, e.g.:
   - git missing → install git
   - no latex engine on PATH → `brew install tectonic` / distro equivalent (one short line, not an essay)
   - sqlite integrity not `ok` → “run `sil project doctor --repair-db` (T5) after that PR; until then restore from backup”
   - For T5-not-shipped: hint may say “database integrity failed; do not delete sources/; backup db.sqlite”
3. Human renderer: `✗ <name> — <detail>` plus indented hint when present.
4. `--json` includes `hint` when set. Old consumers ignore unknown fields — still valid JSON.
5. Tests:
   1. e2e doctor JSON parses and, on a forced-missing optional or the integrity/engine check, schema allows `hint`.
   2. Unit-test the hint picker: `tectonic` absent → hint contains install guidance.
   3. Existing e2e `doctor_reports_project_checks` / `doctor_json_has_checks` still pass (extend assertions, do not drop them).

## Out of scope

- `--repair-db` (T5)
- TUI wizard (O1)
- Changing `--fix` bib semantics
- Rewriting install.sh

## Verify

```bash
cargo test -p sil --test e2e_doctor
cargo test -p sil-core
cargo clippy -p sil --all-targets -- -D warnings
```

## Deliverable

Check schema change, hint table, JSON sample, residual “integrity cannot be auto-fixed until T5”.
