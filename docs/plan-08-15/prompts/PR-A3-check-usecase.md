# PR-A3 - Structured build and shared check use case

## Role

Check-usecase engineer. Own compiler results, the immutable input snapshot, `sil-app` orchestration, and the CLI check adapter.

## Goal

Create `sil_app::run_manuscript_check`, a structured compiler result, and `sil paper check` with quiet draft-default behavior.

## Requirements

1. Build one immutable snapshot from configured paths and A2 dependency graph; compute stable raw/content fingerprints without treating prior values as baselines. Implement the normative `CheckReport { static, run }` shape: canonical findings/metrics/dependencies/template live in byte-stable `static`; build/network/timing/log-path data live only in volatile `run`.
2. Replace path-only build success with structured engine/version/argv/exit/stdout/stderr/duration/error-location/artifact metadata.
3. Prove expected PDF was newly produced by this successful run. Old/stale PDFs cannot satisfy success.
4. Persist optional ignored/capped diagnostics under `.sil/checks/`/`.sil/build/` without making latest a comparison baseline.
5. Compose A1/A2 findings, project invariants, optional build, optional template constraints, and optional explicitly online bibliography checks in `sil-app`.
6. Add `sil paper check --profile draft|submission [--strict] [--online] [--build] [--json] [--verbose] [--all]`.
7. Default text is compact/deduplicated/capped. Result value/word count/hash changes are observations only.
8. Keep `sil paper build` command compatibility while delegating compiler execution to the shared structured path where practical.

## Tests

Fake compilers: success+new PDF, zero/no PDF, nonzero+old PDF, warning+success, engine absent, location parse, timeout. CLI exit/output snapshots for every normative submission blocking code and draft/submission/strict/verbose/json/online-disabled. Repeated identical no-build/no-online input has byte-identical canonical static JSON and stable fingerprint; volatile build fields are excluded and tested separately.

## Out of scope

TUI/MCP/doctor/status/estimate wiring, template release ZIP, previous-run comparison.

## Verify

```bash
cargo test -p sil-latex -p sil-app -p sil
cargo clippy -p sil-latex -p sil-app -p sil --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Use-case/API/CLI summary, fake compiler coverage, noise-policy proof, no commit.
