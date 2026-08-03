# Parent Metadata Gold Labeling Summary Report

This report summarizes the ground-truth parent metadata generated and audited for all 13 sources in the `scientist-in-loop` golden dataset fixture pack.

## Summary Table

| Stem | Gold Title | Gold Year | DOI? | Confidence | Evidence Snippet / Reason |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `2026.gem-main.4` | Self-Anchoring Calibration Drift in Large Language Models: How Multi-Turn Conversations Reshape Model Confidence | 2026 | No | high | `May 2026` (Line 5) |
| `28_Implicit_Ensembles_of_Ensem` | (Implicit) Ensembles of Ensembles: Epistemic Uncertainty Collapse in Large Models | null | No | medium | `null` (No header date signal; prior 2024 was in-text citation bleed) |
| `8708_On_the_Entropy_Calibratio` | ON THE ENTROPY CALIBRATION OF LANGUAGE MODELS | null | No | high | `null` (Double-blind submission without publication date) |
| `BEE-RAG` | BEE-RAG: Balanced Entropy Engineering for Retrieval-Augmented Generation | null | No | medium | `null` (No header date signal; prior 2024 was in-text citation bleed) |
| `GraphRAG` | Retrieval-Augmented Generation with Graphs (GraphRAG) | null | No | medium | `null` (No header date signal; prior 2024 was in-text citation bleed) |
| `HiChunk` | HiChunk: Evaluating and Enhancing Retrieval-Augmented Generation with Hierarchical Chunking | 2025 | No | high | `- **Date:** Sep 15, 2025` (Line 9) |
| `Internak_states_approach` | Unsupervised Real-Time Hallucination Detection based on the Internal States of Large Language Models | null | No | medium | `null` (No header date signal; prior 2024 was in-text citation bleed) |
| `Token_probability_approach` | Detecting Hallucinations in Large Language Model Generation: A Token Probability Approach | null | No | medium | `null` (No header date signal) |
| `knowledge_graph` | Multi-source knowledge graph construction through LLM-assisted incremental fusion | null | No | medium | `null` (No header date signal) |
| `minecraft_graph` | From entity-centric to goal-oriented graphs: Enhancing LLM knowledge retrieval in minecraft | null | No | medium | `null` (No header date signal) |
| `semantic_chunking` | Optimising retrieval performance in RAG systems: A new growing window semantic chunking strategy to address weak semantic boundaries | null | No | medium | `null` (No header date signal; line 19 ChatGPT 2022 is historical context) |
| `semantic_entropy` | Detecting hallucinations in large language models using semantic entropy | 2024 | `10.1038/s41586-024-07421-0` | high | `Published online: 19 June 2024` (Line 11) |
| `structure_predict_hallucination` | When structure predicts hallucination: Aligning LLMs with knowledge graph features | 2026 | `10.1016/j.datak.2026.102630` | high | `Available online 8 July 2026` (Line 34) |

## Audit Key Takeaways

1. **Strict Year Evidence Rules**:
   - `year` is retained **only** when explicit document-level header date signals exist in `content.md` (`May 2026`, `- **Date:** Sep 15, 2025`, `Published online: 19 June 2024`, `Available online 8 July 2026`).
   - 8 preprints/unformatted papers lacked document-level header publication dates. Their `year` fields were updated to `null` to eliminate citation-bleed pollution.

2. **Confidence Classification**:
   - `high`: Given to fixtures with solid title/authors AND either verified header date evidence (`2026.gem-main.4`, `HiChunk`, `semantic_entropy`, `structure_predict_hallucination`) or explicitly null year for anonymous submissions (`8708_On_the_Entropy_Calibratio`).
   - `medium`: Given to non-anonymous published-looking papers whose title/authors are solid but whose year is `null` due to missing header date signals.
