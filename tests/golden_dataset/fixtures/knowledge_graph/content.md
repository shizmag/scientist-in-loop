Contents lists available at [ScienceDirect](https://www.journals.elsevier.com/intelligent-systems-with-applications)

# Intelligent Systems with Applications

journal homepage: [www.journals.elsevier.com/intelligent-systems-with-applications](https://www.journals.elsevier.com/intelligent-systems-with-applications)

# Multi-source knowledge graph construction through LLM-assisted incremental fusion

Ziqiu Huang [∗](#page-0-0) , Wenli Yang , Xiang Li , Quan Bai

*School of Information and Communication Technology, University of Tasmania, Hobart, TAS, Australia*

# A R T I C L E I N F O

Dataset link: [https://github.com/theFirstHuang](https://github.com/theFirstHuang/multisource-fusion-rag) [/multisource-fusion-rag](https://github.com/theFirstHuang/multisource-fusion-rag), [https://huggingface.c](https://huggingface.co/datasets/Cool-EdwardH/heterogeneous-multisource-kg-benchmark) [o/datasets/Cool-EdwardH/heterogeneous-mult](https://huggingface.co/datasets/Cool-EdwardH/heterogeneous-multisource-kg-benchmark) [isource-kg-benchmark](https://huggingface.co/datasets/Cool-EdwardH/heterogeneous-multisource-kg-benchmark)

*Keywords:*

Knowledge graph construction Multi-source data fusion Entity resolution Large language models GraphRAG

# A B S T R A C T

Real-world data often originates from multiple heterogeneous sources describing the same entities. When multiple sources target the same domain, they produce overlapping and conflicting information requiring reconciliation. Traditional approaches rely on rule-based entity matching that struggles with semantic variations, or employ late fusion at query time leading to redundancy and unresolved conflicts. This paper presents a general framework for constructing unified knowledge graphs through entity-level early fusion. We propose a multi-agent incremental fusion pipeline designed to handle diverse data formats. We implement the framework for three data types: structured databases (MySQL), semi-structured data (JSON), and unstructured text. The core innovation is a dual-support conflict detection mechanism combining rule-based text similarity with structure-based graph topology, followed by large language model (LLM)-assisted resolution. We evaluate the framework using a music domain case study integrating four sources (Chinook, Spotify, MusicBrainz, Wikipedia), creating overlaps and property conflicts. Our benchmark comprises 50 questions across five reasoning categories, evaluated against two baselines: multi-agent late fusion retrieval-augmented generation (RAG) and independent GraphRAG. Results demonstrate clear advantages: 72.1% win rate in qualitative evaluation, near-perfect faithfulness (0.955) and context precision (0.932), 18.9% improvement in context recall (0.876 vs 0.737), and 64.6% improvement in multi-hop reasoning context recall (0.874 vs 0.531). These findings validate that entity-level fusion during graph construction provides stronger foundations for retrievalaugmented generation compared to query-time fusion, and the framework is designed to be generalizable to other domains requiring multi-source integration.

# **1. Introduction**

Real-world applications increasingly require integrating heterogeneous data sources to obtain comprehensive information about entities, where multiple sources provide complementary yet conflicting information. Traditional fusion approaches face fundamental challenges: rulebased entity matching struggles with semantic variations and requires manual threshold tuning ([Primpeli](#page--1-0) & Bizer, 2021; Saeedi et al., 2018), while late fusion maintains independent sources until query time, leading to redundant retrieval and unresolved entity conflicts (Salve et al., 2024; Zhu, [Zhang,](#page--1-1) et al., 2024). These difficulties stem from the lack of unified entity-centric representation across heterogeneous sources. This paper addresses the scenario where multiple heterogeneous sources describe overlapping entities within the same domain, and the goal is to construct a unified knowledge graph that supports cross-source question answering.

Knowledge graphs provide an effective representation for multisource data fusion (Dong et al., 2014; Hogan et al., 2021), enabling explicit entity modeling, relationship structures, and provenance tracking. However, constructing unified knowledge graphs through entitylevel fusion remains challenging: determining whether entities from different sources represent the same real-world object requires both textual similarity analysis and structural relationship reasoning. This challenge involves three interrelated subproblems: cross-source entity identification under name variations and format differences, property conflict resolution when sources provide contradictory attribute values, and relationship integration to preserve multi-source provenance while establishing a coherent graph topology. Recent advances in large language models (LLMs) offer semantic understanding capabilities (Li et al., 2024; Peng, Zhang, et al., 2024), yet systematic integration of LLM reasoning with graph topology analysis for intelligent conflict detection and resolution remains underexplored.

This paper presents a novel framework for multi-source knowledge graph construction through LLM-assisted incremental fusion. We propose a dual-support conflict detection mechanism that combines

*E-mail addresses:* [ziqiu.huang@utas.edu.au](mailto:ziqiu.huang@utas.edu.au) (Z. Huang), [yang.wenli@utas.edu.au](mailto:yang.wenli@utas.edu.au) (W. Yang), [x.li@utas.edu.au](mailto:x.li@utas.edu.au) (X. Li), [quan.bai@utas.edu.au](mailto:quan.bai@utas.edu.au) (Q. Bai).

<span id="page-0-0"></span><sup>∗</sup> Corresponding author.

#### **7. Conclusion and future work**

This paper presents a general framework for multi-source knowledge graph construction through LLM-assisted incremental fusion, enabling entity-level early fusion across heterogeneous data sources. The framework comprises a multi-agent fusion pipeline (preprocessing, triplet extraction, conflict detection, conflict resolution, and graph integration), with its core innovation being the dual-support conflict detection mechanism (combining text matching and structure analysis) and tier-based routing for LLM-assisted resolution. While validated in the music domain, the framework is designed to be domain-agnostic and potentially applicable to other domains requiring multi-source data integration.

Experimental evaluation on a benchmark containing 50 cross-source reasoning questions demonstrates clear advantages of entity-level early fusion over late fusion. Compared to Baseline 2, which employs identical GraphRAG technology but performs fusion at query time, the Unified system achieves 8.1% improvement in faithfulness, 18.9% improvement in context recall, and 72.1% overall win rate in LLM Judge qualitative evaluation (82.7% in comprehensiveness dimension). Particularly in the multi-hop reasoning category, the Unified system achieves 0.87 context recall compared to Baseline 2's 0.54, as early fusion preserves continuous cross-source relationship paths while late fusion methods sever these paths across independent graphs. The dualsupport mechanism achieves 72% automation rate with only 28% of ambiguous cases requiring LLM intervention, effectively balancing decision quality and computational cost.

This study has several limitations. First, while the proposed framework is designed to be domain-general, experimental validation was conducted only in the music domain; testing in additional domains (e-commerce, news, biomedical) is needed to fully validate its generalizability. Second, during experimental preparation, we were unable to identify publicly available benchmarks with sufficient relevance and compatibility for our multi-source heterogeneous data scenario; existing multi-source datasets either focus on single data formats (pure text or pure structured data) or lack explicit entity-level fusion evaluation tasks, necessitating construction of a 50-question benchmark with corresponding four-source dataset. Third, we similarly could not find open-source baseline systems deployable within reasonable time and resource constraints; while some related open-source implementations exist, they either suffer from dependency conflicts and runtime errors due to lack of maintenance, or require substantial modifications to adapt to our heterogeneous data scenario, leading us to implement two baseline systems to ensure fair and controlled comparison. Additionally, current experiments were conducted at a scale of approximately 4000 nodes and 12,000 relationships; performance in larger-scale knowledge graph construction scenarios requires further investigation. Finally, the framework is designed for one-time incremental fusion, where each new data source is integrated sequentially into a growing knowledge graph. It does not address scenarios requiring continuous updates to existing entities, versioning of entity states across time, or rollback of conflicting new information; dynamic knowledge graph maintenance is a distinct and open problem beyond the scope of this work.

Future work includes several important directions. First, applying the framework to additional domains to validate its cross-domain effectiveness, particularly in domains with different data characteristics (such as highly structured enterprise data or natural languageintensive historical documents). Second, as multi-source knowledge graph research advances, we anticipate evaluating this framework using community-adopted standardized benchmarks to enable more direct and fair comparisons with other methods. Third, utilizing more advanced baseline methods (including recently published multi-source fusion techniques and knowledge graph construction approaches) for comparative testing to more comprehensively assess performance advantages and applicability. Fourth, conducting systematic ablation studies to quantify the individual contribution of each pipeline component, including text-only versus structure-only versus dual-support matching, alternative tier routing strategies, and the impact of different LLM models on resolution quality. Finally, exploring optimization strategies for larger-scale scenarios, such as distributed fusion processing and incremental update mechanisms. A particularly important direction is extending the framework to support dynamic knowledge graph maintenance, including versioning strategies for entity states, conflict resolution policies when new data contradicts previously fused information, and consistency guarantees for continuously evolving multi-source environments.

This research systematically validates the advantages of entity-level early fusion in multi-source knowledge graph construction, demonstrating that resolving entity conflicts during graph construction outperforms coordinating independent systems at query time for downstream question answering performance. The proposed general framework provides a viable technical approach for multi-source heterogeneous data integration, combining LLM semantic understanding capabilities with knowledge graph structural analysis to achieve efficient and transparent entity fusion decisions. As large language models and knowledge graph technologies continue to advance, multi-source knowledge graph construction will play an increasingly important role in practical application scenarios, providing unified, consistent, and explainable knowledge foundations for intelligent information systems.

#### **CRediT authorship contribution statement**

**Ziqiu Huang:** Software, Investigation, Data curation, Formal analysis, Writing – original draft, Visualization. **Wenli Yang:** Conceptualization, Methodology, Validation, Resources, Supervision, Project administration, Writing – review & editing. **Xiang Li:** Conceptualization, Methodology, Validation, Resources, Supervision, Writing – review & editing. **Quan Bai:** Conceptualization, Methodology, Validation, Resources, Supervision, Project administration, Writing – review & editing.

#### **Funding**

This research did not receive any specific grant from funding agencies in the public, commercial, or not-for-profit sectors.

#### **Code and data availability**

The source code for this work is maintained in a private GitHub repository at <https://github.com/theFirstHuang/multisource-fusion-rag> and will be made available to reviewers upon request and publicly released upon paper acceptance. The benchmark dataset comprising 50 multi-source reasoning questions with ground truth answers and source annotations is publicly available at Hugging Face: [https://huggingface.](https://huggingface.co/datasets/Cool-EdwardH/heterogeneous-multisource-kg-benchmark) [co/datasets/Cool-EdwardH/heterogeneous-multisource-kg-benchmark](https://huggingface.co/datasets/Cool-EdwardH/heterogeneous-multisource-kg-benchmark). The benchmark supports reproducibility and enables future research in multi-source knowledge graph evaluation.

## **Declaration of Generative AI and AI-assisted Technologies in the Manuscript Preparation Process**

During the preparation of this work, the authors used generative AI tools (including ChatGPT, Claude, and GitHub Copilot) to improve language clarity and readability, and to assist with code development. After using these tools, the authors reviewed and edited the content as needed and take full responsibility for the content of the published article.

# **Declaration of competing interest**

The authors declare that they have no known competing financial interests or personal relationships that could have appeared to influence the work reported in this paper.

**Table A.1** Performance metrics by category (All systems)

| Category                          | Sys | F    | AC   | CR   | CP   | AR   | BERT | ROUGE | CU   | NS   |
|-----------------------------------|-----|------|------|------|------|------|------|-------|------|------|
| Source Attribution                | Uni | 0.96 | 0.58 | 0.94 | 0.97 | 0.65 | 0.67 | 0.24  | 0.68 | 0.19 |
|                                   | B2  | 0.86 | 0.53 | 0.81 | 0.94 | 0.64 | 0.64 | 0.21  | 0.59 | 0.20 |
|                                   | B1  | 0.63 | 0.48 | 0.20 | 0.96 | 0.67 | 0.60 | 0.17  | 0.58 | 0.22 |
| Entity Integration                | Uni | 0.95 | 0.61 | 0.84 | 0.89 | 0.66 | 0.66 | 0.25  | 0.58 | 0.19 |
|                                   | B2  | 0.92 | 0.65 | 0.74 | 0.89 | 0.65 | 0.65 | 0.24  | 0.55 | 0.18 |
|                                   | B1  | 0.64 | 0.52 | 0.22 | 0.97 | 0.65 | 0.61 | 0.19  | 0.69 | 0.18 |
| Conflict Analysis                 | Uni | 0.95 | 0.66 | 0.83 | 0.90 | 0.63 | 0.65 | 0.22  | 0.66 | 0.14 |
|                                   | B2  | 0.89 | 0.60 | 0.81 | 0.78 | 0.61 | 0.64 | 0.22  | 0.60 | 0.14 |
|                                   | B1  | 0.57 | 0.49 | 0.34 | 0.87 | 0.61 | 0.60 | 0.17  | 0.58 | 0.13 |
| Relationship & Community Analysis | Uni | 0.96 | 0.54 | 0.89 | 0.96 | 0.58 | 0.66 | 0.24  | 0.76 | 0.16 |
|                                   | B2  | 0.90 | 0.58 | 0.78 | 0.89 | 0.60 | 0.64 | 0.22  | 0.67 | 0.18 |
|                                   | B1  | 0.70 | 0.49 | 0.38 | 0.81 | 0.63 | 0.59 | 0.17  | 0.69 | 0.19 |
| Multi-hop Reasoning               | Uni | 0.95 | 0.62 | 0.87 | 0.95 | 0.59 | 0.67 | 0.25  | 0.74 | 0.12 |
|                                   | B2  | 0.85 | 0.65 | 0.54 | 0.92 | 0.59 | 0.65 | 0.24  | 0.65 | 0.18 |
|                                   | B1  | 0.52 | 0.50 | 0.17 | 1.00 | 0.62 | 0.58 | 0.18  | 0.56 | 0.13 |

F = Faithfulness, AC = Answer Correctness, CR = Context Recall, CP = Context Precision, AR = Answer Relevancy, BERT = BERTScore F1, ROUGE = ROUGE-L F, CU = Context Utilization, NS = Avg Noise Sensitivity (lower is better)

**Table B.1** KG fusion phase configuration.

| Component           | Configuration                                  |
|---------------------|------------------------------------------------|
| LLM Service         | GPT-5-mini, temp = 0.0                         |
|                     | max_tokens = 30k, timeout = 300 s              |
| Triplet Extraction  | Three-branch architecture                      |
|                     | Batch: JSON 3, Wikipedia 4–8 entities          |
| Conflict Detection  | Dual-support (text + structure)                |
|                     | Similarity: Levenshtein, Jaro–Winkler, Jaccard |
|                     | Threshold: 0.3 (general), 0.7 (high)           |
| Conflict Resolution | Tier 1/4: direct; Tier 2/3: LLM                |
|                     | Routing: 70% automatic, 30% LLM                |
|                     | Confidence scoring: 0.0–1.0                    |
| Property Processing | HashMap cache (O(1) lookup)                    |
|                     | LLM-based with transitive inference            |
| Graph Integration   | Neo4j 5.25.1, APOC enabled                     |
|                     | Post-write validation + rollback               |
|                     |                                                |

**Table B.2** GraphRAG implementation phase configuration.

| Component           | Configuration                              |
|---------------------|--------------------------------------------|
| LLM Service         | GPT-5-mini, temp = 0.0                     |
| Embedding Model     | Qwen3-Embedding-0.6B, 768-dim              |
|                     | MPS acceleration, batch = 32               |
|                     | Cosine similarity                          |
| Community Detection | Hierarchical Leiden (graspologic)          |
|                     | Max cluster size: 10 nodes                 |
|                     | Random seed: 0xDEADBEEF                    |
|                     | Hierarchy levels: automatically determined |
| Community Summary   | LLM-generated per community per level      |
| Vector Storage      | ChromaDB v0.4.22                           |
|                     | Dual index (communities + entities)        |
|                     | Cosine similarity                          |
| DRIFT Retrieval     | PRIMER: 3 follow-up questions              |
|                     | FOLLOW-UP: 2 iterations × 3 questions      |
|                     | Top-k: 5 communities, 10 entities          |

#### **Appendix A. Category-wise performance analysis**

See Table A.1

## **Appendix B. System configuration**

See Table B.1. See Table B.2.

#### **Appendix C. LLM conflict resolution examples**

This appendix presents actual structured conflict resolution decisions from the Conflict Resolution Agent. The prompt provides the model with data source context, knowledge graph schema, candidate entities with match evidence, and decision guidelines. The model returns a structured JSON decision containing an action (merge or create\_new), a target node when applicable, a confidence score (0.0–1.0), and a brief reasoning field.

*Example 1: Merge decision (confidence 0.65).* Input entity: ''Começaria Tudo Outra Vez (Maria Creuza)'' (Track, from Spotify). Candidate: ''Outra Vez'' (existing Track in KG).

*Structured JSON output:*

```
{
"entity_name": "Começaria Tudo Outra Vez (Maria Creuza)",
"action": "merge",
"target_node": "Outra Vez",
"confidence": 0.65,
"reasoning": "title variant with performer attribution"
}
```

*Example 2: Create new decision (confidence 0.7).* Input entity: ''Carry On'' (Album, from Spotify). Candidate: ''The Battle Rages On'' (existing Album in KG).

*Structured JSON output:*

```
{
"entity_name": "Carry On",
"action": "create_new",
"target_node": null,
"confidence": 0.70,
"reasoning": "short ambiguous title without supporting
candidate"
}
```

*Example 3: Edge case merge (confidence 0.55).* Input entity: ''BBC Sessions [Disc 2] [Live]'' (Album, from Spotify). Candidate: ''BBC Sessions [Disc 1] [Live]'' (existing Album in KG).

*Structured JSON output:*

```
{
"entity_name": "BBC Sessions [Disc 2] [Live]",
"action": "merge",
"target_node": "BBC Sessions [Disc 1] [Live]",
"confidence": 0.55,
"reasoning": "possible multi-disc variant requiring fur-
ther verification"
}
```

### **Data availability**

The source code for this work is maintained in a private GitHub repository at <https://github.com/theFirstHuang/multisource-fusion-rag> and will be made available to reviewers upon request and publicly released upon paper acceptance. The benchmark dataset comprising 50 multi-source reasoning questions with ground truth answers and source annotations is publicly available at Hugging Face: [https://huggingface.](https://huggingface.co/datasets/Cool-EdwardH/heterogeneous-multisource-kg-benchmark) [co/datasets/Cool-EdwardH/heterogeneous-multisource-kg-benchmark](https://huggingface.co/datasets/Cool-EdwardH/heterogeneous-multisource-kg-benchmark). The benchmark supports reproducibility and enables future research in multi-source knowledge graph evaluation.

#### **References**

- Cheng, J., Lu, C., Yang, L., Chen, G., & Zhang, F. (2025). EasyEA: Large language model is all you need in entity alignment between knowledge graphs. In *Findings of the association for computational linguistics: ACL 2025* (pp. 20981–20995). [http:](http://dx.doi.org/10.18653/v1/2025.findings-acl.1080) [//dx.doi.org/10.18653/v1/2025.findings-acl.1080.](http://dx.doi.org/10.18653/v1/2025.findings-acl.1080)
- Chung, J., Pedigo, B. D., Bridgeford, E. W., Varjavand, B. K., Helm, H. S., & Vogelstein, J. T. (2019). GraSPy: Graph statistics in python. *Journal of Machine Learning Research*, *20*(158), 1–7, URL [http://www.jmlr.org/papers/v20/19-490.html.](http://www.jmlr.org/papers/v20/19-490.html)
- Dong, X. L., Gabrilovich, E., Heitz, G., Horn, W., Murphy, K., Sun, S., & Zhang, W. (2014). From data fusion to knowledge fusion. *Proceedings of the VLDB Endowment*, *7*(10), 881–892. <http://dx.doi.org/10.14778/2732951.2732962>.
- Dou, W., Shen, D., Zhou, X., Bai, H., Kou, Y., Nie, T., Cui, H., & Yu, G. (2024). Enhancing deep entity resolution with integrated blocker-matcher training: Balancing consensus and discrepancy. In *Proceedings of the 33rd ACM international conference on information and knowledge management*. [http://dx.doi.org/10.1145/](http://dx.doi.org/10.1145/3627673.3679843) [3627673.3679843.](http://dx.doi.org/10.1145/3627673.3679843)
- Edge, D., Trinh, H., Cheng, N., Bradley, J., Chao, A., Mody, A., Truitt, S., & Larson, J. (2024). From local to global: A graph RAG approach to query-focused summarization. [arXiv:2404.16130.](http://arxiv.org/abs/2404.16130) URL <https://arxiv.org/abs/2404.16130>.
- Es, S., James, J., Espinosa Anke, L., & Schockaert, S. (2024). RAGAS: Automated evaluation of retrieval augmented generation. In *Proceedings of the 18th conference of the European chapter of the association for computational linguistics: system demonstrations* (pp. 150–158). St. Julians, Malta: Association for Computational Linguistics, <http://dx.doi.org/10.18653/v1/2024.eacl-demo.16>.
- Fan, M., Han, X., Fan, J., Chai, C., Tang, N., Li, G., & Du, X. (2024). Cost-effective in-context learning for entity resolution: A design space exploration. In *Proceedings of the IEEE 40th international conference on data engineering* (pp. 3696–3709). [http://dx.doi.org/10.1109/ICDE60146.2024.00284.](http://dx.doi.org/10.1109/ICDE60146.2024.00284)
- Han, C., Li, Y., & Tang, X. (2025). DocPolicyKG: A lightweight LLM-based framework for knowledge graph construction from Chinese policy documents. In *Proceedings of the 34th ACM international conference on information and knowledge management* (pp. 4753–4757). <http://dx.doi.org/10.1145/3746252.3760904>.
- Hogan, A., Blomqvist, E., Cochez, M., d'Amato, C., de Melo, G., Gutierrez, C., Kirrane, S., Labra Gayo, J. E., Navigli, R., Neumaier, S., Ngonga Ngomo, A.-C., Polleres, A., Rashid, S. M., Rula, A., Schmelzeisen, L., Sequeda, J., Staab, S., & Zimmermann, A. (2021). Knowledge graphs. *ACM Computing Surveys*, *54*(4), 1–37. [http://dx.doi.org/10.1145/3447772.](http://dx.doi.org/10.1145/3447772)
- Huang, Z. (2024). Disambiguate entity matching using large language models through relation discovery. arXiv preprint [arXiv:2403.17344](http://arxiv.org/abs/2403.17344). URL [https://arxiv.org/abs/](https://arxiv.org/abs/2403.17344) [2403.17344.](https://arxiv.org/abs/2403.17344)
- Huang, Z., Li, X., Yang, W., Bai, Q., Green, D., & McMahon, C. (2025). MultiRAG: An agentic multi-modal and multi-source retrieval-augmented generation framework for scientific research. In M. Liu, X. Yu, C. Xu, & Y. Song (Eds.), *Lecture notes in computer science*: *vol. 16370*, *AI 2025: advances in artificial intelligence – 38th australasian joint conference on artificial intelligence, AI 2025, canberra, ACT, Australia, December 1–5, 2025, proceedings, part i* (pp. 15–27). Cham: Springer Nature, [http:](http://dx.doi.org/10.1007/978-981-95-4969-6) [//dx.doi.org/10.1007/978-981-95-4969-6.](http://dx.doi.org/10.1007/978-981-95-4969-6)
- Lewis, P., Perez, E., Piktus, A., Petroni, F., [Karpukhin,](http://refhub.elsevier.com/S2667-3053(26)00049-9/sb12) V., Goyal, N., Küttler, H., Lewis, M., Yih, W.-t., [Rocktäschel,](http://refhub.elsevier.com/S2667-3053(26)00049-9/sb12) T., Riedel, S., & Kiela, D. (2020). Retrievalaugmented generation for [knowledge-intensive](http://refhub.elsevier.com/S2667-3053(26)00049-9/sb12) NLP tasks. *Advances in Neural Information Processing Systems*, *33*, [9459–9474.](http://refhub.elsevier.com/S2667-3053(26)00049-9/sb12)

- Li, H., Feng, L., Li, S., Hao, F., Zhang, C. J., Song, Y., & Chen, L. (2024). On leveraging large language models for enhancing entity resolution: A cost-efficient approach. arXiv preprint [arXiv:2401.03426.](http://arxiv.org/abs/2401.03426) URL <https://arxiv.org/abs/2401.03426>.
- Li, Q., Li, Y., Gao, J., Zhao, B., Fan, W., & Han, J. (2014). Resolving conflicts in heterogeneous data by truth discovery and source reliability estimation. In *Proceedings of the 2014 ACM SIGMOD international conference on management of data* (pp. 1187–1198). ACM, <http://dx.doi.org/10.1145/2588555.2610509>.
- Li, G., Wang, P., & Ke, W. (2023). [Revisiting](http://refhub.elsevier.com/S2667-3053(26)00049-9/sb15) large language models as zero-shot relation extractors. In *Findings of the association for [computational](http://refhub.elsevier.com/S2667-3053(26)00049-9/sb15) linguistics: EMNLP 2023* (pp. 6877–6892). Singapore: Association for [Computational](http://refhub.elsevier.com/S2667-3053(26)00049-9/sb15) Linguistics.
- Lin, C. Y. (2004). ROUGE: A package for automatic evaluation of summaries. In *Text summarization branches out* (pp. 74–81). Barcelona, Spain: Association for Computational Linguistics, URL [https://aclanthology.org/W04-1013/.](https://aclanthology.org/W04-1013/)
- Liu, Y., Wang, H., Zhang, L., Chen, Q., & Zhou, X. (2025). Dynamic vulnerability knowledge graph construction via multi-source data fusion and large language model reasoning. *Electronics*, *14*(12), 2334. [http://dx.doi.org/10.3390/](http://dx.doi.org/10.3390/electronics14122334) [electronics14122334.](http://dx.doi.org/10.3390/electronics14122334)
- Malkov, Y. A., & Yashunin, D. A. (2020). Efficient and robust approximate nearest neighbor search using hierarchical navigable small world graphs. *IEEE Transactions on Pattern Analysis and Machine Intelligence*, *42*(4), 824–836. [http://dx.doi.org/10.](http://dx.doi.org/10.1109/TPAMI.2018.2889473) [1109/TPAMI.2018.2889473.](http://dx.doi.org/10.1109/TPAMI.2018.2889473)
- Mo, B., Yu, K., Kazdan, J., Cabezas, J., Mpala, P., Yu, L., Cundy, C., [Kanatsoulis,](http://refhub.elsevier.com/S2667-3053(26)00049-9/sb19) C., & Koyejo, S. (2025). KGGen: Extracting [knowledge](http://refhub.elsevier.com/S2667-3053(26)00049-9/sb19) graphs from plain text with language models. In *Advances in neural [information](http://refhub.elsevier.com/S2667-3053(26)00049-9/sb19) processing systems*: *Vol. 38*.
- Pan, S., Luo, L., Wang, Y., Chen, C., Wang, J., & Wu, X. (2024). Unifying large language models and knowledge graphs: A roadmap. *IEEE Transactions on Knowledge and Data Engineering*, *36*(7), 3580–3599. [http://dx.doi.org/10.1109/TKDE.2024.3352100.](http://dx.doi.org/10.1109/TKDE.2024.3352100)
- Papaluca, A., Krefl, D., Rodríguez Méndez, S. J., Lensky, A., & Suominen, H. (2024). Zero- and few-shots knowledge graph triplet extraction with large language models. In *Proceedings of the 1st workshop on knowledge graphs and large language models (kaLLM 2024)* (pp. 12–23). Bangkok, Thailand: Association for Computational Linguistics, URL [https://aclanthology.org/2024.kallm-1.2/.](https://aclanthology.org/2024.kallm-1.2/)
- Park, J., Kim, S., Lee, H., & Choi, M. (2024). FusionMaestro: Harmonizing early fusion, late fusion, and LLM reasoning for multi-granular table-text retrieval. In *OpenReview*. URL [https://openreview.net/forum?id=jneVchiRlT.](https://openreview.net/forum?id=jneVchiRlT)
- Peeters, R., Steiner, A., & Bizer, C. (2025). Entity [matching](http://refhub.elsevier.com/S2667-3053(26)00049-9/sb23) using large language models. In *Proceedings of the 28th [international](http://refhub.elsevier.com/S2667-3053(26)00049-9/sb23) conference on extending database technology* (pp. [529–541\).](http://refhub.elsevier.com/S2667-3053(26)00049-9/sb23)
- Peng, B., Galley, M., Scialom, T., Yu, Y., Zhou, P., & Gao, J. (2024). Graph retrievalaugmented generation: A survey. arXiv preprint [arXiv:2408.08921.](http://arxiv.org/abs/2408.08921) URL [https:](https://arxiv.org/abs/2408.08921) [//arxiv.org/abs/2408.08921.](https://arxiv.org/abs/2408.08921)
- Peng, H., Zhang, P., Tang, J., Xu, H., & Zeng, W. (2024). Detect-then-resolve: Enhancing knowledge graph conflict resolution with large language model. *Mathematics*, *12*(15), 2318. [http://dx.doi.org/10.3390/math12152318.](http://dx.doi.org/10.3390/math12152318)
- Primpeli, A., & Bizer, C. (2021). Graph-boosted active learning for multi-source entity resolution. In *Lecture notes in computer science*: *Vol. 12922*, *The semantic web – ISWC 2021* (pp. 182–199). Springer, [http://dx.doi.org/10.1007/978-3-030-88361-4\\_11](http://dx.doi.org/10.1007/978-3-030-88361-4_11).
- Ru, D., Qiu, L., Hu, X., Zhang, T., Shi, P., Chang, S., Jiayang, C., Wang, C., Sun, S., Li, H., Zhang, Z., Wang, B., Jiang, J., He, T., Wang, Z., Liu, P., Zhang, Y., & Zhang, Z. (2024). RAGChecker: A fine-grained framework for diagnosing retrievalaugmented generation. In *Advances in neural information processing systems*: *vol. 37*, Amazon Science. URL [https://arxiv.org/abs/2408.08067.](https://arxiv.org/abs/2408.08067)
- Saeedi, A., Nentwig, M., Peukert, E., & Rahm, E. (2018). Scalable matching and clustering of entities with FAMER. *Complex Systems Informatics and Modeling Quarterly*, *16*, 61–83. [http://dx.doi.org/10.7250/csimq.2018-16.04.](http://dx.doi.org/10.7250/csimq.2018-16.04)
- Salve, A., Padhi, S., Chimmula, S., Sehgal, A., & Singh, G. (2024). A collaborative multiagent approach to retrieval-augmented generation across diverse data sources. arXiv preprint [arXiv:2412.05838](http://arxiv.org/abs/2412.05838). URL <https://arxiv.org/abs/2412.05838>.
- Tang, J., Dou, W., Shen, D., Nie, T., & Kou, Y. (2024). Towards long-text entity resolution with chain-of-thought knowledge augmentation from large language models. In *Lecture notes in computer science*: *Vol. 14854*, *Database systems for advanced applications: DASFAA 2024* (pp. 347–363). Springer, [http://dx.doi.org/](http://dx.doi.org/10.1007/978-981-97-5569-1_20) [10.1007/978-981-97-5569-1\\_20](http://dx.doi.org/10.1007/978-981-97-5569-1_20).
- Thakur, N., Reimers, N., Rücklé, A., Srivastava, A., & Gurevych, I. (2021). BEIR: A heterogeneous benchmark for zero-shot evaluation of information retrieval models. In *Proceedings of the neural information processing systems track on datasets and benchmarks*: *Vol. 1*, URL [https://datasets-benchmarks-proceedings.neurips.cc/paper/](https://datasets-benchmarks-proceedings.neurips.cc/paper/2021/hash/65b9eea6e1cc6bb9f0cd2a47751a186f-Abstract-round2.html) [2021/hash/65b9eea6e1cc6bb9f0cd2a47751a186f-Abstract-round2.html](https://datasets-benchmarks-proceedings.neurips.cc/paper/2021/hash/65b9eea6e1cc6bb9f0cd2a47751a186f-Abstract-round2.html).
- Traag, V. A., Waltman, L., & van Eck, N. J. (2019). From Louvain to Leiden: guaranteeing well-connected communities. *Scientific Reports*, *9*(1), 5233. [http://dx.](http://dx.doi.org/10.1038/s41598-019-41695-z) [doi.org/10.1038/s41598-019-41695-z](http://dx.doi.org/10.1038/s41598-019-41695-z).
- Van Assche, D., Rojas, J., De Meester, B., & Colpaert, P. (2026). Incremental knowledge graph construction from heterogeneous data sources. *Semantic Web*, *17*(2), [http:](http://dx.doi.org/10.1177/22104968251412270) [//dx.doi.org/10.1177/22104968251412270](http://dx.doi.org/10.1177/22104968251412270).
- Wadhwa, S., Amir, S., & Wallace, B. (2023). Revisiting relation extraction in the era of large language models. In *Proceedings of the 61st annual meeting of the association for computational linguistics (volume 1: long papers)* (pp. 15566–15589). Toronto, Canada: Association for Computational Linguistics, [http://dx.doi.org/10.18653/v1/](http://dx.doi.org/10.18653/v1/2023.acl-long.868) [2023.acl-long.868.](http://dx.doi.org/10.18653/v1/2023.acl-long.868)

- Yan, C., Fang, X., Huang, X., Guo, C., & Wu, J. (2023). A solution and practice for combining multi-source heterogeneous data to construct enterprise knowledge graph. *Frontiers in Big Data*, *6*, Article 1278153. [http://dx.doi.org/10.3389/fdata.](http://dx.doi.org/10.3389/fdata.2023.1278153) [2023.1278153.](http://dx.doi.org/10.3389/fdata.2023.1278153)
- Yang, Z., Tao, X., Cai, T., Tang, Y., Xie, H., Li, L., Li, J., & Li, Q. (2025). A survey on multi-view knowledge graph: Generation, fusion, applications and future directions. In *Proceedings of the 34th international joint conference on artificial intelligence* (pp. 10788–10796). [http://dx.doi.org/10.24963/ijcai.2025/1197.](http://dx.doi.org/10.24963/ijcai.2025/1197)
- Yu, S., Cheng, M., Liu, Q., Wang, D., Yang, J., Ouyang, J., Luo, Y., Lei, C., & Chen, E. (2025). Multi-source knowledge pruning for retrieval-augmented generation: A benchmark and empirical study. In *Proceedings of the 34th ACM international conference on information and knowledge management*. ACM, [http://dx.doi.org/10.](http://dx.doi.org/10.1145/3746252.3761340) [1145/3746252.3761340](http://dx.doi.org/10.1145/3746252.3761340).
- Yu, H., Gan, A., Zhang, K., Tong, S., Liu, Q., & Liu, Z. (2024). Evaluation of retrieval-augmented generation: A survey. arXiv preprint [arXiv:2405.07437.](http://arxiv.org/abs/2405.07437) URL [https://arxiv.org/abs/2405.07437.](https://arxiv.org/abs/2405.07437)

- Zhang, T., Kishore, V., Wu, F., Weinberger, K. Q., & Artzi, Y. (2020). BERTScore: Evaluating text generation with BERT. In *International conference on learning representations*. URL <https://openreview.net/forum?id=SkeHuCVFDr>.
- Zhao, X., Jia, Y., Li, A., Jiang, R., & Song, Y. (2020). Multi-source knowledge fusion: a survey. *World Wide Web*, *23*(4), 2567–2592. [http://dx.doi.org/10.1007/s11280-](http://dx.doi.org/10.1007/s11280-020-00811-0) [020-00811-0.](http://dx.doi.org/10.1007/s11280-020-00811-0)
- Zhu, Y., Wang, X., Chen, J., Qiao, S., Ou, Y., Yao, Y., Deng, S., Chen, H., & Zhang, N. (2024). LLMs for knowledge graph construction and reasoning: Recent capabilities and future opportunities. *World Wide Web*, *27*(58), [http://dx.doi.org/10.1007/](http://dx.doi.org/10.1007/s11280-024-01297-w) [s11280-024-01297-w.](http://dx.doi.org/10.1007/s11280-024-01297-w)
- Zhu, Y., Zhang, Y., Lyu, X., Wu, Y., Li, X., & Liu, Y. (2024). FusionQuery: On-demand fusion queries over multi-source heterogeneous data. *Proceedings of the VLDB Endowment*, *17*(6), 1337–1349. [http://dx.doi.org/10.14778/3648160.3648174.](http://dx.doi.org/10.14778/3648160.3648174)