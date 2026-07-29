# Rules for code written by the agent

- All helper scripts, parsers, and reproducibility utilities that you create must live in `agent/`.
- Every script must be documented in `agent/README.md` (purpose + how to run it).
- Prefer small, focused scripts over large monolithic ones.
- Make the code reproducible: fixed seeds, explicit dependency lists, clear input/output paths.
- When a script produces a plot, save it under `figures/plots/` and update `figures/plots/README.md`.
- When a script produces data, save it under `data/` and update `data/README.md`.
