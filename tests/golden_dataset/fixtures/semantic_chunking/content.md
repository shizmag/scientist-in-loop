Contents lists available at [ScienceDirect](www.sciencedirect.com/science/journal/09507051)

# Knowledge-Based Systems

journal homepage: [www.elsevier.com/locate/knosys](https://www.elsevier.com/locate/knosys)

# Optimising retrieval performance in RAG systems: A new growing window semantic chunking strategy to address weak semantic boundaries

Antonio Moreno-Cediel , Eva Garcia-Lopez , Antonio Garcia-Cabot \* , David De-Fitero-Dominguez

*Departamento de Ciencias de la Computaci*´*on, Universidad de Alcala, Alcal* ´ *a de Henares, Madrid 28801, Spain* ´

## A R T I C L E I N F O

*Keywords:* Artificial intelligence NLP RAG Semantic splitting Sentence textual similarity Chunking

#### ABSTRACT

The release of ChatGPT in November 2022 signified a pivotal shift within the domain of Natural Language Processing, particularly in the context of chatbot technology. Since then, chatbots powered by Large Language Models have been widely adopted, demonstrating remarkable capabilities across a wide range of tasks. In light of the substantial impact of this technology, the study of its application in specific domains has emerged as a crucial area of research. However, given that these chatbots are based on Large Language Models, they are subject to the well-known limitations of this technology when handling with knowledge-intensive tasks, and are prone to generate hallucinated answers. The RAG (Retrieval-Augmented Generation) architecture, which comprises an indexing phase, was developed to address this issue. During the indexing phase, text should be split into different chunks using a chunking technique. We thoroughly analyse state-of-the-art text chunking techniques used in RAG pipelines and propose a novel semantic text chunking technique. To evaluate the effectiveness of our technique within RAG pipelines, an exhaustive evaluation framework has been defined and applied. This evaluation also enables the comparison with previous techniques, resulting in a noticeable improvement in respect to state-ofthe-art strategies.

## **1. Introduction**

The development of chatbots has been closely intertwined with the progression of Natural Language Processing (NLP) technologies. However, their actual growth is directly aligned with the emergence of Large Language Models (LLMs) based on the Transformer architecture [[1](#page--1-0)]. Since its introduction in 2017, the Transformer architecture has consistently achieved state of the art results across a range of NLP tasks, including text generation, machine translation, or question answering [1–3]. Consequently, contemporary chatbot implementations leverage this architecture.

Despite the continuous enhancement of LLM performance, these models suffer from hallucinations when employed in knowledgeintensive tasks, defined by Lewis et al. [4] as those tasks in which humans could not reasonably be expected to perform without access to an external knowledge source. Hallucinations manifest as LLM-generated outputs that, while seeming plausible, deviate from the user's input, previously generated context or factual knowledge, thereby compromising the reliability of LLMs in real-world scenarios [5]. This

drawback restricts the use of chatbots in contexts where, despite their usability, the need of a robust and reliable answer is crucial. For example, while chatbots can facilitate the process of retrieving information from extensive documentation, such as aircraft repair manuals, the critical nature of these tasks necessitates the assurance of the accuracy of the information, as well as the exclusion of inaccurate or fabricated information.

To mitigate LLM hallucinations, Lewis et al. [4] introduced the Retrieval Augmented Generation (RAG) architecture in 2020. The RAG architecture enhances the LLM's parametric memory, which is inherently limited to the knowledge acquired during training, by integrating a non-parametric memory component: a retriever. This retriever searches and retrieves relevant information from a vector index in response to user queries [4]. Therefore, the performance of the RAG architecture is completely dependent on the quality of the retrieved information and its relationship with the user query, as the retrieval of irrelevant data may lead to incorrect responses [6,7].

Given the critical role of Information Retrieval (IR) in determining overall RAG performance, optimizing IR system efficacy is paramount.

*E-mail address:* [a.garciac@uah.es](mailto:a.garciac@uah.es) (A. Garcia-Cabot).

<sup>\*</sup> Corresponding author.

**Table 4**  Chunk Length Statistics from different splitting strategies using Spanish MIRACL. First row corresponds to Kamradt's splitting strategy using a window size of three sentences (*w* = 3). Following rows stand for our strategy with the three different configurations of initial size (n) and regular size (m).

| Chunking Strategy |           | Mean   | Std    | Median | Max    | Min | 25th Percentile | 75th Percentile |
|-------------------|-----------|--------|--------|--------|--------|-----|-----------------|-----------------|
| Kamradt w ¼ 3     | Words     | 213.20 | 285.00 | 114.00 | 27,173 | 0   | 54              | 261             |
|                   | Sentences | 8.53   | 9.64   | 5.00   | 1035   | 1   | 4               | 12              |
| n ¼ 4             | Words     | 87.77  | 45.87  | 83.00  | 4999   | 0   | 57              | 112             |
| m ¼ 2             | Sentences | 4.95   | 1.35   | 5.00   | 417    | 1   | 5               | 5               |
| n ¼ 6             | Words     | 124.44 | 64.60  | 121.00 | 5142   | 0   | 82              | 161             |
| m ¼ 3             | Sentences | 6.63   | 2.06   | 7.00   | 414    | 1   | 7               | 7               |
| n ¼ 8             | Words     | 156.56 | 84.74  | 156.00 | 5129   | 0   | 97              | 208             |
| m ¼ 4             | Sentences | 8.11   | 2.85   | 9.00   | 417    | 1   | 7               | 9               |

**Table 5**  Chunk Length Statistics from different splitting strategies using English MIRACL subset. First three rows correspond to Kamradt's splitting strategy using a window size of two, three and four sentences (*w* = 2, *w* = 3, *w* = 4), respectively. Following rows stand for our strategy with the three different configurations of initial size (n) and regular size (m). Final row represents the static splitting strategy using |n as delimiter.

| Chunking Strategy   |           | Mean   | Std     | Median | Max  | Min | 25th Percentile | 75th Percentile |
|---------------------|-----------|--------|---------|--------|------|-----|-----------------|-----------------|
| Kamradt w ¼ 2       | Words     | 192.58 | 251.851 | 101.0  | 7535 | 0   | 43.0            | 245.0           |
|                     | Sentences | 10.64  | 11.41   | 7.0    | 320  | 1   | 4.0             | 13.0            |
| Kamradt w ¼ 3       | Words     | 198.01 | 255.07  | 107.0  | 9705 | 0   | 47.0            | 251.0           |
|                     | Sentences | 10.92  | 11.48   | 7.0    | 363  | 1   | 4.0             | 14.0            |
| Kamradt w ¼ 4       | Words     | 202.72 | 257.34  | 114.0  | 8664 | 0   | 50.0            | 255.0           |
|                     | Sentences | 11.15  | 11.56   | 7.0    | 364  | 1   | 4.0             | 14.0            |
| n ¼ 4               | Words     | 75.78  | 37.70   | 72.0   | 2712 | 0   | 51.0            | 95.0            |
| m ¼ 2               | Sentences | 5.04   | 1.42    | 5.0    | 319  | 1   | 5.0             | 5.0             |
| n ¼ 6               | Words     | 107.73 | 52.93   | 106.0  | 3388 | 0   | 74.0            | 137.0           |
| m ¼ 3               | Sentences | 6.80   | 2.16    | 7.0    | 363  | 1   | 7.0             | 7.0             |
| n ¼ 8               | Words     | 136.51 | 69.33   | 137.0  | 3685 | 0   | 91.0            | 178.0           |
| m ¼ 4               | Sentences | 8.37   | 2.98    | 9.0    | 319  | 1   | 8.0             | 9.0             |
| Static Splitter(\n) | Words     | 49.92  | 55.52   | 33.0   | 2065 | 0   | 5.0             | 75.0            |
|                     | Sentences | 3.49   | 2.51    | 3.0    | 216  | 1   | 2.0             | 4.0             |

**Table 6**  IR metrics results for entire chunks (left) and chunks truncated to 512 words (right) from Spanish articles. Best values are highlighted in bold. K. w3 column stands for Kamradt's strategy with a window size of 3 (w3). Columns with nX – mY indicate the results obtained for our strategy with an initial size (n) of X sentences and a regular size (m) of Y sentences.

|                  | Full-length Chunks |         |         |         |        | 512 Word-Limited Chunks |         |         |  |
|------------------|--------------------|---------|---------|---------|--------|-------------------------|---------|---------|--|
|                  | K. w3              | n4 - m2 | n6 - m3 | n8 - m4 | K. w3  | n4 - m2                 | n6 - m3 | n8 - m4 |  |
| hit_rate@10      | 0.6793             | 0.51426 | 0.61974 | 0.65681 | 0.4850 | 0.3810                  | 0.4597  | 0.5029  |  |
| precision@10     | 0.1604             | 0.08168 | 0.11368 | 0.13118 | 0.0852 | 0.0545                  | 0.0732  | 0.0845  |  |
| recall@10        | 0.3451             | 0.18783 | 0.25771 | 0.29718 | 0.1817 | 0.1239                  | 0.1613  | 0.1883  |  |
| F1@10            | 0.2021             | 0.10466 | 0.145   | 0.16709 | 0.1071 | 0.0695                  | 0.0926  | 0.1073  |  |
| MRR@10           | 0.4933             | 0.31896 | 0.4112  | 0.44785 | 0.3287 | 0.2248                  | 0.2820  | 0.3214  |  |
| NDCG@10          | 0.3409             | 0.1812  | 0.24859 | 0.2836  | 0.1895 | 0.1204                  | 0.1569  | 0.1841  |  |
| SRA@10           | 11,823             | 19,962  | 18,665  | 17,787  | 17,231 | 22,695                  | 22,714  | 22,416  |  |
| SNRA@10          | 7747               | 11,715  | 11,303  | 11,017  | 10,222 | 12,587                  | 12,733  | 12,682  |  |
| RCC@10           | 16,968             | 8937    | 12,565  | 14,521  | 9274   | 5997                    | 8150    | 9317    |  |
| NRCC@10          | 6957               | 3137    | 4771    | 5740    | 3884   | 2188                    | 3289    | 3939    |  |
| SRA@K þ RCC@10   | 28,791             | 28,899  | 31,230  | 32,308  | 26,505 | 28,692                  | 30,864  | 31,733  |  |
| SNRA@K þ NRCC@10 | 14,704             | 14,852  | 16,074  | 16,757  | 14,106 | 14,775                  | 16,022  | 16,621  |  |

parametric alternative to the paired Student's *t*-test. This choice is justified by the nature of the data, which did not meet the normality assumptions required for parametric testing. The Wilcoxon test is

appropriate for analysing matched samples, as in our case, since the same query provides results for both strategies. Tables 7 and 8 present the significance test results obtained for metrics computed using non-

**Table 7**  Wilcoxon significance test results for metrics obtained using non-limited chunks. Tests are performed between Kamradt's approach with window size of three (*w* = 3) and each of our configurations for Spanish articles. P values lower than 0.05 are highlighted in bold, denoting statistical significance.

| K. w3        | n4-m2      |         | n6-m3      |         | n8-m4      |         |
|--------------|------------|---------|------------|---------|------------|---------|
|              | W          | P_value | W          | P_value | W          | P_value |
| hit_rate@10  | 58,446.00  | <0.001  | 58,322.50  | <0.001  | 61,074.00  | 0.0202  |
| precision@10 | 130,906.00 | <0.001  | 233,303.00 | <0.001  | 322,448.50 | <0.001  |
| recall@10    | 159,548.00 | <0.001  | 252,957.00 | <0.001  | 345,816.00 | <0.001  |
| F1@10        | 127,935.00 | <0.001  | 229,438.50 | <0.001  | 322,173.50 | <0.001  |
| MRR@10       | 221,420.00 | <0.001  | 319,525.00 | <0.001  | 361,047.00 | <0.001  |
| NDCG@10      | 258,440.00 | <0.001  | 483,728.00 | <0.001  | 678,380.50 | <0.001  |

**Table 8**  Wilcoxon significance test results for metrics obtained using 512 word-limited chunks. Tests are performed between Kamradt's approach with window size of three (*w*  = 3) and each of our configurations for Spanish articles. P values lower than 0.05 are highlighted in bold, denoting statistical significance.

| K. w3        | n4-m2      |         | n6-m3      |         | n8-m4      |         |
|--------------|------------|---------|------------|---------|------------|---------|
|              | W          | P_value | W          | P_value | W          | P_value |
| hit_rate@10  | 55,419.00  | <0.001  | 60,605.00  | 0.004   | 52,216.00  | 0.0075  |
| precision@10 | 109,207.00 | <0.001  | 174,688.00 | <0.001  | 215,314.50 | 0.2164  |
| recall@10    | 118,041.50 | <0.001  | 179,164.00 | <0.001  | 209,229.50 | 0.0555  |
| F1@10        | 108,694.50 | <0.001  | 174,291.50 | <0.001  | 211,923.50 | 0.1108  |
| MRR@10       | 172,654.50 | <0.001  | 244,832.50 | <0.001  | 269,732.50 | 0.2299  |
| NDCG@10      | 211,924.50 | <0.001  | 353,043.50 | <0.001  | 469,560.50 | 0.5774  |

limited and 512 word-limited chunks, respectively.

Regarding the results of full-length chunks, the splitting strategy proposed by Kamradt has been shown to outperform all configurations of our proposed strategy. Statistical significance tests further support this finding, revealing p-values below 0.05 across all configurations. However, when considering the SRA@K, SNRA@K, RCC@K, and NRCC@K metrics, as well as their compounds, the strategy proposed in this paper is distinguished by its ability to identify a large number of chunks originating from the same article or included in the target chunk (SRA@*K* + RCC@K). Nevertheless, it also retrieves a higher number of chunks from non-relevant articles or contained in non-relevant chunks (SNRA@*K* + NRCC@K).

A more dispersed scenario is found between both strategies and configurations when considering the 512-word truncated chunks. Kamradt's proposal achieves better results in metrics such as precision@K, MRR@K, and NDCG@K. Conversely, our approach, configured with an initial size of eight and a regular size of four, performs better in terms of hit rate, recall, and F1 score. However, statistical significance tests only support this finding for the hit rate metric. The metrics proposed in this paper yield analogous results, with our strategy significantly gathering more chunks from relevant articles or contained in positive labelled chunks (SRA@*K* + RCC@K), while Kamradt's approach retrieves a bit less number of chunks from negative labelled articles and chunks (SNRA@*K* + NRCC@K).

Relevant findings stand out when shifting towards results computed on the article subset from the English MIRACL Wikipedia dump (Tables 9 and 10). Regardless of word limitation, (1) the three sentencesized window is shown to be Kamradt's optimal configuration, and (2) the performance of static splitters is far from that of any well-configured semantic approach.

As shown in Table 9, the results obtained with Spanish articles are aligned with those from full-length chunks. Kamradt's approach outperforms all configurations of our proposed strategy. The statistical significance tests in Table 11 further support these findings, denoting statistical significance for most results. However, our method can identify more chunks from relevant articles (SRA@*K* + RCC@K) and

non-relevant (SNRA@*K* + NRCC@K) articles.

As shown in Table 10, clearer results are obtained with English articles when working with 512 word-limited chunks. Using an initial size of eight and a regular size of four, our approach noticeably surpasses any of Kamradt's splitting configurations on traditional IR metrics. In this case, statistical significance tests only denote significance for the hit rate metric when using an initial size of eight and a regular size of four. Furthermore, the tendency to retrieve a higher number of relevant (SRA@*K* + RCC@K) and non-relevant (SNRA@*K* + RNCC@K) articles is preserved.

Concerning the static splitter results, both the full-length and wordlimited settings perform worse than the three sentence window-sized Kamradt approach and our n8-m4 configuration. Furthermore, statistical tests underscore the significance of these results for all IR metrics (see Table 12).

## *4.3. Retrieval performance obtained for chunks from both strategies using synthetically generated questions*

Since a value for Score@K is obtained whenever a question is queried against the vector database, the total sum (Total score@10 sum), average (Total score@10 avg) and standard deviation (Total score@10 std) of these values have been computed. Table 13 shows the results obtained for Score@10 and the retrieval performance of each strategy using full-length and 512 word-limited chunks. As before, the results obtained for the Spanish MIRACL Wikipedia dump are shown for Kamradt's strategy with a window size of three. For our strategy, the three configurations previously mentioned are evaluated. Next, results are computed for Kamradt's strategy with window sizes of two, three, and four sentences, as well as for the three configurations of our strategy and for the static splitter, using the English MIRACL Wikipedia dump.

As demonstrated in Table 13, the strategy outlined in this paper performs remarkably well in comparison to Kamradt's strategy. Certainly, regardless of the length of the chunks, the *n* = 6; *m* = 3 and *n*  = 8; *m* = 4 configurations demonstrate a substantial improvement and significantly outperform Kamradt's strategy. The results obtained for the

**Table 9**  IR metrics results for entire chunks from English articles. Best values are highlighted in bold. K. wZ columns stand for Kamradt's strategy with a window size of Z (wZ). Columns with nX – mY indicate the results obtained for our strategy with an initial size (n) of X sentences and a regular size (m) of Y sentences.

|                  | Full-length chunks |        |        |         |         |         |                 |  |
|------------------|--------------------|--------|--------|---------|---------|---------|-----------------|--|
|                  | K. w2              | K. w3  | K. w4  | n4 – m2 | n6 - m3 | n8 - m4 | Static splitter |  |
| hit_rate@10      | 0.5671             | 0.5671 | 0.5572 | 0.2636  | 0.4029  | 0.5273  | 0.4228          |  |
| precision@10     | 0.0746             | 0.0761 | 0.0741 | 0.0293  | 0.0452  | 0.0616  | 0.0497          |  |
| recall@10        | 0.2844             | 0.2869 | 0.2772 | 0.1114  | 0.1697  | 0.2450  | 0.1947          |  |
| F1@10            | 0.1062             | 0.1088 | 0.1059 | 0.0425  | 0.0646  | 0.0894  | 0.0714          |  |
| MRR@10           | 0.3886             | 0.3915 | 0.3993 | 0.1740  | 0.2933  | 0.3627  | 0.2737          |  |
| NDCG@10          | 0.2644             | 0.2739 | 0.2713 | 0.1096  | 0.1749  | 0.2337  | 0.1763          |  |
| SRA@10           | 395                | 413    | 415    | 690     | 638     | 602     | 321             |  |
| SNRA@10          | 161                | 219    | 219    | 335     | 333     | 307     | 190             |  |
| RCC@10           | 196                | 164    | 164    | 64      | 99      | 134     | 109             |  |
| NRCC@10          | 133                | 141    | 131    | 38      | 55      | 73      | 52              |  |
| SRA@K þ RCC@10   | 591                | 577    | 579    | 754     | 737     | 736     | 430             |  |
| SNRA@K þ NRCC@10 | 294                | 360    | 350    | 373     | 388     | 380     | 242             |  |

**Table 10**  IR metrics results for chunks truncated to 512 words from English articles. Best values are highlighted in bold. K. wZ column stand for Kamradt's strategy with a window size of Z (wZ). Columns with nX – mY indicate the results obtained for our strategy with an initial size (n) of X sentences and a regular size (m) of Y sentences.

|                  | 512 Word-Limited chunks |        |        |         |         |         |                 |  |
|------------------|-------------------------|--------|--------|---------|---------|---------|-----------------|--|
|                  | K. w2                   | K. w3  | K. w4  | n4 – m2 | n6 - m3 | n8 - m4 | Static splitter |  |
| hit_rate@10      | 0.2786                  | 0.2885 | 0.2786 | 0.1890  | 0.2537  | 0.3432  | 0.2238          |  |
| precision@10     | 0.0318                  | 0.0333 | 0.0333 | 0.0203  | 0.0273  | 0.0383  | 0.0248          |  |
| recall@10        | 0.1418                  | 0.1440 | 0.1408 | 0.0814  | 0.1037  | 0.1566  | 0.0954          |  |
| F1@10            | 0.0466                  | 0.0488 | 0.0483 | 0.0294  | 0.0383  | 0.0549  | 0.0350          |  |
| MRR@10           | 0.2062                  | 0.2038 | 0.2112 | 0.1218  | 0.1775  | 0.2372  | 0.1285          |  |
| NDCG@10          | 0.1388                  | 0.1414 | 0.1431 | 0.0801  | 0.1064  | 0.1533  | 0.0838          |  |
| SRA@10           | 478                     | 489    | 495    | 710     | 681     | 651     | 334             |  |
| SNRA@10          | 261                     | 274    | 282    | 350     | 359     | 338     | 178             |  |
| RCC@10           | 68                      | 71     | 76     | 45      | 59      | 84      | 54              |  |
| NRCC@10          | 51                      | 53     | 51     | 22      | 29      | 40      | 24              |  |
| SRA@K þ RCC@10   | 546                     | 560    | 571    | 755     | 740     | 735     | 388             |  |
| SNRA@K þ NRCC@10 | 312                     | 327    | 333    | 372     | 388     | 378     | 202             |  |

**Table 11**  Wilcoxon significance test results for metrics obtained using 512 word-limited (left) and non-limited chunks (right). Test are performed between Kamradt's approach with window size of three (*w* = 3) and each of our configurations for English articles. P values lower than 0.05 are highlighted in bold, denoting statistical significance.

| K. w3        |       | 512 Word-Limited Chunks |       |         |       |         | Full-length chunks |         |        |         |        |         |
|--------------|-------|-------------------------|-------|---------|-------|---------|--------------------|---------|--------|---------|--------|---------|
|              | n4-m2 |                         | n6-m3 |         | n8-m4 |         | n4-m2              |         | n6-m3  |         | n8-m4  |         |
|              | W     | P_value                 | W     | P_value | W     | P_value | W                  | P_value | W      | P_value | W      | P_value |
| hit_rate@10  | 77.5  | <0.001                  | 252.0 | 0.2367  | 139.5 | 0.0285  | 1050.0             | <0.001  | 480.0  | <0.001  | 780.0  | 0.3621  |
| precision@10 | 126.0 | <0.001                  | 334.5 | 0.1703  | 222.0 | 0.0509  | 1113.0             | <0.001  | 517.5  | <0.001  | 945.0  | 0.0151  |
| recall@10    | 108.5 | <0.001                  | 254.5 | 0.0223  | 260.0 | 0.2504  | 1117.0             | <0.001  | 402.5  | <0.001  | 978.0  | 0.0400  |
| F1@10        | 106.5 | <0.001                  | 266.5 | 0.0333  | 264.0 | 0.2775  | 1074.5             | <0.001  | 371.0  | <0.001  | 929.0  | 0.0202  |
| MRR@10       | 214.5 | <0.001                  | 462.0 | 0.1942  | 617.0 | 0.1981  | 1641.5             | 0.0034  | 1150.0 | <0.001  | 2403.5 | 0.3640  |
| NDCG@10      | 238.0 | <0.001                  | 464.5 | 0.0263  | 715.5 | 0.2783  | 1656.0             | 0.0012  | 1063.5 | <0.001  | 2494.0 | 0.0957  |

**Table 12**  Wilcoxon significance test results for metrics obtained using 512 word-limited (left) and non-limited chunks (right). Test are performed between the static splitter with '\n' as delimiter and each of our configurations for English articles. P values lower than 0.05 are highlighted in bold, denoting statistical significance.

| Static splitter |       | 512 Word-Limited Chunks |       |         |       |         | Full-length chunks |         |        |         |        |         |
|-----------------|-------|-------------------------|-------|---------|-------|---------|--------------------|---------|--------|---------|--------|---------|
|                 | n4-m2 |                         | n6-m3 |         | n8-m4 |         | n4-m2              |         | n6-m3  |         | n8-m4  |         |
|                 | W     | P_value                 | W     | P_value | W     | P_value | W                  | P_value | W      | P_value | W      | P_value |
| hit_rate@10     | 480.0 | 0.3072                  | 348.5 | 0.3428  | 168.0 | <0.001  | 1050.0             | <0.001  | 1404.0 | 0.5688  | 900.0  | 0.0126  |
| precision@10    | 494.5 | 0.1892                  | 380.5 | 0.4667  | 226.5 | <0.001  | 1113.0             | <0.001  | 1790.5 | 0.4402  | 1317.0 | 0.0333  |
| recall@10       | 516.0 | 0.3353                  | 403.0 | 0.7210  | 196.0 | <0.001  | 1117.0             | <0.001  | 1659.0 | 0.2126  | 1300.5 | 0.0440  |
| F1@10           | 508.5 | 0.2991                  | 407.0 | 0.7602  | 197.0 | <0.001  | 1074.5             | <0.001  | 1691.0 | 0.2657  | 1286.5 | 0.0377  |
| MRR@10          | 756.5 | 0.7336                  | 612.0 | 0.0581  | 394.0 | <0.001  | 1641.5             | 0.0034  | 2468.0 | 0.7140  | 1992.0 | 0.0109  |
| NDCG@10         | 753.5 | 0.4293                  | 814.0 | 0.1841  | 429.0 | <0.001  | 1656.0             | 0.0012  | 2862.0 | 0.8039  | 2243.5 | 0.0051  |

**Table 13**  Mean score@10 results for Spanish articles (best values are highlighted in bold).

|                       |                  | Total<br>score@10 sum | Total<br>score@10<br>avg | Total<br>score@10 std |
|-----------------------|------------------|-----------------------|--------------------------|-----------------------|
| Full-length<br>Chunks | Kamradt w<br>¼ 3 | 1072,687.9            | 0.7678                   | 0.3915                |
|                       | n ¼ 4<br>m ¼ 2   | 1036,846.1            | 0.7422                   | 0.4058                |
|                       | n ¼ 6<br>m ¼ 3   | 1101,198.1            | 0.7882                   | 0.3777                |
|                       | n ¼ 8<br>m ¼ 4   | 1128,901.0            | 0.8081                   | 0.3639                |
| 512 Word<br>Limited   | Kamradt w<br>¼ 3 | 1072,161.4            | 0.7674                   | 0.3916                |
| Chunks                | n ¼ 4<br>m ¼ 2   | 1038,877.9            | 0.7436                   | 0.4050                |
|                       | n ¼ 6<br>m ¼ 3   | 1101,651.0            | 0.7885                   | 0.3774                |
|                       | n ¼ 8<br>m ¼ 4   | 1129,416.6            | 0.8084                   | 0.3636                |
|                       |                  |                       |                          |                       |

total score@10 sum and total score@10 average show that our chunking strategy leads to a better retrieval, where the sought chunk is retrieved more often and better ordered within the overall retrieved chunks than when using Kamradt's chunking approach. Moreover, our strategies have a smaller total score@10 standard deviation, providing even more reliability to our results. This is further endorsed by the significance tests

**Table 14**  Wilcoxon significance test results for Score@10 metrics between Kamradt's approach and each of our configurations for Spanish articles using 512 wordlimited (left) and non-limited chunks (right). P values lower than 0.05 are highlighted in bold, denoting statistical significance.

| K. w3          | 512 Word-Limited Chunks |         | Full-Length Chunks |         |  |  |  |
|----------------|-------------------------|---------|--------------------|---------|--|--|--|
|                | W                       | p_value | W                  | p_value |  |  |  |
| n = 4          | 40,666,912,205.00       | 0.00    | 40,443,833,642.50  | 0.00    |  |  |  |
| m = 2<br>n = 6 | 38,388,765,400.50       | 0.00    | 38,442,115,610.50  | 0.00    |  |  |  |
| m = 3<br>n = 8 | 31,678,347,703.00       | 0.00    | 31,478,039,517.00  | 0.00    |  |  |  |
| m = 4          |                         |         |                    |         |  |  |  |

shown in Table 14.

Analogous results are obtained for the articles from the English MIRACL Wikipedia dump in Table 15. In both the *n* = 6; *m* = 3 and *n* = 8; *m* = 4 configurations, a better score@10 sum and average is yielded, outperforming the static splitter, which obtains worse results, as well as any of Kamradt's configurations. Furthermore, the score@10 standard deviation remains smaller in our approach, which lends reliability to our results. Table 16 demonstrates the significance of our results using significance tests. The Wilcoxon test has been computed between each configuration of our strategy and Kamradt's approach with a window size of four sentences, as it is the configuration with better score@10.

## **5. Discussion**

Enhancing retrieval performance is a critical task within any RAG pipeline framework. Deciding how to split the text – or whether to split it at all – significantly impacts subsequent information retrieval. The present study proposes a new semantic splitting technique that overcomes the limitations of Kamradt's strategy and improves the information retrieval phase of a RAG pipeline. Moreover, a rigorous evaluation framework has been proposed and used to determine the impact of splitting strategies on retrieval performance.

Conventional text-splitting approaches often rely on methods that are insensitive to semantic structure, such as character chunking or recursive character splitting. This potentially can result in the creation of text chunks that lack semantic unity and fail to encapsulate a single core concept. As Wu et al. [[68\]](#page--1-0) state, maintaining semantic integrity and ensuring the semantic independence of retrieved passages are critical principles for high-quality RAG architectures. Therefore, relying on these semantically agnostic approaches within a RAG pipeline is a significant limitation. The semantic splitting technique proposed in this study aims to mitigate this weakness by applying a semantic chunking strategy that considers text semantics and overcomes limitations of previous semantic approaches. Kamradt introduced semantic chunking as a promising method for enhancing retrieval performance with his semantic text splitting technique. However, a significant limitation of

**Table 15**  Mean score@10 results for English articles (best values are highlighted in bold).

|                   |                             | Total<br>score@10 sum | Total<br>score@10<br>avg | Total<br>score@10 std |
|-------------------|-----------------------------|-----------------------|--------------------------|-----------------------|
| Full-length       | Kamradt w<br>¼ 2            | 116,157.8             | 0.7941                   | 0.3608                |
| Chunks            | Kamradt w                   | 118,093.4             | 0.8073                   | 0.3531                |
|                   | ¼ 3<br>Kamradt w            | 120,849.4             | 0.8262                   | 0.3382                |
|                   | ¼ 4<br>n ¼ 4                | 115,811.35            | 0.7917                   | 0.3630                |
|                   | m ¼ 2<br>n ¼ 6              | 121,309.92            | 0.8293                   | 0.3390                |
|                   | m ¼ 3<br>n ¼ 8              | 122,932.75            | 0.8404                   | 0.3321                |
|                   | m ¼ 4<br>Static             | 81,795.43             | 0.5592                   | 0.4576                |
| 512 Word          | splitter<br>Kamradt w       | 116,810.10            | 0.7985                   | 0.3567                |
| Limited<br>Chunks | ¼ 2<br>Kamradt w            | 119,230.12            | 0.8151                   | 0.3459                |
|                   | ¼ 3<br>Kamradt w            | 120,935.72            | 0.8268                   | 0.3378                |
|                   | ¼ 4<br>n ¼ 4                | 115,501.91            | 0.7896                   | 0.3648                |
|                   | m ¼ 2<br>n ¼ 6              | 121,245.76            | 0.8289                   | 0.3394                |
|                   | m ¼ 3<br>n ¼ 8              | 122,747.61            | 0.8391                   | 0.3334                |
|                   | m ¼ 4<br>Static<br>splitter | 81,260.89             | 0.5555                   | 0.4587                |

this technique was identified. Including sentences in chunks without considering the meaning of the entire chunk results in a semantically limited text-splitting strategy. Therefore, there is a risk of splitting text into long, non-semantically meaningful chunks due to gradual shifts in meaning. Consequently, the chunks computed using Kamradt's technique may contain irrelevant information, thus leaving the potential of semantic chunking untapped. In contrast, the proposed methodology leverages a fully semantic approach to text splitting, evaluating the semantic coherence of the entire chunk. This nuanced distinction is critical because the presence of irrelevant information in chunks can introduce noise during the generation phase, which may potentially increase hallucinations [6].

The initial hypothesis that Kamradt's strategy results in a longer chunk length is finally substantiated by the statistics shown in Tables 4 and 5. Undoubtedly, the chunks obtained using Kamradt's strategy are longer than those obtained using any of our strategy's configurations. Due to the nature of the evaluation process, however, this will significantly bias the results, since there is a greater probability of MIRACL passages being contained within longer chunks. Consequently, evaluation results obtained for 512 word-limited chunks are most suitable for comparing the performance of both strategies.

To validate the cross-lingual application of the proposed strategy, both Spanish and English Wikipedia articles are used for evaluation. Using Spanish articles, the strategy proposed in this study stands out for its ability to retrieve a higher number of relevant chunks. The Hit rate@K, Recall@K, and F1@K metrics prove this behaviour. Likewise, SRA@K and RCC@K also support the efficacy of our technique by illustrating the retrieval of a higher number of same relevant articles and relevant chunks contained in their corresponding MIRACL article. However, Kamradt's approach performs slightly better for metrics that consider the order of the retrieved documents. The values obtained for the MRR@K and NDCG@K metrics demonstrate this behaviour. Nonetheless, retrieving as many relevant articles as possible is considered more important than ordering them. This is because, later, in a real RAG pipeline, different reranking approaches can be applied. Therefore, if Kamradt's strategy retrieves fewer relevant articles in a better order, performance might be worse, and further improvements, like reranking, would be less effective [69,70].

For English articles, the results demonstrate a consistently higher performance of our approach, despite a general decrease in performance for this language. This degradation may be due to the multilanguage embedding model's inability to fully capture English-specific semantics. Nevertheless, our approach yields demonstrably improved IR metrics compared to Kamradt's optimal configuration, while the static splitter's performance is significantly worse. The SRA@K + RCC@K metric achieves its highest values with *n* = 8; *m* = 4, although the *n* = 4; *m* = 2 configuration yields a higher SRA@K at the expense of RCC@K. Since maximizing the number of relevant chunks contained (RCC@K) is considered more significant and challenging, we identify the *n* = 8; *m* = 4 configuration as superior. Therefore, analysis of IR metrics across both languages reveals that our approach clearly outperforms the established baselines.

Moreover, the evaluation using synthetic questions exhibits clear results regarding the enhancement achieved by our strategy for both English (see Table 13) and Spanish (see Table 15) Wikipedia articles. The splitting strategy proposed in this paper clearly outperforms both Kamradt's strategy and the static splitter. It is imperative to acknowledge that the metric employed for this evaluation (Score@10) is also an order-aware metric. Therefore, these results may differ from the previous ones (see Table 6), as the MRR@K and NDCG@K metrics show slightly superior results for Kamradt's approach in Spanish. However, this discrepancy is attributed to how each metric accentuates the order penalty. As demonstrated in Fig. 5, the value of each metric (y-axis) is plotted against the rank (x-axis) at which the chunk is retrieved. While MRR@K and iDCG@K impose significant penalties on chunks from second to seventh, owing to their logarithmic nature, Score@K adopts a

**Table 16**  Wilcoxon significance test results for Score@10 metrics obtained using non-limited and 512 word-limited chunks between Kamradt's strategy with window size of four sentences (left), and Wilcoxon significance test results for Score@10 metrics obtained for English articles using non-limited and 512 word-limited chunks between Kamradt's strategy and the static splitter (right). P values lower than 0.05 are highlighted in bold, denoting statistical significance.

|                | K. w4                   |         |                    |         | Static splitter         |         |                    |         |
|----------------|-------------------------|---------|--------------------|---------|-------------------------|---------|--------------------|---------|
|                | 512 Word-Limited Chunks |         | Full-Length Chunks |         | 512 Word-Limited Chunks |         | Full-Length Chunks |         |
|                | W                       | p_value | W                  | p_value | W                       | p_value | W                  | p_value |
| n = 4<br>m = 2 | 246,635,446.5           | 0.00    | 249,771,766.0      | 0.00    | 298,548,676.0           | 0.00    | 288,085,928.5      | 0.00    |
| n = 6<br>m = 3 | 195,366,256.0           | 0.00    | 194,281,068.0      | <0.001  | 219,424,592.0           | 0.00    | 219,597,374.0      | 0.00    |
| n = 8<br>m = 4 | 153,193,779.5           | <0.001  | 149,247,366.5      | <0.001  | 209,784,162.0           | 0.00    | 205,896,564.5      | 0.00    |

**Fig. 5.** MRR@K, iDCK@K and Score@K functions.

linear tendency, imposing a considerably smaller penalty on chunks not retrieved in the first position. Nonetheless, it is important to note that Score@K also penalises relevant chunks retrieved from the eighth position onwards compared to iDCG@K. Consequently, this signifies that our chunking strategy sways retrieval by fetching a higher number of sought chunks within the first positions of the ranking. Conversely, chunks built using Kamradt's approach are more likely to be retrieved in the first position; however, a smaller amount of the desired chunks are retrieved since their semantic significance is lower due to the aforementioned limitations.

A thorough statistical analysis of the Score@K metric reveals a significant improvement over the method initially proposed by Kamradt. For the Spanish data, the proposed approach yielded scores of 0.8081 and 0.8084 for full-length and 512 word-limited chunks, respectively. In comparison, Kamradt's method achieved scores of 0.7678 and 0.7674. In English, the proposed approach resulted in scores of 0.8404 and 0.8391 for full-length and 512 word-limited chunks, respectively, while Kamradt's strategy obtained scores of 0.8262 and 0.8268. This improvement results in a 4 % and 2 % increase in the rate of relevant information supplied to the generation phase for Spanish and English, respectively. These findings suggest that the proposed strategy, regardless of the language, increases the probability of providing the generative model with pertinent information, thereby improving the overall RAG pipeline performance.

Finally, regarding the various configurations applied for Kamradt's strategy, a window size of three sentences has been demonstrated to yield more competitive results for IR metrics. However, for the Score@K metric, a window size of four sentences attains superior performance. In the context of our study, the *n* = 8 and *m* = 4 configuration yields better results. However, this is simply because these chunks are larger and the probability of containing MIRACL passages is higher. Therefore, it is important to highlight that, in a real RAG pipeline, the configuration of these parameters will rely on domain-specific needs such as the maximum context length or the expected answer length.

## **6. Conclusions**

Since the inception of Retrieval Augmented Generation, research has focused on improving the reliability of chatbots based on LLMs. Despite the use of diverse models, prompts, and strategies to enhance RAG performance, there is still a huge lack of knowledge on how the text splitting process impacts retrieval performance. This research study analyses state-of-the-art text splitting techniques within the RAG framework. Specially, semantical splitting techniques have been examined, identifying their current limitations and proposing a novel semantical chunking technique.

The proposed strategy successfully addresses limitations from stateof-the-art approaches and enhances retrieval performance by improving the cohesion of the split information, thereby enhancing RAG's performance in the retrieval phase. This milestone has been grounded by the exhaustive evaluation methodology applied in this research. Firstly, our evaluation approach addresses the issue of assessing the implication of the splitting process in the retrieval performance, applying the labels given in an IR dataset for custom chunks. Consequently, an innovative approach has been proposed for leveraging MIRACL labelled passages to obtain well-known IR metrics as well as newly defined ones. Secondly, the evaluation process is concluded by defining a new metric for measuring the retrieval performance by means of synthetic generated questions.

The results of this research have proved that Kamradt's semantic splitting process results in the obtention of longer chunks that seem to lose significant semantic meaning. This phenomenon is evident in the performance of IR metrics where our strategy outperforms Kamradt's one when 512 word-limited chunks are used, as well as with the results obtained for Score@K with synthetic questions. Furthermore, due to the nature of the different order-aware metrics, it can be stated that, when using our splitting approach, the desired chunks are more likely to be retrieved, even if they are not retrieved in the first positions of the score ranking.

Moreover, evaluation using synthetically generated questions clearly demonstrate the efficacy of the proposed approach for improving RAG pipeline performance. The analysis reveals a quantifiable superiority of our approach in Score@K, which leads to a 4 % and a 2 % increase in answer delivery to the generation phase in Spanish and English, respectively, when compared to Kamradt's strategy.

As a result, the proposed chunking strategy offers a pathway to more reliable RAG systems, demonstrating enhanced performance compared to state-of-the-art approaches and exhibiting strong cross-domain and cross-lingual generalizability. By mitigating the generation of inaccurate or hallucinatory responses, this strategy has been demonstrated to improve performance in knowledge-intensive applications. This enhanced reliability is of particular importance in domains where information accuracy is critical, such as healthcare, legal analysis, or aviation maintenance. The broader impact of this work lies in its potential to foster greater trust and utility in RAG-based artificial intelligence, thereby enabling its effective deployment in high-stakes decisionmaking contexts.

Finally, research conducted for the definition of a coherent and solid evaluation framework has revealed a strong bias produced by the length of the chunks. Given the substantial discrepancy in the lengths of Kamradt's chunks relative to those obtained through our strategy, any comparisons drawn between them are inherently imbalanced and potentially misleading. Consequently, when comparing two different chunking strategies, it is imperative to be mindful of the chunk lengths in order to ensure a fair and consistent evaluation.

## **7. Limitations and future work**

Even though promising results were exposed, some limitations about our study have been identified. First and foremost, this chunking methodology does not allow a 'dynamic' chunking threshold, as Kamradt's approach does by using the percentile operation. In our methodology, since the decisions for splitting the text are taken directly while running through the text, it is not possible to apply this operation. Therefore, searching for the optimal threshold used in our strategy might result in a more difficult process, as it might depend on the nature of the text and the embedding model employed.

On the other hand, while the configuration of initial (n) and regular (m) sizes is beneficial for enhancing and optimizing the splitting process, it is necessary to examine which configuration might work best for each specific domain and problem.

Future work will focus on tackling the abovementioned limitations. On the one hand, research about how to automatically determine the optimal threshold might be done. On the other hand, research regarding methods to automatically determine the chunking parameters will be of utmost importance to improve even more the retrieval effectiveness of RAG pipelines.

## **Data availability**

The MIRACL Wikipedia dump used in this research along with many other dumps in different languages are available at the MIRACL GitHub

repository [\(https://github.com/project-miracl/miracl\)](https://github.com/project-miracl/miracl). Specifically, the Spanish and English dumps used in this study can be found at the following links: [https://archive.org/download/eswiki-20220301/es](https://archive.org/download/eswiki-20220301/eswiki-20220301-pages-articles-multistream.xml.bz2)  [wiki-20220301-pages-articles-multistream.xml.bz2](https://archive.org/download/eswiki-20220301/eswiki-20220301-pages-articles-multistream.xml.bz2), [https://archive.](https://archive.org/download/enwiki-20190201/enwiki-20190201-pages-articles-multistream.xml.bz2)  [org/download/enwiki-20190201/enwiki-20190201-pages-articles](https://archive.org/download/enwiki-20190201/enwiki-20190201-pages-articles-multistream.xml.bz2)[multistream.xml.bz2](https://archive.org/download/enwiki-20190201/enwiki-20190201-pages-articles-multistream.xml.bz2), respectively.

## **CRediT authorship contribution statement**

**Antonio Moreno-Cediel:** Writing – original draft, Software, Methodology, Investigation, Conceptualization. **Eva Garcia-Lopez:** Writing – original draft, Supervision, Project administration, Investigation, Funding acquisition. **Antonio Garcia-Cabot:** Writing – original draft, Supervision, Resources, Project administration, Investigation, Funding acquisition. **David De-Fitero-Dominguez:** Writing – review & editing, Methodology, Investigation.

## **Declaration of competing interest**

The authors declare that they have no known competing financial interests or personal relationships that could have appeared to influence the work reported in this paper.

## **Acknowledgements**

This work was supported by the project "Tecnologías Inteligentes para la Fabricacion, el dise ´ no y las Operaciones en entornos iNdustri ˜ ales" (TIFON, PLEC2023-010251) through the call Proyectos de *I* + *D* + *i*  en líneas estrat´egicas - Transmisiones 2023.

## **References**

- [1] A. Vaswani, N. Shazeer, N. Parmar, J. Uszkoreit, L. Jones, A.N. Gomez, L. Kaiser, I. Polosukhin, Attention is all you need, (2023). [https://doi.org/10.48550/arXiv.1](https://doi.org/10.48550/arXiv.1706.03762) [706.03762](https://doi.org/10.48550/arXiv.1706.03762).
- [2] E.Y. Zhang, A.D. Cheok, Z. Pan, J. Cai, Y. Yan, From Turing to transformers: a comprehensive review and tutorial on the evolution and applications of generative transformer models, Science 5 (2023) 46, [https://doi.org/10.3390/sci5040046.](https://doi.org/10.3390/sci5040046)
- [3] I. Akermi, J. Heinecke, F. Herledan, Transformer based natural language generation for question-answering, in: B. Davis, Y. Graham, J. Kelleher, Y. Sripada (Eds.), Proc. 13th Int. Conf. Nat. Lang. Gener, Association for Computational Linguistics, Dublin, Ireland, 2020, pp. 349–359, [https://doi.org/10.18653/v1/](https://doi.org/10.18653/v1/2020.inlg-1.41) [2020.inlg-1.41](https://doi.org/10.18653/v1/2020.inlg-1.41).
- [4] P. Lewis, E. Perez, A. Piktus, F. Petroni, V. Karpukhin, N. Goyal, H. Küttler, M. Lewis, W. Yih, T. Rocktaschel, S. Riedel, D. Kiela, Retrieval-augmented generation ¨ for knowledge-intensive NLP tasks, (2021). [https://doi.org/10.48550/arXiv.2005.](https://doi.org/10.48550/arXiv.2005.11401)  [11401.](https://doi.org/10.48550/arXiv.2005.11401)
- [5] Y. Zhang, Y. Li, L. Cui, D. Cai, L. Liu, T. Fu, X. Huang, E. Zhao, Y. Zhang, Y. Chen, L. Wang, A.T. Luu, W. Bi, F. Shi, S. Shi, Siren's song in the AI ocean: a survey on hallucination in large language models, (2023). [https://doi.org/10.48550/arXiv.2](https://doi.org/10.48550/arXiv.2309.01219)  [309.01219](https://doi.org/10.48550/arXiv.2309.01219).
- [6] F. Shi, X. Chen, K. Misra, N. Scales, D. Dohan, E. Chi, N. Scharli, D. Zhou, Large ¨ language models can be easily distracted by irrelevant context, (2023). [https://doi.](https://doi.org/10.48550/arXiv.2302.00093)  [org/10.48550/arXiv.2302.00093](https://doi.org/10.48550/arXiv.2302.00093).
- [7] O. Yoran, T. Wolfson, O. Ram, J. Berant, Making retrieval-augmented language models robust to irrelevant context, (2024). [https://doi.org/10.48550/arXiv.2310.](https://doi.org/10.48550/arXiv.2310.01558)  [01558.](https://doi.org/10.48550/arXiv.2310.01558)
- [8] S. Setty, K. Jijo, E. Chung, N. Vidra, Improving retrieval for RAG based question answering models on financial documents, (2024). [https://doi.org/10.4855](https://doi.org/10.48550/arXiv.2404.07221) [0/arXiv.2404.07221.](https://doi.org/10.48550/arXiv.2404.07221)
- [9] RetrievalTutorials/tutorials/LevelsOfTextSplitting/5\_Levels\_Of\_Text\_Splitting. Ipynb at main ⋅ FullStackRetrieval-com/RetrievalTutorials, GitHub (n.d.). [https://](https://github.com/FullStackRetrieval-com/RetrievalTutorials/blob/main/tutorials/LevelsOfTextSplitting/5_Levels_Of_Text_Splitting.ipynb)  [github.com/FullStackRetrieval-com/RetrievalTutorials/blob/main/tutorials/Level](https://github.com/FullStackRetrieval-com/RetrievalTutorials/blob/main/tutorials/LevelsOfTextSplitting/5_Levels_Of_Text_Splitting.ipynb)  [sOfTextSplitting/5\\_Levels\\_Of\\_Text\\_Splitting.ipynb](https://github.com/FullStackRetrieval-com/RetrievalTutorials/blob/main/tutorials/LevelsOfTextSplitting/5_Levels_Of_Text_Splitting.ipynb) (accessed September 3, 2024).
- [10] P. Verma, S2 Chunking: a hybrid framework for document segmentation through integrated spatial and semantic analysis, (2025). [https://doi.org/10.4855](https://doi.org/10.48550/arXiv.2501.05485)  [0/arXiv.2501.05485.](https://doi.org/10.48550/arXiv.2501.05485)
- [11] X. Zhang, N. Thakur, O. Ogundepo, E. Kamalloo, D. Alfonso-Hermelo, X. Li, Q. Liu, M. Rezagholizadeh, J. Lin, Making a MIRACL: multilingual information retrieval across a continuum of languages, (2022). [https://doi.org/10.48550/arXiv.2210.](https://doi.org/10.48550/arXiv.2210.09984)  [09984](https://doi.org/10.48550/arXiv.2210.09984).
- [12] [G.G. Chowdhury, Introduction to Modern Information Retrieval, Facet publishing,](http://refhub.elsevier.com/S0950-7051(25)01934-3/sbref0012)  [2010](http://refhub.elsevier.com/S0950-7051(25)01934-3/sbref0012).
- [13] G. Chowdhury, Introduction to modern information retrieval. [http://www.lapw](http://www.lapwing.org.uk/cgi-bin/miva?lap/merchant.mv+Screen=PROD&tnqh_x0026;Store_Code=1&tnqh_x0026;Product_Code=489)  [ing.org.uk/cgi-bin/miva?lap/merchant.mv](http://www.lapwing.org.uk/cgi-bin/miva?lap/merchant.mv+Screen=PROD&tnqh_x0026;Store_Code=1&tnqh_x0026;Product_Code=489)+Screen=PROD&Store\_Code=1&Prod [uct\\_Code](http://www.lapwing.org.uk/cgi-bin/miva?lap/merchant.mv+Screen=PROD&tnqh_x0026;Store_Code=1&tnqh_x0026;Product_Code=489)=489, 2004 accessed October 17, 2024.

- [14] [M. Taube, C.D. Gull, I.S. Wachtel, Unit terms in coordinate indexing, Am. Doc 3](http://refhub.elsevier.com/S0950-7051(25)01934-3/sbref0014)  [\(1952\) 213. Pre-1986.](http://refhub.elsevier.com/S0950-7051(25)01934-3/sbref0014)
- [15] [G. Salton, Automatic Text processing: the transformation, analysis, and Retrieval of](http://refhub.elsevier.com/S0950-7051(25)01934-3/sbref0015)  [Information By Computer, Addison-Wesley Longman Publishing Co., Inc., USA,](http://refhub.elsevier.com/S0950-7051(25)01934-3/sbref0015)  [1989.](http://refhub.elsevier.com/S0950-7051(25)01934-3/sbref0015)
- [16] H.P. Luhn, A statistical approach to mechanized encoding and searching of literary information, IBM J. Res. Dev. 1 (1957) 309–317, [https://doi.org/10.1147/](https://doi.org/10.1147/rd.14.0309) [rd.14.0309](https://doi.org/10.1147/rd.14.0309).
- [17] K. Sparck Jones, A statistical interpretation of term specificity and its application in retrieval, J. Doc. 28 (1972) 11–21, [https://doi.org/10.1108/eb026526.](https://doi.org/10.1108/eb026526)
- [18] S.E. Robertson, K.S. Jones, Relevance weighting of search terms, J. Am. Soc. Inf. Sci. 27 (1976). [https://www.proquest.com/docview/1301247214/citation/F](https://www.proquest.com/docview/1301247214/citation/F28CDD11919C45EEPQ/1) [28CDD11919C45EEPQ/1.](https://www.proquest.com/docview/1301247214/citation/F28CDD11919C45EEPQ/1) accessed October 21, 2024.
- [19] S. Robertson, H. Zaragoza, The Probabilistic Relevance Framework: BM25 and beyond, Found. Trends® Inf. Retr. 3 (2009) 333–389, [https://doi.org/10.1561/](https://doi.org/10.1561/1500000019)  [1500000019](https://doi.org/10.1561/1500000019).
- [20] [G. Salton, Developments in automatic text retrieval, Science 253 \(1991\) 974](http://refhub.elsevier.com/S0950-7051(25)01934-3/sbref0020)–980.
- [21] G. Salton, C. Buckley, Flexible Text Matching For Information Retrieval, Cornell University, 1990.<https://hdl.handle.net/1813/6998>. accessed October 21, 2024.
- [22] [G. Salton, C. Buckley, Global text matching for information retrieval, Science 253](http://refhub.elsevier.com/S0950-7051(25)01934-3/sbref0022)  [\(1991\) 1012](http://refhub.elsevier.com/S0950-7051(25)01934-3/sbref0022)–1015.
- [23] T. Mikolov, K. Chen, G. Corrado, J. Dean, Efficient estimation of word representations in vector space, (2013). [https://doi.org/10.48550/arXiv.1301.378](https://doi.org/10.48550/arXiv.1301.3781)  [1.](https://doi.org/10.48550/arXiv.1301.3781)
- [24] J. Pennington, R. Socher, C. Manning, GloVe: global vectors for word representation, in: A. Moschitti, B. Pang, W. Daelemans (Eds.), Proc. 2014 Conf. Empir. Methods Nat. Lang. Process. EMNLP, Association for Computational Linguistics, Doha, Qatar, 2014, pp. 1532–1543, [https://doi.org/10.3115/v1/D14-](https://doi.org/10.3115/v1/D14-1162)  [1162.](https://doi.org/10.3115/v1/D14-1162)
- [25] [M.T. Pilehvar, J. Camacho-Collados, Embeddings in Natural Language processing:](http://refhub.elsevier.com/S0950-7051(25)01934-3/sbref0025)  [Theory and Advances in Vector Representations of Meaning, Morgan](http://refhub.elsevier.com/S0950-7051(25)01934-3/sbref0025) & Claypool [Publishers, 2020](http://refhub.elsevier.com/S0950-7051(25)01934-3/sbref0025).
- [26] O. Melamud, J. Goldberger, I. Dagan, context2vec: learning generic context embedding with bidirectional LSTM, in: S. Riezler, Y. Goldberg (Eds.), Proc. 20th SIGNLL Conf. Comput. Nat. Lang. Learn., Association for Computational Linguistics, Berlin, Germany, 2016, pp. 51–61, [https://doi.org/10.18653/v1/K16-](https://doi.org/10.18653/v1/K16-1006)  [1006.](https://doi.org/10.18653/v1/K16-1006)
- [27] M.E. Peters, M. Neumann, M. Iyyer, M. Gardner, C. Clark, K. Lee, L. Zettlemoyer, Deep contextualized word representations, (2018). [https://doi.org/10.4855](https://doi.org/10.48550/arXiv.1802.05365) [0/arXiv.1802.05365.](https://doi.org/10.48550/arXiv.1802.05365)
- [28] J. Devlin, M.-W. Chang, K. Lee, K. Toutanova, BERT: pre-training of deep bidirectional transformers for language understanding, (2019). [https://doi.org/10.](https://doi.org/10.48550/arXiv.1810.04805)  [48550/arXiv.1810.04805](https://doi.org/10.48550/arXiv.1810.04805).
- [29] N. Reimers, I. Gurevych, Sentence-BERT: sentence embeddings using siamese BERT-networks, (2019). [https://doi.org/10.48550/arXiv.1908.10084.](https://doi.org/10.48550/arXiv.1908.10084)
- [30] A. Dubey, A. Jauhri, A. Pandey, A. Kadian, A. Al-Dahle, A. Letman, A. Mathur, A. Schelten, A. Yang, A. Fan, A. Goyal, A. Hartshorn, A. Yang, A. Mitra, A. Sravankumar, A. Korenev, A. Hinsvark, A. Rao, A. Zhang, A. Rodriguez, A. Gregerson, A. Spataru, B. Roziere, B. Biron, B. Tang, B. Chern, C. Caucheteux, C. Nayak, C. Bi, C. Marra, C. McConnell, C. Keller, C. Touret, C. Wu, C. Wong, C.C. Ferrer, C. Nikolaidis, D. Allonsius, D. Song, D. Pintz, D. Livshits, D. Esiobu, D. Choudhary, D. Mahajan, D. Garcia-Olano, D. Perino, D. Hupkes, E. Lakomkin, E. AlBadawy, E. Lobanova, E. Dinan, E.M. Smith, F. Radenovic, F. Zhang, G. Synnaeve, G. Lee, G.L. Anderson, G. Nail, G. Mialon, G. Pang, G. Cucurell, H. Nguyen, H. Korevaar, H. Xu, H. Touvron, I. Zarov, I.A. Ibarra, I. Kloumann, I. Misra, I. Evtimov, J. Copet, J. Lee, J. Geffert, J. Vranes, J. Park, J. Mahadeokar, J. Shah, J. van der Linde, J. Billock, J. Hong, J. Lee, J. Fu, J. Chi, J. Huang, J. Liu, J. Wang, J. Yu, J. Bitton, J. Spisak, J. Park, J. Rocca, J. Johnstun, J. Saxe, J. Jia, K.V. Alwala, K. Upasani, K. Plawiak, K. Li, K. Heafield, K. Stone, K. El-Arini, K. Iyer, K. Malik, K. Chiu, K. Bhalla, L. Rantala-Yeary, L. van der Maaten, L. Chen, L. Tan, L. Jenkins, L. Martin, L. Madaan, L. Malo, L. Blecher, L. Landzaat, L. de Oliveira, M. Muzzi, M. Pasupuleti, M. Singh, M. Paluri, M. Kardas, M. Oldham, M. Rita, M. Pavlova, M. Kambadur, M. Lewis, M. Si, M.K. Singh, M. Hassan, N. Goyal, N. Torabi, N. Bashlykov, N. Bogoychev, N. Chatterji, O. Duchenne, O. Çelebi, P. Alrassy, P. Zhang, P. Li, P. Vasic, P. Weng, P. Bhargava, P. Dubal, P. Krishnan, P.S. Koura, P. Xu, Q. He, Q. Dong, R. Srinivasan, R. Ganapathy, R. Calderer, R.S. Cabral, R. Stojnic, R. Raileanu, R. Girdhar, R. Patel, R. Sauvestre, R. Polidoro, R. Sumbaly, R. Taylor, R. Silva, R. Hou, R. Wang, S. Hosseini, S. Chennabasappa, S. Singh, S. Bell, S.S. Kim, S. Edunov, S. Nie, S. Narang, S. Raparthy, S. Shen, S. Wan, S. Bhosale, S. Zhang, S. Vandenhende, S. Batra, S. Whitman, S. Sootla, S. Collot, S. Gururangan, S. Borodinsky, T. Herman, T. Fowler, T. Sheasha, T. Georgiou, T. Scialom, T. Speckbacher, T. Mihaylov, T. Xiao, U. Karn, V. Goswami, V. Gupta, V. Ramanathan, V. Kerkez, V. Gonguet, V. Do, V. Vogeti, V. Petrovic, W. Chu, W. Xiong, W. Fu, W. Meers, X. Martinet, X. Wang, X.E. Tan, X. Xie, X. Jia, X. Wang, Y. Goldschlag, Y. Gaur, Y. Babaei, Y. Wen, Y. Song, Y. Zhang, Y. Li, Y. Mao, Z.D. Coudert, Z. Yan, Z. Chen, Z. Papakipos, A. Singh, A. Grattafiori, A. Jain, A. Kelsey, A. Shajnfeld, A. Gangidi, A. Victoria, A. Goldstand, A. Menon, A. Sharma, A. Boesenberg, A. Vaughan, A. Baevski, A. Feinstein, A. Kallet, A. Sangani, A. Yunus, A. Lupu, A. Alvarado, A. Caples, A. Gu, A. Ho, A. Poulton, A. Ryan, A. Ramchandani, A. Franco, A. Saraf, A. Chowdhury, A. Gabriel, A. Bharambe, A. Eisenman, A. Yazdan, B. James, B. Maurer, B. Leonhardi, B. Huang, B. Loyd, B.D. Paola, B. Paranjape, B. Liu, B. Wu, B. Ni, B. Hancock, B. Wasti, B. Spence, B. Stojkovic, B. Gamido, B. Montalvo, C. Parker, C. Burton, C. Mejia, C. Wang, C. Kim, C. Zhou, C. Hu, C.-H. Chu, C. Cai, C. Tindal, C. Feichtenhofer, D. Civin, D. Beaty, D. Kreymer, D. Li, D. Wyatt, D. Adkins, D. Xu, D. Testuggine, D. David, D. Parikh, D. Liskovich, D. Foss, D. Wang, D. Le, D. Holland, E. Dowling, E. Jamil, E.

- Montgomery, E. Presani, E. Hahn, E. Wood, E. Brinkman, E. Arcaute, E. Dunbar, E. Smothers, F. Sun, F. Kreuk, F. Tian, F. Ozgenel, F. Caggioni, F. Guzman, F. Kanayet, ´ F. Seide, G.M. Florez, G. Schwarz, G. Badeer, G. Swee, G. Halpern, G. Thattai, G. Herman, G. Sizov, Guangyi, Z., G. Lakshminarayanan, H. Shojanazeri, H. Zou, H. Wang, H. Zha, H. Habeeb, H. Rudolph, H. Suk, H. Aspegren, H. Goldman, I. Damlaj, I. Molybog, I. Tufanov, I.-E. Veliche, I. Gat, J. Weissman, J. Geboski, J. Kohli, J. Asher, J.-B. Gaya, J. Marcus, J. Tang, J. Chan, J. Zhen, J. Reizenstein, J. Teboul, J. Zhong, J. Jin, J. Yang, J. Cummings, J. Carvill, J. Shepard, J. McPhie, J. Torres, J. Ginsburg, J. Wang, K. Wu, KH U., K. Saxena, K. Prasad, K. Khandelwal, K. Zand, K. Matosich, K. Veeraraghavan, K. Michelena, K. Li, K. Huang, K. Chawla, K. Lakhotia, K. Huang, L. Chen, L. Garg, L. A., L. Silva, L. Bell, L. Zhang, L. Guo, L. Yu, L. Moshkovich, L. Wehrstedt, M. Khabsa, M. Avalani, M. Bhatt, M. Tsimpoukelli, M. Mankus, M. Hasson, M. Lennie, M. Reso, M. Groshev, M. Naumov, M. Lathi, M. Keneally, M.L. Seltzer, M. Valko, M. Restrepo, M. Patel, M. Vyatskov, M. Samvelyan, M. Clark, M. Macey, M. Wang, M.J. Hermoso, M. Metanat, M. Rastegari, M. Bansal, N. Santhanam, N. Parks, N. White, N. Bawa, N. Singhal, N. Egebo, N. Usunier, N.P. Laptev, N. Dong, N. Zhang, N. Cheng, O. Chernoguz, O. Hart, O. Salpekar, O. Kalinli, P. Kent, P. Parekh, P. Saab, P. Balaji, P. Rittner, P. Bontrager, P. Roux, P. Dollar, P. Zvyagina, P. Ratanchandani, P. Yuvraj, Q. Liang, R. Alao, R. Rodriguez, R. Ayub, R. Murthy, R. Nayani, R. Mitra, R. Li, R. Hogan, R. Battey, R. Wang, R. Maheswari, R. Howes, R. Rinott, S.J. Bondu, S. Datta, S. Chugh, S. Hunt, S. Dhillon, S. Sidorov, S. Pan, S. Verma, S. Yamamoto, S. Ramaswamy, S. Lindsay, S. Lindsay, S. Feng, S. Lin, S.C. Zha, S. Shankar, S. Zhang, S. Zhang, S. Wang, S. Agarwal, S. Sajuyigbe, S. Chintala, S. Max, S. Chen, S. Kehoe, S. Satterfield, S. Govindaprasad, S. Gupta, S. Cho, S. Virk, S. Subramanian, S. Choudhury, S. Goldman, T. Remez, T. Glaser, T. Best, T. Kohler, T. Robinson, T. Li, T. Zhang, T. Matthews, T. Chou, T. Shaked, V. Vontimitta, V. Ajayi, V. Montanez, V. Mohan, V.S. Kumar, V. Mangla, V. Albiero, V. Ionescu, V. Poenaru, V.T. Mihailescu, V. Ivanov, W. Li, W. Wang, W. Jiang, W. Bouaziz, W. Constable, X. Tang, X. Wang, X. Wu, X. Wang, X. Xia, X. Wu, X. Gao, Y. Chen, Y. Hu, Y. Jia, Y. Qi, Y. Li, Y. Zhang, Y. Zhang, Y. Adi, Y. Nam, Yu, W., Y. Hao, Y. Qian, Y. He, Z. Rait, Z. DeVito, Z. Rosnbrick, Z. Wen, Z. Yang, Z. Zhao, The Llama 3 Herd of models, (2024). [https://doi.org/10.48550/arXiv.2407.21783.](https://doi.org/10.48550/arXiv.2407.21783)
- [31] A. Pal, L.K. Umapathi, M. Sankarasubbu, Med-HALT: medical domain hallucination test for large language models, (2023). [https://doi.org/10.48550/arXiv.2307.15](https://doi.org/10.48550/arXiv.2307.15343)  [343.](https://doi.org/10.48550/arXiv.2307.15343)
- [32] Z. Zhang, C. Wang, Y. Wang, E. Shi, Y. Ma, W. Zhong, J. Chen, M. Mao, Z. Zheng, LLM hallucinations in practical code generation: phenomena, mechanism, and mitigation, Proc. ACM Softw. Eng. 2 (2025), <https://doi.org/10.1145/3728894>. ISSTA022:481-ISSTA022:503.
- [33] A Survey on Hallucination in Large Language Models: Principles, Taxonomy, challenges, and open questions | ACM transactions on information systems, (n.d.). <https://dl.acm.org/doi/full/10.1145/3703155> (accessed September 15, 2025).
- [34] Z. Gekhman, G. Yona, R. Aharoni, M. Eyal, A. Feder, R. Reichart, J. Herzig, Does fine-tuning LLMs on new knowledge encourage hallucinations? in: Y. Al-Onaizan, M. Bansal, Y.-N. Chen (Eds.), Proc. 2024 Conf. Empir. Methods Nat. Lang. Process Association for Computational Linguistics, Miami, Florida, USA, 2024, pp. 7765–7784, <https://doi.org/10.18653/v1/2024.emnlp-main.444>.
- [35] Y. Gao, Y. Xiong, X. Gao, K. Jia, J. Pan, Y. Bi, Y. Dai, J. Sun, M. Wang, H. Wang, Retrieval-augmented generation for large language models: a survey, (2024). [htt](https://doi.org/10.48550/arXiv.2312.10997) [ps://doi.org/10.48550/arXiv.2312.10997.](https://doi.org/10.48550/arXiv.2312.10997)
- [36] T.B. Brown, B. Mann, N. Ryder, M. Subbiah, J. Kaplan, P. Dhariwal, A. Neelakantan, P. Shyam, G. Sastry, A. Askell, S. Agarwal, A. Herbert-Voss, G. Krueger, T. Henighan, R. Child, A. Ramesh, D.M. Ziegler, J. Wu, C. Winter, C. Hesse, M. Chen, E. Sigler, M. Litwin, S. Gray, B. Chess, J. Clark, C. Berner, S. McCandlish, A. Radford, I. Sutskever, D. Amodei, Language models are few-shot learners, (2020). [https://doi.org/10.48550/arXiv.2005.14165.](https://doi.org/10.48550/arXiv.2005.14165)
- [37] K. Luo, Z. Liu, S. Xiao, K. Liu, BGE landmark embedding: a chunking-free embedding method for retrieval augmented long-context large language models, (2024). [https://doi.org/10.48550/arXiv.2402.11573.](https://doi.org/10.48550/arXiv.2402.11573)
- [38] S. Chen, S. Wong, L. Chen, Y. Tian, Extending context window of large language models via positional interpolation, (2023). [https://doi.org/10.48550/arXiv.2306.](https://doi.org/10.48550/arXiv.2306.15595)  [15595](https://doi.org/10.48550/arXiv.2306.15595).
- [39] Z. Zhong, H. Liu, X. Cui, X. Zhang, Z. Qin, Mix-of-granularity: optimize the chunking granularity for retrieval-augmented generation, (2024). [https://doi.](https://doi.org/10.48550/arXiv.2406.00456) [org/10.48550/arXiv.2406.00456.](https://doi.org/10.48550/arXiv.2406.00456)
- [40] A.J. Yepes, Y. You, J. Milczek, S. Laverde, R. Li, Financial report chunking for effective retrieval augmented generation, (2024). [https://doi.org/10.4855](https://doi.org/10.48550/arXiv.2402.05131) [0/arXiv.2402.05131.](https://doi.org/10.48550/arXiv.2402.05131)
- [41] N. Muennighoff, N. Tazi, L. Magne, N. Reimers, MTEB: massive text embedding benchmark, (2023). [https://doi.org/10.48550/arXiv.2210.07316.](https://doi.org/10.48550/arXiv.2210.07316)
- [42] S. Sturua, I. Mohr, M.K. Akram, M. Günther, B. Wang, M. Krimmel, F. Wang, G. Mastrapas, A. Koukounas, N. Wang, H. Xiao, jina-embeddings-v3: multilingual embeddings with Task LoRA, (2024). [https://doi.org/10.48550/arXiv.2409.](https://doi.org/10.48550/arXiv.2409.10173)  [10173](https://doi.org/10.48550/arXiv.2409.10173).
- [43] C. Lee, R. Roy, M. Xu, J. Raiman, M. Shoeybi, B. Catanzaro, W. Ping, NV-Embed: improved techniques for training LLMs as generalist embedding models, (2024). [https://doi.org/10.48550/arXiv.2405.17428.](https://doi.org/10.48550/arXiv.2405.17428)
- [44] dunzhang/stella\_en\_1.5B\_v5 ⋅ Hugging Face, n.d. [https://huggingface.co/dunzhan](https://huggingface.co/dunzhang/stella_en_1.5B_v5)  [g/stella\\_en\\_1.5B\\_v5](https://huggingface.co/dunzhang/stella_en_1.5B_v5), 2024. accessed October 28
- [45] Lajavaness/bilingual-embedding-large ⋅ Hugging Face, n.d. [https://huggingface.](https://huggingface.co/Lajavaness/bilingual-embedding-large)  [co/Lajavaness/bilingual-embedding-large,](https://huggingface.co/Lajavaness/bilingual-embedding-large) 2024. accessed November 22
- [46] R. Pope, S. Douglas, A. Chowdhery, J. Devlin, J. Bradbury, A. Levskaya, J. Heek, K. Xiao, S. Agrawal, J. Dean, Efficiently scaling transformer inference, (2022). [htt](https://doi.org/10.48550/arXiv.2211.05102)  [ps://doi.org/10.48550/arXiv.2211.05102.](https://doi.org/10.48550/arXiv.2211.05102)

- [47] W. Kwon, Z. Li, S. Zhuang, Y. Sheng, L. Zheng, C.H. Yu, J. Gonzalez, H. Zhang, I. Stoica, Efficient memory management for large language model serving with paged attention, in: Proc. 29th Symp. Oper. Syst. Princ., Association for Computing Machinery, New York, NY, USA, 2023, pp. 611–626, https://doi.org/10.1145/3600006.3613165.
- [48] K. Guu, K. Lee, Z. Tung, P. Pasupat, M. Chang, Retrieval augmented language model pre-training, in: Proc. 37th Int. Conf. Mach. Learn., PMLR, 2020, pp. 3929–3938, in: https://proceedings.mlr.press/v119/guu20a.html. accessed July 19, 2024
- [49] S. Borgeaud, A. Mensch, J. Hoffmann, T. Cai, E. Rutherford, K. Millican, G. van den Driessche, J.-B. Lespiau, B. Damoc, A. Clark, D. de L Casas, A. Guy, J. Menick, R. Ring, T. Hennigan, S. Huang, L. Maggiore, C. Jones, A. Cassirer, A. Brock, M. Paganini, G. Irving, O. Vinyals, S. Osindero, K. Simonyan, J.W. Rae, E. Elsen, L. Sifre, Improving language models by retrieving from trillions of tokens, (2022). htt ps://doi.org/10.48550/arXiv.2112.04426.
- [50] O. Rubin, J. Berant, Retrieval-pretrained transformer: long-range language modeling with self-retrieval, (2024). https://doi.org/10.48550/arXiv.2306.13421.
- [51] LangChain, n.d. https://www.langchain.com/, 2024. accessed July 24
- [52] LlamaIndex, Data framework for LLM applications, n.d. https://www.llamaindex. ai/, 2024. accessed July 24
- [53] langchain\_text\_splitters.base.TextSplitter LangChain 0.2.10, (n.d.). https://api. python.langchain.com/en/latest/base/langchain\_text\_splitters.base.TextSplitter. html#langchain\_text\_splitters.base.TextSplitter (accessed July 24, 2024).
- [54] langchain text splitters.character.CharacterTextSplitter LangChain 0.2.10, (n. d.). https://api.python.langchain.com/en/latest/character/langchain\_text\_sp litters.character.CharacterTextSplitter.html#langchain\_text\_splitters.character.CharacterTextSplitter (accessed July 24, 2024).
- [55] langchain\_text\_splitters.character.RecursiveCharacterTextSplitter LangChain 0.2.10, (n.d.). https://api.python.langchain.com/en/latest/character/langchain\_text\_splitters.character.RecursiveCharacterTextSplitter.html#langchain\_text\_splitters.character,RecursiveCharacterTextSplitter (accessed July 24, 2024).
- [56] Split by HTML section | LangChain, (n.d.). https://python.langchain.com/v0.1/do cs/modules/data\_connection/document\_transformers/HTML\_section\_aware\_splitt er/(accessed September 3, 2024).
- [57] langchain\_text\_splitters.markdown.MarkdownTextSplitter LangChain 0.2.10, (n. d.). https://api.python.langchain.com/en/latest/markdown/langchain\_text\_splitters.markdown.MarkdownTextSplitter.html#langchain\_text\_splitters.markdown.MarkdownTextSplitter (accessed July 24, 2024).

- [58] langchain\_text\_splitters.python.PythonCodeTextSplitter LangChain 0.2.10, (n. d.). https://api.python.langchain.com/en/latest/python/langchain\_text\_splitters.python.PythonCodeTextSplitter.html#langchain\_text\_splitters.python.PythonCodeTextSplitter (accessed July 24, 2024).
- [59] M.H. Hadid, Z.T. Al-Qaysi, Q.M. Hussein, R.A. Aljanabi, I.R. Abdulqader, M. S. Suzani, W.L. Shir, Semantic image retrieval analysis based on deep learning and singular value decomposition, Appl. Data Sci. Anal. 2024 (2024) 17–31, https://doi.org/10.58496/ADSA/2024/003.
- [60] M. Sallam, M.A. Shnan, Enhancing semantic image retrieval using self-supervised learning: a label-efficient approach, Babylon. J. Mach. Learn. 2025 (2025) 42–60, https://doi.org/10.58496/BJML/2025/004.
- [61] S. Sivasothy, S. Barnett, S. Kurniawan, Z. Rasool, R. Vasa, RAGProbe: an automated approach for evaluating RAG applications, (2024). https://doi.org/10.4855 0/arXiv.2409.19019.
- [62] G. Attardi, WikiExtractor, GitHub repos. https://github.com/attardi/wikiextractor, 2015.
- [63] M. Sanderson, Test collection based evaluation of information retrieval systems, Found. Trends® Inf. Retr. 4 (2010) 247–375, https://doi.org/10.1561/ 150000009
- [64] R.K. Hamad, Integrating machine learning and genetic algorithms to enhance genedisease classification: an XBNet-based framework, Babylon. J. Mach. Learn. 2025 (2025) 1–12, https://doi.org/10.58496/BJML/2025/001.
- [65] E. Bassani, AmenRa/ranx. https://github.com/AmenRa/ranx, 2024 accessed November 4, 2024.
- [66] K. Järvelin, J. Kekäläinen, Cumulated gain-based evaluation of IR techniques, ACM Trans. Inf. Syst. 20 (2002) 422–446, https://doi.org/10.1145/582415.582418.
- [67] Z. Rackauckas, A. Câmara, J. Zavrel, Evaluating RAG-Fusion with RAGElo: an automated Elo-based framework, (2024). https://doi.org/10.48550/arXiv.2406 14782
- [68] S. Wu, Y. Xiong, Y. Cui, H. Wu, C. Chen, Y. Yuan, L. Huang, X. Liu, T.-W. Kuo, N. Guan, C.J. Xue, Retrieval-augmented generation for natural language processing: a survey, (2025). https://doi.org/10.48550/arXiv.2407.13193.
- [69] T. Niu, S. Joty, Y. Liu, C. Xiong, Y. Zhou, S. Yavuz, JudgeRank: leveraging large language models for reasoning-intensive reranking, (2024). https://doi.org/10. 48550/arXiv.2411.00142.
- [70] M. Mukherjee, S. Kim, X. Chen, D. Luo, T. Yu, T. Mai, From documents to dialogue: building KG-RAG enhanced AI assistants, (2025). https://doi.org/10.4855 0/arXiv.2502.15237.