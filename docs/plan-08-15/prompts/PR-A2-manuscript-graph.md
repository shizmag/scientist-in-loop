# PR-A2 - TeX dependency graph, citations, labels, and assets

## Role

LaTeX graph engineer. Own deterministic static manuscript analysis in `sil-latex`.

## Goal

Build one scoped, comment-aware dependency snapshot for the configured main TeX file and expose structured citations, labels, assets, citation contexts, and findings through the A1 contract.

## Requirements

1. Resolve reachable `\input`/`\include` files relative to the including file, with stable ordering, cycle detection, missing-input evidence, and no accidental duplicate scan of `.sil/draft_sections`.
2. Strip/understand TeX comments sufficiently that commented citations/labels/assets are inactive; document unsupported dynamic macro cases.
3. Parse supported citation and reference macro families consistently across current CLI/TUI behavior.
4. Detect duplicate labels, undefined references even when no labels exist, undefined citation keys even when bibliography is empty/missing, and duplicate BibTeX keys.
5. Resolve `\includegraphics`, configured extensions, nested relative paths, and `\graphicspath`; canonicalize every dependency. Allow only the project root or canonical roots explicitly declared by config (including existing absolute paths); record external roots. A manuscript/runtime reference cannot add a new root.
6. Produce one canonical dependency list and citation-context records. Move CLI-private asset logic behind this API without changing CLI in this PR.
7. Findings are current-state errors/warnings/observations only. Do not compare file hashes to prior runs.

## Tests

Nested includes, cycle, comments, multiline/common macros, missing/empty bib, duplicate keys/labels, undefined ref with no labels, graphicspath, relative assets, duplicate asset references, path escape, deterministic order.

## Out of scope

Compiler execution, CLI/TUI/MCP wiring, venue/template constraints, full TeX macro expansion.

## Verify

```bash
cargo test -p sil-latex
cargo clippy -p sil-latex --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Supported syntax contract, fixture coverage, limitations, no commit.
