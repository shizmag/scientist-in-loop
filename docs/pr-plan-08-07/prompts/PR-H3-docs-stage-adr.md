# PR-H3 — Docs, STAGES, ADR hygiene

## Role

Docs agent. Ship ONLY PR-H3. Last after code PRs.

## Goal

Docs claim only what code does (especially ONNX feature truth). ADR numbering fixed.

## Requirements

1. ADR-007 keep parent-metadata; renumber split-view → ADR-008 + redirect header.
2. Write ADR-011 onnx feature + MCP bib + SciAction.
3. Update all in-repo links; STAGES Stage 8+; README MVP table truth.
4. Remove `/Volumes/happy-disk` hardcodes; point to `~/.cache/sil/models`.
5. Cross-link `docs/pr-plan-08-07/`.
6. Command names match clap.

## Out of scope

- Logic changes; new features

## Verify

Spot-check README vs doctor RAG line vs ONNX feature; no dual ADR-007.

## Deliverable

Files changed, residual risk.
