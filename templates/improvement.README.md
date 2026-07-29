# Improvement proposals

Store concrete improvement proposals here as `suggestion_n` files or directories
(e.g. `suggestion_1`, `suggestion_2`, …). Numbering is free-form natural numbers;
agents and humans pick the next free `n`.

## Convention

```
.sil/improvement/
  README.md          # this file
  suggestion_1       # file or directory with a proposal
  suggestion_2/
    README.md
    notes.md
  …
```

Each proposal should describe: problem, proposed change, affected paths, and
expected benefit. This tree is **not** gitignored — version proposals with the
project so reviews can track them.

Do not put paper manuscript content here; write prose in `paper_draft.tex` and
structure in `.sil/structure.yaml`.
