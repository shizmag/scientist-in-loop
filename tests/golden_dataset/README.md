# Golden Dataset Fixture Pack & Evaluation Framework

This directory contains the golden-dataset fixture pack, schemas, validation tools, baseline evaluation reports, and scoring contracts for `scientist-in-loop` metadata and bibliography extraction.

> [!WARNING]
> **DO NOT TREAT `current_extraction.json` AS GOLD LABELS!**
> The `current_extraction.json` files represent raw outputs from the initial/legacy extractor (`crates/sil-parse`) and contain known bugs, in-text citation pollution, missed references, and page span noise.
> Always evaluate extractor performance against `gold_parent.yaml` and `gold_references.yaml`.

---

## Purpose

1. **Isolation**: Isolates Marker-parsed Markdown documents (`content.md`), raw bibliography text blocks (`references_block.md`), and initial extraction outputs without requiring live database connections or PDF re-parsing during tests.
2. **Standardized Benchmarking**: Provides human-labeled ground truth (`gold_parent.yaml`, `gold_references.yaml`) for measuring parent paper metadata quality and reference extraction accuracy.
3. **CI Quality Gates**: Defines objective pass/fail metrics and thresholds (`EVALUATION.md`) to prevent regressions when refactoring or replacing extraction engines.

---

## Python tooling (uv)

From the **repository root** (not this directory):

```bash
uv sync --group dev          # pypdf + pyyaml + jsonschema
uv run tests/golden_dataset/scripts/validate_dataset.py
uv run tests/golden_dataset/scripts/score_against_current.py
```

See root `pyproject.toml` and `uv.lock`. Do not use ad-hoc `pip install` for these scripts.

---

## Directory Map

```text
tests/golden_dataset/
├── README.md                      # Top-level dataset guide and usage instructions
├── EVALUATION.md                  # Metric specifications and CI quality gate contract
├── LABELING.md                    # Guidelines & schema reference for human annotators
├── manifest.yaml                  # Global index of all sources and quality flags
├── schema/                        # JSON Schema validation files
│   ├── manifest.schema.json
│   ├── gold_parent.schema.json
│   └── gold_references.schema.json
├── scripts/
│   ├── export_from_db.py          # Exporter script (creates fixtures from db.sqlite)
│   ├── validate_dataset.py        # Dataset integrity & schema checker script
│   └── score_against_current.py   # Baseline evaluator (scores current extractions vs gold)
├── reports/
│   ├── baseline_scorecard.md      # Auto-generated baseline extraction scorecard
│   ├── parent_label_summary.md    # Summary of labeled parent metadata
│   └── references_label_summary.md# Summary of labeled reference anchors & counts
└── fixtures/
    └── <source_stem>/
        ├── meta.yaml               # Source metadata (PDF path, SHA-256 checksum, byte counts)
        ├── content.md              # Raw Marker-parsed Markdown input text
        ├── references_block.md     # Extracted bibliography section text
        ├── current_extraction.json # Legacy extraction output (DO NOT TREAT AS GOLD)
        ├── gold_parent.yaml        # Ground truth parent metadata & hard negative rules
        └── gold_references.yaml    # Ground truth reference count band, anchors & pattern filters
```

---

## How to Export (Regenerate Fixtures)

To re-export fixtures from the SQLite database (`db.sqlite`), run:

```bash
# From repository root (once): uv sync --group dev
uv run tests/golden_dataset/scripts/export_from_db.py
```

### Custom Database or PDF Paths

Pass custom paths via CLI flags:

```bash
uv run tests/golden_dataset/scripts/export_from_db.py \
    --db-path /path/to/db.sqlite \
    --pdf-dir /path/to/sources \
    --output-dir /path/to/output/golden_dataset
```

---

## How to Label New Fixtures

When adding or updating ground-truth annotations:
1. Refer to [`LABELING.md`](LABELING.md) for step-by-step annotation guidelines.
2. Create `gold_parent.yaml` (title, author list, year, DOI, venue, hard negative rules).
3. Create `gold_references.yaml` (expected count range `[min, max]`, anchor items, and negative patterns to avoid).
4. Run `uv run tests/golden_dataset/scripts/validate_dataset.py` to ensure schema compliance.

---

## How Implementers Plug a New Extractor

To test a new extraction implementation (in Rust or Python):

1. **Input Interface**:
   - Feed `content.md` from `fixtures/<source_stem>/content.md` into your extractor.
2. **Output Format**:
   - Format your extractor's output as JSON matching the `current_extraction.json` schema:
     ```json
     {
       "source": {
         "title": "Extracted Title",
         "authors": "Author 1, Author 2",
         "year": 2026,
         "doi": null,
         "venue": null
       },
       "references": [
         {
           "ref_index": 1,
           "raw_text": "Full ref text...",
           "title": "Ref Title",
           "authors": "Ref Authors",
           "year": 2024,
           "doi": null,
           "arxiv_id": null
         }
       ]
     }
     ```
3. **Scoring & Verification**:
   - Run the baseline evaluator or your scorer against `gold_parent.yaml` and `gold_references.yaml`:
     ```bash
     uv run tests/golden_dataset/scripts/score_against_current.py
     ```
   - Compare your results with the evaluation criteria in [`EVALUATION.md`](EVALUATION.md).

---

## Validation & Integrity Check

Before submitting changes to the dataset, ensure the validation suite passes:

```bash
uv run tests/golden_dataset/scripts/validate_dataset.py
```

This verifies file presence, SHA-256 checksums, and JSON Schema validity across all fixtures.
