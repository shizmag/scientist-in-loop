# SYSTEM RULES FOR THIS PROJECT

You are working inside a `sil`-managed scientific project.

## Directory conventions (do not invent new top-level folders)
- `sources/`          – original PDFs only
- `data/`             – experimental / collected data (see data/README.md)
- `figures/plots/`    – code-generated plots
- `figures/images/`   – external images
- `agent/`            – code that you (the agent) write
- `paper_draft.tex`   – the working manuscript (source of truth for prose)
- `paper.tex`         – the cleaned version (created later)
- `agent/skills/`     – agent skill definitions
- `.sil/`             – configuration, database
- `.sil/draft_sections/` – deterministic per-section split of `paper_draft.tex` (agent cache; regenerate with `sil split`; do not edit as source of truth)
- `.sil/improvement/` – improvement proposals as `suggestion_n` (versioned; not gitignored)

## Mandatory workflow
1. Always read this SYSTEM.md first (located at `agent/skills/SYSTEM.md`).
2. Read additional skill files (`agent/skills/paper.md`, `agent/skills/agent-code.md`) on demand when relevant to your task.

3. Consult `.sil/structure.yaml` before changing the paper.
4. After any significant change, a commit will be proposed. Write a clear commit message and keep the `Sci-Action` trailer that `sil` suggests.
5. Never auto-commit. Never create new top-level directories.
6. When you add data or figures, update the corresponding README.md.

## Skill loading rules (Thin-Server Local Skill Routing)
- **SYSTEM.md** (`agent/skills/SYSTEM.md`) is the agent routing index and is always included in the default context payload.
- **paper.md** (`agent/skills/paper.md`) - Load on demand when the task touches `structure.yaml`, `paper_draft.tex`, `paper.tex`, or section completion.
- **agent-code.md** (`agent/skills/agent-code.md`) - Load on demand when the task creates, modifies, or references anything inside `agent/`.
- **review.md** (`agent/skills/review.md`) - Load on demand for manuscript estimate / peer-review / critique tasks (`sil paper estimate`, multi-perspective review).

## Context
Use `sil context` to obtain a fresh, structured view of the project state.

## Idea & TODO Blocks (# -- X -- #)
- In `paper_draft.tex`, human scientists or AI agents bound ideas, questions, or TODO notes using:
  ```latex
  % # -- X -- #
  % TODO: Re-evaluate section 3 baseline comparisons.
  % Idea: Add an ablation table comparing model A vs model B.
  % # -- X -- #
  ```
- Use `sil todo` or `sil context` to inspect active idea/TODO blocks parsed into SQLite memory.
- When an idea/TODO item is completed in prose, remove or update the `# -- X -- #` block.

