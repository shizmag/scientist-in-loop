Contents lists available at [ScienceDirect](https://www.elsevier.com/locate/datak)

# Data & Knowledge Engineering

journal homepage: [www.elsevier.com/locate/datak](https://www.elsevier.com/locate/datak)

# When structure predicts hallucination: Aligning LLMs with knowledge graph features[✩](#page-0-0)

[Ushtar](#page--1-0) Ali [a](#page-0-1),[b](#page-0-2) ,[∗](#page-0-3) , Steven Lynden [b](#page-0-2) , [Akiyoshi](#page--1-1) Matono [b](#page-0-2) , Toshiyuki Amagasa [a](#page-0-1)

- <span id="page-0-1"></span><sup>a</sup> *University of Tsukuba 1-1-1 Tennodai, Tsukuba, Ibaraki 305-8577, Japan*
- <span id="page-0-2"></span><sup>b</sup> *National Institute of Advanced Industrial Science and Technology, 2-4-7 Aomi, Koto-ku, Tokyo 135-0064, Japan*

## A R T I C L E I N F O

#### *Keywords:*

Large language models Graph management and analytics Data science techniques LLMs hallucination

## A B S T R A C T

Large Language Models (LLMs) have demonstrated remarkable factual accuracy in producing human-like and AI-generated texts across a wide range of natural language tasks, including question answering. Despite these advances, their tendency to hallucinate and produce fabricated, false or incorrect responses is a persistent limitation. This limitation undermines their reliability and remains a critical challenge, especially in areas where high precision and trustworthiness are required. To address this challenge, we investigate whether the features derived from Knowledge Graphs (KGs) align with the accuracy of answers produced by the LLMs. In particular, we focus on entropy-based KG features, which capture diversity and uncertainty within structured knowledge. By analyzing the correlation between the entropybased KG features and the accuracy of LLM responses, we are able to identify ''blind spots'' where LLMs are prone to hallucination. This provides insights not only into when an LLM is correct, but also into the conditions under which it fails. We present results across several datasets, including two developed for this study, demonstrating that entropy-based KG features can effectively align with the accuracy of LLM responses. Motivated by these findings, we propose a probing strategy for assessing LLM accuracy by focusing on areas where LLM accuracy is weak. The experimental results confirm that KG features can guide the probing effectively, highlighting the importance of using structured features from KGs in building more reliable and hallucination-free AI based systems.

## **1. Introduction**

In recent years, large language models (LLMs) have exhibited significant advancements, achieving state-of-the-art performance across a wide range of natural language processing (NLP) tasks, including technical writing, question answering, code generation, and even multi-modal applications such as image generation. Despite these significant capabilities, LLMs exhibit a persistent limitation: *the generation of hallucinated content*, i.e., information that is factually incorrect or unverifiable through external knowledge sources. Such tendencies undermine their reliability, particularly in knowledge-intensive domains and decision-making applications where factual consistency is critical.

To address this issue, recent research has explored the coupling of Knowledge Graphs (KGs) with LLMs to improve their factual accuracy by providing contextual grounding. The Knowledge injection approach integrates structured facts or entity embeddings into model parameters during pretraining (e.g., KnowBERT [1]), enhancing factual recall but requiring model retraining. Retrieval

#### <https://doi.org/10.1016/j.datak.2026.102630>

Received 9 January 2026; Received in revised form 12 June 2026; Accepted 2 July 2026

Available online 8 July 2026

0169-023X/© 2026 The Authors. Published by Elsevier B.V. This is an open access article under the CC BY license ( [http://creativecommons.org/licenses/by/4.0/ \)](http://creativecommons.org/licenses/by/4.0/).

<span id="page-0-0"></span><sup>✩</sup> This article is part of <sup>a</sup> Special issue entitled: 'DEXA-DAWAK-2025\_Akoka' published in Data & Knowledge Engineering.

<span id="page-0-3"></span><sup>∗</sup> Corresponding author at: University of Tsukuba 1-1-1 Tennodai, Tsukuba, Ibaraki 305-8577, Japan. *E-mail addresses:* [ushtar.ali@kde.cs.tsukuba.ac.jp](mailto:ushtar.ali@kde.cs.tsukuba.ac.jp), [ushtar.ali@aist.go.jp](mailto:ushtar.ali@aist.go.jp) (U. Ali).

**Fig. 5.** Pearson Correlation with Model's Accuracy for LC-QuAD dataset.

compare this probing strategy against two baselines: (i) random probing, where *𝑁* Q/As are randomly selected from the dataset, and (ii) a parameterized subgraph-based approximation approach based on KGLens [3].

The original KGLens approach probes subsets of the Wikidata KG to assess their alignment with LLM responses, whereas our WikiWebQuestions-based setting includes arbitrary queries spanning the entire Wikidata. As of December 2025, Wikidata contains 1.65 billion statements, making it infeasible to cover the entire dataset. Therefore, we adopt a simplified approximation of KGLens described here. Starting from a seed entity, we extract its direct neighbors and then expand once more from each first-hop neighbor to form a two-hop neighborhood, limiting the expansion to at most 50 edges per hop to retain tractability. We filter all the triples to retain only triples with meaningful factual relations and human-readable English labels, i.e., we discard links to external databases, etc., resulting in a two-hop subgraph of the Wikidata-KG centered on the seed entity. Each triple (*𝑠, 𝑝, 𝑜*) in the subgraph is associated with a confidence value representing the model's likelihood of failing on that fact. Following [3], this confidence is modeled as a Beta-distributed random variable where *𝜃𝑠,𝑝,𝑜* ∼ Beta(*𝛼𝑠,𝑝,𝑜, 𝛽𝑠,𝑝,𝑜*) denotes the probability of the LLM failing to answer the question about that fact. All triples start with a uniform prior: *𝛼* = 1, *𝛽* = 1. After querying the model, these parameters are updated based on its performance: if the model answers incorrectly, *𝛼* ← *𝛼* + 1; if correctly, *𝛽* ← *𝛽* + 1. These updates are propagated to neighboring edges as well, i.e., all triples sharing the same subject or object—under the assumption that related facts share correlated knowledge gaps. At each iteration, we select the triple with the highest expected failure probability *𝜃𝑠,𝑝,𝑜* for querying. For simplicity, and with a slight deviation from the method in [3], at each iteration, we select the triple with the highest expected failure probability *𝜃𝑠,𝑝,𝑜* and generate questions for each selected triple with the template as follows: ''What is the relation between s and o?'' where *𝑠* and *𝑜* are replaced with the subject and object entities in the triple. The generated question is prompted to the LLM and the answer is evaluated against the actual *𝑝* value from Wikidata-KG. We use another model (GPT4o-mini) to evaluate the model response. The evaluation prompt contains the question, the model's answer, and the actual Wikidata predicate and query: ''Does the answer match this information? The correct relation is: **<relation>**''.

**Fig. 6.** Evaluated LLMs answers distributions for Wikidata complex questions dataset.

**Fig. 7.** Pearson Correlation with Model's Accuracy WikidataComplexQuestions dataset.

Based on GPT4o-mini answers, (*𝛼, 𝛽*) are updated as follows. After a number of iterations, alignment between a subgraph centered around an entity and model's responses is measured by the average of all current *𝜃* values for all triples in the subgraph. A lower *𝜃̄ <sup>𝑒</sup>* value reflects a closer factual alignment between the LLM and the KG around the seed entity. At each iteration (up to 20 in our setting), we choose the edge (*𝑠, 𝑝, 𝑜*) with the highest expected failure probability *𝜃* = E[*𝜃𝑠,𝑝,𝑜*]. This constraint balances breadth (by exploring two-hop neighborhoods) and depth (by targeting the most uncertain facts), allowing the method to produce a meaningful alignment score without exhaustive querying.

## *7.1. Experiments*

For the probing evaluation, we used Mistral-7B because it achieved the strongest overall performance across the benchmark tasks when considering correct answers that were not accompanied by irrelevant content. This choice reduces the risk that downstream judgments are driven by noisy, off-topic, or over-generated responses rather than by the factual correctness of the answer itself. In particular, models with higher rates of irrelevant content may introduce additional variance into the evaluation, making it harder to distinguish genuine factual errors from artifacts of poor response quality. To evaluate our method against baselines, we randomly

**Fig. 8.** Evaluated LLMs answers distributions for WikiWeb2DBpedia dataset.

selected 250 single Q/As and 250 multiple Q/As from the WikiWebQuestions dataset (as it shows most significant correlation results). We ensure that each associated SPARQL query has exactly one bound entity, allowing us to construct a parameterized confidence graph centered on that entity. We compare the following approaches to select the top *𝑁* Q/As from the benchmark and compute their performance (% of correct questions):

- Theta: selecting *𝑁* lowest average theta values computed from the parameterized confidence subgraph built around an entity i.e., prioritizing Q/As that are estimated to be poorly aligned with Wikidata, similar to the KGLens [3] approach.
- Random: selecting random *𝑁* Q/A pairs.
- Entropy: combined correlated features explained below.

#### *7.1.1. Aggregating entropy-based correlated features*

We compute the combined score *𝑆*(*𝑑<sup>𝑖</sup>* ) for each question-answer pair *𝑞<sup>𝑖</sup>* by using the proposed six normalized features scaled to range [0*,* 1] where property entropy *𝑝<sup>𝑖</sup>* and entity entropy *𝑒<sup>𝑖</sup>* are negatively correlated features while number of sitelinks *𝑠<sup>𝑖</sup>* , number

**Fig. 9.** Pearson Correlation with Model's Accuracy for WikiWeb2DBpedia dataset.

of statements *𝑡 𝑖* , number of references *𝑟<sup>𝑖</sup>* , and last updated timestamp *𝑢<sup>𝑖</sup>* are correlated positively with LLM accuracy. We inverted the positively correlated features to ensure the uniform directionality. Final score is computed as:

$$S(d_i) = \frac{1}{6} \left[ (e_i + p_i) + \sum_{k \in \{s_i, t_i, r_i, u_i\}} (1 - k) \right]$$

Lower values of *𝑆*(*𝑑<sup>𝑖</sup>* ) exhibit higher accuracy. We select the top-*𝑁* entries ranked by *𝑆*(*𝑑<sup>𝑖</sup>* ) in descending order to probe the LLM.

#### *7.2. Results*

In this section we present and explore the probing results for both single-answer and multiple-answer questions. Fig. 11 shows the probing results for single-answer questions and Fig. 12 shows the results for questions with multiple answers. It is clearly visible that entropy based method outperforms both random probing method and KGLens approach. Mistral hallucinated more for entropy selected questions (both single answer and multiple answer questions). For questions with multiple answers, random probing method and KGLens almost perform identical when *𝑁* is lower. Results shows the effectiveness of entropy-based features in predicting LLM ''blind spots'' and measuring the KG-LLM alignments. Although we employed a simplified version of the KGLens approach to ensure computational feasibility for the dataset used, the entropy-based selection still proves to be an effective and practical KG–LLM alignment method for detecting LLM blind spots.

**Fig. 10.** Pearson correlation between entropy-based and structural KG features and language model accuracy across all models. Each bar represents a feature–model pair, where black bars denote statistically significant correlations (*𝑝 <* 0*.*05) and gray bars indicate less significant correlations (*𝑝 >* 0*.*05).

#### **8. From correlation to application: Practical utilization of entropy-guided insights**

This section outlines how the proposed entropy-guided approach can inform the improvement of large language models, offering insights for developers and researchers seeking to enhance factual grounding, reduce hallucination, and train language models more efficiently.

(1) **Training data:** It sample more training data from entities and properties with high entropy, since these correspond to regions where LLMs are most uncertain or hallucinate, thereby helps the model to handle uncertainty and improve its factual accuracy effectively. It prioritize the inclusion of low-frequency or sparsely linked entities, ensuring diverse structural coverage in the dataset. Also, it refreshes training data for entities whose entropy or last-modified timestamps changes rapidly ensuring factual

**Fig. 11.** LLM alignment scores for single-answer questions.

**Fig. 12.** LLM alignment scores for multiple-answer questions.

consistency over time. Moreover, it can act as a weighting factor during data selection for training by giving importance to the areas where uncertainty is high.

- (2) **Evaluation:** Evaluates LLMs knowledge in certain domains more effectively by using high entropy samples reflecting real world challenges. Identify systematic failure clusters by analyzing questions associated with high-entropy features. Use entropy–accuracy correlation coefficients as an additional diagnostic metric to compute confidence or model answer accuracy. Possibly combine model confidence scores with entropy-based risk into a single ''hallucination likelihood'' index.
- (3) **Prompts/interactions:** Trigger KG lookups or RAG modules automatically when an entity's entropy exceeds a threshold. Encourage model reasoning by self-verification prompts like ''double check your facts about X'' for responses involving high-entropy properties. Also, it can guide prompt tuning by showing the features leading to more stable outputs.

(4) **Fine-Tuning:** Helping models to focus on blind spots by conducting targeted fine-tuning on high-entropy, low-accuracy samples to improve performance. Use curriculum learning, starting from low-entropy (easy) examples and gradually increasing entropy. It can help to prioritize fine-tuning objectives i.e., weighting loss heavily where entropy value is greater. Moreover, it can be beneficial in handling the overfitting of the model by exposing the samples with diverse and less familiar entities in the dataset.

### **9. Conclusions and future work**

In this paper, we proposed that correlations may exist between LLM answer accuracy and KG structural features. We tested this hypothesis by extracting KG features and analyzing their correlations across four datasets targeting two general-purpose knowledge graphs. Our findings demonstrate that KG structural features correlate with LLM answer quality, particularly in the case of Wikidata, whose rich relational structure and curated references amplify the impact of entropy-based features on model accuracy. In particular, entities associated with higher structural uncertainty and weaker supporting graph structure were more likely to correspond to lower-quality or less reliable generated answers, while richer and more densely connected graph regions were generally associated with more stable responses.

Moreover, our entropy-based probing method highlights blind spots where KGs and LLMs are not aligned, supporting more efficient LLM evaluation and providing insight into when external structured knowledge may be beneficial. These findings also suggest several practical mitigation strategies, including uncertainty-aware gating, selective KG injection, deeper retrieval, and additional verification steps for high-risk entities or graph regions. Rather than uniformly applying KG augmentation, our results indicate that KG structural signals may help determine when external knowledge is likely to improve answer quality and when it may instead introduce additional noise.

In future work, we will further investigate these LLM-engineering implications and explore practical applications of adaptive KG-aware generation strategies, as outlined in Section 8.

#### **CRediT authorship contribution statement**

**Ushtar Ali:** Writing – original draft, Visualization, Validation, Data curation. **Steven Lynden:** Writing – review & editing, Methodology, Formal analysis, Conceptualization. **Akiyoshi Matono:** Writing – review & editing, Formal analysis, Conceptualization. **Toshiyuki Amagasa:** Writing – review & editing, Supervision, Formal analysis.

#### **Declaration of competing interest**

The authors declare that they have no known competing financial interests or personal relationships that could have appeared to influence the work reported in this paper.

#### **Acknowledgments**

This paper is based on results obtained from the project, ''Research and Development Project of the Enhanced infrastructures for Post-5G Information and Communication Systems'' (JPNP20017), commissioned by the New Energy and Industrial Technology Development Organization (NEDO) and JST CREST Grant Number JPMJCR22M2 .

#### **Appendix A. Prompt examples**

## *A.1. Wikidata complex dataset generation prompt example*

You are a Wikidata SPARQL expert and dataset curator. Your task is to create a dataset entry designed to test the factual accuracy and knowledge coverage of large language models.

### Requirements:

- The question must test factual information that can be directly verified through Wikidata.
- Use simple factual relations (either 1-hop or straightforward 2-hop), avoiding complex reasoning or inference chains.
- Do not use overly familiar entities such as Albert Einstein, Marie Curie, Barack Obama, or Napoleon.
- Cover a range of topics, including history, literature, art, architecture, sports, medicine, music, technology, philosophy, transportation, and the environment.

- Prioritize lesser-known or less frequently referenced entities.
- The SPARQL query should return a clear, verifiable answer directly from Wikidata.

The dataset entry must include:

```
 1. A natural language ''Question'' (English).
```

- 2. A valid ''SPARQL Query'' (executable on Wikidata).
- 3. The ''Answer'' as retrieved from Wikidata.

```
 Format output strictly as a JSON object:
 {
 ''Question'': ''...'',
 ''SPARQL'': ''...'',
 ''Answer'': ''...''
 }
 Generate ONE such entry.
```

#### *A.2. Dbpedia query generation prompt example*

You are given a question and its corresponding SPARQL query written for Wikidata. Convert the SPARQL query into valid DBpedia syntax that runs correctly on the public DBpedia Virtuoso endpoint.

Question: what is the first book Sherlock Holmes appeared in?

```
Wikidata Query:
```

```
SELECT DISTINCT ?x WHERE { ?x wdt:P674 wd:Q4653; wdt:P577 ?y. }
ORDER BY ?y LIMIT 1
```

Output ONLY the converted SPARQL query. No explanations, no backticks.

#### *A.3. Evaluation prompt example (LLM responses)*

Given the following question and correct answer, evaluate the provided model answer.

Question: Who succeeded the only British Prime Minister to have won the Nobel Prize in Literature in his final term?

Correct Answer: Anthony Eden Model Answer: James Callaghan.

Classify the model answer into one of the following categories:

- 1. Perfectly accurate. 2. Very accurate. 3. Accurate but with some redundant or irrelevant information. 4. Inaccurate (probable intrinsic error e.g from inaccurate/out-of-date training data).
- 5. Inaccurate (probable fabrication/hallucination).
- 6. Inaccurate (irrelevant). 7. Completely inaccurate.

Respond with only the classification number and its label.

#### **Data availability**

Data will be made available on request.

## **References**

- [1] M.E. Peters, M. Neumann, R.L. Logan IV, R. Schwartz, V. Joshi, S. Singh, N.A. Smith, Knowledge enhanced contextual word representations, 2019, arXiv preprint [arXiv:1909.04164.](http://arxiv.org/abs/1909.04164)
- [2] X. Guan, Y. Liu, H. Lin, Y. Lu, B. He, X. Han, L. Sun, Mitigating large language model [hallucinations](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb2) via autonomous knowledge graph-based retrofitting, in: Proceedings of the AAAI Conference on Artificial Intelligence, vol. 38, (16) 2024, pp. [18126–18134.](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb2)
- [3] S. Zheng, H. Bai, Y. Zhang, Y. Su, X. Niu, N. Jaitly, KGLens: Towards efficient and effective knowledge probing of large language models with knowledge graphs, 2024, arXiv preprint [arXiv:2312.11539](http://arxiv.org/abs/2312.11539).

- [4] H. Sansford, N. Richardson, H.P. Maretic, J.N. Saada, Grapheval: A knowledge-graph based llm hallucination evaluation framework, 2024, arXiv preprint [arXiv:2407.10793.](http://arxiv.org/abs/2407.10793)
- [5] D. Vrandečić, M. Krötzsch, Wikidata: A free collaborative [knowledgebase,](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb5) Commun. ACM 57 (10) (2014) 78–85.
- [6] S. Auer, C. Bizer, G. Kobilarov, J. Lehmann, R. Cyganiak, Z. Ives, Dbpedia: A nucleus for a web of open data, in: [International](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb6) Semantic Web Conference, 2007, pp. [722–735.](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb6)
- [7] U. Ali, S. Lynden, A. Matono, T. Amagasa, [Entropy-guided](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb7) probing for predicting LLM hallucinations with knowledge graph features, in: International Conference on Database and Expert Systems [Applications,](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb7) 2025, pp. 68–82.
- [8] S. Farquhar, J. Kossen, L. Kuhn, Y. Gal, Detecting [hallucinations](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb8) in large language models using semantic entropy, Nature 630 (8017) (2024) 625–630.
- [9] L. Huang, W. Yu, W. Ma, W. Zhong, Z. Feng, H. Wang, Q. Chen, W. Peng, X. Feng, B. Qin, et al., A survey on [hallucination](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb9) in large language models: Principles, taxonomy, [challenges,](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb9) and open questions, ACM Trans. Inf. Syst. 43 (2) (2025) 1–55.
- [10] S. Lin, J. Hilton, O. Evans, Truthfulqa: Measuring how models mimic human falsehoods, in: [Proceedings](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb10) of the 60th Annual Meeting of the Association for [Computational](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb10) Linguistics (Volume 1: Long Papers), 2022, pp. 3214–3252.
- [11] S. Min, K. Krishna, X. Lyu, M. Lewis, W.t. Yih, P. Koh, M. Iyyer, L. [Zettlemoyer,](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb11) H. Hajishirzi, Factscore: Fine-grained atomic evaluation of factual precision in long form text generation, in: Proceedings of the 2023 Conference on Empirical Methods in Natural Language Processing, 2023, pp. [12076–12100.](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb11)
- [12] T. Ceritli, S. Ozkan, J. Min, E. Noh, C.J. Min, M. Ozay, A study of parameter efficient [fine-tuning](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb12) by learning to efficiently fine-tune, in: Findings of the Association for [Computational](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb12) Linguistics: EMNLP 2024, 2024, pp. 15819–15836.
- [13] P. Manakul, A. Liusie, M.J. Gales, Selfcheckgpt: Zero-resource black-box hallucination detection for generative large language models, 2023, arXiv preprint [arXiv:2303.08896.](http://arxiv.org/abs/2303.08896)
- [14] Y. Yehuda, I. Malkiel, O. Barkan, J. Weill, R. Ronen, N. Koenigstein, [InterrogateLLM:](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb14) Zero-resource hallucination detection in LLM-generated answers, in: Proceedings of the 62nd Annual Meeting of the Association for [Computational](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb14) Linguistics (Volume 1: Long Papers), 2024, pp. 9333–9347.
- [15] J. Chen, H. Lin, X. Han, L. Sun, Benchmarking large language models in [retrieval-augmented](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb15) generation, in: Proceedings of the AAAI Conference on Artificial Intelligence, vol. 38, (16) 2024, pp. [17754–17762.](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb15)
- [16] C. Niu, Y. Wu, J. Zhu, S. Xu, K. Shum, R. Zhong, J. Song, T. Zhang, Ragtruth: A hallucination corpus for developing trustworthy retrieval-augmented language models, 2023, arXiv preprint [arXiv:2401.00396.](http://arxiv.org/abs/2401.00396)
- [17] P. Radhakrishnan, J. Chen, B. Xu, P. Ramaswami, H. Pho, A. Olmos, J. Manyika, R. Guha, Knowing when to ask–bridging large language models and data, 2024, arXiv preprint [arXiv:2409.13741.](http://arxiv.org/abs/2409.13741)
- [18] P. Chitale, J. Gala, R. Dabre, An empirical study of in-context learning in LLMs for machine translation, in: Findings of the Association for [Computational](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb18) Linguistics, ACL 2024, 2024, pp. [7384–7406.](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb18)
- [19] E. Lavrinovics, R. Biswas, J. Bjerva, K. Hose, Knowledge graphs, large language models, and [hallucinations:](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb19) An NLP perspective, J. Web Semant. 85 (2025) [100844.](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb19)
- [20] G. Agrawal, T. Kumarage, Z. Alghamdi, H. Liu, Can knowledge graphs reduce [hallucinations](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb20) in LLMs? : A survey, in: Proceedings of the 2024 Conference of the North American Chapter of the Association for [Computational](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb20) Linguistics: Human Language Technologies (Volume 1: Long Papers), 2024, pp. [3947–3960.](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb20)
- [21] B. Škrlj, B. Koloski, S. Pollak, N. Lavrač, From symbolic to neural and back: Exploring knowledge [graph–large](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb21) language model synergies, 2025, pp. 181–197, Challenges and Algorithms for [Knowledge](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb21) Discovery from Data: Essays Dedicated to Arno Siebes on the Occasion of His 67th Birthday.
- [22] R. Wagner, E. Kitzelmann, I. Boersch, Mitigating [hallucination](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb22) by integrating knowledge graphs into LLM inference–a systematic literature review, in: Proceedings of the 63rd Annual Meeting of the Association for [Computational](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb22) Linguistics (Volume 4: Student Research Workshop), 2025, pp. 795–805.
- [23] T. Zhou, Y. Chen, K. Liu, J. Zhao, Cogmg: Collaborative augmentation between large language model and knowledge graph, 2024, arXiv preprint [arXiv:2406.17231.](http://arxiv.org/abs/2406.17231)
- [24] M. Rashad, A. Zahran, A. Amin, A. Abdelaal, M. Altantawy, FactAlign: Fact-level [hallucination](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb24) detection and classification through knowledge graph alignment, in: Proceedings of the 4th Workshop on [Trustworthy](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb24) Natural Language Processing, TrustNLP 2024, 2024, pp. 79–84.
- [25] Y. Zhu, J. Xiao, Y. Wang, J. Sang, KG-FPQ: Evaluating factuality hallucination in llms with knowledge graph-based false premise questions, 2024, arXiv preprint [arXiv:2407.05868.](http://arxiv.org/abs/2407.05868)
- [26] V. Chekalina, A. Razzhigaev, E. Goncharova, A. Kuznetsov, Addressing hallucinations in language models with knowledge graph embeddings as an additional modality, 2024, arXiv preprint [arXiv:2411.11531.](http://arxiv.org/abs/2411.11531)
- [27] S. Tian, Y. Luo, T. Xu, C. Yuan, H. Jiang, C. Wei, X. Wang, [KG-adapter:](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb27) Enabling knowledge graph integration in large language models through [parameter-efficient](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb27) fine-tuning, in: Findings of the Association for Computational Linguistics, ACL 2024, 2024, pp. 3813–3828.
- [28] B. Jiang, Y. Wang, Y. Luo, D. He, P. Cheng, L. Gao, Reasoning on efficient [knowledge](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb28) paths: knowledge graph guides large language model for domain question answering, in: 2024 IEEE [International](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb28) Conference on Knowledge Graph, ICKG, 2024, pp. 142–149.
- [29] S. Pan, L. Luo, Y. Wang, C. Chen, J. Wang, X. Wu, Unifying large language models and [knowledge](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb29) graphs: A roadmap, IEEE Trans. Knowl. Data Eng. 36 (7) (2024) [3580–3599.](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb29)
- [30] Z. Zeng, Q. Cheng, X. Hu, Z. Liu, J. Shen, Y. Zhang, Aligning the [representation](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb30) of knowledge graph and large language model for causal question answering, in: 2024 IEEE [International](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb30) Conference on Big Data, BigData, 2024, pp. 1177–1186.
- [31] J. Xu, M.S. Lam, Fine-tuned WikiSP: A semantic parser for question answering over wikidata, in: Findings of the Association for [Computational](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb31) Linguistics, ACL [2023,](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb31) 2023.
- [32] P. Trivedi, G. Maheshwari, M. Dubey, J. Lehmann, Lc-quad: A corpus for complex question answering over knowledge graphs, in: [International](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb32) Semantic Web [Conference,](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb32) 2017, pp. 210–218.
- [33] L. Zheng, W.L. Chiang, Y. Sheng, S. Zhuang, Z. Wu, Y. Zhuang, Z. Lin, Z. Li, D. Li, E.P. Xing, H. Zhang, J.E. Gonzalez, I. Stoica, Judging LLM-asa-judge with MT-bench and chatbot arena, in: Advances in Neural Information Processing Systems (NeurIPS), vol. 36, 2023, pp. 46595–46623, URL <https://openreview.net/forum?id=uccHPGDlao>.

**Ushtar Ali** received his BSCS degree from PMAS-Arid Agriculture University, Rawalpindi, and his MSCS degree from the University of Central Punjab, Lahore, in 2020 and 2023, respectively. He is currently a Ph.D. student at the Center for Computational Sciences (CCS), University of Tsukuba. He is also a research assistant (RA) at the Intelligent Platforms Research Institute, National Institute of Advanced Industrial Science and Technology (AIST), Japan. His research interests include knowledge-graph–augmented natural language processing, large language model (LLM) grounding and hallucination detection, and graphbased methods for reasoning, alignment, and evaluation.

**Steven Lynden** is a Senior Researcher at the Intelligent Platforms Research Institute, National Institute of Advanced Industrial Science and Technology (AIST), Japan. He received his Ph.D. in Computer Science from Cardiff University in 2004. His research builds on a background in data management and Semantic Web technologies, and currently focuses on knowledge-graph–augmented natural language processing, large language model (LLM) grounding and hallucination detection, and graph-based methods for reasoning, alignment, and evaluation. His work aims to improve the robustness, interpretability, and trustworthiness of intelligent systems through the integration of structured knowledge with machine learning-based models.

**Akiyoshi Matono** received his B.E. and M.E. degrees from Okayama Prefectural University in 2000 and 2002, respectively, and his Ph.D. degree from the Nara Institute of Science and Technology in 2005. He is currently the Research Group Leader of the Data Platform Research Group at the Intelligent Platforms Research Institute, National Institute of Advanced Industrial Science and Technology (AIST), Japan. His research interests include database technologies and graph processing. He is a member of DBSJ, IEICE, and IPSJ.

**Toshiyuki Amagasa** received B.E., M.E., and Ph.D. degrees from the Department of Computer Science, Gunma University in 1994, 1996, and 1999, respectively. He is currently a full professor at the Center for Computational Sciences (CCS) and the Center for Artificial Intelligence Research (C-AIR), University of Tsukuba. His research interests cover data engineering, database systems, data mining, and database applications in scientific domains. He is a senior member of IPSJ, IEICE, and IEEE, a board member of DBSJ, and a member of ACM.