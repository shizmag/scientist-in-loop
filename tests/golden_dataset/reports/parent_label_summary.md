# Parent Metadata Gold Labeling Summary Report

This report summarizes the ground-truth parent metadata generated for all 13 sources in the `scientist-in-loop` golden dataset fixture pack.

## Summary Table

| Stem | Gold Title | Gold Year | DOI? | Confidence | Main Difficulty |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `2026.gem-main.4` | Self-Anchoring Calibration Drift in Large Language Models: How Multi-Turn Conversations Reshape Model Confidence | 2026 | No | high | In-text citation bleed (`Kadavath et al.`) |
| `28_Implicit_Ensembles_of_Ensem` | (Implicit) Ensembles of Ensembles: Epistemic Uncertainty Collapse in Large Models | 2024 | No | high | False year (1992) & author pollution from citations |
| `8708_On_the_Entropy_Calibratio` | ON THE ENTROPY CALIBRATION OF LANGUAGE MODELS | null | No | high | Double-blind margin numbers & `ABSTRACT` heading trap |
| `BEE-RAG` | BEE-RAG: Balanced Entropy Engineering for Retrieval-Augmented Generation | 2024 | No | high | Dense superscripts & email address fusion |
| `GraphRAG` | Retrieval-Augmented Generation with Graphs (GraphRAG) | 2024 | No | high | 18 authors in byline & email block noise |
| `HiChunk` | HiChunk: Evaluating and Enhancing Retrieval-Augmented Generation with Hierarchical Chunking | 2025 | No | high | Benchmark name trap (`HiCBench` vs paper title) |
| `Internak_states_approach` | Unsupervised Real-Time Hallucination Detection based on the Internal States of Large Language Models | 2024 | No | high | Complex superscript markers & affiliation bleed |
| `Token_probability_approach` | Detecting Hallucinations in Large Language Model Generation: A Token Probability Approach | 2024 | No | high | Fused emails & ORCID/IEEE membership noise |
| `knowledge_graph` | Multi-source knowledge graph construction through LLM-assisted incremental fusion | 2025 | No | high | Journal title trap (`Intelligent Systems with Applications`) |
| `minecraft_graph` | From entity-centric to goal-oriented graphs: Enhancing LLM knowledge retrieval in minecraft | 2024 | No | high | Header chrome trap (`ScienceDirect`) & ORCID link noise |
| `semantic_chunking` | Optimising retrieval performance in RAG systems: A new growing window semantic chunking strategy to address weak semantic boundaries | 2024 | No | high | Journal title trap (`Knowledge-Based Systems`) |
| `semantic_entropy` | Detecting hallucinations in large language models using semantic entropy | 2024 | `10.1038/s41586-024-07421-0` | high | Nature header layout & superscript noise |
| `structure_predict_hallucination` | When structure predicts hallucination: Aligning LLMs with knowledge graph features | 2026 | `10.1016/j.datak.2026.102630` | high | Journal title trap (`Data & Knowledge Engineering`) |

## Key Insights

1. **Title Extraction Quality**:
   - 4 out of 13 papers suffered from venue-as-title or header-chrome errors (e.g. Elsevier header `ScienceDirect` or journal titles extracted instead of article titles).
   - 1 paper (`HiChunk`) suffered from benchmark name confusion (`HiCBench` in abstract).
   - 1 paper (`8708_On_the_Entropy_Calibratio`) suffered from `ABSTRACT` heading extraction.

2. **Author Extraction Quality**:
   - In-text citation bleed was a primary failure mode for previous extractors (e.g., pulling authors cited in Introduction paragraphs).
   - Affiliation and email address pollution occurred frequently when superscripts were fused with author names.

3. **Year Accuracy**:
   - In-text citations (such as `MacKay, 1992`) led to false publication years in earlier baseline extractions.

All 13 sources now have verified ground-truth `gold_parent.yaml` files.
