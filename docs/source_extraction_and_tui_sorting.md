# Source Extraction, Metadata Hydration, and TUI Reference Sorting

## Overview
This document records the architectural design for improving source metadata extraction, reference extraction, rate-limited external API querying, and interactive reference sorting in Scientist-In-Loop (SIL).

## Key Architecture Decisions

### 1. Document vs Reference Metadata Scoping
Previously, `extract_doi` searched the entire markdown text of a document. As a result, papers without a header DOI picked up the DOI of their first cited reference in `## References`, causing Crossref hydration to overwrite the paper's title, authors, year, and venue with reference metadata.
**Decision:** Document-level DOI and arXiv extraction is strictly scoped to the header portion (first 3000 characters or before the `## References` section).

### 2. Reference Schema Enhancement (`venue`)
`ReferenceEntry` in `sil-core` and SQLite table `source_references` in `sil-db` now track `venue: Option<String>` (journal / conference name).
- Schema Migration: `SilDb::open` automatically executes `ALTER TABLE source_references ADD COLUMN venue TEXT` if the column is absent.

### 3. API Throttling & Rate Limiting
To comply with external provider policies (Crossref, OpenAlex, ArXiv):
- All outgoing HTTP requests are throttled with a mandatory delay (min 250ms between calls).
- User-Agent header includes polite pool metadata (`scientist-in-loop/0.1.0`).

### 4. Interactive Reference Sorting in TUI
`sil-tui` supports dynamic sorting of reference lists by:
- `y`: Publication Year
- `s`: Source Document
- `v` / `j`: Venue (Journal / Conference)
- `i` / `n`: Original Citation Index

### 5. Pure Rust / Fast Parser Fallback
Heavy Python marker dependencies are decoupled from core reference/author extraction, allowing `sil source doctor` to run reliably and rapidly from scratch across all environments.
