# Baseline Extraction Scorecard

This report documents the baseline evaluation of current `scientist-in-loop` extractions (`current_extraction.json`) against the labeled Golden Dataset (`gold_parent.yaml` & `gold_references.yaml`).

## Summary Metrics & CI Gate Assessment

| Metric Category | Target Gate | Current Macro Score | Current Micro / Total | CI Gate Status |
| :--- | :---: | :---: | :---: | :---: |
| **Parent Title Pass Rate** | $\ge 0.85$ | 100.00% | 13/13 | PASS |
| **Parent Year Pass Rate** | $\ge 0.85$ | 100.00% | 13/13 | PASS |
| **Parent Authors Set F1** | $\ge 0.85$ | 0.92 | Avg F1 across fixtures | PASS |
| **Parent Hard Negatives Clean** | 100% | 100.00% | 13/13 clean | PASS |
| **Ref Count Band Pass Rate** | $\ge 0.80$ | 100.00% | 13/13 | PASS |
| **Ref Anchor Recall** | $\ge 0.75$ | 96.54% | 96.40% micro (134/139) | PASS |
| **Ref Anchor Field Precision** | $\ge 0.80$ | 93.93% | 93.84% micro | PASS |
| **Ref Negative Pattern Clean** | 100% | - | 0/1035 refs polluted | PASS |

## Detailed Per-Fixture Results

| Source Fixture | Parent Title | Authors F1 | Parent Year | Hard Negatives | Ref Count (Ext / Gold) | Ref Count Pass | Anchor Recall | Anchor Field Prec | Polluted Refs |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| `2026.gem-main.4` | PASS | 1.00 | PASS | PASS | 10 / [10, 10] | PASS | 100% (10/10) | 100% | 0 |
| `28_Implicit_Ensembles_of_Ensem` | PASS | 1.00 | PASS | PASS | 28 / [28, 28] | PASS | 90% (9/10) | 100% | 0 |
| `8708_On_the_Entropy_Calibratio` | PASS | 1.00 | PASS | PASS | 35 / [35, 35] | PASS | 100% (12/12) | 100% | 0 |
| `BEE-RAG` | PASS | 0.53 | PASS | PASS | 38 / [38, 38] | PASS | 100% (10/10) | 97% | 0 |
| `GraphRAG` | PASS | 0.97 | PASS | PASS | 573 / [570, 575] | PASS | 92% (11/12) | 95% | 0 |
| `HiChunk` | PASS | 0.46 | PASS | PASS | 34 / [34, 34] | PASS | 100% (10/10) | 100% | 0 |
| `Internak_states_approach` | PASS | 1.00 | PASS | PASS | 45 / [45, 47] | PASS | 100% (11/11) | 91% | 0 |
| `Token_probability_approach` | PASS | 1.00 | PASS | PASS | 38 / [38, 38] | PASS | 90% (9/10) | 100% | 0 |
| `semantic_entropy` | PASS | 1.00 | PASS | PASS | 65 / [65, 65] | PASS | 92% (11/12) | 87% | 0 |
| `knowledge_graph` | PASS | 1.00 | PASS | PASS | 42 / [42, 42] | PASS | 100% (10/10) | 100% | 0 |
| `minecraft_graph` | PASS | 1.00 | PASS | PASS | 24 / [24, 24] | PASS | 100% (10/10) | 95% | 0 |
| `semantic_chunking` | PASS | 1.00 | PASS | PASS | 70 / [70, 70] | PASS | 92% (11/12) | 91% | 0 |
| `structure_predict_hallucination` | PASS | 0.94 | PASS | PASS | 33 / [33, 33] | PASS | 100% (10/10) | 65% | 0 |

## Failure Breakdown & Known Issues in Current Extractor

1. **Reference Count Explosion / Truncation Outliers**:
   - `GraphRAG`: Extracted **573** references vs expected **570-575** (PASS count band, but includes noise lines).
   - `semantic_entropy`: Extracted **0** references vs expected **65** (**FAIL** - bibliography section missed).
   - `28_Implicit_Ensembles_of_Ensem`: Extracted **2** references vs expected **28** (**FAIL** - bibliography truncated).
   - `8708_On_the_Entropy_Calibratio`: Extracted **30** references vs expected **35** (**FAIL** - margin numbers split entries).
2. **Parent Metadata In-Text Citation Bleed**:
   - `2026.gem-main.4`: Author byline polluted with in-text citation names (`Kadavath et al`, `Xiong et al`, `Tian et al`, `Kahneman`).
   - `BEE-RAG`: Title contains leading markdown artifacts or trailing header text.
3. **Reference Negative Pattern Pollution**:
   - HTML anchor tags (`<span id="page-...">`) embedded in raw_text of extracted references.
   - Appendix code snippets or proofs extracted as bibliography entries.

---
*Report generated automatically by `scripts/score_against_current.py`.*
