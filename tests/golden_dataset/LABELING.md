# Gold Parent Labeling Guidelines & Edge Case Documentation

This document outlines the strict guidelines and edge cases established for creating `gold_parent.yaml` metadata fixtures in the `scientist-in-loop` golden dataset.

## Labeling Rules (Strict)

1. **Title**:
   - The paper's own official title.
   - Prefer the first real title H1 line in `content.md`.
   - **Reject** publisher header chrome (e.g. `ScienceDirect`), journal names (e.g. `Knowledge-Based Systems`, `Data & Knowledge Engineering`, `Intelligent Systems with Applications`), section headings (e.g. `ABSTRACT`), or internal benchmark names (e.g. `HiCBench`).
   - If multiple title candidates exist, put the primary title in `title` and acceptable short forms/variants in `title_aliases`.

2. **Authors**:
   - Only true paper byline authors.
   - **Strip**: footnote numbers (`1, 2`), superscript markers (`*`, `†`, `‡`, `⋈`), ORCID icons/links (`[ID]`), membership titles (`Senior, IEEE`), affiliation names, department details, and email addresses.
   - **Do NOT include**: in-text cited authors from Introduction, Related Work, or Bibliography sections (e.g., `Kadavath et al.`, `MacKay`, `Xiong et al.`).
   - **Anonymous Submissions**: If double-blind (e.g., `8708_On_the_Entropy_Calibratio.pdf`), set authors explicitly to `["Anonymous authors"]`.

3. **Year**:
   - The publication year of THIS paper.
   - Prefer Crossref/DOI header lines, formal publication dates, or explicit dates (e.g. `Sep 15, 2025` -> `2025`).
   - **Do NOT use**: years cited inside text or references.

4. **DOI / arXiv ID**:
   - Only document-level identifiers located in the top header or abstract region of the paper.
   - **Do NOT use**: DOIs or arXiv IDs belonging to cited references.

5. **Hard Negatives**:
   - `bad_titles_must_not_match`: Explicit list of incorrect/noisy titles that extraction algorithms are prone to mistakenly select (e.g. `ScienceDirect`, `ABSTRACT`, journal titles).
   - `author_pollution_must_not_include`: Strings or cited author surnames that indicate citation bleed or affiliation contamination.

## Discovered Edge Cases & Marker Extraction Pitfalls

### 1. Journal Title Traps (Elsevier Headers)
* **Observed in**: `knowledge_graph.pdf`, `semantic_chunking.pdf`, `structure_predict_hallucination.pdf`, `minecraft_graph.pdf`.
* **Pattern**: Elsevier Marker outputs often place `# Journal Name` or `Contents lists available at ScienceDirect` above the actual paper title.
* **Resolution**: Extraction algorithms must look past publisher header blocks to locate the first true article title header.

### 2. Heading Traps (`ABSTRACT`)
* **Observed in**: `8708_On_the_Entropy_Calibratio.pdf`.
* **Pattern**: In double-blind submissions with margin line numbers (`001`, `003`...), Marker may place `#### **ABSTRACT**` lower down without an explicit `# Title` tag.
* **Resolution**: Use H1 line 1 (`ON THE ENTROPY CALIBRATION OF LANGUAGE MODELS`) despite line numbers between title and abstract.

### 3. Internal Benchmark Name Traps
* **Observed in**: `HiChunk.pdf`.
* **Pattern**: The paper introduces both a benchmark (`HiCBench`) and a framework (`HiChunk`).
* **Resolution**: Ensure title reflects the paper's title (`HiChunk: Evaluating and Enhancing Retrieval-Augmented Generation with Hierarchical Chunking`) and not the benchmark name.

### 4. Citation Bleed & False Years
* **Observed in**: `28_Implicit_Ensembles_of_Ensem.pdf`, `2026.gem-main.4.pdf`.
* **Pattern**: Introduction paragraphs containing inline citations like `[MacKay, 1992]` caused previous extraction systems to select `1992` as the publication year and add `MacKay` to the authors list.
* **Resolution**: Restrict author and year candidate searching to the structural byline region above the Abstract.

## Bibliography Reference Gold Labeling & Edge Cases

### 1. Non-Standard Reference Headings (`semantic_entropy.pdf`)
* **Observed in**: `semantic_entropy.pdf`.
* **Pattern**: Nature publications use `### **Online content**` or `References` without a standard Markdown `# References` header. Marker produced a 0-byte `references_block.md`.
* **Resolution**: Reference extraction algorithms must scan `content.md` for non-standard section titles (`Online content`, `References and Notes`) when `references_block.md` is empty.

### 2. Early Truncation on Keywords (`28_Implicit_Ensembles_of_Ensem.pdf`)
* **Observed in**: `28_Implicit_Ensembles_of_Ensem.pdf`.
* **Pattern**: A reference title containing prose terms like "epistemic uncertainty" triggered `is_biography_or_prose_line()` early termination in `extract_references_block()`, losing 24 references out of 28.
* **Resolution**: Guard prose termination checks so lines starting with list markers (`- `, `[N]`, `N.`) or containing DOIs/arXiv IDs are never misclassified as prose/biography.

### 3. Appendix & Math Proof Leaks (`8708_On_the_Entropy_Calibratio.pdf`, `HiChunk.pdf`, `Internak_states_approach.pdf`)
* **Observed in**: `8708_On_the_Entropy_Calibratio.pdf`, `HiChunk.pdf`, `Internak_states_approach.pdf`.
* **Pattern**: Marker included Appendix sections (`## A PROOFS`, `# A Appendix`, `### A Pseudocode Description`) inside `references_block.md`.
* **Resolution**: Reference boundary parsers must treat structural H1/H2 Appendix headings (`# Appendix`, `## A PROOFS`, `#### Listing A`) as strict hard-stop boundaries.

### 4. False Splits from Page-Break Hyphens (`BEE-RAG.pdf`)
* **Observed in**: `BEE-RAG.pdf`.
* **Pattern**: A page break inserted a hyphen bullet (`- `) into a multi-line reference entry (`- Language Models. In...`), causing it to be extracted as an extra reference item.
* **Resolution**: Extractors should check whether a new bullet line starts with an author name or citation number before treating it as a new entry.

### 5. Double-Blind Submission Margin Line Number Noise (`8708_On_the_Entropy_Calibratio.pdf`)
* **Observed in**: `8708_On_the_Entropy_Calibratio.pdf`.
* **Pattern**: Margin line numbers (`**558 559 560**`) inserted between reference entries caused line breaks and parser confusion.
* **Resolution**: Filter out standalone bold integer patterns (`^\*\*\d+`) within the reference block.

