# ADR-007: Split-View References TUI and Direct `sil tui` Entrypoint

## Status
Accepted

## Context
1. `sil tui` previously required subcommand aliases like `sil tui dashboard`, but the `Dashboard` screen is deprecated in favor of unified direct workflow access.
2. Managing academic paper references requires comparing and copying extracted source literature citations into the paper's main BibTeX file (`references.bib`).

## Decision Drivers
1. **Direct TUI Entrypoint**: `sil tui` now launches the interactive terminal user interface directly. `Dashboard` subcommand is deleted.
2. **BibTeX Conversion**: `ReferenceEntry::to_bibtex` in `sil-core` formats extracted reference items into standard `@article{cite_key, title={...}, author={...}, journal={...}, year={...}, doi={...}}` BibTeX blocks.
3. **Split-Screen Vertical References View**:
   - **Left Pane (`references.bib`)**: Displays all active entries from the manuscript's `references.bib` file.
   - **Right Pane (Source References)**: Displays extracted source citations from the SQLite database (`source_references`).
   - **Tab Key**: Seamlessly toggles active focus between Left Pane and Right Pane.
   - **`Space` Marking & `p` (Paste)**: Items in the Right Pane can be marked/selected with `Space` (showing `[x]`). Pressing `p` formats marked items as BibTeX and appends them directly to `references.bib` on disk.
   - **`/` Search Filter**: Real-time query filtering across titles, authors, venues, and citation text.

## Consequences
- Research workflow efficiency is significantly improved: users can visually browse extracted literature references alongside `references.bib` and append citations with a single keypress.
- Clean separation between library crates (`sil-core`, `sil-db`) and UI views (`sil-tui`).
