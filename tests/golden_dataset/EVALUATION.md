# Golden Dataset Evaluation Contract & Quality Metrics

This document defines the formal evaluation contract for assessing paper extraction quality in `scientist-in-loop`. Any Rust or Python quality scorer developed for `scientist-in-loop` MUST implement the metric definitions, matching heuristics, and pass/fail thresholds defined here.

---

## 1. Overview & Data Contract

An extractor processes a Markdown file (`content.md`) and emits a JSON payload adhering to the `current_extraction.json` structure:

```json
{
  "source": {
    "title": "Extracted Paper Title",
    "authors": "Author One, Author Two",
    "year": 2026,
    "doi": "10.1234/example.doi",
    "venue": "Conference/Journal"
  },
  "references": [
    {
      "ref_index": 1,
      "raw_text": "Full reference text",
      "title": "Reference Paper Title",
      "authors": "Author A, Author B",
      "year": 2024,
      "doi": null,
      "arxiv_id": "2401.12345",
      "venue": "NeurIPS"
    }
  ]
}
```

The scorer evaluates this output against:
1. `gold_parent.yaml`: Ground truth parent metadata & hard negative constraints.
2. `gold_references.yaml`: Target reference count band, key anchors, and negative pollution patterns.

---

## 2. Parent Metadata Metrics

### 2.1 Title Match (`parent_title`)
- **Normalization**:
  - Convert to lowercase.
  - Remove all punctuation (`[^\w\s]`).
  - Collapse all whitespace sequences into single spaces and trim.
- **Pass Condition**:
  - Exact match: `norm(pred_title) == norm(gold_title)`, OR
  - Fuzzy match: `fuzzy_ratio(norm(pred_title), norm(gold_title)) >= 0.90` (using Gestalt Pattern Matching / Levenshtein ratio), OR
  - Alias match: `norm(pred_title)` matches any entry in `gold_parent.title_aliases` (exact or fuzzy ratio $\ge 0.90$).

### 2.2 Authors Set F1 (`parent_authors_f1`)
- **Normalization & Tokenization**:
  - Split author strings into individual tokens/words by stripping punctuation, lowercasing, and splitting on whitespace/commas/delimiters.
  - Exclude single-character initials or stop words (e.g. `et`, `al`, `and`) when evaluating match sets. Last-name tokens are retained for matching.
- **Set F1 Metric**:
  - Let $G$ be the set of gold author tokens and $P$ be the set of predicted author tokens.
  - $TP = |G \cap P|$
  - $\text{Precision} = \frac{TP}{|P|}$ (if $|P| = 0$, Precision is 1.0 if $|G|=0$ else 0.0)
  - $\text{Recall} = \frac{TP}{|G|}$ (if $|G| = 0$, Recall is 1.0 if $|P|=0$ else 0.0)
  - $F_1 = \frac{2 \cdot \text{Precision} \cdot \text{Recall}}{\text{Precision} + \text{Recall}}$ (if $\text{Precision} + \text{Recall} = 0$, $F_1 = 0.0$)

### 2.3 Year Match (`parent_year`)
- **Pass Condition**:
  - If `gold_parent.year` is non-null: `pred_year == gold_year`.
  - If `gold_parent.year` is `null`: "empty OK" (predicted `year` may be `null` or integer).

### 2.4 DOI Match (`parent_doi`)
- **Pass Condition**:
  - If `gold_parent.doi` is non-null: `norm(pred_doi) == norm(gold_doi)`.
  - If `gold_parent.doi` is `null`: "empty OK" (pass regardless of predicted `doi`).

### 2.5 Hard Negatives & Pollution (`parent_hard_negatives`)
- **Pass Condition**:
  - **Title Check**: Fail if `pred_title` matches or contains any string listed in `gold_parent.hard_negatives.bad_titles_must_not_match` (e.g., `"Abstract"`, `"1 Introduction"`).
  - **Author Pollution Check**: Fail if `pred_authors` contains any prohibited pollution token in `gold_parent.hard_negatives.author_pollution_must_not_include` (e.g., in-text citation bleed like `"Kadavath"`, `"et al"`).

---

## 3. Reference Metrics

### 3.1 Reference Count Band (`ref_count_band`)
- Let $N_{\text{pred}} = |\text{emitted references}|$.
- **Pass Condition**:
  - $N_{\text{pred}} \in [\text{expected\_ref\_count.min}, \text{expected\_ref\_count.max}]$.

### 3.2 Anchor Recall (`ref_anchor_recall`)
- **Anchor Matching Heuristic**: An emitted reference $R$ matches a gold anchor $A$ if ANY of the following hold:
  1. **DOI match**: `A.match.doi` is non-null AND `norm(R.doi) == norm(A.match.doi)`.
  2. **arXiv match**: `A.match.arxiv_id` is non-null AND `norm(R.arxiv_id) == norm(A.match.arxiv_id)`.
  3. **Title Substring + Year match**: `A.match.title_contains` is non-null AND `norm(A.match.title_contains)` is present in `norm(R.title + " " + R.raw_text)` AND (`A.match.year` is `null` OR `R.year == A.match.year`).
- **Metric**:
  $$\text{Anchor Recall} = \frac{\text{Number of Matched Gold Anchors}}{\text{Total Gold Anchors in Fixture}}$$

### 3.3 Anchor Field Precision (`ref_anchor_field_precision`)
- Among all **matched** anchors, evaluate field agreement against `A.expected`:
  - `year`: `R.year == A.expected.year`
  - `title`: `fuzzy_ratio(R.title, A.expected.title) >= 0.80` OR `norm(A.expected.title)` in `norm(R.raw_text)`
  - `doi`: `R.doi == A.expected.doi` (if expected DOI is non-null)
  - `authors`: All tokens in `A.expected.authors_contains` present in `norm(R.authors + " " + R.raw_text)`
- **Metric**:
  $$\text{Field Precision} = \frac{\text{Total Correct Expected Fields Across Matched Anchors}}{\text{Total Evaluated Expected Fields Across Matched Anchors}}$$

### 3.4 Negative Pattern Pollution (`ref_negative_pollution`)
- **Pollution Heuristic**:
  - For each emitted reference $R$, check `R.raw_text` and `R.title` against `gold_references.must_not_extract_as_reference`:
    - Regex pattern match (e.g. `span id="page-`, margin line numbers `^\*\*\d+`).
    - Literal substring match (e.g. section headings, paper title headers).
- **Pass Condition**:
  - Reference set is clean if $0$ emitted references violate any negative pattern.

---

## 4. Aggregate Scorecard & CI Quality Gate

The scorecard aggregates scores across all test fixtures using both **Macro Average** (unweighted mean across fixtures) and **Micro Average** (pooled item counts across dataset).

### Initial CI Pass/Fail Gate Proposal (Tunable)

| Metric | Target Threshold | Description |
| :--- | :---: | :--- |
| **Parent Title Pass Rate** | $\ge 0.85$ | At least 85% of fixtures pass Parent Title match |
| **Parent Year Pass Rate** | $\ge 0.85$ | At least 85% of fixtures pass Parent Year match |
| **Parent Authors Set F1** | $\ge 0.85$ | Macro average Set F1 $\ge 0.85$ |
| **Ref Count Band Pass Rate**| $\ge 0.80$ | At least 80% of fixtures fall within expected reference count band |
| **Ref Anchor Recall** | $\ge 0.75$ | Macro/Micro anchor recall $\ge 0.75$ across anchors |
| **Parent Hard Negatives** | **100% (Zero Tolerance)** | 0 fixtures allowed with in-text citation bleed or section title as paper title |
| **Ref Negative Patterns** | **100% (Zero Tolerance)** | 0 emitted references containing HTML page anchors or margin line noise |

---

## 5. Instructions for Implementers

1. **Input Interface**:
   - The extractor under test accepts `content.md` from a fixture directory (`fixtures/<source_stem>/content.md`).
2. **Output Format**:
   - The extractor emits a JSON object conforming to the `current_extraction.json` structure (schema defined above).
3. **Execution & Validation**:
   - From the repo root: `uv sync --group dev` (once), then `uv run tests/golden_dataset/scripts/validate_dataset.py`.
   - Score with `uv run tests/golden_dataset/scripts/score_against_current.py` (or a Rust equivalent) against `gold_parent.yaml` and `gold_references.yaml`.
4. **Important**:
   - **Do NOT treat `current_extraction.json` as ground truth gold labels!** It contains known baseline bugs documented in `reports/baseline_scorecard.md`.
