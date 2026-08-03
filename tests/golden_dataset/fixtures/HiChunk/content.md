# **HiChunk: Evaluating and Enhancing Retrieval-Augmented Generation with Hierarchical Chunking**

Wensheng Lu \* 1 Keyu Chen \* 1 Ruizhi Qiao <sup>1</sup> Xing Sun <sup>1</sup>

<sup>1</sup>Tencent Youtu Lab

Retrieval-Augmented Generation (RAG) enhances the response capabilities of language models by integrating external knowledge sources. However, document chunking as an important part of RAG system often lacks effective evaluation tools. This paper first analyzes why existing RAG evaluation benchmarks are inadequate for assessing document chunking quality, specifically due to evidence sparsity. Based on this conclusion, we propose HiCBench, which includes manually annotated multi-level document chunking points, synthesized evidence-dense question answer(QA) pairs, and their corresponding evidence sources. Additionally, we introduce the HiChunk framework, a multilevel document structuring framework based on fine-tuned LLMs, combined with the Auto-Merge retrieval algorithm to improve retrieval quality. Experiments demonstrate that HiCBench effectively evaluates the impact of different chunking methods across the entire RAG pipeline. Moreover, HiChunk achieves better chunking quality within reasonable time consumption, thereby enhancing the overall performance of RAG systems.

- **Date:** Sep 15, 2025
- **Correspondence:** Ruizhi.Qiao@tencent.com
- **Code:** <https://github.com/TencentYoutuResearch/HiChunk.git> **Data:** <https://huggingface.co/datasets/Youtu-RAG/HiCBench>

## **1 Introduction**

RAG (Retrieval-Augmented Generation) enhances the quality of LLM responses to questions beyond their training corpus by flexibly integrating external knowledge through the retrieval of relevant content chunks as prompts[\[Lewis et al.,](#page-10-0) [2020\]](#page-10-0). This approach helps reduce hallucinations[\[Chen et al.,](#page-10-1) [2024,](#page-10-1) [Zhang et al.,](#page-10-2) [2025\]](#page-10-2), especially when dealing with real-time information[\[He et al.,](#page-10-3) [2022\]](#page-10-3) and specialized domain knowledge[\[Wang](#page-10-4) [et al.,](#page-10-4) [2023,](#page-10-4) [Li et al.,](#page-11-0) [2023\]](#page-11-0). Document chunking, a crucial component of RAG systems, significantly impacts the quality of retrieved knowledge and, consequently, the quality of responses. Poor chunking methods may separate continuous fragments, leading to information loss, or combine unrelated information, making it more challenging to retrieve relevant content. For instance, as noted in [Bhat et al.](#page-11-1) [\[2025\]](#page-11-1), the optimal chunk size varies significantly across different datasets.

Although numerous benchmarks exist for evaluating RAG systems[\[Bai et al.,](#page-11-2) [2024,](#page-11-2) [Dasigi et al.,](#page-11-3) [2021,](#page-11-3) [Duarte](#page-11-4) [et al.,](#page-11-4) [2024,](#page-11-4) [Zhang et al.,](#page-11-5) [2024,](#page-11-5) [Yang et al.,](#page-11-6) [2018,](#page-11-6) [Koˇcisky et al.](#page-11-7) ` , [2018,](#page-11-7) [Pang et al.,](#page-11-8) [2021\]](#page-11-8), they mostly focus on assessing either the retriever's capability or the reasoning ability of the response model, without effectively evaluating chunking methods. We analyzed several datasets to determine the average word and sentence count of evidence. As shown in [Table 1,](#page-1-0) existing benchmarks generally suffer from evidence sparsity, where only a few sentences in the document are relevant to the query. As illustrated in [Figure 1,](#page-1-1) this sparsity of

<sup>∗</sup> Equal Contribution

evidence makes these datasets inadequate for evaluating the performance of chunking methods. In reality, user tasks might be evidence-dense, such as enumeration or summarization tasks, requiring chunking methods to accurately and completely segment semantically continuous fragments. Therefore, it is essential to effectively evaluate chunking methods.

To address this, we introduce **Hi**erarchical **C**hunking Benchmark(**HiCBench**), a benchmark for document QA designed to effectively evaluate the impact of chunking methods on different components of RAG systems, including the performance of document chunking, retrievers, and response models. HiCBench's original documents are sourced from OHRBench. We curated documents of appropriate length for the corpus and manually annotated chunking points at various hierarchical levels for evaluation purposes. These points are used to assess the chunker's performance and construct QA pairs, followed by using LLMs and the annotated document structure to create evidence-dense QA, and finally extracting relevant evidence sentences and filtering non-compliant samples using LLMs.

Additionally, existing document chunking methods only consider linear document structure[\[Duarte et al.,](#page-11-4) [2024,](#page-11-4) [Xiao et al.,](#page-11-9) [2024,](#page-11-9) [Zhao et al.,](#page-11-10) [2025,](#page-11-10) [Wang et al.,](#page-12-0) [2025\]](#page-12-0), while user problems may involve fragments with different semantic granularity, and linear document structure makes it difficult to adaptively adjust during retrieval. Therefore, we propose the **Hi**erarchical **C**hunking framework(**HiChunk**), which employs fine-tuned LLMs for hierarchical document structuring and incorporates iterative reasoning to address the challenge of adapting to extremely long documents. For hierarchically structured documents, we introduce the Auto-Merge retrieval algorithm, which adaptively adjusts the granularity of retrieval chunks based on the query, thereby maximizing retrieval quality.

In this work, our main contributions are as follows:

- We introduce HiCBench, a benchmark designed to assess the performance of chunker and the impact of chunking methods on retrievers and response models within RAG systems. HiCBench includes information on chunking points at different hierarchical levels of documents, as well as sources of evidence and factual answers related to evidence-dense QA, enabling better evaluation of chunking methods.
- We propose the HiChunk framework, a document hierarchical structuring framework that allows RAG systems to dynamically adjust the semantic granularity of retrieval chunks.
- We conduct comprehensive performance evaluations on several open-source datasets and HiCBench, analyzing the impact of different chunking methods across three dimensions: performance of chunker, retriever, and responser.

<span id="page-1-0"></span>**Table 1.** Statistics of document QA benchmark.

| Dataset | Qasper | OHRBench | GutenQA |
|---------|--------|----------|---------|
| Numdoc  | 416    | 1261     | 100     |
| Sentd   | 164    | 176      | 5,373   |
| Wordd   | 4.2k   | 5.4k     | 146.5k  |
| Numqa   | 1,372  | 8,498    | 3,000   |
| Wordq   | 8.9    | 20.6     | 16.0    |
| Worda   | 16.0   | 5.6      | 26.0    |
| Worde   | 239.4  | 36.5     | 39.3    |
| Sente   | 10.5   | 1.7      | 1.7     |

<span id="page-1-1"></span>**Figure 1.** Different chunk methods produce same answer.

# **2 Related Works**

**Traditional Text Chunking.** Text chunking divides continuous text into meaningful units like sentences, phrases, and words, with our focus on sentence-level chunking. Recent works have explored various approaches: [\[Cho et al.,](#page-12-1) [2022\]](#page-12-1) combines text chunking with extractive summarization using hierarchical representations and determinantal point processes (DPPs) to minimize redundancy, [\[Liu et al.,](#page-12-2) [2021\]](#page-12-2) presents a pipeline integrating topical chunking with hierarchical summarization, and [\[Zhang et al.,](#page-12-3) [2021\]](#page-12-3) develops an adaptive sliding-window model for ASR transcripts using phonetic embeddings. However, these LSTM and BERT[\[Devlin et al.,](#page-12-4) [2019\]](#page-12-4) based methods face limitations from small context windows and single-level chunking capabilities.

**RAG-oriented Document Chunking.** Recent research has explored content-aware document chunking strategies for RAG systems. LumberChunker[\[Duarte et al.,](#page-11-4) [2024\]](#page-11-4) uses LLMs to identify semantic shifts,but may miss hierarchical relationships. PIC[\[Wang et al.,](#page-12-0) [2025\]](#page-12-0) proposes pseudo-instruction for document chunking, guide chunking via document summaries, though its single-level approach may oversimplify document structure. AutoChunker[\[Jain et al.,](#page-12-5) [2025\]](#page-12-5) employs tree-based representations but primarily focuses on noise reduction rather than multi-level granularity. Late Chunking[\[Günther et al.,](#page-12-6) [2024\]](#page-12-6) embeds entire documents before chunking to preserve global context, but produces flat chunk lists without modeling hierarchical relationships. In contrast, our HierarchyChunk method creates multi-level document representations, chunking from coarse sections to fine-grained paragraphs. This enables RAG systems to retrieve information at appropriate abstraction levels, effectively bridging fragmented knowledge gaps and providing comprehensive document understanding.

**Limitations of Existing Text Chunking Benchmarks.** The evaluation of text chunking and RAG methods heavily relies on benchmark datasets. Wiki-727[\[Koshorek et al.,](#page-12-7) [2018\]](#page-12-7),VT-SSum[\[Lv et al.,](#page-12-8) [2021\]](#page-12-8) and News-Net[\[Wu et al.,](#page-12-9) [2023\]](#page-12-9) are typically chunked into flat sequences of paragraphs or sentences, without capturing the multi-level organization (e.g., sections, subsections, paragraphs) inherent in many real-world documents. This single-level representation limits the ability to evaluate chunking methods that aim to preserve or leverage document hierarchy, which is crucial for comprehensive knowledge retrieval in complex RAG scenarios. While Qasper[\[Dasigi et al.,](#page-11-3) [2021\]](#page-11-3), HotpotQA[\[Yang et al.,](#page-12-10) [2018\]](#page-12-10) and GutenQA[\[Duarte et al.,](#page-11-4) [2024\]](#page-11-4) are designed for RAG-related tasks, they do not specifically provide mechanisms or metrics for evaluating the efficacy of document chunking strategies themselves. Their focus is primarily on end-to-end RAG performance, where the impact of chunking is implicitly measured through retrieval and generation quality. This makes it challenging to isolate and assess the performance of different chunking methods independently, hindering systematic advancements in hierarchical document chunking. Our work addresses these gaps by proposing a method that explicitly considers multi-level document chunking and constructs a novel benchmark from a chunking perspective.

## **3 HiCBench Construction**

In order to construct the HiCBench dataset, we performed additional document hierarchical structuring and created QA pairs to evaluate document chunking quality, building on the OHRBench document corpus[\[Zhang et al.,](#page-11-5) [2024\]](#page-11-5). We filter documents with fewer than 4,000 words and those exceeding 50 pages. For retained documents, we manually annotated the hierarchical structure and used these annotations to assist in the generation of QA pairs and to assess the accuracy of document chunking.

**Task Criteria** To ensure that the constructed QA pairs could effectively evaluate the quality of document chunking, we aimed for the evidence associated with each QA pair to be widely distributed across a complete

semantic chunk. Failure to fully recall such a semantic chunk would result in missing evidence, thereby degrading the quality of the generated responses. To achieve this objective, we established the following standards to regulate the generation of QA pairs:

- **Evidence Completeness and Density**: Evidence completeness ensures that the evidence relevant to the question is comprehensive and necessary within the context. Evidence density requires that evidence constitutes a significant proportion of the context, enhancing the QA pair's utility for evaluating chunking methods.
- **Fact Consistency**: To ensure the constructed samples can evaluate the entire retrieval-based pipeline, it is essential that the generated responses remain consistent with the answers when provided with full context, and that the questions are answerable.

**Task Definition** Additionally, we define three different task types to evaluate the quality of chunking:

- **Evidence-Sparse QA (***T*0**)**: The evidence related to the QA is confined to one or two sentences within the document.
- **Single-Chunk Evidence-Dense QA (***T*1**)**: Evidence sentences related to the QA constitute a substantial portion of the context within a single complete semantic chunk. The size of chunk between 512 and 4096 words.
- **Multi-Chunk Evidence-Dense QA (***T*2**)**: Evidence sentences related to the QA are distributed across multiple complete semantic chunks, covering a significant portion of the context. The size of chunk between 256 and 2048 words.

**QA Construction** We use a prompt-based approach using DeepSeek-R1-0528 to generate candidate QA pairs, followed by a series of filtering processes to ensure the retained QA pairs meet the criteria of evidence completeness, density, and fact consistency. The specific process is as follows:

- 1. **Document Hierarchical Annotation and Summarization**: To enable LLMs to gain an overall understanding of the specific document *D* while constructing QA pairs, we first generated summaries for corresponding sections based on the annotated hierarchical structure, denoted as *S* ← *LLMs*(*D*). These summaries will be used in QA pair generation.
- 2. **Generation of Questions and Answers**: We randomly selected one or two chunks from all eligible document fragments as context *C*, then generated candidate QA pairs using (*S*, *C*), where (*Q*, *A*) ← *LLMqa*(*S*, *C*).
- 3. **Ensuring Evidence Completeness and Density**: Referring to [Friel et al.](#page-12-11) [\[2024\]](#page-12-11), we use LLMs to extracted sentences from context *C* related to the QA pair as evidence, denoted as *E* ← *LLMee*(*C*, *Q*, *A*). To mitigate hallucination effects, this step will be repeated five times, retaining sentences that appeared at least four times as the final evidence. Furthermore, to ensure evidence density, we remove samples which the ratio of evidence is less than 10% of context *C*.
- 4. **Ensuring Fact Consistency**: We applied Fact-Cov metric[\[Xiang et al.,](#page-12-12) [2025\]](#page-12-12) to filter test samples. We first extract the facts from answer *A*, denoted as *F* ← *LLMf e*(*Q*, *A*) [1](#page-3-0) . Contexts *C* used for constructing QA pairs will be provided to LLMs to generate response *R* ′ , denoted as *R* ′ ← *LLMr*(*Q*, *C*). Then, the Fact-Cov metric will be calculated by Fact\_Cov ← *LLMf c*(*F*, *R* ′ ) [1](#page-3-0) . This process will be repeated 5 times. We retain samples with an average Fact-Cov metric exceeding 80%. Samples below this threshold are deemed unanswerable. All prompts used for QA construction are provided in [subsection A.3.](#page-15-0)

<span id="page-3-0"></span><sup>1</sup><https://github.com/GraphRAG-Bench/GraphRAG-Benchmark>

<span id="page-4-0"></span>Figure 2. Framework. (a) Iterative inference for HiChunk on long documents. (b) Auto-Merge retrieval algorithm.

# 4 Methodology

This section primarily introduces the HiChunk framework. The overall framework is illustrated in Figure 2. The aim is for the fine-tuned LLMs to comprehend the hierarchical relationships within a document and ultimately organize the document into a hierarchical structure. This involves two subtasks: identification of chunking points and determination of hierarchy levels. Through prompt, HiChunk converts these two subtasks into text generation task. In model train of HiChunk, we use Gov-report[Huang et al., 2021], Qasper[Dasigi et al., 2021] and Wiki-727[Koshorek et al., 2018] to construct training instructions, which are publicly available datasets with explicit document structure. Meanwhile, we augment the training set by randomly shuffling document chapters and deleting document content. During inference, HiChunk begins by processing document D with sentence chunking, assigning each sentence an independent ID, resulting in S[1:N], where N is the number of sentences. Through HiChunk, the document's global chunk points  $GCP_{1:k}$  are obtained, denoted as  $GCP_{1:k} \leftarrow$  HiChunk(S[1:N]), with k being the maximum number of predicted chunk point levels, and  $GCP_i$  representing all chunk points at level k. Additionally, an iterative inference optimization strategy is proposed to handle the structuring of extremely long documents.

Although the HiChunk hierarchical tree structure has semantic integrity, the variability in the chunk length distribution caused by the semantic chunking method can lead to disparities in semantic granularity, which can affect retrieval quality. To mitigate this, we apply a fixed-size chunking approach on the results of HiChunk to produce C[1:M], and propose the Auto-Merge retrieval algorithm to balance issues of varying semantic granularity and the semantic integrity of retrieved chunks.

**Iterative Inference** For documents that exceed the predefined inference input length L, an iterative inference approach is required. The process begins by initializing the start sentence id of the current iteration as a=0 and determining the end sentence id b to ensure the constructed input length is less than the maximum predefined length L. This is optimally set as  $b=\arg\max_{\hat{b}}(S[a:\hat{b}]<L)$ . Through this setup, local chunk points  $LCP_{1:k}\leftarrow HiChunk(S[a:b])$  are obtained via inference. These local chunk points  $LCP_{1:k}$  are then merged into the global chunk points  $GCP_{1:k}$ . Based on these local inference results, the new value of a is determined for the next iteration. This iterative process continues until a=N, signifying the completion of inference for the entire document. However, iterative inference suffers from hierarchical drift when there is only one level 1 chunk in the local inference result. To mitigate this problem, we construct residual text lines from known document structures to guide the model making correct hierarchical judgments. The complete iterative inference procedure is illustrated in algorithm 1.

**Auto-Merge Retrieval Algorithm** To balance the semantic richness and completeness of recalled contexts, we propose Auto-Merge retrieval algorithm. This algorithm uses a series of conditions to control the extent to which child nodes are merged upward into parent nodes. Auto-Merge algorithm traverses the query-ranked chunks  $C_{[1:M]}^{sorted}$ , using node<sub>ret</sub> to record the nodes that have been recalled. During the *i*-th step of the traversal, we first record the current used token budget,  $tk_{cur} = \sum_{n \in \text{node}_{\text{ret}}} \text{len}(n)$ . We then add  $C_{[i]}^{sorted}$  to node<sub>ret</sub> and denote the parent of  $C_{[i]}^{sorted}$  by p. Finally, we merge upward when the following conditions are met:

- $Cond_1$ : The intersection between the children of p and node<sub>ret</sub> contains at least two elements, denoted as  $\sum_{n \in \text{node}_{\text{rot}}} \mathbb{1}(n \in p.\text{children}) \ge 2$ .
- $Cond_2$ : The text length of the nodes in  $node_{ret}$  belonging to p's children is greater than or equal to  $\theta^*$ , denoted as  $\sum_{n \in (node_{ret} \cap p. children)} len(n) \ge \theta^*$ . Here,  $\theta^*$  is an adaptively adjusted threshold, defined as  $\theta^*(tk_{cur}, p) = \frac{len(p)}{3} \times (1 + \frac{tk_{cur}}{T})$ . As  $tk_{cur}$  increases,  $\theta^*$  gradually increases from  $\frac{len(p)}{3}$  to  $\frac{2 \times len(p)}{3}$ . This means that higher-ranking chunks are more likely to merge upward.
- *Cond*<sub>3</sub>: The remaining token budget is greater than the length of the chunk corresponding to p, denoted as  $T tk_{cur} \ge \text{len}(p)$ .

The entire retrieval algorithm process is illustrated in algorithm 2.

```
Algorithm 1: iterative inference
   input: Document D, Inference lengths L
   output:Global chunk points GCP<sub>1:k</sub>
 1 S[1:N] \leftarrow SentTokenize(D);
 a \leftarrow 1;
 з b \leftarrow \operatorname{argmax}_{\hat{b}}(S[a:\hat{b}] \leq L);
 4 res_lines ← None;
 5 GCP_{1:k} \leftarrow [] * k;
 6 while 1 \le a < b \le N do
        LCP_{1:k} \leftarrow HiChunk(S[a:b], res\_lines);
        GCP_{1:k} \leftarrow Merge(GCP_{1:k}, LCP_{1:k});
        if len(LCP_1) \geq 2 then
             // the last chunk point at
                  first level
10
             a \leftarrow LCP_1[-1];
            res\_lines \leftarrow None;
11
        else
12
13
            res\_lines \leftarrow GetResLines(GCP_{1:k});
        b \leftarrow \operatorname{argmax}_{\hat{b}}(S[a:\hat{b}] \leq L);
16 return GCP_{1:k}
```

```
Algorithm 2: retrieval algorithm
    input: Token budget T, Chunks C_{[1:M]}, Query q
    output:Retrieval context ctx
 _{1} \ C_{[1:M]}^{sorted} \leftarrow \texttt{Sorted}(C_{[1:M]},q);
 2 node<sub>ret</sub> ← [], tk_{cur} ← 0;
 3 for i ← 1 to M do
         node_{ret} \leftarrow node_{ret} + C_{[i]}^{sorted};
         \mathsf{ctx}, tk_{\mathit{cur}} \leftarrow \mathsf{BuildContext}(\mathsf{node}_{\mathsf{ret}});
         p \leftarrow C_{[i]}^{sorted}.parent;
         while Cond<sub>1</sub> and Cond<sub>2</sub> and Cond<sub>3</sub> do
              if tk_{cur} \geq T then
               break
              // add p to node<sub>ret</sub> and remove
                    nodes covered by p
              node_{ret} \leftarrow Merge(node_{ret}, p);
10
              \mathsf{ctx}, tk_{cur} \leftarrow \mathsf{BuildContext}(\mathsf{node}_{\mathsf{ret}});
11
12
              p \leftarrow p .parent;
         if tk_{cur} \geq T then
13
              break
15 return ctx [: T]
```

# 5 Experiments

#### 5.1 Datasets and Metrics

The test subsets of Gov-report[Huang et al., 2021] and Qasper[Dasigi et al., 2021] datasets will be used for evaluation of chunking accuracy. For the Gov-report dataset, we only retain documents with document word count greater than 5k for experiments. To evaluate the accuracy of the chunking points, we use the F1 metrics of the chunking points. The  $F1_{L_1}$  and  $F1_{L_2}$  correspond to the chunking points of the level 1 and level 2 chunks, respectively. And the  $F1_{L_{all}}$  metric does not consider the level of the chunking point. The Qasper, GutenQA[Duarte et al., 2024], and OHRBench[Zhang et al., 2024] datasets contain evidence relevant to the question. These datasets will be used in the evaluation for context retrieval.

For the full RAG pipeline evaluation, we used the publicly available datasets LongBench[Bai et al., 2024], Qasper, GutenQA, and OHRBench. the LongBench RAG evaluation contains 8 subsets from different datasets, with a total of 1,550 qa pairs, which can be categorized into single document qa and multiple document qa. The Qasper dataset contains 1,372 qa pairs from 416 documents. The GutenQA dataset contains 3,000 qa pairs based on 100 documents. In GutenQA, the average number of words in a document is 146,506, which is significantly higher than the other datasets. The documents of OHRBench come from seven different areas. We keep the documents with word counts greater than 4k in OHRBench and use the original qa pairs corresponding to these documents as a representative of the task  $T_0$ , denoted as OHRBench( $T_0$ ). We use the F1 score and Rouge metrics to assess the quality of LLM responses. All experiments are conducted in the code repository of LongBench<sup>1</sup>.

<span id="page-6-1"></span>Furthermore, HiCBench will be used for comprehensive evaluation, including chunking accuracy, evidence recall rate, and RAG response quality assessment. To avoid biases from sparse text quality evaluation metrics, we employ the Fact-Cov[Xiang et al., 2025] metric for response quality evaluation of HiCBench. The Fact-Cov metric is repeatedly calculated 5 times to take the average. Statistics information of datasets used in experiment are shown in Table 2.

| Dataset            | Qasper | GutenQA | <b>OHRBench</b> ( $T_0$ ) | $HiCBench(T_1, T_2)$ |
|--------------------|--------|---------|---------------------------|----------------------|
| Num <sub>doc</sub> | 416    | 100     | 214                       | 130                  |
| $Sent_d$           | 164    | 5,373   | 886                       | 298                  |
| $Word_d$           | 4.2k   | 146.5k  | 26.8k                     | 8.5k                 |
| Num <sub>qa</sub>  | 1,372  | 3,000   | 4,702                     | (659, 541)           |
| $Word_q$           | 8.9    | 16.0    | 22.2                      | (31.0, 33.0)         |
| $Word_a$           | 16.0   | 26.0    | 4.8                       | (130.1, 126.4)       |
| Word <sub>e</sub>  | 239.4  | 39.3    | 39.1                      | (561.5, 560.5)       |
| Sent <sub>e</sub>  | 10.5   | 1.7     | 1.7                       | (20.5, 20.4)         |

**Table 2.** Statistics of dataset used in experiments.

### 5.2 Comparison Methods

We primarily compared tow types of chunking methods: rule-based chunking methods and semantic-based chunking methods. All the comparison methods are as follows:

- FC200: Fixed chunking is a rule-based method, which first divide the document into sentences and then merge sentences based on a fixed chunking size. Here, the fixed chunking size is 200.
- SC: Semantic chunker[Xiao et al., 2024] uses an embedding model to calculate the similarity between adjacent paragraphs for chunking. We use bge-large-en-v1.5[Xiao et al., 2024] as the embedding model.

<span id="page-6-0"></span><sup>1</sup>https://github.com/THUDM/LongBench/tree/main

- **LC**: LumberChunker[\[Duarte et al.,](#page-11-4) [2024\]](#page-11-4) employs LLMs to predict the positions for chunking. In our experiments, we use Deepseek-r1-0528[\[DeepSeek-AI,](#page-13-0) [2025\]](#page-13-0) as the prediction model. The sampling temperature set to 0.1.
- **HC200**: Hierarchical chunker is the proposed method. In the model training for HiChunk. We further chunk the chunks of HiChunk by the fixed chunking method. The fixed chunking size is set to 200, denoted as HC200.
- **HC200+AM**: "+AM" represents the result of introducing Auto-Merge retrieval algorithm on the basis of HC200.

### **5.3 Experimental Settings**

In the model training of HiChunk, Gov-report[\[Huang et al.,](#page-12-13) [2021\]](#page-12-13), Qasper[\[Dasigi et al.,](#page-11-3) [2021\]](#page-11-3) and Wiki-727[\[Koshorek et al.,](#page-12-7) [2018\]](#page-12-7) are the train datasets, which are publicly available datasets with explicit document structure. We use Qwen3-4B[\[Team,](#page-13-1) [2025\]](#page-13-1) as the base model, with a learning rate of 1e-5 and a batch size of 64. The maximum length of training and inference is set to 8192 and 16384 tokens, respectively. Meanwhile, the length of each sentence is limited to within 100 characters. Due to the varying sizes of chunks resulting from semantic-based chunking, we limit the length of the retrieved context based on the number of tokens rather than the number of chunks for a fair comparison. The maximum length of the retrieved context is set to 4096 tokens. We also compare the performance of different chunking methods under different retrieved context length settings in [subsection 5.6.](#page-8-0) In the RAG evaluation process, we consistently use Bge-m3[\[Chen](#page-13-2) [et al.,](#page-13-2) [2024\]](#page-13-2) as the embedding model for context retrieval. As for the response model, we use three different series of LLMs with varying scales: Llama3.1-8b[\[Dubey et al.,](#page-13-3) [2024\]](#page-13-3), Qwen3-8b, and Qwen3-32b[\[Team,](#page-13-1) [2025\]](#page-13-1).

### **5.4 Chunking Accuracy**

To comprehensively evaluate the performance of the semantic-based chunking method, we conducted experiments using two publicly available datasets, along with the proposed benchmark, to assess the cutpoint accuracy of the chunking method. Since the SC and LC chunking methods are limited to performing single-level chunking, we evaluated only the F1 scores for the initial level of chunking points and the F1 scores without regard for the hierarchy of chunking points. The evaluation results are presented in [Table 3.](#page-7-0) In the Qasper and Gov-report datasets, which serve as in-domain test sets, the HC method shows a significant improvement in chunk accuracy compared to the SC and LC methods. Additionally, in HiCBench, an out-of-domain test set, the HC method exhibits even more substantial accuracy improvements. These findings demonstrate that HC enhances the base model's performance in document chunking by focusing exclusively on the chunking task. Moreover, as indicated in the subsequent experimental results presented in [subsection 5.5,](#page-8-1) the accuracy improvement of the HC method in document chunking leads to enhanced performance throughout the RAG pipeline. This includes improvements in the quality of evidence retrieval and model responses.

<span id="page-7-0"></span>**Table 3.** Chunking accuracy. **HC** means the result of HiChunk without fixed size chunking. The best result is in **bold**.

| Chunk<br>Qasper |        |        |        | Gov-Report |        | HiCBench |        |        |        |
|-----------------|--------|--------|--------|------------|--------|----------|--------|--------|--------|
| Method          | F1L1   | F1L2   | F1Lall | F1L1       | F1L2   | F1Lall   | F1L1   | F1L2   | F1Lall |
| SC              | 0.0759 | -      | 0.1007 | 0.0298     | -      | 0.0616   | 0.0487 | -      | 0.1507 |
| LC              | 0.5481 | -      | 0.6657 | 0.1795     | -      | 0.5631   | 0.2849 | -      | 0.4858 |
| HC              | 0.6742 | 0.5169 | 0.9441 | 0.9505     | 0.8895 | 0.9882   | 0.4841 | 0.3140 | 0.5450 |

#### <span id="page-8-1"></span>**5.5 RAG-pipeline Evaluation**

We evaluated the performance of various chunking methods on the LongBench, Qasper, GutenQA, OHRBench and HiCBench datasets, with the results detailed in [Table 4.](#page-8-2) The performance of each subset in LongBench is shown in [Table A1.](#page-14-0) The results demonstrate that the HC200+AM method achieves either optimal or suboptimal performance on most LongBench subsets. When considering average scores, LumberChunk remains a strong baseline. However, as noted in [Table 2,](#page-6-1) both GutenQA and OHRBench datasets exhibit the feature of evidence sparsity, meaning that the evidence related to QA pairs is derived from only a few sentences within the document. Consequently, the different chunking methods show minimal variation in evidence recall and response quality metrics on these datasets. For instance, using Qwen3-32B as the response model on the GutenQA dataset, the evidence recall metrics of FC200 and HC200+AM are 64.5 and 65.53, and the Rouge metrics are 44.86 and 44.94, respectively. Another example is OHRBench dataset, the evidence recall metrics and Rouge metrics of FC200, LC, HC200 and HC200+AM are very close. In contrast, the Qasper and HiCBench datasets contain denser evidence, where a better chunking method results in higher evidence recall and improved response quality. Again using Qwen3-32B as an example, on the *T*<sup>1</sup> task of HiCBench dataset, the evidence recall metric for FC200 and HC200+AM are 74.06 and 81.03, the Fact-Cov metrics are 63.20 and 68.12, and the Rouge metrics are 35.70 and 37.29, respectively. These findings suggest that the evidence-dense QA in the HiCBench dataset is better suited for evaluating the quality of chunking methods, enabling researchers to more effectively identify bottlenecks within the overall RAG pipeline.

<span id="page-8-2"></span>**Table 4.** RAG-pipeline evaluation on LongBench, Qasper, GutenQA, OHRBench and HiCBench. Evidence Recall and Fact Coverage metric is represented by ERec and FC respectively. The best result is in **bold**, and the sub-optimal result is in underlined

| Chunk       | LongBench |       | Qasper |       | GutenQA |           | OHRBench(T0) |       | HiCBench(T1) |       | HiCBench(T2) |       |       |
|-------------|-----------|-------|--------|-------|---------|-----------|--------------|-------|--------------|-------|--------------|-------|-------|
| Method      | Score     | ERec  | F1     | ERec  | Rouge   | ERec      | Rouge        | ERec  | FC           | Rouge | ERec         | FC    | Rouge |
| Llama3.1-8B |           |       |        |       |         |           |              |       |              |       |              |       |       |
| FC200       | 42.49     | 84.08 | 47.26  | 64.43 | 30.03   | 67.03     | 51.01        | 74.84 | 47.82        | 28.43 | 74.61        | 46.79 | 30.97 |
| SC          | 42.12     | 82.08 | 47.47  | 58.30 | 28.58   | 62.65     | 49.10        | 72.14 | 46.80        | 28.43 | 73.49        | 45.28 | 30.92 |
| LC          | 42.73     | 87.08 | 48.20  | 63.67 | 30.22   | 68.42     | 51.85        | 76.64 | 50.84        | 29.62 | 76.12        | 49.12 | 32.01 |
| HC200       | 43.17     | 86.16 | 48.09  | 65.13 | 29.95   | 68.25     | 51.33        | 78.52 | 49.87        | 29.38 | 78.76        | 49.11 | 31.80 |
| +AM         | 42.90     | 87.49 | 48.95  | 65.47 | 30.33   | 67.84     | 51.92        | 81.59 | 55.58        | 30.04 | 80.96        | 53.66 | 33.04 |
|             | Qwen3-8B  |       |        |       |         |           |              |       |              |       |              |       |       |
| FC200       | 43.95     | 84.32 | 45.10  | 64.50 | 33.47   | 67.07     | 48.18        | 74.06 | 47.35        | 33.83 | 72.95        | 43.45 | 35.27 |
| SC          | 43.54     | 82.22 | 44.55  | 58.37 | 32.71   | 62.18     | 46.79        | 71.42 | 46.07        | 33.30 | 72.36        | 42.97 | 34.76 |
| LC          | 44.83     | 87.43 | 46.05  | 63.67 | 33.87   | 68.79     | 49.28        | 75.53 | 48.27        | 34.12 | 75.14        | 46.80 | 35.93 |
| HC200       | 43.90     | 86.49 | 45.95  | 65.20 | 33.89   | 68.57     | 49.06        | 77.68 | 47.37        | 34.30 | 78.10        | 46.20 | 36.32 |
| +AM         | 44.41     | 87.85 | 45.82  | 65.53 | 34.15   | 68.31     | 49.61        | 81.03 | 50.75        | 35.26 | 80.65        | 49.02 | 37.28 |
|             |           |       |        |       |         | Qwen3-32B |              |       |              |       |              |       |       |
| FC200       | 46.33     | 84.32 | 46.49  | 64.50 | 44.86   | 67.07     | 46.89        | 74.06 | 63.20        | 35.70 | 72.95        | 60.87 | 37.17 |
| SC          | 46.29     | 82.22 | 46.39  | 58.37 | 43.59   | 62.18     | 45.43        | 71.26 | 61.09        | 35.64 | 72.36        | 59.23 | 37.09 |
| LC          | 47.43     | 87.43 | 46.82  | 63.67 | 44.45   | 68.79     | 47.92        | 75.53 | 64.76        | 36.15 | 75.14        | 62.75 | 38.02 |
| HC200       | 46.71     | 86.49 | 46.99  | 65.20 | 44.83   | 68.57     | 47.71        | 77.68 | 63.93        | 36.55 | 78.10        | 62.51 | 38.26 |
| +AM         | 46.92     | 87.85 | 47.25  | 65.53 | 44.94   | 68.31     | 47.89        | 81.03 | 68.12        | 37.29 | 80.65        | 66.36 | 37.37 |

#### <span id="page-8-0"></span>**5.6 Influence of Retrieval Token Budget**

Since HiCBench is more effective in assessing the performance of chunking methods, we evaluated the impact of our proposed method on the *T*<sup>1</sup> task of HiCBench under different retrieve token budgets: 2k, 2.5k, 3k, 3.5k and 4k tokens. We compared the effects of various chunking methods by calculating the Rouge metrics between responses and answers, as well as the Fact-Cov metrics. The experimental findings are illustrated in [Figure 3.](#page-9-0) The results demonstrate that a larger retrieval token budget usually leads to better response quality, so it is necessary to compare different chunking methods under the same retrieval token budget. HC200+AM consistently achieves superior response quality across various retrieve token budget

settings. These experimental results underscore the effectiveness of HC200+AM method. We further present the correspond curves of the evidence recall metrics in [subsection A.2.](#page-14-1)

<span id="page-9-0"></span>**Figure 3.** Performance of HiCBench(*T*<sup>1</sup> ) under different retrieval token budget from 2k to 4k.

### **5.7 Effect of Maximum Hierarchical Level**

In this section, we examine the impact of limiting the maximum hierarchical level of document structure obtained by HiChunk. The maximum level ranges from 1 to 4, denoted as *L*1 to *L*4, while *LA* represents no limitation on the maximum level. We measure the evidence recall metric on different settings. As shown in [Figure 4.](#page-9-1) This result reveals that the Auto-Merge retrieval algorithm degrades the performance of RAG system in the *L*1 setting due to the overly coarse-grained semantics of *L*1 chunks. As the maximum level increases from 1 to 3, the evidence recall metric also gradually improves and remains largely unchanged thereafter. These findings highlight the importance of document hierarchical structure for enchancing RAG systems.

<span id="page-9-1"></span>**Figure 4.** Evidence recall metric across different maximum level on HiCBench(*T*<sup>1</sup> and *T*2).

#### **5.8 Time Cost for Chunking**

As document chunking is essential for RAG systems, it must meet specific timeliness requirements. In this section, we analyze the time costs associated with different semantic-based chunking methods, as presented in [Table 5.](#page-10-5) Although the SC method exhibits superior real-time performance, it consistently falls short in quality across various datasets compared to other baselines. However, the LC method demonstrates reasonably good performance, but its chunking speed is considerably slower than other semantic-based methods, limiting its applicability within RAG systems. In contrast, the HC method achieves the highest chunking quality among all baseline methods while maintaining an acceptable time cost, making it well suited for implementation in real scenarios RAG systems.

<span id="page-10-5"></span>

| Dataset      |           | SC          |          | LC          |        | HC          |        |  |
|--------------|-----------|-------------|----------|-------------|--------|-------------|--------|--|
|              | Avg. Word | Time(s/doc) | Chunks   | Time(s/doc) | Chunks | Time(s/doc) | Chunks |  |
| Qasper       | 4,166     | 0.4867      | 43.83    | 5.4991      | 18.32  | 1.4993      | 15.08  |  |
| Gov-report   | 13,153    | 1.3219      | 114.72   | 15.4321     | 40.89  | 4.3382      | 29.79  |  |
| OHRBench(T0) | 26,808    | 3.0943      | 249.14   | 37.3935     | 89.68  | 14.5776     | 92.23  |  |
| GutenQA      | 146,507   | 16.5028     | 1,453.00 | 132.4900    | 393.52 | 60.1921     | 232.85 |  |
| HiCBench     | 8,519     | 1.0169      | 80.12    | 13.4414     | 41.48  | 5.7506      | 51.35  |  |

**Table 5.** Time cost of different chunking methods.

# **6 Conclusion**

This paper begins by analyzing the shortcomings of current benchmarks used for evaluating RAG systems, specifically highlighting how evidence sparsity makes them unsuitable for assessing different chunking methods. As a solution, we introduce HiCBench, a QA benchmark focused on hierarchical document chunking, which effectively evaluates the impact of various chunking methods on the entire RAG process. Additionally, we propose the HiChunk framework, which, when combined with the Auto-Merge retrieval algorithm, significantly enhances the quality of chunking, retrieval, and model responses compared to other baselines.

## **References**

- <span id="page-10-0"></span>[1] Patrick Lewis, Ethan Perez, Aleksandra Piktus, Fabio Petroni, Vladimir Karpukhin, Naman Goyal, Heinrich Küttler, Mike Lewis, Wen-tau Yih, Tim Rocktäschel, et al. Retrieval-augmented generation for knowledge-intensive nlp tasks. *Advances in neural information processing systems*, 33:9459–9474, 2020.
- <span id="page-10-1"></span>[2] Jiawei Chen, Hongyu Lin, Xianpei Han, and Le Sun. Benchmarking large language models in retrievalaugmented generation. In *Proceedings of the AAAI Conference on Artificial Intelligence*, volume 38, pages 17754–17762, 2024.
- <span id="page-10-2"></span>[3] Yue Zhang, Yafu Li, Leyang Cui, Deng Cai, Lemao Liu, Tingchen Fu, Xinting Huang, Enbo Zhao, Yu Zhang, Yulong Chen, et al. Siren's song in the ai ocean: A survey on hallucination in large language models. *Computational Linguistics*, pages 1–46, 2025.
- <span id="page-10-3"></span>[4] Hangfeng He, Hongming Zhang, and Dan Roth. Rethinking with retrieval: Faithful large language model inference. *arXiv preprint arXiv:2301.00303*, 2022.
- <span id="page-10-4"></span>[5] Cunxiang Wang, Xiaoze Liu, Yuanhao Yue, Xiangru Tang, Tianhang Zhang, Cheng Jiayang, Yunzhi Yao, Wenyang Gao, Xuming Hu, Zehan Qi, et al. Survey on factuality in large language models: Knowledge, retrieval and domain-specificity. *arXiv preprint arXiv:2310.07521*, 2023.

- <span id="page-11-0"></span>[6] Xianzhi Li, Samuel Chan, Xiaodan Zhu, Yulong Pei, Zhiqiang Ma, Xiaomo Liu, and Sameena Shah. Are chatgpt and gpt-4 general-purpose solvers for financial text analytics? a study on several typical tasks. *arXiv preprint arXiv:2305.05862*, 2023.
- <span id="page-11-1"></span>[7] Sinchana Ramakanth Bhat, Max Rudat, Jannis Spiekermann, and Nicolas Flores-Herr. Rethinking chunk size for long-document retrieval: A multi-dataset analysis. *arXiv preprint arXiv:2505.21700*, 2025.
- <span id="page-11-2"></span>[8] Yushi Bai, Xin Lv, Jiajie Zhang, Hongchang Lyu, Jiankai Tang, Zhidian Huang, Zhengxiao Du, Xiao Liu, Aohan Zeng, Lei Hou, Yuxiao Dong, Jie Tang, and Juanzi Li. LongBench: A bilingual, multitask benchmark for long context understanding. In Lun-Wei Ku, Andre Martins, and Vivek Srikumar, editors, *Proceedings of the 62nd Annual Meeting of the Association for Computational Linguistics (Volume 1: Long Papers)*, pages 3119–3137, Bangkok, Thailand, August 2024. Association for Computational Linguistics. doi: 10.18653/v1/2024.acl-long.172. URL <https://aclanthology.org/2024.acl-long.172/>.
- <span id="page-11-3"></span>[9] Pradeep Dasigi, Kyle Lo, Iz Beltagy, Arman Cohan, Noah A. Smith, and Matt Gardner. A dataset of information-seeking questions and answers anchored in research papers. In Kristina Toutanova, Anna Rumshisky, Luke Zettlemoyer, Dilek Hakkani-Tur, Iz Beltagy, Steven Bethard, Ryan Cotterell, Tanmoy Chakraborty, and Yichao Zhou, editors, *Proceedings of the 2021 Conference of the North American Chapter of the Association for Computational Linguistics: Human Language Technologies*, pages 4599–4610, Online, June 2021. Association for Computational Linguistics. doi: 10.18653/v1/2021.naacl-main.365. URL <https://aclanthology.org/2021.naacl-main.365/>.
- <span id="page-11-4"></span>[10] André V. Duarte, João DS Marques, Miguel Graça, Miguel Freire, Lei Li, and Arlindo L. Oliveira. LumberChunker: Long-form narrative document segmentation. In Yaser Al-Onaizan, Mohit Bansal, and Yun-Nung Chen, editors, *Findings of the Association for Computational Linguistics: EMNLP 2024*, pages 6473–6486, Miami, Florida, USA, November 2024. Association for Computational Linguistics. doi: 10.18653/v1/2024.findings-emnlp.377. URL [https://aclanthology.org/2024.findings-emnlp.](https://aclanthology.org/2024.findings-emnlp.377/) [377/](https://aclanthology.org/2024.findings-emnlp.377/).
- <span id="page-11-5"></span>[11] Junyuan Zhang, Qintong Zhang, Bin Wang, Linke Ouyang, Zichen Wen, Ying Li, Ka-Ho Chow, Conghui He, and Wentao Zhang. Ocr hinders rag: Evaluating the cascading impact of ocr on retrieval-augmented generation. *arXiv preprint arXiv:2412.02592*, 2024.
- <span id="page-11-6"></span>[12] Zhilin Yang, Peng Qi, Saizheng Zhang, Yoshua Bengio, William W Cohen, Ruslan Salakhutdinov, and Christopher D Manning. Hotpotqa: A dataset for diverse, explainable multi-hop question answering. *arXiv preprint arXiv:1809.09600*, 2018.
- <span id="page-11-7"></span>[13] Tomáš Koˇcisky, Jonathan Schwarz, Phil Blunsom, Chris Dyer, Karl Moritz Hermann, Gábor Melis, and ` Edward Grefenstette. The narrativeqa reading comprehension challenge. *Transactions of the Association for Computational Linguistics*, 6:317–328, 2018.
- <span id="page-11-8"></span>[14] Richard Yuanzhe Pang, Alicia Parrish, Nitish Joshi, Nikita Nangia, Jason Phang, Angelica Chen, Vishakh Padmakumar, Johnny Ma, Jana Thompson, He He, et al. Quality: Question answering with long input texts, yes! *arXiv preprint arXiv:2112.08608*, 2021.
- <span id="page-11-9"></span>[15] Shitao Xiao, Zheng Liu, Peitian Zhang, Niklas Muennighoff, Defu Lian, and Jian-Yun Nie. C-pack: Packed resources for general chinese embeddings. In *Proceedings of the 47th International ACM SIGIR Conference on Research and Development in Information Retrieval*, SIGIR '24, page 641–649, New York, NY, USA, 2024. Association for Computing Machinery. ISBN 9798400704314. doi: 10.1145/3626772.3657878. URL <https://doi.org/10.1145/3626772.3657878>.
- <span id="page-11-10"></span>[16] Jihao Zhao, Zhiyuan Ji, Zhaoxin Fan, Hanyu Wang, Simin Niu, Bo Tang, Feiyu Xiong, and Zhiyu Li. MoC: Mixtures of text chunking learners for retrieval-augmented generation system. In Wanxiang Che, Joyce Nabende, Ekaterina Shutova, and Mohammad Taher Pilehvar, editors, *Proceedings of the 63rd Annual Meeting of the Association for Computational Linguistics (Volume 1: Long Papers)*, pages 5172–5189, Vienna, Austria, July 2025. Association for Computational Linguistics. ISBN 979-8-89176-251-0. doi: 10.18653/v1/2025.acl-long.258. URL <https://aclanthology.org/2025.acl-long.258/>.

- <span id="page-12-0"></span>[17] Zhitong Wang, Cheng Gao, Chaojun Xiao, Yufei Huang, Shuzheng Si, Kangyang Luo, Yuzhuo Bai, Wenhao Li, Tangjian Duan, Chuancheng Lv, et al. Document segmentation matters for retrievalaugmented generation. In *Findings of the Association for Computational Linguistics: ACL 2025*, pages 8063–8075, 2025.
- <span id="page-12-1"></span>[18] Sangwoo Cho, Kaiqiang Song, Xiaoyang Wang, Fei Liu, and Dong Yu. Toward unifying text segmentation and long document summarization. *arXiv preprint arXiv:2210.16422*, 2022.
- <span id="page-12-2"></span>[19] Yang Liu, Chenguang Zhu, and Michael Zeng. End-to-end segmentation-based news summarization. *arXiv preprint arXiv:2110.07850*, 2021.
- <span id="page-12-3"></span>[20] Qinglin Zhang, Qian Chen, Yali Li, Jiaqing Liu, and Wen Wang. Sequence model with self-adaptive sliding window for efficient spoken document segmentation. In *2021 IEEE Automatic Speech Recognition and Understanding Workshop (ASRU)*, pages 411–418. IEEE, 2021.
- <span id="page-12-4"></span>[21] Jacob Devlin, Ming-Wei Chang, Kenton Lee, and Kristina Toutanova. Bert: Pre-training of deep bidirectional transformers for language understanding. In *Proceedings of the 2019 conference of the North American chapter of the association for computational linguistics: human language technologies, volume 1 (long and short papers)*, pages 4171–4186, 2019.
- <span id="page-12-5"></span>[22] Arihant Jain, Purav Aggarwal, and Anoop Saladi. Autochunker: Structured text chunking and its evaluation. In *Proceedings of the 63rd Annual Meeting of the Association for Computational Linguistics (Volume 6: Industry Track)*, pages 983–995, 2025.
- <span id="page-12-6"></span>[23] Michael Günther, Isabelle Mohr, Daniel James Williams, Bo Wang, and Han Xiao. Late chunking: contextual chunk embeddings using long-context embedding models. *arXiv preprint arXiv:2409.04701*, 2024.
- <span id="page-12-7"></span>[24] Omri Koshorek, Adir Cohen, Noam Mor, Michael Rotman, and Jonathan Berant. Text segmentation as a supervised learning task. In Marilyn Walker, Heng Ji, and Amanda Stent, editors, *Proceedings of the 2018 Conference of the North American Chapter of the Association for Computational Linguistics: Human Language Technologies, Volume 2 (Short Papers)*, pages 469–473, New Orleans, Louisiana, June 2018. Association for Computational Linguistics. doi: 10.18653/v1/N18-2075. URL <https://aclanthology.org/N18-2075/>.
- <span id="page-12-8"></span>[25] Tengchao Lv, Lei Cui, Momcilo Vasilijevic, and Furu Wei. Vt-ssum: A benchmark dataset for video transcript segmentation and summarization. *arXiv preprint arXiv:2106.05606*, 2021.
- <span id="page-12-9"></span>[26] Haoqian Wu, Keyu Chen, Haozhe Liu, Mingchen Zhuge, Bing Li, Ruizhi Qiao, Xiujun Shu, Bei Gan, Liangsheng Xu, Bo Ren, et al. Newsnet: A novel dataset for hierarchical temporal segmentation. In *Proceedings of the IEEE/CVF Conference on Computer Vision and Pattern Recognition*, pages 10669–10680, 2023.
- <span id="page-12-10"></span>[27] Zhilin Yang, Peng Qi, Saizheng Zhang, Yoshua Bengio, William Cohen, Ruslan Salakhutdinov, and Christopher D. Manning. HotpotQA: A dataset for diverse, explainable multi-hop question answering. In Ellen Riloff, David Chiang, Julia Hockenmaier, and Jun'ichi Tsujii, editors, *Proceedings of the 2018 Conference on Empirical Methods in Natural Language Processing*, pages 2369–2380, Brussels, Belgium, October-November 2018. Association for Computational Linguistics. doi: 10.18653/v1/D18-1259. URL <https://aclanthology.org/D18-1259/>.
- <span id="page-12-11"></span>[28] Robert Friel, Masha Belyi, and Atindriyo Sanyal. Ragbench: Explainable benchmark for retrievalaugmented generation systems. *arXiv preprint arXiv:2407.11005*, 2024.
- <span id="page-12-12"></span>[29] Zhishang Xiang, Chuanjie Wu, Qinggang Zhang, Shengyuan Chen, Zijin Hong, Xiao Huang, and Jinsong Su. When to use graphs in rag: A comprehensive analysis for graph retrieval-augmented generation. *arXiv preprint arXiv:2506.05690*, 2025.
- <span id="page-12-13"></span>[30] Luyang Huang, Shuyang Cao, Nikolaus Parulian, Heng Ji, and Lu Wang. Efficient attentions for long document summarization. In *Proceedings of the 2021 Conference of the North American Chapter of the Association for Computational Linguistics: Human Language Technologies*, pages 1419–1436, Online,

- June 2021. Association for Computational Linguistics. doi: 10.18653/v1/2021.naacl-main.112. URL <https://aclanthology.org/2021.naacl-main.112>.
- <span id="page-13-0"></span>[31] DeepSeek-AI. Deepseek-r1: Incentivizing reasoning capability in llms via reinforcement learning, 2025. URL <https://arxiv.org/abs/2501.12948>.
- <span id="page-13-1"></span>[32] Qwen Team. Qwen3 technical report, 2025. URL <https://arxiv.org/abs/2505.09388>.
- <span id="page-13-2"></span>[33] Jianlyu Chen, Shitao Xiao, Peitian Zhang, Kun Luo, Defu Lian, and Zheng Liu. M3-embedding: Multi-linguality, multi-functionality, multi-granularity text embeddings through self-knowledge distillation. In Lun-Wei Ku, Andre Martins, and Vivek Srikumar, editors, *Findings of the Association for Computational Linguistics: ACL 2024*, pages 2318–2335, Bangkok, Thailand, August 2024. Association for Computational Linguistics. doi: 10.18653/v1/2024.findings-acl.137. URL [https:](https://aclanthology.org/2024.findings-acl.137/) [//aclanthology.org/2024.findings-acl.137/](https://aclanthology.org/2024.findings-acl.137/).
- <span id="page-13-3"></span>[34] Abhimanyu Dubey, Abhinav Jauhri, Abhinav Pandey, Abhishek Kadian, Ahmad Al-Dahle, Aiesha Letman, Akhil Mathur, Alan Schelten, Amy Yang, Angela Fan, et al. The llama 3 herd of models. *arXiv e-prints*, pages arXiv–2407, 2024.

# A Appendix

#### A.1 Detail of LongBench

In this section, we present the metric of each subset of the different chunking methods on LongBench, and the results are shown in Table A1.

<span id="page-14-0"></span>**Table A1.** RAG-pipeline evaluation on LongBench and each subset. The best result is in **bold**, and the sub-optimal result is in <u>underlined</u>. Qasper\* is the subset of LongBench.

| Chunk       |              | Single-I     | Ooc QA       |              | Multi-Doc QA |              |              |              |              |  |
|-------------|--------------|--------------|--------------|--------------|--------------|--------------|--------------|--------------|--------------|--|
| Method      | NarrativeQA  | Qasper*      | MFQA-en      | MFQA-zh      | HotpotQA     | 2WikiM       | MuSiQue      | DuReader     | Avg          |  |
| Llama3.1-8B |              |              |              |              |              |              |              |              |              |  |
| FC200       | 24.59        | 42.68        | 52.54        | 56.14        | 56.81        | 46.66        | 29.99        | 30.51        | 42.49        |  |
| SC          | 24.59        | 42.12        | 52.10        | 57.43        | 54.34        | 45.44        | 30.24        | 30.68        | 42.12        |  |
| LC          | 22.93        | 42.64        | <u>52.65</u> | 58.54        | 55.85        | <u>47.00</u> | 31.58        | 30.68        | 42.73        |  |
| HC200       | 23.75        | <u>43.57</u> | 54.04        | <u>57.51</u> | 56.52        | 48.29        | 31.06        | 30.65        | 43.17        |  |
| +AM         | <u>24.46</u> | 43.85        | 52.10        | 56.65        | 57.27        | 46.24        | 31.82        | 30.84        | <u>42.90</u> |  |
|             |              |              |              | Qwen3-       | 8B           |              |              |              |              |  |
| FC200       | 22.60        | 44.47        | 53.46        | 57.26        | 61.13        | 48.63        | 36.59        | 27.43        | 43.95        |  |
| SC          | 24.73        | 43.69        | 52.83        | 58.66        | 56.22        | 46.77        | 37.83        | <u>27.59</u> | 43.54        |  |
| LC          | 24.55        | 43.41        | 54.58        | 59.60        | 60.50        | 51.00        | 37.37        | 27.60        | 44.83        |  |
| HC200       | 21.96        | 42.38        | 51.23        | 58.47        | 62.84        | <u>49.57</u> | 38.03        | 26.74        | 43.90        |  |
| +AM         | 21.79        | 46.37        | 52.81        | <u>58.86</u> | <u>61.94</u> | 47.17        | 39.09        | 27.28        | <u>44.41</u> |  |
|             |              |              |              | Qwen3-3      | 32B          |              |              |              |              |  |
| FC200       | 26.09        | 43.70        | 50.87        | 60.44        | 63.61        | 58.03        | 39.40        | 28.50        | 46.33        |  |
| SC          | 26.19        | 43.47        | 49.54        | 61.63        | 61.37        | 58.13        | 40.65        | 29.34        | 46.29        |  |
| LC          | 26.35        | 44.75        | 50.21        | 63.01        | 63.31        | 60.22        | 42.69        | 28.91        | 47.43        |  |
| HC200       | 27.01        | 44.44        | 49.69        | <u>62.16</u> | 61.85        | 61.24        | 38.54        | 28.54        | 46.71        |  |
| +AM         | <u>26.97</u> | 44.49        | <u>50.28</u> | 60.47        | 61.67        | 62.37        | <u>40.80</u> | 28.29        | <u>46.92</u> |  |

#### <span id="page-14-1"></span>A.2 Evidence Recall under Different Token Budget

In this section, we further present the curve of evidence recall metric at different retrieval context length settings (from 2k to 4k). The results are shown in Figure A1. Compared with other chunking methods, the HC200+AM method always maintains the best performance.

<span id="page-14-2"></span>**Figure A1.** Evidence recall metric across different token budget on  $HiCBench(T_1)$ .

## <span id="page-15-0"></span>**A.3 Prompts**

Listing A1: Prompt for segment summarization.

```
** Task :**
You are tasked with analyzing the provided document sections and their hierarchical
    structure . Your goal is to generate a concise and informative paragraph describing the
    content of each section and subsection .
** Instructions :**
1. Each section or subsection is identified by a header in the format '=== SECTION xxx === '
    ( for example , '=== SECTION 1=== ' , '=== SECTION 2.1=== ' , etc .) .
2. For every section and subsection , write a brief , clear , and informative paragraph
    summarizing its content . Do not omit any section or subsection .
3. Present your output as a JSON object with the following structure :
''' json
{
    " SECTION 1": " description of section 1" ,
    " SECTION 1.1": " description of section 1.1" ,
    ...
    " SECTION n .m ": " description of section n .m"
}
'''
4. Ensure that each key in the JSON object matches the exact section identifier (e.g .,
    '" SECTION 2.1.3" ') , and do not include any sections or subsections that are not
    present in the provided document fragment .
5. Do not add any commentary or explanation outside the JSON object .
** Document Fragment :**
```

#### Listing A2: Prompt for QA cunstruction.

```
You are provided with a document that includes a detailed structure of sections and
    subsections , along with descriptions for each . Additionally , complete contents are
    provided for a few selected sections . Your task is to create a question and answer pair
    that effectively captures the essence of the selected sections . Finally , you need to
    extract the facts which are mentioned in the answer .
< Type of Generated Q &A Task : Evidence - dense Dependent Understanding task >
Understanding task means that , the generated question - answering pairs that require the
    responser to extract information from documents . The answer should be able to find
    directly in the documents without any reasoning .
Evidence - dense dependent means that the facts about generated question are wildly
    distributed across all parts of the retrieved sections .
< Criteria >
- The question MUST be detailed and be based explicitly on information in the document .
- The question MUST include at least one entity .
- Question must not contain any ambiguous references , such as 'he ', 'she ' , 'it ', ' the
    report ', ' the paper ' , and ' the document '. You MUST use their complete names .
- The context sentence the question is based on MUST include the name of the entity . For
    example , an unacceptable context is " He won a bronze medal in the 4 * 100 m relay ". An
    acceptable context is " Nils Sandstrom was a Swedish sprinter who competed at the 1920
    Summer Olympics ."
- ** THE MOST IMPORTANT : Evidence - dense dependency ** , Questions must require understanding
    of ENTIRE selected sections . Never base Q&A on isolated few sentences . For example , a
    question comply the ** Evidence - dense dependency ** criteria means that the facts about
    this question should be wildly distributed across all parts of the retrieved sections .
< Output Format >
Your response should be structured as follows :
''' json
{{
    " question ": " Your generated question here ",
    " answer ": " Your generated answer here "
}}
'''
```

```
< Document Structure and Description >
{ section_description }
< Retrieved Section and Content >
{ section_content }
```

#### Listing A3: Prompt for evidence retrieval.

```
** Task :**
Analyze the relationship between context sentences and answer sentences .
** Instructions :**
1. You are given :
    - A context fragment , with each sentence numbered as follows : '[ serial number ]: context
    sentence content '
    - A question and its corresponding answer , with each answer sentence numbered as
    follows : '< serial number >: answer sentence content '
2. For each sentence in the answer , identify which sentence (s) from the context provide the
    information used to construct that answer sentence .
3. Present your findings in the following JSON format :
''' json
{{
    "< answer_sentence_id_1 >": "[ context_sentence_id_1 ] , ... , [ context_sentence_id_n ]" ,
    "< answer_sentence_id_2 >": "[ context_sentence_id_1 ] , ... , [ context_sentence_id_m ]" ,
    ...
    "< answer_sentence_id_i >": "[ context_sentence_id_1 ] , ... , [ context_sentence_id_j ]"
}}
'''
** Notes :**
- Only include answer sentences that have supporting evidence in the context .
- If an answer sentence does not have a source in the context , do not include it in the
    JSON output .
- Use only the serial numbers ( not the full sentences ) for both context and answer
    sentences in your JSON output .
- If multiple context sentences support an answer sentence , list all relevant context
    sentence numbers , separated by commas .
** Context Sentences :**
{ context_sentence_list }
** Question :**
{ question }
** Answer Sentences :**
{ answer_sentence_list }
```

#### Listing A4: Prompt for model training.

```
You are an assistant good at reading and formatting documents , and you are also skilled at
    distinguishing the semantic and logical relationships of sentences between document
    context . The following is a text that has already been divided into sentences . Each
    line is formatted as : "{ line number } @ { sentence content }". You need to segment this
    text based on semantics and format . There are multiple levels of granularity for
    segmentation , the higher level number means the finer granularity of the segmentation .
    Please ensure that each Level One segment is semantically complete after segmentation .
    A Level One segment may contain multiple Level Two segments , and so on . Please
    incrementally output the starting line numbers of each level of segments , and determine
    the level of the segment , as well as whether the content of the sentence at the
    starting line number can be used as the title of the segment . Finally , output a list
    format result , where each element is in the format of : "{ line number }, { segment level },
    { be a title ?}".
>>> Input text :
```