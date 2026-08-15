# PR-A4 - Check parity across doctor, status, TUI, estimate, and MCP

## Role

Check-surface engineer. Make existing surfaces thin adapters over A3.

## Goal

Remove duplicated health/build interpretation so CLI doctor/status/assets/build, TUI dashboard/jobs, L0 estimate, and MCP review expose one report fingerprint and policy.

## Requirements

1. `sil project doctor` embeds/links deterministic check results but keeps host/recovery checks separate. Online checks stay explicit.
2. `sil status --json` includes stable check summary/fingerprint without mutating global recent-project state.
3. `sil paper assets` delegates to dependency data and preserves documented JSON compatibility or migration notes.
4. TUI stores/caches one report, invalidates on relevant input change/reload, shows compact class counts/details, and does not rerun during every render.
5. Build job uses structured result/log/error location. Dashboard TODO totals are not capped by preview rows.
6. `sil-agent` estimate consumes the shared report and the same structure/config across CLI/TUI/MCP. Preserve L0 scoring unless fixing an identified parity bug.
7. MCP adds `sil_review action=check`; `build` actually calls A3. Structured outputs match CLI JSON semantics and remain never-auto-commit.
8. Keep six MCP tool names.

## Tests

One fixture has identical fingerprint/counts across CLI/TUI/MCP/doctor/status/estimate; repeated TUI render no rerun; external edit invalidates; 20 TODO total; MCP build executes; estimate parity; status read-only; draft warning stays nonblocking.

## Out of scope

New checker rules, estimate rubric redesign, discovery surfaces, template archive.

## Verify

```bash
cargo test -p sil-agent -p sil-tui -p sil-mcp -p sil
cargo clippy -p sil-agent -p sil-tui -p sil-mcp -p sil --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Surface mapping, parity fixture results, behavior changes, no commit.
