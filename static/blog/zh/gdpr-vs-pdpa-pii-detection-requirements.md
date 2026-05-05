---
title: "GDPR 与 PDPA：PII 检测要求对比"
date: "2026-05"
tag: "Compliance"
description: "欧盟 GDPR 与东南亚 PDPA 法规的并列比较——什么算作个人数据、检测要求有何不同，以及 PII Engineer 如何映射到这两个框架。"
---

## 两个框架，一个问题

在欧洲和东南亚运营的组织面临重叠但不同的隐私法规。欧盟的《通用数据保护条例》（GDPR）和新加坡的《个人数据保护法》（PDPA）都要求组织识别、保护和管理个人数据——但它们对范围、义务和处罚的定义不同。

自动化 PII 检测需要兼顾两个框架。仅针对 GDPR 调优的系统会遗漏东南亚的标识符。为 PDPA 构建的系统可能无法覆盖 GDPR 下更广泛的"个人数据"定义。

## 什么算作个人数据？

### GDPR 定义（第 4 条）

> "与已识别或可识别的自然人相关的任何信息。"

这是刻意宽泛的定义。它包括：

- 直接标识符：姓名、证件号码、电子邮件
- 间接标识符：位置数据、IP 地址、Cookie ID
- 特殊类别：种族/民族出身、政治观点、健康数据、生物特征数据、性取向

GDPR 的范围扩展到*可能*识别某人的任何数据，即使是与其他数据组合后才能识别。

### PDPA 定义（第 2 条）

> "关于可从该数据识别的个人的数据（无论真实与否）；或从该数据和组织拥有或可能拥有访问权的其他信息中识别的个人的数据。"

PDPA 在实践中更窄：

- 侧重于识别*特定*个人的数据
- 没有明确的"特殊类别"概念（尽管"请勿来电"登记制度增加了电话特定规则）
- 用于商业目的的商业联系信息通常被排除

### 实际差异

| 方面 | GDPR | PDPA（新加坡） |
|---|---|---|
| 范围 | 可能识别个人的任何数据 | 识别特定个人的数据 |
| IP 地址 | 个人数据 | 未明确涵盖 |
| Cookie ID | 个人数据（配合 ePrivacy） | 通常不属于个人数据 |
| 名片 | 个人数据 | 用于商业目的时排除 |
| 已故人员 | 不涵盖 | 不涵盖 |
| 匿名化数据 | 如果真正匿名则不属于个人数据 | 如果真正匿名则不属于个人数据 |
| 假名化数据 | 仍属于个人数据 | 取决于重新识别的风险 |

## 关键义务对比

### 同意

| | GDPR | PDPA |
|---|---|---|
| 默认 | 需要选择加入 | 需要同意（默示或明示） |
| 合法利益 | 是——可以无需同意处理 | 无等效概念——需要同意或例外 |
| 撤回 | 必须与给予同意同样简便 | 必须能够撤回 |
| 儿童 | 16 岁以下需要家长同意 | 无特定年龄门槛 |

### 数据泄露通知

| | GDPR | PDPA |
|---|---|---|
| 监管机构通知 | 72 小时 | "尽快"（2021 年更新：3 个日历日） |
| 个人通知 | 当对权利有"高风险"时 | 当"重大伤害"可能发生时 |
| 门槛 | 基于风险评估 | 500+ 人或重大伤害 |

### 跨境传输

| | GDPR | PDPA |
|---|---|---|
| 默认规则 | 除非有充分保护否则禁止 | 除非有同等保护否则禁止 |
| 机制 | 充分性决定、SCC、BCR | 合同安排、有约束力的企业规则 |
| 关键关注点 | 数据流向非欧盟国家 | 数据流向新加坡以外 |

## PII 实体类型：覆盖映射

PII Engineer 检测 9 种实体类型。以下是每种类型如何映射到 GDPR 和 PDPA 要求：

| 实体类型 | GDPR 相关性 | PDPA 相关性 | 检测优先级 |
|---|---|---|---|
| `person_name` | 直接标识符（第 4 条） | 直接标识符（第 2 条） | 对两者都是关键 |
| `government_id` | 国民身份证号码（第 87 条） | NRIC（咨询指引） | 对两者都是关键 |
| `phone_number` | 联系数据 | DNC 登记制度（第 IX 部分） | 对两者都是高优先级 |
| `email_address` | 直接标识符 | 直接标识符 | 对两者都是高优先级 |
| `street_address` | 位置数据（鉴于条款 30） | 直接标识符 | 对两者都是高优先级 |
| `date_of_birth` | 间接标识符 | 间接标识符 | 中等——组合时风险更高 |
| `passport_number` | 旅行证件（第 87 条） | 政府签发的证件 | 对两者都是关键 |
| `bank_account_number` | 金融数据 | 金融数据 | 对两者都是高优先级 |
| `license_plate` | 间接标识符（可追踪车主） | 间接标识符 | 对两者都是中等 |

### GDPR 特定的覆盖缺口

GDPR 更广泛的定义意味着某些个人数据类型超出了标准 NER 检测的范围：

- **IP 地址** —— 不像实体，但在 GDPR 下属于个人数据。在 NER 之外使用正则表达式模式。
- **Cookie 标识符** —— 技术字符串，不是自然语言实体
- **遗传/生物特征数据** —— 特殊类别，需要领域特定的检测
- **位置数据** —— GPS 坐标、基站 ID。PII Engineer 检测街道地址但不检测原始坐标。

### PDPA 特定的重点

新加坡的 PDPA 特别强调：

- **NRIC 号码** —— 新加坡专门发布了限制 NRIC 收集的咨询指引。PII Engineer 验证 NRIC 格式（字母 + 7 位数字 + 校验字母）。
- **电话号码** —— "请勿来电"（DNC）登记制度使电话号码检测对营销合规特别重要。

## 按法规的检测策略

### 用于 GDPR 合规

```python
# GDPR: detect all standard PII types + augment with regex for technical identifiers
entities = detect_pii(text, labels=[
    "person_name",
    "government_id",
    "phone_number",
    "email_address",
    "street_address",
    "date_of_birth",
    "passport_number",
    "bank_account_number",
    "license_plate"
])

# Supplement with regex for GDPR-specific technical identifiers
import re
ip_pattern = re.compile(r'\b(?:\d{1,3}\.){3}\d{1,3}\b')
ips = [{"label": "ip_address", "text": m.group(), "start": m.start(), "end": m.end()}
       for m in ip_pattern.finditer(text)]
entities.extend(ips)
```

### 用于 PDPA 合规

```python
# PDPA: focus on direct identifiers, especially NRIC and phone
entities = detect_pii(text, labels=[
    "person_name",
    "government_id",    # catches NRIC, FIN
    "phone_number",     # DNC registry compliance
    "email_address",
    "street_address",
    "date_of_birth"
])

# Filter for Singapore-specific ID formats if needed
sg_entities = [e for e in entities
    if e["label"] != "government_id"
    or re.match(r'^[STFGM]\d{7}[A-Z]$', e["text"])]
```

### 用于双重合规（GDPR + PDPA）

```python
# Use the full label set — covers both frameworks
entities = detect_pii(text)

# Classify each entity by regulatory relevance
for entity in entities:
    entity["gdpr_relevant"] = True  # All PII types are GDPR-relevant
    entity["pdpa_relevant"] = entity["label"] in [
        "person_name", "government_id", "phone_number",
        "email_address", "street_address", "date_of_birth"
    ]
```

## 自托管：合规捷径

GDPR 和 PDPA 都对跨境数据传输施加限制。使用基于云的 PII 检测服务会产生数据传输事件——您正在将个人数据发送给第三方进行处理。

自托管 PII Engineer 完全消除了这一问题：

| 合规关注点 | 云端 PII 服务 | 自托管 PII Engineer |
|---|---|---|
| GDPR 数据处理者协议 | 必需（第 28 条） | 不需要 |
| PDPA 数据中介 | 必须通过合同约束 | 不适用 |
| 跨境传输评估 | 如果服务在管辖范围外则必需 | 不发生传输 |
| 数据处理影响评估 | 可能需要（第 35 条） | 简化——无外部处理 |
| 供应商审计权 | 必须协商 | 不适用 |

## 区域 PII 模式

PII Engineer 的多语言模型处理欧盟和东盟中不同的个人数据模式：

| 地区 | 姓名模式 | 证件格式 | 地址格式 |
|---|---|---|---|
| EU (Germany) | "Hans Müller" | Personalausweisnr | Straße + PLZ + Stadt |
| EU (France) | "Jean-Pierre Dubois" | Numéro de sécurité sociale | Rue + Code postal + Ville |
| Singapore | "Tan Ah Kow" / "S. Ramasamy" | NRIC: S9012345A | Blk + Street + #unit |
| Malaysia | "Ahmad bin Ismail" | MyKad: 820315-10-5523 | Jalan + Taman + Poskod |
| Vietnam | "Nguyen Thi Lan" | CCCD: 024198006789 | Số + Đường + Quận |
| Indonesia | "Budi Santoso" | NIK: 3201011234560001 | Jalan + RT/RW + Kecamatan |

GLiNER2 模型无需语言特定配置即可处理所有这些模式。单个 API 调用即可检测 PII，无论输入文本中使用的语言或区域格式。

## 建议

1. **从完整检测开始** —— 使用全部 9 种实体类型。在下游过滤结果比遗漏实体更容易。
2. **映射到您的具体义务** —— 并非每个检测到的实体都需要相同的处理。GDPR 特殊类别比标准个人数据需要更强的保护。
3. **记录您的检测覆盖范围** —— GDPR（第 30 条）和 PDPA（第 24 条）都要求展示您的数据保护措施。有文档记录的 PII 检测管线加强了您的合规态势。
4. **自托管以简化** —— 消除数据处理者关系可以移除整个类别的合规义务。

## 源代码

PII Engineer 在 Apache-2.0 下开源：

- 仓库：[github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- 模型：[huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
