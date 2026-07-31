# ADR-006: Refactoring Author and Reference Extraction for Marker Artifact Noise Resilience

## Status
Accepted

## Context
When parsing scientific PDFs with Marker (or converting to Markdown), output often includes heavy formatting artifacts:
- HTML `<span id="...">` and `<a id="...">` tags.
- Internal markdown anchor links like `[Author](#page-1-0)` or `[Paper](#refhub)`.
- Author footnote superscripts (`<sup>1</sup>`, `[\*1]`, `[a]`, `†`).
- Section headers (`# 1. Introduction`, `## Background`) and affiliation addresses captured as authors.
- Math equations (`$$...$$`) and body step lists (`1. For each input...`) parsed into `source_references`.

Previous heuristic extraction was fragile and degraded reference/author quality in SQLite.

## Decision Drivers
1. **Header Scoping**: Document-level metadata (DOIs, arXiv IDs, author lines) must be scoped strictly to the text preceding `## References` and before section headings (`# 1. Introduction`).
2. **Noise Line Filtering**: Lines containing affiliation keywords ("University", "Department", "Lab", "Inc.") or section headers ("Abstract", "Preliminaries", "Methodology", "Keywords", "Date:", "Code:") must be excluded from author lines.
3. **Marker Artifact Stripping**: Stripping markdown links, HTML spans, footnote markers, and normalizing spacing prior to author/reference field parsing.
4. **Reference Filtering & Validation**:
   - Math equations (`$$...$$`) and body numbered lists (`1. For each input...`) are excluded prior to joining reference entries.
   - Reference entry validation requires citation signals (publication year, DOI, arXiv, venue, "et al.", or author comma structure).
5. **Interactive Sorting**: Dynamic reference sorting in TUI by Year (`y`), Source (`s`), Venue (`v`/`j`), and Index (`i`/`n`).
6. **Rate Limiting**: Enforcing a minimum 250ms delay with custom User-Agent for all external API requests.

## Consequences
- `sil source doctor` heals source documents deterministically from scratch without Marker dependency.
- Extracted references and authors in SQLite are clean, readable, and noise-free.
- Interactive TUI displays accurate year, venue, author, and source sorting.
