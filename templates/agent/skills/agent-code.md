# Rules for code written by the agent

- All helper scripts, parsers, and reproducibility utilities that you create must live in `agent/`.
- Every script must be documented in `agent/README.md` (purpose + how to run it).
- Prefer small, focused scripts over large monolithic ones.
- Make the code reproducible: fixed seeds, explicit dependency lists, clear input/output paths.
- When a script produces a plot, save it under `figures/plots/` and update `figures/plots/README.md`.
- When a script produces data, save it under `data/` and update `data/README.md`.

## Documentation requirements
- For each script in `agent/README.md`, record: purpose, how to run it, inputs, and outputs.
- Prefer pinned or listed dependencies when the environment is non-trivial.
- Prefer relative paths from the project root so agents and humans can re-run the same commands.

## Integration with the paper workspace
- Never place helper code in `sources/`, `data/`, or the project root unless it is a documented exception.
- After generating plots or data, keep figure references in `paper_draft.tex` consistent with the README lists.
