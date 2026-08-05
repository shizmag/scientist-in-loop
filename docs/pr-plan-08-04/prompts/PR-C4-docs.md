# PR-C4 — Docs & ADR consolidation

Copy the block below into an agent session after the code PRs you are documenting have landed.

---

## Role

Docs implementer. Ship ONLY PR-C4 after the code PRs you are documenting have landed (or document only merged subset).

## Goal

Align docs with implemented behavior for bib lifecycle, TUI jobs/help, resolve chain, digest.

## Files to update (as applicable)

- `docs/adr/ADR-009-background-bibtex-hydration.md` (races, re-mark rules, job chrome if done)
- `docs/similarity_and_sources_bib.md` (upsert completeness, cite-key preserve, pretty format)
- `docs/reference_extraction_pipeline.md` (resolve fallback + confidence; segmentation continuation)
- `docs/source_extraction_and_tui_sorting.md` if sort keys / help changed
- `README.md` TUI + marker wording (`% [sil: tui-added]` vs status unproved)
- Optional new: `docs/adr/ADR-010-bib-lifecycle-tui-jobs-and-parse-hardening.md` summarizing normative policies
- Keep this plan pack accurate: `docs/pr-plan-08-04/pr-plan.md` status if needed

## Shared invariants

1. No aspirational docs for unmerged features — only describe what is in the tree
2. Code changes beyond tiny comment fixes are out of scope
3. Keep ADRs concise; link to code modules

## Requirements

1. Include cite-key preserve, completeness upsert, promote/hydrate rule, footer job chrome, `?` help if present
2. Document resolve fallback chain and Crossref confidence threshold if C2 landed
3. Document digest native-first behavior if C3 landed
4. Markdown links must resolve

## Out of scope

- Marketing rewrite of entire README
- Implementing missing features just to document them

## Verify

Skim for contradictions with `app.rs` / `bib.rs` / `journal_digest.rs`. Confirm links under `docs/` exist.

## Deliverable

List of doc files + one-paragraph release note for the feature set.
