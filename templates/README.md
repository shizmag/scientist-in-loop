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
| `paper_draft.tex` | Working manuscript (source of truth) |
| `paper.tex` | Cleaned manuscript (promoted later) |
| `references.bib` | Bibliography |
| `.sil/` | Config, structure map, rebuildable SQLite DB, checks, locks, and staged builds |
| `.sil/draft_sections/` | Per-section split of the draft for agents (`sil paper split`) |
| `agent/skills/managed/` | Verified managed skill-pack projections |
| `agent/skills/local/` | User-authored skills, never overwritten by updates |
| `.sil/improvement/` | Improvement proposals as `suggestion_n` (tracked) |

## Quick commands

```bash
sil status [--json]     # project overview
sil source parse        # parse unparsed PDFs in sources/
sil source list         # parsed vs unparsed sources
sil source search "query" # full-text search over parsed sources
sil project context       # agent/human context dump
sil paper split            # refresh .sil/draft_sections/ from paper_draft.tex
sil git propose            # Sci-Action proposal (never auto-commits)
sil paper promote          # copy draft -> paper.tex + propose
sil paper structure set … # update section completion
sil source cite <source|q> # suggest BibTeX / \cite{…}
sil paper check            # deterministic current-state invariants
sil paper template install ./pack --approve
sil paper template verify <manifest-id>
sil paper template stage <manifest-id>
sil paper build [--source-only] # compile or explicitly publish sources
sil project mcp --project <path> --quiet
sil mcp install|status|uninstall
sil init --update       # refresh skills / managed .gitignore after upgrading sil
```

Large artifacts (SQLite DB, PDFs everywhere including `sources/`, figure/image binaries, experiment data under `data/`) are
gitignored by default. Document literature and assets in folder READMEs. `.sil/improvement/` and `.sil/draft_sections/` are
**not** gitignored.

## For agents

1. Read `agent/skills/SYSTEM.md` first.
2. Run `sil context` (add `--paper`, `--agent`, or skill flags as needed).
3. For focused section reads, open a file under `.sil/draft_sections/` (after `sil split`); write prose back to `paper_draft.tex`.
4. File improvement ideas under `.sil/improvement/suggestion_n`.
5. Update `.sil/structure.yaml` when changing the high-level plan.
6. Never auto-commit; use the Sci-Action commit proposals that `sil` prints.
