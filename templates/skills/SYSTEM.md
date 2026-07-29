# SYSTEM RULES FOR THIS PROJECT

You are working inside a `sil`-managed scientific project.

## Directory conventions (do not invent new top-level folders)
- `sources/`          – original PDFs only
- `data/`             – experimental / collected data (see data/README.md)
- `figures/plots/`    – code-generated plots
- `figures/images/`   – external images
- `agent/`            – code that you (the agent) write
- `paper_draft.tex`   – the working manuscript
- `paper.tex`         – the cleaned version (created later)
- `.sil/`             – configuration, database, skills (do not put paper content here)

## Mandatory workflow
1. Always read this SYSTEM.md first.
2. Read the additional skill files that are relevant to your current task (see loading rules).
3. Consult `.sil/structure.yaml` before changing the paper.
4. After any significant change, a commit will be proposed. Write a clear commit message and keep the `Sci-Action` trailer that `sil` suggests.
5. Never auto-commit. Never create new top-level directories.
6. When you add data or figures, update the corresponding README.md.

## Context
Use `sil context` to obtain a fresh, structured view of the project state.
