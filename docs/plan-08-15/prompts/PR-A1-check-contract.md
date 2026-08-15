# PR-A1 - Check report contract and quiet policy

## Role

Check-contract engineer. Own domain types and policy only.

## Goal

Introduce a stable, serializable deterministic check contract in `sil-core` that separates invariant errors, actionable warnings, and observations. Encode draft/submission/strict exit policy without adding TeX parsing or UI behavior.

## Requirements

1. Read Section 5 KD-A1 through KD-A12 and contract 6.1 in `../pr-plan.md`.
2. Add `FindingClass`, stable-code `CheckFinding`, `CheckProfile`, `CheckSummary`, deterministic `CheckStaticReport`, volatile `CheckRunMetadata`, combined `CheckReport { static, run }`, and input-fingerprint support. The static object is the canonical byte-stable artifact; build/network/timing fields can only live in `run`.
3. Keep blocking policy separate from finding class. Implement the exact plan Section 6.1 matrix: draft blocks invariant errors; submission blocks its normative core warning-code set plus template additions; strict blocks every actionable warning. Observations never block.
4. Add deterministic ordering/deduplication helpers and a compact formatter contract with a 20-finding default cap.
5. Do not add implicit previous-run comparison, result drift, artifact staleness, network access, or score policy.
6. Preserve or provide a deliberate migration path for existing `DiagnosticLevel`/`ManuscriptHealthReport` consumers; do not break unrelated crates without adapters/tests.
7. Stable codes and JSON fields are API. Include `schema_version` and project-relative path shape.

## Tests

- Serialization/round-trip and deterministic ordering.
- Deduplication by code/path/line/evidence.
- Draft warnings exit zero; invariant errors fail; submission/strict promotions are explicit.
- Observations never fail.
- Twenty-five findings are capped in compact output but retained in JSON/report.
- No test or type contains implicit baseline/last-run logic.

## Out of scope

TeX scanning, compiler execution, CLI command, doctor/TUI/MCP, venue/template policy.

## Verify

```bash
cargo test -p sil-core
cargo clippy -p sil-core --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Contract/API summary, compatibility notes, test results, no commit.
