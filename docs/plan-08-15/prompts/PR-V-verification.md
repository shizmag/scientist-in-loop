# PR-V - Stage 15 verification gate

## Role

Read-mostly verifier/test engineer. Add missing tests/fixtures only; do not add product behavior.

## Goal

Prove the Stage-15 contracts across crates and surfaces, including quiet check policy, venue ambiguity, provider partial failure, package confinement, deterministic releases, MCP conformance, safe installers, and licensing boundaries.

## Requirements

1. Read all verification matrices in `../pr-plan.md` Sections 10 and 11 and map each row to a test or documented manual check.
2. Run focused crate tests first, then workspace test/clippy/fmt. Record pre-existing versus introduced failures accurately.
3. Add one offline scenario fixture used across CLI/TUI/MCP: included manuscript, citations/assets, fake compiler, discovery providers, candidates, template/skill locks.
4. Prove ordinary scientific table/plot/value/hash/word-count changes do not fail draft check and are not compared to last run.
5. Validate catalogue target/count/provenance/collisions and alias hard cases.
6. Run provider error/pagination/partial-run fixtures with no public network.
7. Run package traversal/symlink/hash/rollback and two-run byte-identical release tests.
8. Run MCP protocol/conformance/root/confinement/cancellation/structured-result and six-name tests.
9. Run installer backup/idempotency/uninstall/OpenCode fixtures.
10. Audit embedded assets for ARS content; verify Visualize Article/third-party notices.
11. Produce `docs/plan-08-15/verification-report.md` with command results, scenario matrix, residual failures, and ship/no-ship verdict.

## Out of scope

Feature redesign, weakening tests to pass, live provider credentials, external experiment runner.

## Verify

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Verification report, added test/fixture list, exact failures, ship/no-ship verdict, no commit.
