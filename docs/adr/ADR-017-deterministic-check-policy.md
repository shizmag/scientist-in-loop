# ADR-017: Deterministic Check Policy

## Status

Accepted for Stage 15; ship status is gated by `docs/plan-08-15/verification-report.md`.

## Context

The workspace needs one check contract shared by CLI, TUI, MCP, builds, and
estimates without treating legitimate scientific edits as regressions.

## Decision

`sil paper check` evaluates current-state invariants from one normalized input
snapshot. Draft output is quiet and capped; `--json`, `--verbose`, and `--all`
expose additional detail. `draft`, `submission`, and `strict` are explicit
profiles, and network checks require `--online`.

The report separates invariant errors, actionable warnings, and observations.
There is no implicit baseline, stale-result alarm, or scientific-truth claim.
Changes to values, plots, hashes, word counts, and estimate scores are
observations and do not fail the check merely because they changed.

## Consequences

Consumers can cache and compare the deterministic static report, while volatile
build and network metadata stays in the run envelope. Submission policy can be
stricter without changing the draft default.
