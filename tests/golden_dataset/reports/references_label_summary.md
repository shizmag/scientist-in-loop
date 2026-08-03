# Bibliography Reference Gold Labeling Summary Report

This report summarizes the ground-truth bibliography reference extraction labels (`gold_references.yaml`) generated and audited for all 13 sources in the `scientist-in-loop` golden dataset fixture pack.

## Summary Table

| Stem | current_n | gold_min–max | n_anchors | style | confidence | notes |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `2026.gem-main.4` | 8 | 10–10 | 10 | `author_year` | high | current_extraction missed 2 entries (Tian et al., Xiong et al.) due to missing line breaks between unnumbered items |
| `28_Implicit_Ensembles_of_Ensem` | 2 | 28–28 | 10 | `author_year` | high | references_block.md was truncated after 3 entries during initial export; 28 true references recovered from content.md |
| `8708_On_the_Entropy_Calibratio` | 30 | 35–35 | 12 | `author_year` | high | references_block.md included Appendix A math proofs (67 KB); 35 true references extracted after filtering line number noise and appendix |
| `BEE-RAG` | 39 | 38–38 | 10 | `author_year` | high | current_extraction oversplit Ratner et al. entry into 2 items due to page break line wrap |
| `GraphRAG` | 573 | 570–575 | 15 | `bracketed` | medium | Outlier literature survey paper with 573 bracketed references ([1] to [573]) |
| `HiChunk` | 34 | 34–34 | 10 | `bracketed` | high | Correctly parsed 34 entries; references_block.md included Appendix A text and QA prompt listings |
| `Internak_states_approach` | 41 | 45–47 | 11 | `author_year` | medium | 45–47 true items before Appendix pseudocode; current_extraction under-extracted due to wrapped author blocks across page boundaries |
| `Token_probability_approach` | 38 | 38–38 | 10 | `bracketed` | high | Clean bracketed list [1]–[38] |
| `knowledge_graph` | 42 | 42–42 | 10 | `author_year` | high | Clean APA author-year list with DOIs |
| `minecraft_graph` | 24 | 24–24 | 10 | `bracketed` | high | Clean bracketed list [1]–[24] |
| `semantic_chunking` | 70 | 70–70 | 12 | `bracketed` | high | Clean bracketed list [1]–[70] |
| `semantic_entropy` | 0 | 65–65 | 12 | `dot` | high | references_block.md was 0 bytes; 65 true dot-numbered references recovered from content.md lines 113-159 and 37-57 |
| `structure_predict_hallucination` | 33 | 33–33 | 10 | `bracketed` | high | Clean bracketed list [1]–[33] |

## Audit Key Takeaways & Recommendations

- **anchors_verified: true**: 100% of all 139 anchor `title_contains` strings across all 13 fixtures have been strictly verified to be exact contiguous substrings of raw fixture text (`references_block.md` / `content.md`). Author names have been completely purged from `title_contains`.
- **Expected Count Changes**: 0 (all count bands remain identical).

1. **Empty `references_block.md` Fallback (`semantic_entropy`)**:
   - Marker output failed to extract `references_block.md` because Nature formatted the section under `### **Online content**`.
   - Ground truth requires checking `content.md` when `references_block.md` is 0 bytes.

2. **Truncation and Prose Termination Bug (`28_Implicit_Ensembles_of_Ensem`)**:
   - Keywords matching prose/biography inside reference lines (e.g., "epistemic uncertainty") caused premature stopping in the legacy parser.

3. **Appendix Bleed (`8708_On_the_Entropy_Calibratio`, `HiChunk`, `Internak_states_approach`)**:
   - Headings such as `## A PROOFS`, `# A Appendix`, `### A Pseudocode` leaked into reference text blocks and must be filtered by downstream extractors.

4. **Line-Wrapped Entry False Splits (`BEE-RAG`, `2026.gem-main.4`)**:
   - Multi-line author names or page-break hyphens caused single references to split into 2 extracted items or adjacent unnumbered items to merge.

