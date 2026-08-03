Contents lists available at [ScienceDirect](https://www.elsevier.com/locate/knosys)

# Knowledge-Based Systems

journal homepage: [www.elsevier.com/locate/knosys](https://www.elsevier.com/locate/knosys)

# From entity-centric to goal-oriented graphs: Enhancing LLM knowledge retrieval in minecraft

Jonathan Leung [a,](#page-0-0)[∗](#page-0-1) , Yongjie Wang [a](#page-0-0) , Zhiqi She[n](https://orcid.org/0000-0001-7626-7295) [b](#page-0-2)

- <sup>a</sup> *Alibaba-NTU e-Sustainability CorpLab (ANGEL), Nanyang Technological University, 50 Nanyang Avenue, 639798, Singapore*
- <span id="page-0-2"></span><span id="page-0-0"></span><sup>b</sup> *College of Computing and Data Science, Nanyang Technological University, 50 Nanyang Avenue, 639798, Singapore*

# a r t i c l e i n f o

### *Keywords:* Large language models Retrieval-augmented generation Knowledge graph AI agents Procedural reasoning

# a b s t r a c t

Large Language Models (LLMs) demonstrate impressive general capabilities but often struggle with step-by-step procedural reasoning, a critical challenge in complex interactive environments. While retrieval-augmented methods like GraphRAG attempt to bridge this gap, their fragmented entity-relation graphs hinder the construction of coherent, multi-step plans. In this paper, we propose a novel framework based on Goal-Oriented Graphs (GoGs), where each node represents a goal and edges encode logical dependencies between them. This structure enables the explicit retrieval of causal reasoning paths by identifying a high-level goal and recursively retrieving its prerequisites, forming a coherent chain to guide the LLM. Through extensive experiments on the Minecraft testbed, a domain that demands robust multi-step planning and provides rich procedural knowledge, we demonstrate that GoG substantially improves procedural reasoning and significantly outperforms GraphRAG and other state-of-the-art baselines.

# **1. Introduction**

Large Language Models (LLMs) have recently been applied as reasoning and planning components in interactive environments, where they enable dynamic decision-making for agents such as non-player characters (NPCs) and virtual assistants. Games, in particular, have become valuable testbeds for studying the reasoning capabilities of LLMs because they combine structured rules with open-ended objectives [\[1\]](#page--1-0). While early research has explored strategic domains like chess [2], the frontier has moved toward open-world settings that demand long-horizon, hierarchical goal decomposition. Environments like Minecraft [3,4] have emerged as critical benchmarks for this challenge due to their combinatorial action space and the need for multi-step procedural reasoning.

To ground LLMs in such complex domains, Retrieval-Augmented Generation (RAG) has become a standard approach [5]. State-of-theart methods like GraphRAG [6] structure external knowledge into entity-relation graphs to facilitate retrieval. However, this entity-centric paradigm is fundamentally ill-suited for procedural tasks. It fragments causal knowledge into an excessive number of low-granularity triples, making it difficult to reconstruct a coherent, step-by-step plan. This is not just a theoretical issue; in our experiments, this fragmentation introduces significant noise that hinders the agent's performance.

Reconstructing a coherent plan from this fragmented knowledge is akin to the saying, "tearing paper is easy, putting it back together

We focus on Minecraft as a deep and open-ended benchmark for multi-step reasoning rather than as a generic game environment. While the GoG framework is conceptually general, in this work we explicitly target domains where procedural knowledge can be externalized into goal–precondition structures, and we do not claim applicability to arbitrary game environments. This focused scope allows us to rigorously evaluate how goal-oriented knowledge retrieval improves LLM reasoning in a well-studied, reproducible setting.

Our contributions are summarized as follows:

- We introduce Goal-Oriented Graphs (GoGs), a novel framework designed to enhance the procedural, multi-step reasoning of LLMs. GoG is designed to leverage external procedural descriptions that can be organized into goals, preconditions, and postconditions, a common characteristic of instructional and technical domains. GoGs model how complex tasks decompose into actionable subgoals, shifting the paradigm from entity-centric relations to logical goal dependencies.
- We propose a goal-driven retrieval algorithm that traverses the GoG to construct coherent and explicit reasoning chains, overcoming the

*E-mail address:* [jonathan.leung@ntu.edu.sg](mailto:jonathan.leung@ntu.edu.sg) (J. Leung).

is hard", illustrated in Fig. 1. This motivates our work: to design a knowledge framework that explicitly captures procedural dependencies. We introduce the Goal-Oriented Graph (GoG), a novel structure where nodes represent goals and directed edges encode the prerequisite relationships between them. Using this graph, our retrieval process identifies a high-level goal and recursively retrieves its subgoals, forming a coherent reasoning chain to guide the LLM.

<span id="page-0-1"></span><sup>∗</sup> Corresponding author.

external procedural knowledge can be reasonably extracted. We hope this work contributes a meaningful step toward bridging structured planning and LLM-based reasoning, and lays the foundation for future agents capable of dynamically learning, adapting, and reasoning about goals in complex, open-ended environments.

# **Acknowledgements**

This research is supported by the RIE2025 Industry Alignment Fund – Industry Collaboration Projects (IAF-ICP) (Award I2301E0026), administered by A\*STAR, as well as supported by Alibaba Group and NTU Singapore through Alibaba-NTU Global e-Sustainability CorpLab (AN-GEL).

# **CRediT authorship contribution statement**

**Jonathan Leung:** Writing – review & editing, Writing – original draft, Visualization, Validation, Software, Methodology, Investigation, Formal analysis, Data curation; **Yongjie Wang:** Writing – review & editing, Writing – original draft, Visualization, Supervision, Resources, Conceptualization; **Zhiqi Shen:** Supervision, Resources, Project administration, Funding acquisition, Conceptualization.

### **Data availability**

Data will be made available on request.

### **Declaration of competing interest**

The authors declare that they have no known competing financial interests or personal relationships that could have appeared to influence the work reported in this paper.

### **Appendix A. Prompts**

In this section, we provide various prompts used in our experiments. Fig. A.4 contains the prompt for goal extraction used for the construction of GoG. Fig. A.5 contains the prompt used to determine which goal the agent should pursue. Fig. A.6 contains the prompt used to generate the plan. Fig. A.7 contains the prompt used to extract entities and relationships from the source text (Minecraft Wiki pages and recipe files), which is used to build the knowledge graph used by GraphRAG. Text in curly braces represent placeholders that should be replaced by contextual information at inference time.

**Fig. A.4.** The prompt used to extract goals and subgoals from source texts to build our knowledge base.

**Fig. A.5.** The prompt used for goal inference for our proposed method. "Context" consists of the top-*𝑘* retrieved goals from the knowledge base.

**Fig. A.6.** The prompt used for planning for our proposed method.

# Prompt for GraphRAG Entity Extraction

-Goal

Given a text document about the game "Minecraft" and a list of entity types, identify all game-related entities of those types from the text and all relationships among the identified entities. The document comes from the Minecraft Wiki, and may contain headers, tables, recipes, and text in other formats. Focus on entities related to items, tools, and crafting. You can ignore entities related to game patches and versions, entities outside of the Minecraft game, and abstract entities that are unrelated to gameplay.

-Steps-

- 1. Identify all game-related entities. For each identified entity, extract the following information:
- entity\_name: Name of the entity, capitalized
- entity\_type: One of the following types: [{entity\_types}]
- entity\_description: Comprehensive description of the entity's attributes and activities

Format each entity as

("entity" {tuple\_delimiter} < entity\_name > {tuple\_delimiter} < entity\_type > {tuple\_delimiter} < entity\_description > )

2. From the entities identified in step 1, identify all pairs of (source\_entity, target\_entity) that are \*clearly related\* to each other.

For each pair of related entities, extract the following information:

- source\_entity: name of the source entity, as identified in step 1
- target\_entity: name of the target entity, as identified in step 1
- relationship\_description: explanation as to why you think the source entity and the target entity are related to each other
- ${\text -}$  relationship\_strength: a numeric score indicating strength of the relationship between the source entity and target entity

Format each relationship as

("relationship" {tuple\_delimiter} < source\_entity > {tuple\_delimiter}

 $<\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!\!$ 

{tuple\_delimiter}<relationship\_strength>)

- 3. Return output in English as a single list of all the entities and relationships identified in steps 1 and 2. Use \*\*{record\_delimiter}\*\* as the list delimiter.
- 4. When finished, output {completion\_delimiter}

########################

-Examples-

###########################

{examples}

##########################

-Real Data-

#########################

Entity\_types: {entity\_types}

Text: {input\_text}

##########################

Output:

Fig. A.7. The prompt used for extracting entities and relationships for the construction of the knowledge graph used by GraphRAG. The entity types given to the LLM are: "item", "block", "equipment", "location", "event", "npc".

# Appendix B. Experimental tasks

 there are 66 tasks, categorized into seven groups: wood, stone, iron, gold, diamond, redstone, and armor. As expected, more complex tasks require a greater number of steps to complete.

**Table B.1** Tasks used in our main experiments.

| Task Group | #Tasks | Task description                                                                                                                                                                                                                                                                                                          | Max Steps |
|------------|--------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-----------|
| Wood       | 10     | craft a wooden shovel, craft a wooden pickaxe, craft a wooden axe, craft a wooden hoe, craft a stick, craft a crafting table, craft a wooden<br>sword, craft a chest, craft a bowl, craft a ladder                                                                                                                        | 2400      |
| Stone      | 9      | craft a stone shovel, craft a stone pickaxe, craft a stone axe, craft a stone hoe, smelt a charcoal, craft a smoker, craft a stone sword, craft a<br>furnace, craft a torch                                                                                                                                               |           |
| Iron       | 16     | craft a iron shovel, craft a iron pickaxe, craft a iron axe, craft a iron hoe, craft a bucket, craft a hopper, craft a rail, craft a iron sword, craft<br>a shears, craft a smithing table, craft a tripwire hook, craft a chain, craft an iron bars, craft an iron nugget, craft a blast furnace, craft a<br>stonecutter | 24,000    |
| Gold       | 6      | craft a golden shovel, craft a golden pickaxe, craft a golden axe, craft a golden hoe, craft a golden sword, smelt and craft a gold ingot                                                                                                                                                                                 | 36,000    |
| Diamond    | 6      | craft a diamond shovel, craft a diamond pickaxe, craft a diamond axe, craft a diamond hoe, craft a diamond sword, craft a jukebox                                                                                                                                                                                         | 36,000    |
| Redstone   | 6      | craft a piston, craft a redstone torch, craft an activator rail, craft a compass, craft a dropper, craft a note block                                                                                                                                                                                                     | 36,000    |
| Armor      | 13     | craft shield, craft iron chestplate, craft iron boots, craft iron leggings, craft iron helmet, craft diamond helmet, craft diamond chestplate,<br>craft diamond leggings, craft diamond boots, craft golden helmet, craft golden leggings, craft golden boots, craft golden chestplate                                    | 36,000    |

### **Appendix C. GoG construction**

In this section, we provide cost estimations for the construction and inference phases of our proposed GoG framework in Tables C.1 and C.2, respectively.

**Table C.1** GoG construction costs (incurred once).

| Step                              | LLM/Embedding Cost                                                                                         |
|-----------------------------------|------------------------------------------------------------------------------------------------------------|
| Chunking                          | N/A                                                                                                        |
| Extracting Goals and Subgoals     | One LLM call for each chunk (costing<br>about US\$2-3 in total for 2.7M text source<br>using GPT-4o-mini). |
| Goal Merge and Subgoal Derivation | Cosine similarity between the embeddings<br>of each goal or sub-goal property (low<br>cost).               |

**Table C.2** Online inference costs (incurred per task).

| Step           | LLM/Embedding Cost                                                                                                              |
|----------------|---------------------------------------------------------------------------------------------------------------------------------|
| Query          | Embed the task query.                                                                                                           |
| Retrieval      | Cosine similarity between the embedded query and<br>each goal in the graph.                                                     |
| Goal Inference | One LLM call at the start of the task, and one LLM call<br>for replanning if the agent does not make progress<br>after 𝑥 steps. |
| Planning       | One LLM call at the start of the task, and one LLM call<br>for replanning if the agent does not make progress<br>after 𝑥 steps. |

# **References**

- [1] R. Gallotta, G. Todd, M. Zammit, S. Earle, A. Liapis, J. Togelius, G.N. [Yannakakis,](http://refhub.elsevier.com/S0950-7051(26)00446-6/sbref0001) Large language models and games: a survey and roadmap, IEEE [Transactions](http://refhub.elsevier.com/S0950-7051(26)00446-6/sbref0001) on Games, 1–18 [\(2024\).](http://refhub.elsevier.com/S0950-7051(26)00446-6/sbref0001)
- [2] X. Feng, Y. Luo, Z. Wang, H. Tang, M. Yang, K. Shao, D. Mguni, Y. Du, J. Wang, ChessGPT: bridging policy learning and language modeling, in: A. Oh, T. Naumann, A. Globerson, K. Saenko, M. Hardt, S. Levine (Eds.), Advances in Neural Information Processing Systems, 36, Curran Associates, Inc., 2023, pp. 7216–7262. [https://proceedings.neurips.cc/paper\\_files/paper/2023/file/](https://proceedings.neurips.cc/paper_files/paper/2023/file/16b14e3f288f076e0ca73bdad6405f77-Paper-Datasets_and_Benchmarks.pdf) [16b14e3f288f076e0ca73bdad6405f77-Paper-Datasets\\_and\\_Benchmarks.pdf.](https://proceedings.neurips.cc/paper_files/paper/2023/file/16b14e3f288f076e0ca73bdad6405f77-Paper-Datasets_and_Benchmarks.pdf)
- [3] G. Wang, Y. Xie, Y. Jiang, A. Mandlekar, C. Xiao, Y. Zhu, L. Fan, A. Anandkumar, Voyager: an open-ended embodied agent with large language models, Transactions on Machine Learning Research (2024). [https://openreview.net/forum?id=](https://openreview.net/forum?id=ehfRiF0R3a) [ehfRiF0R3a](https://openreview.net/forum?id=ehfRiF0R3a).
- [4] X. Zhu, Y. Chen, H. Tian, C. Tao, W. Su, C. Yang, G. Huang, B. Li, L. Lu, X. Wang, Y. Qiao, Z. Zhang, J. Dai, Ghost in the Minecraft: Generally Capable Agents for Open-World Environments via Large Language Models with Text-based Knowledge and Memory, 2023, [arXiv:2305.17144.](http://arxiv.org/abs/2305.17144)

- [5] Y. Gao, Y. Xiong, X. Gao, K. Jia, J. Pan, Y. Bi, Y. Dai, J. Sun, M. Wang, H. Wang, Retrieval-Augmented Generation for Large Language Models: A Survey, 2023, [arXiv:2312.10997.](http://arxiv.org/abs/2312.10997)
- [6] D. Edge, H. Trinh, N. Cheng, J. Bradley, A. Chao, A. Mody, S. Truitt, D. Metropolitansky, R.O. Ness, J. Larson, From Local to Global: A Graph RAG Approach to Query-Focused Summarization, 2025, [arXiv:2404.16130.](http://arxiv.org/abs/2404.16130)
- [7] Y. Qin, E. Zhou, Q. Liu, Z. Yin, L. Sheng, R. Zhang, Y. Qiao, J. Shao, MP5: a multimodal open-ended embodied system in minecraft via active perception, in: 2024 IEEE/CVF Conference on Computer Vision and Pattern Recognition (CVPR), 2024, pp. 16307–16316. <https://doi.org/10.1109/CVPR52733.2024.01543>
- [8] Z. Li, Y. Xie, R. Shao, G. Chen, D. Jiang, L. Nie, Optimus-1: hybrid multimodal memory empowered agents excel in long-horizon tasks, in: The Thirty-eighth Annual Conference on Neural Information Processing Systems, 2024. [https://openreview.](https://openreview.net/forum?id=XXOMCwZ6by) [net/forum?id=XXOMCwZ6by](https://openreview.net/forum?id=XXOMCwZ6by).
- [9] J. Wei, X. Wang, D. Schuurmans, M. Bosma, B. Ichter, F. Xia, E. Chi, Q.V. Le, D. Zhou, Chain-of-thought prompting elicits reasoning in large language models, in: S. Koyejo, S. Mohamed, A. Agarwal, D. Belgrave, K. Cho, A. Oh (Eds.), Advances in Neural Information Processing Systems, 35, Curran Associates, Inc., 2022, pp. 24824–24837. [https://proceedings.neurips.cc/paper\\_files/paper/2022/file/](https://proceedings.neurips.cc/paper_files/paper/2022/file/9d5609613524ecf4f15af0f7b31abca4-Paper-Conference.pdf) [9d5609613524ecf4f15af0f7b31abca4-Paper-Conference.pdf](https://proceedings.neurips.cc/paper_files/paper/2022/file/9d5609613524ecf4f15af0f7b31abca4-Paper-Conference.pdf).
- [10] S. Yao, J. Zhao, D. Yu, N. Du, I. Shafran, K.R. Narasimhan, Y. Cao, ReAct: synergizing reasoning and acting in language models, in: The Eleventh International Conference on Learning Representations, 2023. [https://openreview.net/forum?id=WE\\_](https://openreview.net/forum?id=WE_vluYUL-X) [vluYUL-X.](https://openreview.net/forum?id=WE_vluYUL-X)
- [11] N. Shinn, F. Cassano, A. Gopinath, K. Narasimhan, S. Yao, Reflexion: language agents with verbal reinforcement learning, in: A. Oh, T. Naumann, A. Globerson, K. Saenko, M. Hardt, S. Levine (Eds.), Advances in Neural Information Processing Systems, 36, Curran Associates, Inc., 2023, pp. 8634–8652. [https://proceedings.neurips.cc/paper\\_files/paper/2023/file/](https://proceedings.neurips.cc/paper_files/paper/2023/file/1b44b878bb782e6954cd888628510e90-Paper-Conference.pdf) [1b44b878bb782e6954cd888628510e90-Paper-Conference.pdf.](https://proceedings.neurips.cc/paper_files/paper/2023/file/1b44b878bb782e6954cd888628510e90-Paper-Conference.pdf)
- [12] S. Yao, D. Yu, J. Zhao, I. Shafran, T. Griffiths, Y. Cao, K. Narasimhan, Tree of thoughts: deliberate problem solving with large language models, in: A. Oh, T. Naumann, A. Globerson, K. Saenko, M. Hardt, S. Levine (Eds.), Advances in Neural Information Processing Systems, 36, Curran Associates, Inc., 2023, pp. 11809–11822. [https://proceedings.neurips.cc/paper\\_files/paper/2023/](https://proceedings.neurips.cc/paper_files/paper/2023/file/271db9922b8d1f4dd7aaef84ed5ac703-Paper-Conference.pdf) [file/271db9922b8d1f4dd7aaef84ed5ac703-Paper-Conference.pdf](https://proceedings.neurips.cc/paper_files/paper/2023/file/271db9922b8d1f4dd7aaef84ed5ac703-Paper-Conference.pdf).
- [13] X. Wang, J. Wei, D. Schuurmans, Q.V. Le, E.H. Chi, S. Narang, A. Chowdhery, D. Zhou, Self-consistency improves chain of thought reasoning in language models, in: The Eleventh International Conference on Learning Representations, 2023. [https:](https://openreview.net/forum?id=1PL1NIMMrw) [//openreview.net/forum?id=1PL1NIMMrw](https://openreview.net/forum?id=1PL1NIMMrw).
- [14] P. Lewis, E. Perez, A. Piktus, F. Petroni, V. Karpukhin, N. Goyal, H. Küttler, M. Lewis, W.-t. Yih, T. Rocktäschel, S. Riedel, D. Kiela, Retrieval-augmented generation for knowledge-intensive NLP tasks, in: H. Larochelle, M. Ranzato, R. Hadsell, M.F. Balcan, H. Lin (Eds.), Advances in Neural Information Processing Systems, 33, Curran Associates, Inc., 2020, pp. 9459–9474. [https://proceedings.neurips.cc/paper\\_files/](https://proceedings.neurips.cc/paper_files/paper/2020/file/6b493230205f780e1bc26945df7481e5-Paper.pdf) [paper/2020/file/6b493230205f780e1bc26945df7481e5-Paper.pdf](https://proceedings.neurips.cc/paper_files/paper/2020/file/6b493230205f780e1bc26945df7481e5-Paper.pdf).
- [15] J. Wu, J. Zhu, Y. Qi, J. Chen, M. Xu, F. Menolascina, V. Grau, Medical graph rag: towards safe medical large language model via graph retrieval-augmented generation, 2024, [arXiv:2408.04187.](http://arxiv.org/abs/2408.04187)
- [16] Z. Guo, L. Xia, Y. Yu, T. Ao, C. Huang, Lightrag: simple and fast retrieval-augmented generation, 2024, [arXiv:2410.05779.](http://arxiv.org/abs/2410.05779)
- [17] M. Ghallab, D. Nau, P. Traverso, [Automated](http://refhub.elsevier.com/S0950-7051(26)00446-6/sbref0017) Planning: Theory and Practice, Elsevier, [2004.](http://refhub.elsevier.com/S0950-7051(26)00446-6/sbref0017)
- [18] K. Erol, J. Hendler, D.S. Nau, HTN planning: complexity and [expressivity,](http://refhub.elsevier.com/S0950-7051(26)00446-6/sbref0018) in: AAAI, 94, 1994, pp. [1123–1128.](http://refhub.elsevier.com/S0950-7051(26)00446-6/sbref0018)
- [19] T.G. Dietterich, et al., The MAXQ method for hierarchical [reinforcement](http://refhub.elsevier.com/S0950-7051(26)00446-6/sbref0019) learning, in: ICML, 98, 1998, pp. [118–126.](http://refhub.elsevier.com/S0950-7051(26)00446-6/sbref0019)
- [20] R.S. Sutton, D. Precup, S. Singh, Between MDPs and [semi-MDPs:](http://refhub.elsevier.com/S0950-7051(26)00446-6/sbref0020) a framework for temporal abstraction in [reinforcement](http://refhub.elsevier.com/S0950-7051(26)00446-6/sbref0020) learning, Artif. Intell. 112 (1-2) (1999) [181–211.](http://refhub.elsevier.com/S0950-7051(26)00446-6/sbref0020)

- [21] S. Lifshitz, K. Paster, H. Chan, J. Ba, S. McIlraith, STEVE-1: a generative model for text-to-behavior in minecraft, in: A. Oh, T. Naumann, A. Globerson, K. Saenko, M. Hardt, S. Levine (Eds.), Advances in Neural Information Processing Systems, 36, Curran Associates, Inc., 2023, pp. 69900–69929. [https://proceedings.](https://proceedings.neurips.cc/paper_files/paper/2023/file/dd03f856fc7f2efeec8b1c796284561d-Paper-Conference.pdf) [neurips.cc/paper\\_files/paper/2023/file/dd03f856fc7f2efeec8b1c796284561d-](https://proceedings.neurips.cc/paper_files/paper/2023/file/dd03f856fc7f2efeec8b1c796284561d-Paper-Conference.pdf)[Paper-Conference.pdf](https://proceedings.neurips.cc/paper_files/paper/2023/file/dd03f856fc7f2efeec8b1c796284561d-Paper-Conference.pdf).
- [22] M. Ahn, A. Brohan, N. Brown, Y. Chebotar, O. Cortes, B. David, C. Finn, C. Fu, K. Gopalakrishnan, K. Hausman, et al., Do as i can, not as i say: grounding language in robotic affordances, 2022, [arXiv:2204.01691.](http://arxiv.org/abs/2204.01691)
- [23] L. Fan, G. Wang, Y. Jiang, A. Mandlekar, Y. Yang, H. Zhu, A. Tang, D.-A. Huang, Y. Zhu, A. Anandkumar, MineDojo: building open-ended embodied agents with internet-scale knowledge, in: S. Koyejo, S. Mohamed, A. Agarwal, D. Belgrave, K. Cho, A. Oh (Eds.), Advances in Neural Information Processing Systems, 35, Curran Associates, Inc., 2022, pp. 18343–18362. [https://proceedings.neurips.cc/paper\\_](https://proceedings.neurips.cc/paper_files/paper/2022/file/74a67268c5cc5910f64938cac4526a90-Paper-Datasets_and_Benchmarks.pdf) [files/paper/2022/file/74a67268c5cc5910f64938cac4526a90-Paper-Datasets\\_](https://proceedings.neurips.cc/paper_files/paper/2022/file/74a67268c5cc5910f64938cac4526a90-Paper-Datasets_and_Benchmarks.pdf) [and\\_Benchmarks.pdf](https://proceedings.neurips.cc/paper_files/paper/2022/file/74a67268c5cc5910f64938cac4526a90-Paper-Datasets_and_Benchmarks.pdf).
- [24] Z. Nussbaum, J.X. Morris, B. Duderstadt, A. Mulyar, Nomic embed: Training a reproducible long context text embedder, 2024, [arXiv:2402.01613.](http://arxiv.org/abs/2402.01613)