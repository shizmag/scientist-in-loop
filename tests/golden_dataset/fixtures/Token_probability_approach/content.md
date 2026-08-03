# Detecting Hallucinations in Large Language Model Generation: A Token Probability Approach

Ernesto Quevedo [ID](https://orcid.org/0000-0002-8938-2230) Department of Computer Science School of Eng. & Computer Science Baylor University Email: Ernesto Quevedo1@Baylor.edu

Pablo Rivas [ID](https://orcid.org/0000-0002-8690-0987) , *Senior, IEEE* Department of Computer Science School of Engineering & Computer Science Baylor University Email: Pablo Rivas@Baylor.edu

Jorge Yero Salazar [ID](https://orcid.org/0000-0002-5033-4805) Department of Computer Science School of Eng. & Computer Science Baylor University Email: Jorge Yero1@Baylor.edu

Rachel Koerner [ID](https://orcid.org/0009-0009-5294-4349) Department of Computer Science School of Eng. & Computer Science Baylor University Email: Rachel Koerner1@Baylor.edu

Tomas Cerny [ID](https://orcid.org/0000-0002-5882-5502) Department of Systems & Industrial Engineering College of Engineering The University of Arizona Email: Tomas Cerny@Baylor.edu

*Abstract*—Concerns regarding the propensity of Large Language Models (LLMs) to produce inaccurate outputs, also known as hallucinations, have escalated. Detecting them is vital for ensuring the reliability of applications relying on LLM-generated content. Current methods often demand substantial resources and rely on extensive LLMs or employ supervised learning with multidimensional features or intricate linguistic and semantic analyses difficult to reproduce and largely depend on using the same LLM that hallucinated. This paper introduces a supervised learning approach employing two simple classifiers utilizing only four numerical features derived from tokens and vocabulary probabilities obtained from other LLM evaluators, which are not necessarily the same. The method yields promising results, surpassing state-of-the-art outcomes in multiple tasks across three different benchmarks. Additionally, we provide a comprehensive examination of the strengths and weaknesses of our approach, highlighting the significance of the features utilized and the LLM employed as an evaluator. We have released our code publicly at [https://github.com/Baylor-AI/HalluDetect.](https://github.com/Baylor-AI/HalluDetect)

*Index Terms*—Large Language Models, Hallucinations

## I. INTRODUCTION

Large Language Models (LLMs) have become the core of many state-of-the-art Natural Language Processing (NLP) algorithms and have revolutionized various domains in NLP and computer vision and even more specialized applications in healthcare, finance, and the creative arts. Because of their impressive Natural Language Generation (NLG) capabilities [\[1,](#page-6-0) [2\]](#page-6-1), they have attracted great interest from the public with great modern tools like ChatGPT [\[3\]](#page-6-2), Github-Copilot [\[4\]](#page-7-0), Dalle [\[5\]](#page-7-1), and others [\[1\]](#page-6-0). These models, with millions to billions of parameters, are often praised for their impressive ability to generate human-like text and tackle intricate tasks with limited to no fine-tuning with techniques like In-Context-Learning [\[6\]](#page-7-2).

Since many of the most popular applications and stateof-the-art algorithms in NLP rely on LLMs, any error they produce affects the results. Particularly in the cases of a Chatbot like ChatGPT, the generated responses are expected

to maintain factual consistency with the source text [\[7\]](#page-7-3). Currently, a pressing concern with LLMs is their propensity to "hallucinate," which intuitively means to produce outputs that, while seemingly coherent, might be misleading, fictitious, or not genuinely reflective of their training data or actual facts [\[8\]](#page-7-4).

Furthermore, the consequences of hallucinatory-generated text when used by the public are a significant ethical concern. This fictitious content can lead to misinformation and have severe implications in delicate medical, legal, educational, and financial fields. Besides the ethical consequences, these errors can lead to limitations in the use of the LLMs to automate programming tasks completely and tedious hand-work, limiting their contribution to NLP tasks [\[8,](#page-7-4) [2\]](#page-6-1).

While there have been efforts to detect and mitigate hallucinations, many of the prevalent methods rely on supervised learning with many multidimensional features. In contrast, others used in-context-learning techniques based on intricate linguistic and semantic analyses [\[9,](#page-7-5) [10,](#page-7-6) [7,](#page-7-3) [11\]](#page-7-7). These methods affect the latency for use in real time. Additionally, current research has shown that even state-of-the-art approaches [\[8,](#page-7-4) [2,](#page-6-1) [12,](#page-7-8) [7\]](#page-7-3) struggle to detect hallucinations.

However, recent research has hinted at the potential of numerical features mathematically [\[13\]](#page-7-9) and empirically [\[10,](#page-7-6) [14,](#page-7-10) [15\]](#page-7-11), as indicators of hallucinations on LLM outputs. These features could provide a resource-efficient method to detect and mitigate hallucinations. In this paper, we introduce a supervised learning approach employing two classifiers that use four numerical features derived from tokens and vocabulary probabilities obtained from other LLM evaluators, usually different ones.

Our research not only highlights the effectiveness of this method in comparison with current approaches in some scenarios but also paves the way for potential uses that validate the credibility of LLM outputs. Our main contributions are:

• Propose a supervised learning approach using four features to detect hallucinations in conditional text generated by

- LLMs, achieving success with two classifiers.
- Evaluate the performance of this approach across three datasets, comparing it with state-of-the-art methods and highlighting its strengths and weaknesses.
- Explore the impact of using the same LLM-Generator vs. different LLMs as evaluators, finding that alternative LLMs provide better indicators to identify hallucinations.
- Compare the difference in performance when using smaller LLM evaluators concerning bigger LLM evaluators like LLaMa-Chat-7b [16].
- Feature importance study with ablation and coefficients analysis.

This paper is structured as follows. First, we present the related works. Second, we describe our methodology. Next, we offer the experiments performed and the results in three datasets. After that, we present an ablation analysis, followed by the conclusions. Finally, we conclude the paper with the limitations section.

#### II. RELATED WORK

The occurrence of hallucinations in LLMs raises concerns, compromising performance in practical implementations like chatbots producing incorrect information. Various research directions have been explored to detect and mitigate hallucinations in different Natural Language Generation tasks [8]. A text summarization verification system has been proposed to detect and mitigate inaccuracies [17, 8]. In dialogue generation, hallucinations have been studied with retrieval augmentation methods [18, 8]. Also, researchers aim to understand why hallucinations occur in different tasks and how these reasons might be connected [19, 20].

Recent approaches to detect and mitigate hallucinations include self-evaluation [21] and self-consistency decoding for intricate reasoning tasks [11]. Knowledge graphs are proposed for gathering evidence [22]. Token probabilities as an indicator of model certainty have been used, addressing uncertainty in sequential generation tasks [23, 24]. Scores from conditional language models are used to assess text characteristics [25, 26].

Additionally, Azaria et al. [14] trained a classifier that outputs the probability that a statement is truthful based on the hidden layer activations of the LLM as it reads or generates the statement. Recently, the work SelfCheckGPT suggests that LLM's probabilities correlate with factuality [10]. Furthermore, Su et al. [15] introduced Modeling of Internal states for hallucination Detection (MIND), an unsupervised training framework that leverages the internal states of LLMs for real-time hallucination detection without requiring manual annotations. Finally, a mathematical investigation by Lee et al. [13] suggests that token probabilities are crucial in generating hallucinations in GPT models under certain assumptions.

#### III. METHODOLOGY

We implement two classifiers, a Logistic Regression (LR) and a Simple Neural Network (SNN), using four numerical features obtained from the token and vocabulary probabilities

from a forward pass to an LLM with the conditional generation approach [27]. In this section, we described our entire methodology to detect hallucinations on a generated text by an LLM conditioned on a piece of text.

#### A. Problem Statement

Given a pair of texts (condition-text, generated-text) that represent the text used to condition the LLM to its generation. We want to detect if a given generated-text is a hallucination.

## B. General Pipeline

Given a set of pairs of texts of the type (condition-text, generated-text) from an LLM (we will call it the LLM-Generator  $(LLM_G)$ ), we extract four numerical features based on the generated tokens and vocabulary tokens probabilities from another LLM that we call the LLM-Evaluator  $(LLM_E)$ .

Then, using these four features, we trained two different classifiers: a Logistic Regression (LR) and a Simple Neural Network (SNN). Finally, we evaluate these classifiers on a test set they did not see before. Figure 1 illustrates the process.

#### C. Features Description

This section provides more details on each feature extracted. Every feature is computed using token probabilities and the vocabulary probability distribution corresponding to each token on the *generated-text*. These can be formally defined as follows: (i) The token probability of each token of the Vocabulary of the  $LLM_E$  corresponding to  $t_i$  as  $P_{LLM_{E_i}}(v_k) = (v_k|t_{i-1},...,t_1,c_m,...,c_1;\theta)$  for every k. (ii) The token with the highest probability at position i in the *generated-text* according to  $LLM_E$  as the  $v^* = \arg\max_k P_{LLM_{E_i}}(v_k)$ . (iii) The token with the lowest probability at position i according to  $LLM_E$  as the  $v^- = \arg\min_k P_{LLM_{E_i}}(v_k)$ .

Next is a natural language description of the four features and, the mathematical definition:

- Minimum Token Probability (mtp): Minimum of the probabilities that the LLM<sub>E</sub> gives to the tokens on the generated-text.
- Average Token Probability (avgtp): Average of the probabilities that the LLM<sub>E</sub> gives to the tokens on the generated-text.
- Maximum  $LLM_E$  Probability Deviation (Mpd): Maximum from all the differences between the token with the highest probability according to  $LLM_E$  at position i and the assigned probability from  $LLM_E$  to  $t_i$  which is the token generated by  $LLM_G$ .
- Minimum  $LLM_E$  Probability Spread (mps): Maximum from all the differences between the token with the highest probability according to  $LLM_E$  at position i ( $v^*$ ) and the token with the lowest probability according to  $LLM_E$  at position i ( $v^-$ ).

<span id="page-1-0"></span><sup>&</sup>lt;sup>1</sup>Which could be the same as  $LLM_G$ .

<span id="page-2-0"></span>Fig. 1. General Pipeline of the Proposed Methodology.

Formally, these features can be defined as:

$$mtp = \min_{i} P_{LLM_E}(t_i) \quad avgtp = \frac{\sum_{i=1}^{n} P_{LLM_E}(t_i)}{n}$$

$$Mpd = \max_{1 \le i \le n} (P_{LLM_E}(v^*) - P_{LLM_E}(t_i))$$

$$mps = \min_{1 \le i \le n} (P_{LLM_{E_i}}(v^*) - P_{LLM_{E_i}}(v^-))$$

These four numerical features are inspired by the mathematical investigation of the GPT model in [\[13\]](#page-7-9), and recent results in [\[10,](#page-7-6) [15\]](#page-7-11), suggesting there is a correlation between the minimum token probability on the generation, the average of the token probabilities, and the average and maximum entropy.

Lee et al. [\[13\]](#page-7-9) proposes that a reliable indicator of hallucination during GPT model generation is the low probability of a token being generated. This is based on the assumption that forcing the model to generate such a low-probability token occurs when the difference between the token with the highest probability and all other tokens is less than a small constant δ. Here, mps is an estimator to avoid the cost of calculating differences across a large vocabulary and generated text.

Additionally, Azaria et al. [\[14\]](#page-7-10) trained a simple classifier called Statement Accuracy Prediction, based on Language Model Activations (SAPLMA) that outputs the probability that a statement is truthful, based on the hidden layer activations of the LLM as it reads or generates the statement. The authors showed that SAPLMA, which leverages the LLM's internal states, performs better than prompting the LLM to state explicitly whether a statement is true or false. Different from [\[14\]](#page-7-10), Su et al. [\[15\]](#page-7-11) introduced MIND, an unsupervised training framework that leverages the internal states of LLMs for hallucination detection requiring no manual annotations.

## *D. Distinctive Methodological Approach*

Diverging from these three previous papers [\[14,](#page-7-10) [13,](#page-7-9) [10,](#page-7-6) [15\]](#page-7-11), the approach here differs in several aspects. This is an empirical paper, not a theoretical one like [\[13\]](#page-7-9). We do not use Self-Consistency as [\[10\]](#page-7-6) does, and also, our approach is not zeroshot or few-shot learning. Our approach follows supervised learning like [\[14\]](#page-7-10). However, instead of using the contextual embeddings and hidden layers, we only use four features that result from aggregations of the token and vocabulary

probabilities. We also test a simpler Logistic Regression classifier besides a Simple Dense Neural network.

Moreover, instead of using only the LLM generating the text (LLMG), the argument is made that depending on the task and model type, different LLM-Evaluators (LLME) can provide consistent but quantitatively different results than using probabilities from LLMG. The belief is that probabilities from a different model, varying in architecture, size, parameters, context length, and training data, can also serve as reliable indicators of hallucinations in the text generated by LLMG. Since LLM<sup>E</sup> and LLM<sup>G</sup> are not always the same, an additional numerical feature, M pd (Maximum LLM<sup>E</sup> Probability Deviation), is introduced. This feature indicates the difference between the maximum probability token in the vocabulary of LLM<sup>E</sup> and the token generated by LLMG.

Using different LLMs as evaluators takes advantage of the diversity of training data among different language models (LLMs) and captures various linguistic patterns and styles. Detecting hallucinations in LLM outputs, such as in LLMG, might be possible through analyzing probability distributions. Yet, specific patterns may remain undetectable, potentially addressed by other models specialized in particular topics. Evaluations from multiple models enhance robustness by mitigating biases inherent in individual models' training data.

## *E. Feature Extraction*

In the previous section, we described the numerical features selected, but the process of extracting these features is still ongoing. To extract the features, we used LLM<sup>E</sup> models that can be used for the Conditional Generation Task. Notably, in our case, it is a force decoding since the tokens of *generatedtext* were potentially generated by a different LLM (LLMG). Instead of letting the model generate the answer token-by-token from the *condition-text* alone, we provide it with the token predicted by LLM<sup>G</sup> at each step. This way, LLM<sup>E</sup> is forced to follow the path to generate the *generated-text* and, from there, extract the token probabilities from LLM<sup>E</sup> if it would generate that sequence itself. Then, using these token probabilities, we compute the four numerical features previously described.

## *F. Models Specification*

The classifiers used are a Logistic Regression [\[28\]](#page-8-4) (LR) and a Simple Neural Network (SNN). Both for a data point of the type *(condition-text, generated-text)* only use the four numerical features extracted. We selected the LR for its simplicity, fast training, and effectiveness in binary classification tasks. However, we implemented a SNN to explore complex nonlinear relationships in the data. The SNN architecture consists of an input layer with four neurons representing each features, followed by two hidden layers, each comprising 512 neurons with the ReLU activation function. Followed by an output layer, containing a single neuron activated by a sigmoid function, suitable for binary classification tasks.

## IV. EXPERIMENTAL SETUP AND RESULTS

In this section, we describe the details of our experimental setup, the dataset we use, and the results we obtain.

## *A. Datasets*

Here, we list the three datasets we are using for experimentation and comparison.

- HaluEval: Hallucination Evaluation for Large Language Models (HaluEval) benchmark is a collection of generated and human-annotated hallucinated samples for evaluating the performance of LLMs in recognizing hallucinations. HaluEval includes 5,000 general user queries with Chat-GPT responses and 30,000 task-specific examples (10,000 per task) from three tasks: question answering, knowledgegrounded dialogue, and text summarization [\[12\]](#page-7-8).
- HELM: Hallucination detection Evaluation for multiple LLMs (HELM) benchmark is a list of 3582 sentences from randomly sampling 50,000 articles from WikiText-103 [\[29\]](#page-8-5) where the selected LLMs were tasked with prompt-based continuation writing. The resulting sentences were annotated as hallucination or not [\[15\]](#page-7-11).
- True-False: Comprises 6,084 sentences divided into the topics of "Cities," "Inventions," "Chemical Elements," "Animals," "Companies," and "Scientific Facts." All sentences in each category are conformed of true statements and false statements [\[14\]](#page-7-10). Unlike HaluEval, this dataset only has *generated-text* and does not include any *condition-text*.

## *B. LLM Evaluators Used*

<span id="page-3-0"></span>The LLMs selected as evaluators to study the impact of factors such as architecture, training method, and training data include *GPT-2*, its large version (gpt2-large) [\[30\]](#page-8-6); Bidirectional and Auto-Regressive Transformers (*BART*), its CNN-Large version (bart-large-cnn) [\[31\]](#page-8-7); Longformer Encoder-Decoder (*LED*) [\[32\]](#page-8-8), with the version fine-tuned on the arXiv dataset (led-large-16384-arxiv). Also, we used four bigger LLMs like OPT-6.7B (*OPT*) [\[33\]](#page-8-9), GPT-J-6.7B (*GPT-J*) [\[34\]](#page-8-10), LLaMA-2-Chat-7B (*LLC-7b*) [\[16\]](#page-7-12) and Gemma-7b (*Gemma*) [\[35\]](#page-8-11). We used the known transformers library.[2](#page-3-0) In most cases, we utilized the Conditional Generation

setup. For *GPT-2*, we employed the GPT2LMHeadModel setup. Additionally, when forwarding inputs to these models with a pair of *(condition-text, generated-text)*, we encountered the challenge of context limitation, which varied depending on the LLM. To address this issue, we did not truncate the *generated-text* if possible. Instead, if truncation was necessary (with a truncation length of truncate\_len), we removed the excess from the *condition-text*. If additional knowledge was included, we evenly split the truncation between the knowledge and the *condition-text*.

## *C. Training Process of the Classifiers*

To train LR, we used the sklearn library[3](#page-3-1) with the Limited-memory Broyden-Fletcher-Goldfarb-Shanno Algorithm solver [\[37\]](#page-8-12) and default parameters. The SNN was trained during 10<sup>4</sup> epochs. We used the Adam [\[38\]](#page-8-13) optimizer with learning rate of 10−<sup>3</sup> . All experiments, including feature extraction, training, and evaluation of the classifiers, were conducted on an NVIDIA L40S GPU core with 48GB of memory. Given a dataset and a single LLME, training takes anywhere from 30 minutes (HELM) to 4.5 hours (HaluEval), depending on the size of the training data and the length of the *condition-text* and *generated-text*.

*1) HaluEval:* We train both classifiers for each of the tasks in the HaluEval benchmark. Each data point is split into two data points: *(condition-text, right-answer)* and *(condition-text, hallucinated-answer)*. Therefore, the datasets would be of 20,000 examples for each of the Question Answering (QA), Knowledge-Grounded Dialogue (KGD), and Summarization tasks where in each case half of the dataset is comprised of data points of the type *(condition-text, right-answer)* and the other half are of the type *(condition-text, hallucinated-answer)*. In the case of the General-User Queries, the dataset is already in that format, with each data point classified as a hallucination. Therefore, the dataset size is the same, which is 5,000.

Then, with this adaptation of the HaluEval benchmark dataset when we were approaching a given task, we will sample 10% of the data points (half with the *right-answer* and the other with a *hallucinated-answer*) randomly. These 10% data points are used to train both classifiers, and we test the model capabilities on the remaining 90% of the dataset for a given task.

- *2) HELM:* In the HELM dataset [\[15\]](#page-7-11) the sentences were separated into categories depending on which LLM generated them. Therefore, when we wanted to evaluate the sentences generated by a given LLM like LlaMa-Chat-7b (LLC-7b), we would train on all other sentences produced by other LLMs.
- *3) True-False:* We followed the same process as Azaria et al. [\[14\]](#page-7-10). We pick a category like "animals" for testing and train in all other categories.

## *D. Results*

<span id="page-3-1"></span>*1) HaluEval:* We evaluate each classifier trained on the 10% data of the given task on the other 90%. We selected the following metrics: Accuracy, F1, and Precision-Recall <span id="page-4-2"></span>AVERAGE RESULTS IN THE TEST SET FOR EACH TASK IN THE HALUEVAL BENCHMARK GIVEN THE SELECTED LLM<sup>E</sup> . NCACC STANDS FOR ACCURACY WITHOUT *condition-text* AND KACC FOR ACCURACY INCLUDING EXTRA KNOWLEDGE.

|        | Summarization |      |       |       |      | Question Answering |       |       |      |      | KGD  |       |       |      |
|--------|---------------|------|-------|-------|------|--------------------|-------|-------|------|------|------|-------|-------|------|
| Models | Acc           | F1   | PRAUC | NCAcc | Acc  | F1                 | PRAUC | NCAcc | KAcc | Acc  | F1   | PRAUC | NCAcc | KAcc |
| GPT-2  | 0.82          | 0.78 | 0.89  | 0.90  | 0.82 | 0.82               | 0.86  | 0.78  | 0.88 | 0.62 | 0.60 | 0.70  | 0.64  | 0.62 |
| BART   | 0.77          | 0.78 | 0.83  | 0.99  | 0.95 | 0.95               | 0.97  | 0.96  | 0.94 | 0.66 | 0.63 | 0.74  | 0.65  | 0.60 |
| LED    | 0.97          | 0.76 | 0.81  | 0.97  | 0.88 | 0.88               | 0.91  | 0.86  | 0.87 | 0.62 | 0.61 | 0.70  | 0.62  | 0.60 |
| OPT    | 0.98          | 0.98 | 0.99  | 0.99  | 0.79 | 0.78               | 0.85  | 0.76  | 0.79 | 0.66 | 0.67 | 0.74  | 0.67  | 0.61 |
| GPT-J  | 0.98          | 0.98 | 0.99  | 0.99  | 0.77 | 0.78               | 0.84  | 0.73  | 0.83 | 0.66 | 0.67 | 0.74  | 0.67  | 0.66 |
| LLC-7b | 0.67          | 0.68 | 0.69  | 0.77  | 0.73 | 0.69               | 0.81  | 0.75  | 0.77 | 0.60 | 0.54 | 0.64  | 0.63  | 0.61 |
| Gemma  | 0.51          | 0.51 | 0.52  | 0.57  | 0.76 | 0.73               | 0.82  | 0.79  | 0.71 | 0.58 | 0.58 | 0.66  | 0.62  | 0.55 |

<span id="page-4-0"></span>TABLE II RESULTS TAKEN FROM [\[36\]](#page-8-14) MEASURED IN ACCURACY (%) ON THE HALU-EVAL DATASET.

| Models      | QA    | KGD   | Summ. | GUQ   |
|-------------|-------|-------|-------|-------|
| ChatGPT     | 62.59 | 72.40 | 58.53 | 79.44 |
| Claude 2    | 69.78 | 64.73 | 57.75 | 75.00 |
| Claude      | 67.60 | 64.83 | 53.76 | 73.88 |
| Davinci-003 | 49.65 | 68.37 | 48.07 | 80.40 |
| Davinci-002 | 60.05 | 60.81 | 47.77 | 80.42 |
| GPT-3       | 49.21 | 50.02 | 51.23 | 72.72 |
| Llama-2-Ch  | 49.60 | 43.99 | 49.55 | 20.46 |
| ChatGLM 6B  | 47.93 | 44.41 | 48.57 | 30.92 |
| Falcon 7B   | 39.66 | 29.08 | 42.71 | 18.98 |
| Vicuna 7B   | 60.34 | 46.35 | 45.62 | 19.48 |
| Alpaca 7B   | 6.68  | 17.55 | 20.63 | 9.54  |

<span id="page-4-1"></span>TABLE III OUR RESULTS FOR EACH LLM<sup>E</sup> AND TASK USING THE LR CLASSIFIER AND MEASURE IN ACCURACY ON THE TEST SET.

| Model  | Summ. | QA   | KGD  | GUQ  |
|--------|-------|------|------|------|
| GPT-2  | 0.66  | 0.77 | 0.62 | 0.77 |
| BART   | 0.65  | 0.94 | 0.49 | 0.54 |
| LED    | 0.55  | 0.87 | 0.62 | 0.52 |
| OPT    | 0.73  | 0.75 | 0.61 | 0.80 |
| GPT-J  | 0.90  | 0.75 | 0.61 | 0.81 |
| LLC-7b | 0.70  | 0.74 | 0.55 | 0.81 |
| Gemma  | 0.52  | 0.73 | 0.53 | 0.80 |

Area Under Curve (PRAUC). Table [II](#page-4-0) shows current stateof-the-art results in HaluEval. All the results were extracted from [\[12,](#page-7-8) [36\]](#page-8-14), which is the paper that introduces the HaluEval benchmark and an empirical study from the same authors on factuality hallucination in LLMs. Next, Table [III](#page-4-1) shows the accuracy results on the test set for each task using every LLM<sup>E</sup> selected and the Logistic Regression as the classifier. As it can be appreciated, the Logistic Regression obtains great results compared to what previous approaches would have gotten on the 90% of the dataset. Finally, Table [I](#page-4-2) shows our average [4](#page-4-3) results per model of our approach in each metric evaluated on the test set for each of the tasks of Summarization, QA, and KGD respectively.

The methods of Table [II](#page-4-0) are based on In-Context-Learning approaches and evaluated in 100% of the dataset. In contrast, our approach utilizes supervised learning, but we believe it's

fair to compare it to existing methods. We train our models using only 10% of the data, reserving the remaining 90% for testing. We argue that the performance of current approaches on the 90% test set will not deviate significantly from their performance on the full dataset, especially when there's a significant accuracy difference. Consequently, we've chosen not to present our results alongside the baseline results in Table [II.](#page-4-0) For an exact comparison, we would need to apply their methods to the same test set we've used but we do not have the resources for such heavy computation. The key findings in the results showed in Tables [III](#page-4-1) and Table [I](#page-4-2) are summarized as follows:

First, our classifiers trained on only 10% of the data demonstrate effectiveness on the test set (the other 90%) for the Summarization and Question Answering tasks using as LLM<sup>E</sup> the *GPT-J* and *BART* respectively. The best results in both tasks have an accuracy and F<sup>1</sup> over 98% with the SNN classifier and over 90% with the LR classifier in Summarization and F<sup>1</sup> over 95% in QA. These results outperform the results previous approaches would have gotten on the same test set.

Additionally, while not surpassing the state-of-the-art, we achieved competitive results in the dialogue task. Finally, on the GUQ task, a table was not included for the SNN classifier since we obtained similar results for each LLME. When employing various LLM<sup>E</sup> models with the SNN classifier, results suggest overfitting to the negative class, yielding an accuracy of 81%, F<sup>1</sup> of 1%, and PRAUC of 10%. This can be attributed to data imbalance, where only about 20% are not hallucinations. An alternative attempt with a training set of 500 positive and 500 negative examples tested on the remaining 4,000 revealed little success, with a best accuracy at 69% and F<sup>1</sup> at 0.23%.

Also, in the tasks of QA and KGD, our method, including the knowledge in the *condition-text*, improved the accuracy, while in a few, it did not. Adding knowledge can help LLM evaluators provide meaningful token probabilities for any task with our approach.

An unexpected finding we encountered was that when the LLM<sup>E</sup> only provided probabilities for the *generated-text* without the *condition-text*, it yielded remarkably high results in Summarization and QA tasks with specific LLM<sup>E</sup> models like *GPT-J*, achieving up to 99% accuracy or *BART* with 96%. This anomaly prompted us to verify that we had not

<span id="page-4-3"></span><sup>4</sup>Average results with data sampled randomly in three runs.

inadvertently used testing data for training or made similar errors. However, this was not the case and supported by the fact that it did not happen in KGD and GUQ, using the same code, nor did it occur with all LLM<sup>E</sup> models in Summarization and QA. We hypothesize that there may be a probabilistic pattern in the Summarization and QA tasks within the HaluEval dataset generation process that our approach can learn with some LLME. Note that this observation does not render the benchmark useless; instead, it suggests that a supervised approach may not be the most suitable fit, and instead, unsupervised approaches might be more appropriate for evaluation in this benchmark.

Despite the significant improvement achieved by selecting specific LLM<sup>E</sup> models in certain tasks, using other LLM<sup>E</sup> models still yielded competitive results, with some instances even surpassing state-of-the-art benchmarks. For example, *OPT* coupled with the SNN classifier achieved an F<sup>1</sup> score of 79% in the QA task, while *GPT-2* attained an 82% F<sup>1</sup> score.

*2) HELM:* Table [IV](#page-5-0) shows the results of our approach in the HELM benchmark [\[15\]](#page-7-11). The overall results showed that our approach did not surpass MIND [\[15\]](#page-7-11) except with LLC-13B sentences. However, we only use four features, and still, our approach surpasses the results of other methods like SAPLMA [\[14\]](#page-7-10) and, in some cases, SelfCheckGPT with Natural Language Inference (SCG-NLI) [\[10\]](#page-7-6) and others reported in [\[15\]](#page-7-11). Also, we showed in the Appendix section how removing the *condition-text* affects our approach, which, different from the Halu-Eval, in this dataset, decreases performance as expected.

Still, unlike MIND, which gets its training data unsupervised without annotation, we are training with the annotated data they provided in their HELM benchmark.

*3) True-False:* The results obtained in this dataset using any of our selected LLMs were below the baseline provided by [\[14\]](#page-7-10) and significantly lower than their approach. This highlights a major weakness in our methodology and points out the importance of utilizing hidden layers as features. We recommend that any future method demonstrate its performance on this challenging dataset. Detailed results are provided in

<span id="page-5-0"></span>TABLE IV RESULTS OF OUR APPROACH AND PREVIOUS METHODS IN THE HELM BENCHMARK MEASURED IN PRAUC.

| Baselines | Falcon | GPT-J | LLB<br>-7B | LLC<br>-13B | LLC<br>-7B | OPT   |
|-----------|--------|-------|------------|-------------|------------|-------|
| PE-max    | 0.648  | 0.750 | 0.685      | 0.444       | 0.493      | 0.726 |
| SCG-NLI   | 0.685  | 0.868 | 0.764      | 0.583       | 0.657      | 0.810 |
| SAPLMA    | 0.513  | 0.699 | 0.578      | 0.305       | 0.407      | 0.621 |
| MIND      | 0.790  | 0.877 | 0.788      | 0.604       | 0.676      | 0.884 |
| Ours      |        |       |            |             |            |       |
| GPT-2     | 0.683  | 0.847 | 0.759      | 0.618       | 0.616      | 0.850 |
| BART      | 0.710  | 0.828 | 0.695      | 0.569       | 0.568      | 0.825 |
| LED       | 0.683  | 0.809 | 0.722      | 0.527       | 0.548      | 0.829 |
| OPT       | 0.719  | 0.839 | 0.773      | 0.634       | 0.637      | 0.864 |
| GPT-J     | 0.702  | 0.808 | 0.751      | 0.642       | 0.588      | 0.834 |
| LLC-7b    | 0.727  | 0.855 | 0.785      | 0.563       | 0.644      | 0.842 |
| Gemma     | 0.738  | 0.850 | 0.786      | 0.601       | 0.651      | 0.843 |

the Appendix section.

*4) Overall Conclusions from Results:* In general, our results demonstrate that our supervised learning approach, utilizing only four features, exhibits competitive performance compared to current methods. Our approach surpasses the current state-ofthe-art methods across various scenarios in tasks and datasets.

In the HaluEval benchmark, where generations originate from ChatGPT powered by GPT-3.5, we found that some of the top-performing LLMs were two GPT-based models (*GPT-2* and *GPT-J*), which are the nearest to the LLM-Generator that we were able to test. Interestingly, even models not based on the LLM-Generator, such as *OPT*, *BART*, *LED*, achieved comparable results and surpassed them in some tasks. Notably, smaller models like *BART* outperformed all LLM evaluators in the QA task, suggesting that varying LLMs, regardless of size, can yield superior results due to training data and architecture differences.

In the HELM benchmark, where each test set comprised sentences generated by accessible LLMs, we explored the impact of using the actual LLM-Generator as the LLM-Evaluator (like the case of *GPT-J* or *LLC-7B*) compared to using a different one. Results revealed that different LLMs as evaluators often yielded similar or better results than the corresponding LLM-Generator, showing the advantages of employing diverse LLMs for evaluation purposes. Furthermore, our experiments demonstrated that the performance disparity between larger LLMs like *GPT-J* and *LLC-7B* versus smaller ones such as *GPT-2* and *BART* is not significant in many scenarios.

Finally, in the True-False dataset, it became evident that our method exhibits weaknesses when applied to this type of data, and the features lack the necessary significance for detecting hallucinations present in that dataset.

## *E. Feature Importance Analysis - Ablation*

We performed experiments using single numerical feature to determine which features were significant or not to the results. Table [V](#page-6-3) showed for three tasks in HaluEval and three LLM<sup>E</sup> how the results in accuracy were affected by which features were used or not. In most cases, the use of all features provides the best results. Then, when each feature was used alone, we discovered that the most meaningful features in our approach were *mtp* and *avgtp*, especially in Summarization and QA. However, in the KGD task, it was more important for bigger models like *GPT-J* and *LLC-7b*, the feature introduced by this paper: M pd.

Additionally, in the Appendix, we conduct a feature analysis using the coefficients obtained from the LR classifier.

## V. CONCLUSIONS AND FUTURE WORKS

This work introduced a supervised learning approach for identifying hallucinations in conditional text generated by LLMs. Leveraging just four features derived from conditional token probabilities, our approach demonstrated competitive performance compared to existing methodologies across various tasks and datasets. Future work includes exploring hybrid

TABLE V Feature importance based on accuracy for three tasks in the HaluEval benchmark given three  $LLM_E$  .

<span id="page-6-3"></span>

| Features   |        |        | S        | ummarizat                            | arization Question Answering                |                                      |                                      | wering                               | KGD                                  |                                      |                                             |                                             |
|------------|--------|--------|----------|--------------------------------------|---------------------------------------------|--------------------------------------|--------------------------------------|--------------------------------------|--------------------------------------|--------------------------------------|---------------------------------------------|---------------------------------------------|
| mtp        | avgtp  | Mpd    | mps      | GPT-J                                | BART                                        | LLC-7b                               | GPT-J                                | BART                                 | LLC-7b                               | GPT-J                                | BART                                        | LLC-7b                                      |
| <b>√</b> ✓ | √<br>√ | √<br>√ | √  <br>√ | 0.98<br>0.50<br>0.98<br>0.50<br>0.51 | 0.77<br><b>0.79</b><br>0.64<br>0.61<br>0.62 | 0.69<br>0.50<br>0.57<br>0.54<br>0.60 | 0.76<br>0.64<br>0.69<br>0.72<br>0.62 | 0.95<br>0.92<br>0.95<br>0.90<br>0.64 | 0.74<br>0.50<br>0.51<br>0.73<br>0.57 | 0.66<br>0.56<br>0.62<br>0.62<br>0.53 | 0.65<br>0.60<br><b>0.66</b><br>0.58<br>0.53 | 0.60<br>0.50<br>0.50<br><b>0.61</b><br>0.52 |

methods that combine In-Context-Learning approaches with probabilistic-based methods, including supervised classifiers.

Through extensive evaluation across three datasets, we uncovered insights into the effectiveness of our approach. Our exploration of using different LLMs as evaluators further emphasized the advantages of employing diverse models for evaluation purposes. By comparing results obtained from using the actual LLM-Generator as the evaluator against those from different LLMs, we showcased the potential for alternative models to yield comparable or even superior evaluation outcomes. In future work, it is possible to investigate advanced ensemble learning techniques to further enhance the performance of hallucination detection systems by effectively combining predictions from multiple LLM evaluators. Furthermore, we identified weaknesses in our approach when applied to datasets like the True-False dataset. Future research could explore whether augmenting our four features with hidden layer features could improve the state-of-the-art performance on that dataset.

This research extends to every domain relying on LLMs. By enhancing the reliability of LLM outputs, the proposed method contributes to the ethical use of these models in sensitive applications, such as medical, legal, educational, and financial domains. This work is a step toward creating a reliable method to detect hallucinations in LLMs based on token and vocabulary probabilities.

## LIMITATIONS

The first limitation is the numerical features and models selected as  $LLM_E$ . While our current approach has demonstrated effectiveness in specific tasks, it may only capture the richness and complexity of some textual content types. The derived features must be more meaningful for tasks like Knowledge-Grounded Dialogue (KGD), which involve intricate context and real-time exchanges.

Our method outperformed state-of-the-art tasks like Summarization and Question Answering in HaluEval. However, in KGD and General User Queries, it achieved competitive but not leading results. This could hint at potential over-specialization or the need for task-specific feature engineering. Another reason could be the inherent limitations of the LLMs selected as  $LLM_E$ .

Additionally, because of the context length limitation of some of the LLMs, we needed to truncate the *condition-text* or extra knowledge, which might cause us to lose the necessary context to get the correct token probabilities to classify correctly.

However, some of the LLMs needed more context length to use everything without losing information.

One of the main limitations is that our approach's results and effectiveness may be tied to the characteristics of the datasets used. The model's performance could be skewed if the dataset has inherent biases or lacks diversity in certain aspects. For instance, it might be in the specific patterns obtained on the HaluEval benchmark that these four numerical features are good indicators for detecting this type of hallucination. However, it does not change the fact that current complex state-of-the-art approaches have yet to show this level of performance under the same circumstances.

Although we achieved competitive results in the HELM benchmark, it is worth noting that, apart from SALPMA, most other approaches relied on unsupervised methods. As our method is supervised, it inherently depends on curated and annotated data, which poses a limitation. Additionally, the performance in the True-False dataset highlights a weakness in our approach, suggesting that it may need to be augmented with additional features to be effective in a broader range of scenarios.

Finally, this method is grounded in binary classification. In real-world scenarios, hallucination might be more nuanced, with varying degrees of severity, which our current approach might not account for. Furthermore, there needs to be more interpretability; even when we can get intuition from the numerical features, we cannot obtain the exact explanation of what specific wrong fact or fictitious information is being added. We intend to explore other ideas on datasets that make this separation to increase the interpretability.

## REFERENCES

- <span id="page-6-0"></span>[1] W. X. Zhao, K. Zhou, J. Li, T. Tang, X. Wang, Y. Hou, Y. Min, B. Zhang, J. Zhang, Z. Dong *et al.*, "A survey of large language models," *arXiv preprint arXiv:2303.18223*, 2023.
- <span id="page-6-1"></span>[2] J. Kaddour, J. Harris, M. Mozes, H. Bradley, R. Raileanu, and R. McHardy, "Challenges and applications of large language models," 2023.
- <span id="page-6-2"></span>[3] M. Hosseini, C. A. Gao, D. M. Liebovitz, A. M. Carvalho, F. S. Ahmad, Y. Luo, N. MacDonald, K. L. Holmes, and A. Kho, "An exploratory survey about using chatgpt in education, healthcare, and research," *medRxiv*, pp. 2023– 03, 2023.

- <span id="page-7-0"></span>[4] M. Chen, J. Tworek, H. Jun, Q. Yuan, H. P. d. O. Pinto, J. Kaplan, H. Edwards, Y. Burda, N. Joseph, G. Brockman *et al.*, "Evaluating large language models trained on code," *arXiv preprint arXiv:2107.03374*, 2021.
- <span id="page-7-1"></span>[5] L. Zeqiang, Z. Xizhou, D. Jifeng, Q. Yu, and W. Wenhai, "Mini-dalle3: Interactive text to image by prompting large language models," *arXiv preprint arXiv:2310.07653*, 2023.
- <span id="page-7-2"></span>[6] S. Lu, I. Bigoulaeva, R. Sachdeva, H. T. Madabushi, and I. Gurevych, "Are emergent abilities in large language models just in-context learning?" *arXiv preprint arXiv:2309.01809*, 2023.
- <span id="page-7-3"></span>[7] D. Lei, Y. Li, M. Wang, V. Yun, E. Ching, E. Kamal *et al.*, "Chain of natural language inference for reducing large language model ungrounded hallucinations," *arXiv preprint arXiv:2310.03951*, 2023.
- <span id="page-7-4"></span>[8] Z. Ji, N. Lee, R. Frieske, T. Yu, D. Su, Y. Xu, E. Ishii, Y. J. Bang, A. Madotto, and P. Fung, "Survey of hallucination in natural language generation," *ACM Computing Surveys*, vol. 55, no. 12, pp. 1–38, 2023.
- <span id="page-7-5"></span>[9] Y. Zhang, Y. Li, L. Cui, D. Cai, L. Liu, T. Fu, X. Huang, E. Zhao, Y. Zhang, Y. Chen *et al.*, "Siren's song in the ai ocean: A survey on hallucination in large language models," *arXiv preprint arXiv:2309.01219*, 2023.
- <span id="page-7-6"></span>[10] P. Manakul, A. Liusie, and M. Gales, "SelfCheckGPT: Zero-resource black-box hallucination detection for generative large language models," in *Proceedings of the 2023 Conference on Empirical Methods in Natural Language Processing*, H. Bouamor, J. Pino, and K. Bali, Eds. Singapore: Association for Computational Linguistics, Dec. 2023, pp. 9004–9017. [Online]. Available:<https://aclanthology.org/2023.emnlp-main.557>
- <span id="page-7-7"></span>[11] X. Wang, J. Wei, D. Schuurmans, Q. V. Le, E. H. Chi, S. Narang, A. Chowdhery, and D. Zhou, "Self-consistency improves chain of thought reasoning in language models," in *The Eleventh International Conference on Learning Representations, ICLR 2023, Kigali, Rwanda, May 1-5, 2023*. OpenReview.net, 2023. [Online]. Available: <https://openreview.net/pdf?id=1PL1NIMMrw>
- <span id="page-7-8"></span>[12] J. Li, X. Cheng, X. Zhao, J.-Y. Nie, and J.-R. Wen, "HaluEval: A large-scale hallucination evaluation benchmark for large language models," in *Proceedings of the 2023 Conference on Empirical Methods in Natural Language Processing*, H. Bouamor, J. Pino, and K. Bali, Eds. Singapore: Association for Computational Linguistics, Dec. 2023, pp. 6449–6464. [Online]. Available:<https://aclanthology.org/2023.emnlp-main.397>
- <span id="page-7-9"></span>[13] M. Lee, "A mathematical investigation of hallucination and creativity in gpt models," *Mathematics*, vol. 11, no. 10, p. 2320, 2023.
- <span id="page-7-10"></span>[14] A. Azaria and T. Mitchell, "The internal state of an LLM knows when it's lying," in *Findings of the Association for Computational Linguistics: EMNLP 2023*, H. Bouamor, J. Pino, and K. Bali, Eds. Singapore: Association for Computational Linguistics, Dec. 2023, pp. 967–976. [Online]. Available: [https:](https://aclanthology.org/2023.findings-emnlp.68)

- [//aclanthology.org/2023.findings-emnlp.68](https://aclanthology.org/2023.findings-emnlp.68)
- <span id="page-7-11"></span>[15] W. Su, C. Wang, Q. Ai, Y. Hu, Z. Wu, Y. Zhou, and Y. Liu, "Unsupervised real-time hallucination detection based on the internal states of large language models," *arXiv preprint arXiv:2403.06448*, 2024.
- <span id="page-7-12"></span>[16] H. Touvron, L. Martin, K. Stone, P. Albert, A. Almahairi, Y. Babaei, N. Bashlykov, S. Batra, P. Bhargava, S. Bhosale *et al.*, "Llama 2: Open foundation and fine-tuned chat models," *arXiv preprint arXiv:2307.09288*, 2023.
- <span id="page-7-13"></span>[17] Z. Zhao, S. B. Cohen, and B. Webber, "Reducing quantity hallucinations in abstractive summarization," in *Findings of the Association for Computational Linguistics: EMNLP 2020*, T. Cohn, Y. He, and Y. Liu, Eds. Online: Association for Computational Linguistics, Nov. 2020, pp. 2237–2249. [Online]. Available: [https:](https://aclanthology.org/2020.findings-emnlp.203) [//aclanthology.org/2020.findings-emnlp.203](https://aclanthology.org/2020.findings-emnlp.203)
- <span id="page-7-14"></span>[18] K. Shuster, S. Poff, M. Chen, D. Kiela, and J. Weston, "Retrieval augmentation reduces hallucination in conversation," in *Findings of the Association for Computational Linguistics: EMNLP 2021*, M.-F. Moens, X. Huang, L. Specia, and S. W.-t. Yih, Eds. Punta Cana, Dominican Republic: Association for Computational Linguistics, Nov. 2021, pp. 3784–3803. [Online]. Available: [https://aclanthology.org/2021.findings-emnlp.](https://aclanthology.org/2021.findings-emnlp.320) [320](https://aclanthology.org/2021.findings-emnlp.320)
- <span id="page-7-15"></span>[19] S. Zheng, J. Huang, and K. C.-C. Chang, "Why does chatgpt fall short in answering questions faithfully?" *arXiv preprint arXiv:2304.10513*, 2023.
- <span id="page-7-16"></span>[20] S. Das, S. Saha, and R. Srihari, "Diving deep into modes of fact hallucinations in dialogue systems," in *Findings of the Association for Computational Linguistics: EMNLP 2022*, Y. Goldberg, Z. Kozareva, and Y. Zhang, Eds. Abu Dhabi, United Arab Emirates: Association for Computational Linguistics, Dec. 2022, pp. 684–699. [Online]. Available: [https:](https://aclanthology.org/2022.findings-emnlp.48) [//aclanthology.org/2022.findings-emnlp.48](https://aclanthology.org/2022.findings-emnlp.48)
- <span id="page-7-17"></span>[21] Z. Yin, Q. Sun, Q. Guo, J. Wu, X. Qiu, and X. Huang, "Do large language models know what they don't know?" in *Findings of the Association for Computational Linguistics: ACL 2023*, A. Rogers, J. Boyd-Graber, and N. Okazaki, Eds. Toronto, Canada: Association for Computational Linguistics, Jul. 2023, pp. 8653–8665. [Online]. Available: <https://aclanthology.org/2023.findings-acl.551>
- <span id="page-7-18"></span>[22] J. Jiang, K. Zhou, Z. Dong, K. Ye, X. Zhao, and J.-R. Wen, "StructGPT: A general framework for large language model to reason over structured data," in *Proceedings of the 2023 Conference on Empirical Methods in Natural Language Processing*, H. Bouamor, J. Pino, and K. Bali, Eds. Singapore: Association for Computational Linguistics, Dec. 2023, pp. 9237–9251. [Online]. Available:<https://aclanthology.org/2023.emnlp-main.574>
- <span id="page-7-19"></span>[23] Y. Xiao and W. Y. Wang, "On hallucination and predictive uncertainty in conditional language generation," in *Proceedings of the 16th Conference of the European Chapter of the Association for Computational Linguistics: Main Volume*, P. Merlo, J. Tiedemann, and

- R. Tsarfaty, Eds. Online: Association for Computational Linguistics, Apr. 2021, pp. 2734–2744. [Online]. Available:<https://aclanthology.org/2021.eacl-main.236>
- <span id="page-8-0"></span>[24] A. Malinin and M. Gales, "Uncertainty estimation in autoregressive structured prediction," *arXiv preprint arXiv:2002.07650*, 2020.
- <span id="page-8-1"></span>[25] W. Yuan, G. Neubig, and P. Liu, "Bartscore: Evaluating generated text as text generation," *Advances in Neural Information Processing Systems*, vol. 34, pp. 27 263– 27 277, 2021.
- <span id="page-8-2"></span>[26] J. Fu, S.-K. Ng, Z. Jiang, and P. Liu, "Gptscore: Evaluate as you desire," *arXiv preprint arXiv:2302.04166*, 2023.
- <span id="page-8-3"></span>[27] H. Zhang, H. Song, S. Li, M. Zhou, and D. Song, "A survey of controllable text generation using transformerbased pre-trained language models," *ACM Comput. Surv.*, vol. 56, no. 3, oct 2023. [Online]. Available: <https://doi.org/10.1145/3617680>
- <span id="page-8-4"></span>[28] R. E. Wright, "Logistic regression." *Reading and Understanding Multivariate Statistics*, pp. 217–244, 1995.
- <span id="page-8-5"></span>[29] S. Merity, C. Xiong, J. Bradbury, and R. Socher, "Pointer sentinel mixture models," *arXiv preprint arXiv:1609.07843*, 2016.
- <span id="page-8-6"></span>[30] A. Radford, J. Wu, R. Child, D. Luan, D. Amodei, I. Sutskever *et al.*, "Language models are unsupervised multitask learners," *OpenAI blog*, vol. 1, no. 8, p. 9, 2019.
- <span id="page-8-7"></span>[31] M. Lewis, Y. Liu, N. Goyal, M. Ghazvininejad, A. Mohamed, O. Levy, V. Stoyanov, and L. Zettlemoyer, "BART: Denoising sequence-to-sequence pre-training for natural language generation, translation, and comprehension," in *Proceedings of the 58th Annual Meeting of the Association for Computational Linguistics*, D. Jurafsky, J. Chai, N. Schluter, and J. Tetreault, Eds. Online: Association for Computational Linguistics, Jul. 2020, pp. 7871–7880. [Online]. Available: [https:](https://aclanthology.org/2020.acl-main.703) [//aclanthology.org/2020.acl-main.703](https://aclanthology.org/2020.acl-main.703)
- <span id="page-8-8"></span>[32] I. Beltagy, M. E. Peters, and A. Cohan, "Longformer: The long-document transformer," *arXiv preprint arXiv:2004.05150*, 2020.
- <span id="page-8-9"></span>[33] S. Zhang, S. Roller, N. Goyal, M. Artetxe, M. Chen, S. Chen, C. Dewan, M. Diab, X. Li, X. V. Lin *et al.*, "Opt: Open pre-trained transformer language models," *arXiv preprint arXiv:2205.01068*, 2022.
- <span id="page-8-10"></span>[34] B. Wang and A. Komatsuzaki, "Gpt-j-6b: A 6 billion parameter autoregressive language model," 2021.
- <span id="page-8-11"></span>[35] G. Team, T. Mesnard, C. Hardin, R. Dadashi, S. Bhupatiraju, S. Pathak, L. Sifre, M. Riviere, M. S. Kale, J. Love ` *et al.*, "Gemma: Open models based on gemini research and technology," *arXiv preprint arXiv:2403.08295*, 2024.
- <span id="page-8-14"></span>[36] J. Li, J. Chen, R. Ren, X. Cheng, W. X. Zhao, J.-Y. Nie, and J.-R. Wen, "The dawn after the dark: An empirical study on factuality hallucination in large language models," *arXiv preprint arXiv:2401.03205*, 2024.
- <span id="page-8-12"></span>[37] D. R. S. Saputro and P. Widyaningsih, "Limited memory broyden-fletcher-goldfarb-shanno (l-bfgs) method for the parameter estimation on geographically weighted ordinal logistic regression model (gwolr)," *AIP Conference*

- *Proceedings*, 2017.
- <span id="page-8-13"></span>[38] D. P. Kingma and J. Ba, "Adam: A method for stochastic optimization," *arXiv preprint arXiv:1412.6980*, 2014.

## APPENDIX

## *A. True-False Dataset results*

In this appendix section, Table [VI](#page-8-15) provides the exact numerical results of our approach in the True-False dataset, which, as discussed in the paper, yielded significantly weak results. Once again, we want to highlight that while this paper demonstrates competitive and achievable performance on many tasks of HaluEval and HELM using only four features, the performance in this dataset was notably poor. Therefore, we strongly recommend that future hallucination evaluations with supervised approaches be conducted on a dataset like this or on a dataset where a rapid method like ours has been tested to ensure no clear probabilistic pattern.

## *B. HELM results without condition-text*

In this section, Table [VII](#page-8-16) presents the results of our approach in the HELM benchmark using the token probabilities obtained solely from the *generated-text*. We observe how the performance was impacted when using most of the LLM<sup>E</sup> models in comparison with the results in Table [IV.](#page-5-0) This

<span id="page-8-15"></span>TABLE VI RESULTS OF OUR APPROACH AND PREVIOUS METHODS IN THE TRUE-FALSE DATASET MEASURED IN ACCURACY. THE SALPMA RESULTS SHOWN ARE USING THE 16TH HIDDEN LAYER WITH LLC-7B.

| Model                 | Cities           | Invent.          | Elem.            | Anim.            | Comp             | Facts            |
|-----------------------|------------------|------------------|------------------|------------------|------------------|------------------|
| BERT-5-shot<br>SAPLMA | 0.5416<br>0.9223 | 0.4799<br>0.8938 | 0.5676<br>0.6939 | 0.5643<br>0.7774 | 0.5540<br>0.8658 | 0.5148<br>0.8254 |
| Ours                  |                  |                  |                  |                  |                  |                  |
| GPT-2                 | 0.4312           | 0.5353           | 0.4924           | 0.4920           | 0.5041           | 0.5049           |
| BART                  | 0.3846           | 0.5365           | 0.5172           | 0.4920           | 0.4550           | 0.4607           |
| LED                   | 0.4985           | 0.4954           | 0.5182           | 0.5357           | 0.5191           | 0.4787           |
| OPT                   | 0.4950           | 0.5479           | 0.5118           | 0.4573           | 0.5050           | 0.5392           |
| GPT-J                 | 0.5023           | 0.5308           | 0.5268           | 0.4871           | 0.5283           | 0.5408           |
| LLC-7b                | 0.5182           | 0.5216           | 0.5267           | 0.5287           | 0.5208           | 0.5669           |
| Gemma                 | 0.5091           | 0.5205           | 0.4870           | 0.4692           | 0.4983           | 0.4705           |

<span id="page-8-16"></span>TABLE VII RESULTS OF OUR APPROACH AND PREVIOUS METHODS IN THE HELM BENCHMARK MEASURED IN PRAUC WITHOUT *condition-text*.

| Baselines      | Falcon           | GPT<br>J         | LLB<br>7B        | LLC<br>13B       | LLC<br>7B        | OPT              |
|----------------|------------------|------------------|------------------|------------------|------------------|------------------|
| PE-max         | 0.6479           | 0.7497           | 0.6851           | 0.4439           | 0.4931           | 0.7263           |
| SCG-NLI        | 0.6846           | 0.8680           | 0.7644           | 0.5834           | 0.6565           | 0.8103           |
| SAPLMA<br>MIND | 0.5128<br>0.7895 | 0.6987<br>0.8774 | 0.5777<br>0.7876 | 0.3047<br>0.6043 | 0.4066<br>0.6755 | 0.6212<br>0.8835 |
| Ours           |                  |                  |                  |                  |                  |                  |
| GPT-2          | 0.7110           | 0.8097           | 0.7384           | 0.5194           | 0.6085           | 0.7994           |
| BART           | 0.6853           | 0.8129           | 0.7139           | 0.5624           | 0.5686           | 0.8258           |
| LED            | 0.7017           | 0.8424           | 0.6931           | 0.5194           | 0.5494           | 0.8204           |
| OPT            | 0.7163           | 0.7748           | 0.6695           | 0.5608           | 0.6751           | 0.7773           |
| GPT-J          | 0.7051           | 0.7873           | 0.6984           | 0.6121           | 0.5856           | 0.7989           |
| LLC-7b         | 0.6968           | 0.8403           | 0.7423           | 0.5464           | 0.6370           | 0.8395           |
| Gemma          | 0.6872           | 0.8256           | 0.7319           | 0.5788           | 0.6428           | 0.8265           |

TABLE VIII SCALED LOGISTIC REGRESSION COEFFICIENTS ASSIGNED TO EACH INPUT FEATURE BASED ON THE SELECTED LLM<sup>E</sup> .

<span id="page-9-0"></span>

|        | Summ. |         |       |       |       | QA     |        |        | KGD   |       |      |       |
|--------|-------|---------|-------|-------|-------|--------|--------|--------|-------|-------|------|-------|
| Models | mtp   | avgt    | Mpd   | mps   | mtp   | avgt   | Mpd    | mps    | mtp   | avgt  | Mpd  | mps   |
| GPT-2  | 1.0   | 127.8   | 0.69  | 17.69 | 1.01  | 1.0    | 0.01   | 957.36 | 1.0   | 1.1   | 0.03 | 15.88 |
| BART   | 0.14  | 1.04    | 51.28 | 0.017 | 0.005 | 0.0017 | 1.49   | 0.48   | 1.5   | 0.003 | 4.4  | 2.03  |
| LED    | 0.05  | 0.81    | 0.61  | 1.34  | 1.0   | 1.26   | 0.007  | 31.0   | 24.86 | 0.004 | 0.1  | 2.9   |
| OPT    | 1.0   | 1791.86 | 0.93  | 70.01 | 1.0   | 1.0    | 0.006  | 232.66 | 1.0   | 3.02  | 0.02 | 3.41  |
| GPT-J  | 1.0   | 1956.69 | 0.93  | 3.45  | 1.1   | 1.0    | 0.006  | 223.04 | 1.0   | 3.38  | 0.03 | 4.6   |
| LLC-7b | 1.0   | 3.03    | 1.1   | 0.03  | 1.0   | 11.3   | 0.002  | 22.97  | 1.0   | 0.85  | 0.03 | 3.94  |
| Gemma  | 1.0   | 2.36    | 1.0   | 5.75  | 1.0   | 4.7    | 0.0004 | 21.5   | 1.0   | 0.86  | 0.1  | 3.29  |

highlights the significance of the *condition-text* and once again suggests that the anomaly observed in the HaluEval dataset in the Summarization and QA tasks may be attributed to a probabilistic pattern in the *generated-text*.

## *C. Feature Importance Analysis - Logistic Regression Coefficients*

Additionally, we extracted the Logistic Regression coefficients for each feature in each task. Due to the utilization of the logit function, the coefficients in logistic regression signify the logarithm of the odds that an observation belongs to the target class ("1") based on the values of its input variables. Therefore, to interpret these coefficients appropriately, they must be transformed into regular odds. We achieved this by exponentiating the logarithmic odds coefficients, a task easily accomplished using the np.exp() function.

The interpretation in this case can be read as: for each incremental unit rise in the given input variable (for example, the feature avgtp), the likelihood of the observation belonging to the positive class increases by a factor of the value of the coefficient, while maintaining all other variables constant, compared to the odds of it being in the negative class.

Table [VIII](#page-9-0) indicates that for the LR classifier, the most significant features during Summarization were predominantly avgtp, but also occasionally included Mpd and mps. However, in the QA task, the most relevant feature consistently remained mps. Similarly, in the KGD task, mps was predominantly critical, but occasionally mtp and Mpd also played a role. However, it's important to note that LR does not achieve results as strong as the SNN. Therefore, the feature importance of SNN, as shown in Table [V,](#page-6-3) may carry more weight.