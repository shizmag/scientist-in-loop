---
id: paper
version: 1.0.0
title: Manuscript Drafting and Structure
triggers:
  - paper
  - draft
  - latex
  - structure
  - tex
  - write
  - section
  - abstract
  - introduction
required_capabilities:
  - tectonic
inputs:
  - paper_draft.tex
  - structure.yaml
outputs:
  - paper_draft.tex
  - structure.yaml
permissions:
  - "write:paper_draft.tex"
  - "write:structure.yaml"
verification: "sil build"
---
# Working with the paper: Manuscript Drafting and Structure

Guidelines and operational workflow for editing scientific manuscripts and keeping `.sil/structure.yaml` aligned.

## Core Rules

- The single source of truth for the high-level plan is `.sil/structure.yaml`.
- Write all new content into `paper_draft.tex`.
- Only promote content to `paper.tex` when the corresponding sections are at least `draft`.
- Keep claims in `structure.yaml` short and precise; the detailed text belongs in the `.tex` file.
- When you add a figure, reference it both in the `.tex` and in the appropriate `figures/*/README.md`.

## Workflow: Inspect -> Propose -> Modify -> Verify

### 1. Inspect
- Inspect `.sil/structure.yaml` to check section IDs, required content checklists, and current completion levels (`empty`, `outline`, `draft`, `polished`).
- For focused edits, open the corresponding section file under `.sil/draft_sections/` (from `sil split`) or inspect `paper_draft.tex`.
- Check active idea or TODO blocks (`% # -- X -- #`) using `sil todo` or `sil context`.

### 2. Propose
- Formulate focused, reviewable edits rather than large rewrites.
- Plan the specific updates to `paper_draft.tex` and corresponding `completion` transitions in `.sil/structure.yaml`.
- File major improvement proposals under `.sil/improvement/suggestion_n` (see that directory's README).

### 3. Modify
- Apply changes directly to `paper_draft.tex` (always write final prose back into `paper_draft.tex`, not `.sil/draft_sections/`).
- Update the `completion` field in `.sil/structure.yaml` when you change the status of a section.
- If resolving idea/TODO items, remove or update the corresponding `% # -- X -- #` comments.

### 4. Verify
- Run verification command: `sil build` (or `sil check`) to compile and validate the LaTeX document.
- Ensure that the manuscript compiles cleanly with `tectonic`.
- Verify that section titles in `paper_draft.tex` stay aligned with `.sil/structure.yaml`.

## structure.yaml fields
- `completion` must be one of: `empty` | `outline` | `draft` | `polished`.
- `main_claim` and `secondary_points` are free text but should stay concise.
- Agents must update `completion` when they change the corresponding part of the paper.
- Every section has an `id`, `title`, `level`, and optional `required_content` checklist.

## Manuscript conventions
- Prefer small, reviewable edits over large rewrites.
- Keep section titles in the `.tex` aligned with `.sil/structure.yaml` when practical.
- Do not invent new top-level project folders for paper content.
