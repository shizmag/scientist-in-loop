# Working with the paper

- The single source of truth for the high-level plan is `.sil/structure.yaml`.
- Update the `completion` field of a section when you change its status.
- Write all new content into `paper_draft.tex`.
- Only promote content to `paper.tex` when the corresponding sections are at least `draft`.
- Keep claims in `structure.yaml` short and precise; the detailed text belongs in the `.tex` file.
- When you add a figure, reference it both in the `.tex` and in the appropriate figures/*/README.md.

## structure.yaml fields
- `completion` must be one of: `empty` | `outline` | `draft` | `polished`.
- `main_claim` and `secondary_points` are free text but should stay concise.
- Agents must update `completion` when they change the corresponding part of the paper.
- Every section has an `id`, `title`, `level`, and optional `required_content` checklist.

## Manuscript conventions
- Prefer small, reviewable edits over large rewrites.
- Keep section titles in the `.tex` aligned with `.sil/structure.yaml` when practical.
- Do not invent new top-level project folders for paper content.
- For focused edits, open a single file under `.sil/draft_sections/` (from `sil split`) instead of scanning all of `paper_draft.tex`; always write final prose back into `paper_draft.tex`.
- File improvement ideas under `.sil/improvement/suggestion_n` (see that directory’s README).
