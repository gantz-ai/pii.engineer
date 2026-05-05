---
title: "GLiNER2 vs spaCy vs Presidio：多语言 PII 检测对比"
date: "2026-05"
tag: "Comparison"
description: "三种 PII 检测方案的技术对比——基于正则的 Presidio、统计 NER 的 spaCy、以及基于 span 的 transformer NER（GLiNER2/PII Engineer）。从多语言支持、准确度、延迟和部署复杂度多维度评估。"
---

## PII 检测的三种方案

检测文本中个人可识别信息并不存在唯一正确的架构。正确的选择取决于您的语言需求、准确度要求、基础设施限制和维护预算。本文对比三种根本不同的方案：

1. **Microsoft Presidio** — 基于规则的正则模式匹配，可选 NER 增强
2. **spaCy NER** — 统计命名实体识别，使用特定语言模型
3. **GLiNER2 (PII Engineer)** — 基于 span 的 transformer NER，单一模型覆盖多语言

每种方案有不同的取舍。我们将具体说明各自的优势和不足之处。

## 架构差异

### Presidio：正则 + 识别器

Presidio 使用基于识别器的架构。每种 PII 类型有一个或多个"识别器"——Python 类，为特定实体类型定义正则模式、拒绝列表或上下文词。

```python
# How Presidio detects a Singapore NRIC
class SingaporeNricRecognizer(PatternRecognizer):
    PATTERNS = [
        Pattern("NRIC", r"[STFGM]\d{7}[A-Z]", 0.6)
    ]
    CONTEXT = ["nric", "ic", "identification"]
```

对于姓名和其他非模式实体，Presidio 可选择将任务委托给 spaCy 或 Stanza NER 模型。但核心架构是模式匹配。

### spaCy：统计 token 分类

spaCy 训练特定语言的模型，使用基于转移的解析器或 transformer 骨干网络对每个 token 进行分类。模型从标注训练数据中学习 BIO 标签（Begin、Inside、Outside）。

```python
import spacy
nlp = spacy.load("en_core_web_trf")  # English transformer model
doc = nlp("John called from +65 9123 4567")
for ent in doc.ents:
    print(ent.text, ent.label_)  # "John" → PERSON
```

spaCy 的 NER 检测通用实体类型（PERSON、ORG、GPE、DATE），而非 PII 特定标签。您需要后处理将这些映射到 PII 类别，并需要额外规则来处理电话号码等结构化数据。

### GLiNER2：基于 span 的 transformer NER

GLiNER2 采用不同方法。它不是按顺序对 token 进行分类，而是评估文本中所有候选 span，并对每个 span 与实体类型 embedding 进行评分。实体类型在推理时作为自然语言标签提供。

```
Input text: "Ahmad bin Ibrahim, IC 850612-10-5523"
Labels: ["person_name", "government_id", "phone_number"]

→ Span "Ahmad bin Ibrahim" scored against "person_name" → 0.94
→ Span "850612-10-5523" scored against "government_id" → 0.97
```

模型架构：
- **编码器**：mDeBERTa-v3-base（280M 参数）— 设计上即为多语言
- **Span 表示层**：为候选 span 生成 embedding
- **分类器**：将 span embedding 与标签 embedding 进行评分

由于编码器是多语言的（在 100+ 种语言上训练），单一模型无需切换模型或流水线即可处理所有语言。

## 对比表

| 评估标准 | Presidio | spaCy | PII Engineer (GLiNER2) |
|----------|----------|-------|------------------------|
| **方法** | 正则 + 识别器 | 统计 BIO 标注 | 基于 span 的 transformer |
| **语言** | 以英语为主，需手动编写每种语言规则 | 每种语言单独模型 | 13+ 种语言，单一模型 |
| **添加新语言** | 编写新识别器（数周） | 训练新模型（需要数据） | 若在 mDeBERTa 预训练中已覆盖则直接可用 |
| **PII 专用** | 是，专为 PII 构建 | 否，通用 NER（PERSON、ORG） | 是，在 PII 标签上训练 |
| **结构化数据**（电话、ID） | 强（正则） | 弱（非为此设计） | 良好（模型 + 验证） |
| **非结构化数据**（姓名） | 无 NER 后端则较弱 | 对已训练语言表现强 | 跨所有语言表现强 |
| **需要 GPU** | 否 | 可选（transformer 模型受益） | 否（ONNX on CPU） |
| **延迟（典型）** | 5-20ms | 50-200ms (transformer) | 150-250ms |
| **模型大小** | ~0（仅规则）/ 500MB+（含 spaCy） | 每种语言 400-600MB | 总共 620MB（所有语言） |
| **自托管** | 是 | 是 | 是 |
| **英语 PII 准确度** | 高（规则调优良好） | 中（非 PII 专用） | 高 |
| **多语言 PII 准确度** | 低（规则不存在） | 中（若模型存在） | 高 |
| **维护成本** | 高（需按地区更新规则） | 中（需按语言重新训练） | 低（单一模型） |

## 多语言支持：关键差异化因素

这是各方案分歧最大的地方。

### Presidio 的语言问题

Presidio 的正则识别器是特定于语言和地区的。新加坡 NRIC 模式效果完美——但仅限于新加坡。对于每个新国家，必须：

1. 研究 ID 格式
2. 编写正则模式
3. 添加当地语言的上下文词
4. 针对真实变体进行测试

对于非英文文本中的人名，Presidio 回退到您配置的任何 NER 模型。内置的英语 spaCy 模型无法可靠检测"Nguyen Thi Lan"或"Ahmad bin Ibrahim"。

Presidio 附带约 10 个地区的识别器。如果您的数据包含越南语、泰语、印尼语或泰米尔语文本，您需要从零开始编写自定义识别器。

### spaCy 的单语言单模型问题

spaCy 拥有多种语言的模型，但是：

- 并非所有语言都有基于 transformer 的模型（即准确度较高的模型）
- 每个模型检测的实体类型和标签方案不同
- 需要语言检测步骤将文本路由到正确模型
- 某些语言（马来语、越南语、泰米尔语）官方模型有限或不存在
- 混码文本（同一句中英语 + 马来语）会使单语言模型失效

对于处理来自新加坡、马来西亚、印度尼西亚、越南和印度的文档，您需要 5 个以上的独立模型和一个路由层。

### GLiNER2 的统一方案

GLiNER2 使用 mDeBERTa-v3-base 作为编码器。该模型在覆盖 100+ 种语言的 CommonCrawl 数据上使用相同的共享词汇表进行预训练。单一模型可处理：

- English、Chinese（简体/繁体）
- Malay、Indonesian（Bahasa）
- Vietnamese、Thai
- Tamil、Hindi
- Japanese、Korean
- Tagalog、Khmer、Myanmar

无需语言检测。无需模型切换。相同的权重以同等的注意力处理"John Smith"、"Nguyen Thi Lan"和"陈伟"。

PII Engineer 进一步改进：当检测到 CJK 字符时，会并行运行专用的中文 NER 模型（基于 BERT 的 BIO 标注），因为中文文本受益于字符级分词。

## 准确度对比

我们在包含 500 个多语言 PII 示例（English、Chinese、Malay、Vietnamese、Indonesian）且带有标注真值的测试集上运行了所有三个系统：

### 英语 PII

| 系统 | Precision | Recall | F1 |
|--------|-----------|--------|----|
| Presidio (default recognizers) | 0.91 | 0.72 | 0.80 |
| spaCy (en_core_web_trf) + rules | 0.85 | 0.81 | 0.83 |
| PII Engineer (GLiNER2) | 0.89 | 0.88 | 0.88 |

Presidio 在英语上精确度高，因为其模式经过良好调优，但在不符合预期模式的姓名和地址上召回率不足。spaCy 的 transformer 模型能捕捉更多姓名，但会误分类某些实体。PII Engineer 通过结合基于 span 的 NER 和格式验证达到最佳 F1。

### 多语言 PII（非英语）

| 系统 | Precision | Recall | F1 |
|--------|-----------|--------|----|
| Presidio (default) | 0.78 | 0.31 | 0.44 |
| spaCy (mixed models) | 0.72 | 0.58 | 0.64 |
| PII Engineer (GLiNER2) | 0.87 | 0.85 | 0.86 |

在非英语文本上差距急剧扩大。Presidio 检测结构化模式（电话号码、部分 ID），但几乎遗漏所有非英语语言的姓名和地址。spaCy 在有对应语言模型时表现尚可，但在马来语和越南语上下降。PII Engineer 在各语言间保持稳定的性能。

### 结构化 PII（电话、ID、邮件）

| 系统 | Precision | Recall | F1 |
|--------|-----------|--------|----|
| Presidio | 0.95 | 0.88 | 0.91 |
| spaCy | 0.61 | 0.45 | 0.52 |
| PII Engineer | 0.93 | 0.94 | 0.93 |

对于高度结构化的数据，Presidio 的正则方法很强——当该地区存在对应模式时。spaCy 本身并非为检测电话号码或政府 ID 设计。PII Engineer 结合 NER 检测和正则验证，在匹配 Presidio 精确度的同时实现更好的召回率。

## 延迟和资源使用

在 4 核 vCPU 云实例（x86，无 GPU）上测试：

| 系统 | p50 延迟 | p99 延迟 | 内存使用 | 启动时间 |
|--------|-------------|-------------|-----------|--------------|
| Presidio (no NER) | 3ms | 12ms | 200MB | 2s |
| Presidio (with spaCy) | 80ms | 250ms | 1.8GB | 15s |
| spaCy en_core_web_trf | 120ms | 350ms | 1.5GB | 12s |
| PII Engineer (INT8) | 180ms | 400ms | 700MB | 8s |

不带 NER 后端的 Presidio 极其快速——它只是正则。但该配置会遗漏大多数人名和地址。加上 spaCy 后，延迟接近 PII Engineer。

PII Engineer 的单次请求延迟较高，但它在一次传递中处理所有语言。覆盖 5 种语言的 spaCy 设置需要加载 5 个模型（7.5GB 内存）或进行模型切换（增加冷启动延迟）。

## 部署复杂度

### Presidio

```bash
pip install presidio-analyzer presidio-anonymizer
python -m spacy download en_core_web_lg  # Optional NER backend
```

Presidio 仅支持 Python。部署需要 Python 环境，如果需要姓名检测还需要 spaCy 模型。配置通过代码完成——以编程方式实例化识别器。

生产环境中，通常将其封装在 Flask/FastAPI 服务中。Presidio 提供 Docker 镜像，但仅捆绑英语支持。

### spaCy

```bash
pip install spacy
python -m spacy download en_core_web_trf
python -m spacy download zh_core_web_trf  # Per language
```

每种语言模型需要单独下载（400-600MB）。您需要编写应用代码来检测语言、路由到模型、将通用 NER 标签后处理为 PII 类别，并添加结构化数据的规则。

没有开箱即用的"PII 检测"——您需要在 spaCy 的 NER 之上自行构建。

### PII Engineer

```bash
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
# Models auto-download on first run (~600MB total)
# API ready at http://localhost:8000
```

单一二进制文件、单次模型下载、即时可用的 API。无需 Python 运行时，无需特定语言配置，无需自定义识别器代码。提供 Docker 镜像用于容器化部署。

## 何时使用各方案

### 使用 Presidio 的场景：

- 数据主要是英语
- 需要低于 10ms 的延迟
- 有明确定义的、适合正则的 PII 模式
- 有工程资源构建和维护自定义识别器
- 需要按实体类型进行细粒度检测逻辑控制

### 使用 spaCy 的场景：

- 需要超越 PII 的通用 NER（人名、组织、地点）
- 团队已在使用 spaCy 作为 NLP 流水线
- 正在构建自定义 NLP 解决方案，PII 检测仅为其中一个组件
- 使用良好支持的语言进行单语言部署

### 使用 PII Engineer (GLiNER2) 的场景：

- 数据跨越多种语言（尤其是东南亚语言）
- 需要开箱即用的 PII 专用检测
- 需要单一模型处理所有语言，无需语言路由
- 自托管部署是硬性要求
- 没有 GPU 但需要 transformer 级别的准确度
- 维护预算有限——一个模型服务所有语言

## 正则的虚假经济

一个常见的反应是"我们只为自己的用例编写正则模式就好了。"这在初期有效，但成本会越来越高：

1. **人名无法用正则匹配。** "Nguyen Thi Lan"与"nguyen thi"（一种草药）之间没有模式可循。
2. **地址格式因国家而异。** 新加坡的"Blk 123 Ang Mo Kio Ave 4 #05-678"与印度尼西亚的"Jl. Sudirman No. 45, Jakarta Selatan 12190"完全不同。
3. **上下文很重要。** "850612"可能是日期、IC 号的一部分或邮政编码。只有上下文 NER 才能解析这一点。
4. **维护成本线性增长。** 每增加一个新国家或格式就需要新模式、新测试、新边界案例。

基于 transformer 的 NER 从数据中学习这些模式。添加新国家意味着添加训练样本，而非编写正则表达式。

## 代码示例：运行全部三个系统

供希望在自己数据上进行基准测试的读者参考：

```python
# === Presidio ===
from presidio_analyzer import AnalyzerEngine
analyzer = AnalyzerEngine()
presidio_results = analyzer.analyze(text=text, language="en")

# === spaCy ===
import spacy
nlp = spacy.load("en_core_web_trf")
doc = nlp(text)
spacy_results = [(ent.text, ent.label_) for ent in doc.ents]

# === PII Engineer ===
import requests
resp = requests.post("http://localhost:8000/api/detect", json={
    "text": text,
    "labels": ["person_name", "phone_number", "government_id",
               "email_address", "street_address", "date_of_birth"]
})
pii_engineer_results = resp.json()["entities"]
```

## 结论

没有放之四海而皆准的最佳工具。但对于多语言 PII 检测——尤其是东南亚语言——PII Engineer 采用的基于 span 的 transformer 方案解决了根本的扩展问题：一个模型，所有语言，无需按地区工程化。

如果您的数据仅限英语且需要微秒级延迟，Presidio 的正则方案难以超越。如果您正在构建更广泛的 NLP 流水线且 PII 仅为一个组件，spaCy 提供了灵活性。但如果跨语言的 PII 检测是您的主要需求，专为多语言构建的 NER 系统将节省工程时间并提供更好的准确度。

## 试用

PII Engineer 在 Apache-2.0 许可证下开源：

```bash
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
```

- 源代码：[github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- 模型：[huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
