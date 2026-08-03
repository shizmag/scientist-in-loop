# ADR-007: Parent Document Title & Author Extraction Improvements

## Context
Goal E1 focused on raising parent document metadata extraction quality for scientific PDFs parsed via Marker markdown, evaluated against the 13-fixture frozen `golden_dataset`.

Prior to this work:
- Parent title pass rate was 69.23% (9/13), failing on Elsevier journal headers (`Knowledge-Based Systems`, `Data & Knowledge Engineering`), publisher chrome (`ScienceDirect`), and section headers (`Abstract`, `ARTICLE INFO`).
- Parent authors macro F1 was 0.51, failing on TeX math superscripts (`$^{1*\dagger}$`), ORCID links, inline email/affiliation noise, IEEE badges, and premature heading cutoffs.
- Parent hard negatives clean rate was 69.23% (9/13), polluted by affiliation text and in-text citation bleed.

## Decisions

1. **Publisher Chrome & Journal Header Filtering**:
   - Implemented `sil_regex::is_journal_or_publisher_title` to filter out journal titles (`Knowledge-Based Systems`, `Data & Knowledge Engineering`, `Intelligent Systems with Applications`), publisher names (`Elsevier`, `ScienceDirect`), and section header noise before selecting the true article title.

2. **Byline Scoping & Author Line Cleaning**:
   - Byline scanning begins strictly after the selected title header line.
   - Double-blind papers containing `"Anonymous authors"` or `"Paper under double-blind review"` map directly to `"Anonymous authors"`.
   - `clean_author_byline_line` strips split markdown links (`She[n](...)`), ORCID badges (`[ID](...)`), inline emails (`user@domain.com`), TeX math superscripts (`$^{1*\dagger}$`), HTML `<sup>` tags, IEEE badges (`Senior Member, IEEE`), and affiliation suffixes.
   - `split_author_names` handles multi-author lines both with standard delimiters (`,`, `;`, `and`, `&`) and un-delimited capitalized name pairs.
   - `is_valid_author_name` filters out affiliations, locations, email fragments, month names, and non-author keywords.

3. **Publication Year Extraction**:
   - Header lines and the first 80 lines of document content are scanned for explicit date headers (`Published:`, `Received:`, `Available online`, `©`, DOI paths `10.1016/...`) to prevent missing valid publication years.

## Results against Golden Dataset (E1)

| Parent Metric Category | Baseline Score | Post-Fix Score | Target Gate | Result |
| :--- | :---: | :---: | :---: | :---: |
| **Parent Title Pass Rate** | 69.23% (9/13) | **100.00% (13/13)** | $\ge 0.85$ | **PASS** |
| **Parent Year Pass Rate** | 92.31% (12/13) | **100.00% (13/13)** | $\ge 0.85$ | **PASS** |
| **Parent Authors Set F1** | 0.51 (macro) | **0.92 (macro)** | $\ge 0.70$ (stretch 0.85) | **PASS (EXCEEDED STRETCH)** |
| **Parent Hard Negatives Clean** | 69.23% (9/13) | **100.00% (13/13)** | 100% | **PASS** |
