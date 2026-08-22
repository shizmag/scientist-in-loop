---
id: agent-code
version: 1.0.0
title: Agent Reproducibility and Code
triggers:
  - agent-code
  - agent
  - python
  - script
  - parser
  - reproducibility
required_capabilities:
  - python
  - git
inputs:
  - agent/
outputs:
  - agent/
permissions:
  - "write:agent/"
verification: "python -m pytest"
---
# Rules for code written by the agent

Operational workflow and requirements for helper scripts, parsers, analysis routines, and reproducibility utilities written under `agent/`.

## Core Rules

- All helper scripts, parsers, and reproducibility utilities that you create must live in `agent/`.
- Every script must be documented in `agent/README.md` (purpose + how to run it).
- Prefer small, focused scripts over large monolithic ones.
- Make the code reproducible: fixed seeds, explicit dependency lists, clear input/output paths.
- When a script produces a plot, save it under `figures/plots/` and update `figures/plots/README.md`.
- When a script produces data, save it under `data/` and update `data/README.md`.
- Never place helper code in `sources/`, `data/`, or the project root unless it is a documented exception.

## Workflow: Inspect -> Propose -> Modify -> Verify

### 1. Inspect
- Inspect `agent/README.md` and existing scripts in `agent/` to avoid duplicating functionality.
- Review existing data files in `data/` and figure targets in `figures/plots/`.
- Check required Python dependencies and runtime environment.

### 2. Propose
- Define the script's interface, CLI arguments, expected inputs/outputs, and random seed handling.
- Plan corresponding documentation updates for `agent/README.md`, `data/README.md`, or `figures/plots/README.md`.

### 3. Modify
- Write modular Python code in `agent/`.
- Use relative paths from the project root so humans and agents can re-run the same commands.
- For each script in `agent/README.md`, record: purpose, how to run it, inputs, and outputs.
- After generating plots or data, keep figure references in `paper_draft.tex` consistent with the README lists.

### 4. Verify
- Run test and execution checks: `python -m pytest` (or execute the script with sample inputs).
- Verify that generated output files exist at expected relative paths and conform to schema.
- Verify git status to confirm only intended files under `agent/`, `data/`, or `figures/` are modified.

## Documentation requirements
- For each script in `agent/README.md`, record: purpose, how to run it, inputs, and outputs.
- Prefer pinned or listed dependencies when the environment is non-trivial.
- Prefer relative paths from the project root so agents and humans can re-run the same commands.
