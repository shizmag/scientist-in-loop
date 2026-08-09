# ADR-012: Manuscript Estimate, Co-Author MCP, and Stage 9 Governance

## Status
Accepted (partial implementation in Wave 09-08)

## Context
After Stage 8, agents could manage bibliography and structure but lacked:

1. A first-class **manuscript estimate / multi-perspective review** path.
2. MCP tools to **edit draft sections** and **ground claims** against literature.
3. Honest documentation of MCP tool counts and optional ONNX.

[Academic Research Skills](https://github.com/Imbad0202/academic-research-skills) (`academic-paper-reviewer`) provides a mature multi-persona review methodology under **CC-BY-NC 4.0**. sil must not vendor that tree; it adopts the *pattern* with original skill prose and attribution.

## Decision

1. **Estimate skill** lives at `agent/skills/review.md` (+ `review/` supporting files), installed by `sil init` / `sil init --update`.
2. **L0 heuristic** runs offline in `sil-agent::estimate` (structure completion, word count, empty sections, missing cites, TODOs) → JSON schema v1 + markdown report.
3. **CLI** `sil paper estimate [--mode] [--json] [--write]` writes only under `.sil/reviews/`; never mutates `paper_draft.tex`.
4. **MCP** `sil_estimate_paper`, `sil_edit_section`, `sil_ground_claims` return Sci-Action proposals and set `never_committed: true`.
5. **Sci-Action** variants: `estimate-paper`, `ground-claims`.
6. **Advisory lock** `.sil/workspace.lock` coordinates writers without hard mutex.
7. **Attribution** in skill and report JSON citing ARS academic-paper-reviewer (CC-BY-NC 4.0); sil-native text.

## Consequences

- Agents can estimate and patch drafts without TUI while humans retain git authority.
- Scores are labeled L0/L1 and are **not** peer-review truth.
- Full Stage 9 tracks (golden fixture lifts, embed cache, Releases, TUI estimate job) remain in `docs/pr-plan-09-08/`.
