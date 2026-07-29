# Scientific paper project

This directory is managed by [`sil`](https://github.com/scientist-in-loop/scientist-in-loop) (scientist-in-loop).

## Layout

| Path | Purpose |
|------|---------|
| `sources/` | Original scientific PDF files only |
| `data/` | Experimental / collected data |
| `figures/plots/` | Code-generated plots |
| `figures/images/` | External images |
| `agent/` | Helper code written by the agent |
| `paper_draft.tex` | Working manuscript |
| `paper.tex` | Cleaned manuscript (promoted later) |
| `references.bib` | Bibliography |
| `.sil/` | Config, structure map, SQLite DB, skills |

## Quick commands

```bash
sil status              # project overview
sil parse               # parse unparsed PDFs in sources/
sil search "query"      # full-text search over parsed sources
sil context             # agent/human context dump
sil build               # compile LaTeX
sil log                 # Sci-Action annotated git history
```

## For agents

1. Read `.sil/skills/SYSTEM.md` first.
2. Run `sil context` (add `--paper`, `--agent`, or skill flags as needed).
3. Update `.sil/structure.yaml` when changing the high-level plan.
4. Never auto-commit; use the Sci-Action commit proposals that `sil` prints.
