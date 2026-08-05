# PR-C1 — Golden negative-pattern close (BEE-RAG wrap)

Copy the block below into an agent session. Parallel-safe with A1/B1.

---

## Role

Focused implementer. Ship ONLY PR-C1. Parallel-safe with A1/B1 (touches sil-parse references).

## Goal

Drive golden **Ref negative pattern** gate to PASS (0 polluted refs). Known sole FAIL: BEE-RAG line-wrap continuation extracted as its own reference.

## Repo context

- Segmentation: `crates/sil-parse/src/references.rs` (`split_raw_entries`, entry-start guards, `clean_reference_text`)
- Golden fixtures: `tests/golden_dataset/fixtures/BEE-RAG/` (`gold_references.yaml` has must_not_extract patterns)
- Eval: `crates/sil-parse/tests/golden_dataset_eval.rs` → `target/candidate_extractions/`
- Scorecard: `tests/golden_dataset/reports/candidate_scorecard.md`
- Scoring script: `tests/golden_dataset/scripts/score_against_current.py` (or project’s documented path)

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. Prefer unit tests co-located with modules; keep clippy clean on touched crates.
3. Do not regress golden ref-count band / anchor recall gates while fixing negatives.

## Requirements

1. Reproduce: run golden eval; confirm BEE-RAG polluted continuation (e.g. venue/proceedings wrap like `Language Models. In *The 61st...` style fragments).
2. Fix continuation joining:
   - Prefer: if next line matches continuation prefixes (`In *`, `pp.`, `vol.`, `Proceedings`, mid-title fragments) and lacks year/DOI/arXiv as a real new entry, join to previous.
   - Keep existing incomplete-author guard; extend carefully without tanking ref-count bands.
3. Unit test with the exact failure substring/fixture slice.
4. Re-run golden eval + scorecard; **negative patterns 100% clean**; do not regress ref count band / anchor recall gates.
5. Update `candidate_scorecard.md` if that is part of repo workflow.

## Out of scope

- Parent author F1 campaigns (HiChunk/BEE-RAG authors)
- Official resolve (PR-C2)
- xberg vs MD policy overhaul

## Verify

```bash
cargo test -p sil-parse --test golden_dataset_eval
# plus score script per tests/golden_dataset/EVALUATION.md or README
cargo clippy -p sil-parse --all-targets -- -D warnings
```

## Deliverable

Before/after pollution counts; any fixtures still soft-weak on field precision.
