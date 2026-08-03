# Golden Dataset Fixture Pack

This directory contains a golden-dataset *fixture pack* exported from the indexed scientific project database (`db.sqlite`) for `scientist-in-loop` testing and quality evaluation.

## Purpose

The fixture pack isolates Marker-parsed Markdown documents, raw bibliography blocks, and initial (noisy) DB extraction outputs without requiring runtime database connection or live PDF re-parsing during tests.

> [!NOTE]
> This export captures current state fixtures. No gold labels or manually corrected reference annotations have been created yet.

## Directory Structure

```text
tests/golden_dataset/
├── README.md                 # This documentation file
├── manifest.yaml             # Global index of all sources and quality flags
├── scripts/
│   └── export_from_db.py     # Python exporter script
└── fixtures/
    └── <source_stem>/
        ├── meta.yaml               # Metadata: PDF path, sha256 checksum, lengths, DB fields
        ├── content.md              # Raw Marker-parsed Markdown (sources.content)
        ├── references_block.md     # Raw bibliography text block (sources.references_text)
        └── current_extraction.json # Extraction outputs (sources & source_references rows)
```

## How to Regenerate

To re-export fixtures from the SQLite database, run the following Python script:

```bash
python3 tests/golden_dataset/scripts/export_from_db.py
```

### Custom Database or PDF Paths

You can pass custom paths via CLI flags:

```bash
python3 tests/golden_dataset/scripts/export_from_db.py \
    --db-path /path/to/db.sqlite \
    --pdf-dir /path/to/sources \
    --output-dir /path/to/output/golden_dataset
```

Alternatively, you can inspect or extract directly with `sqlite3` one-liners:

```bash
sqlite3 /path/to/db.sqlite "SELECT id, title, year FROM sources;"
```

## Manifest Quality Flags

The `manifest.yaml` includes metadata and quality flags for every source:

- `empty_content`: True if `sources.content` is empty/null.
- `empty_references_text`: True if `sources.references_text` is empty/null.
- `ref_count_outlier`: True if extracted reference count > 150 (e.g. `GraphRAG.pdf` with 573 refs) or < 3 when `references_text` is non-empty (e.g. `28_Implicit_Ensembles_of_Ensem.pdf` with 2 refs).
- `needs_reparse`: True if `content` is empty for a source with an existing PDF file.
