---
title: "PDPA 合规：东南亚 PII 检测自动化"
date: "2026-05"
tag: "Compliance"
description: "如何使用 PII Engineer 扫描文档、聊天记录和数据库，满足 PDPA、PDPD、PDP Law 和 PIPL 法规对个人数据的要求。"
---

## 监管环境

东南亚和东亚地区已快速推进数据保护立法。跨区域运营的组织面对的是一套错综复杂的法规要求——每部法律对"个人数据"的定义略有不同，对数据收集、存储和处理也有不同的义务规定。

| 法律 | 国家 | 颁布年份 | 核心范围 |
|------|------|----------|----------|
| PDPA | Singapore | 2012 | 任何可识别个人的数据，包括 NRIC、电话、地址 |
| PDPA | Malaysia | 2010 | 涉及个人数据处理的商业交易 |
| PDPD | Vietnam | 2023 | 基本和敏感个人数据，包括生物特征、健康、财务信息 |
| PDP Law | Indonesia | 2022 | 一般和特定个人数据（NIK、健康、生物特征） |
| PIPL | China | 2021 | 广义的个人信息范围，包括姓名、证件号、电话、位置 |

## 常见合规任务

无论适用哪部法规，实际操作要求大体相似：

1. **数据盘点** — 识别 PII 在系统各处的分布位置
2. **权限审计** — 验证仅授权人员可访问 PII
3. **数据最小化** — 删除或脱敏不再需要的 PII
4. **泄露检测** — 监控日志、导出文件或消息中的未授权 PII 暴露
5. **同意管理** — 确保 PII 收集时已获得适当授权

PII Engineer 解决任务 1、3 和 4——检测 PII 出现的所有位置、按需脱敏，以及实时标记数据暴露。

## 各辖区的 PII 类型

不同法律侧重不同的数据类型。PII Engineer 的 9 种实体类型覆盖了最常受监管的类别：

| PII 类型 | PDPA (SG) | PDPD (VN) | PDP Law (ID) | PIPL (CN) |
|----------|-----------|-----------|--------------|-----------|
| person_name | ✓ | ✓ | ✓ | ✓ |
| phone_number | ✓ | ✓ | ✓ | ✓ |
| government_id | NRIC/FIN | CCCD | NIK/KTP | 身份证 |
| street_address | ✓ | ✓ | ✓ | ✓ |
| date_of_birth | ✓ | ✓ | ✓ | ✓ |
| email_address | ✓ | ✓ | ✓ | ✓ |
| passport_number | ✓ | ✓ | ✓ | ✓ |
| license_plate | — | ✓ | ✓ | ✓ |
| bank_account_number | ✓ | ✓ | ✓ | ✓ |

## 扫描文档

PII Engineer 通过 REST API 接受任意文本输入。扫描文档时，先提取文本（使用 PDF 解析器、OCR 或纯文本读取器），然后发送至检测端点：

```
curl -X POST http://localhost:8000/api/detect \
  -H "Content-Type: application/json" \
  -d '{"text": "Patient Nguyen Van An, CCCD 012345678901, phone 0901234567"}'
```

响应包含检测到的实体及其类型、位置、置信度分数，以及文本的脱敏版本：

```json
{
  "entities": [
    {"label": "person_name", "text": "Nguyen Van An", "score": 0.87},
    {"label": "government_id", "text": "012345678901", "score": 0.92},
    {"label": "phone_number", "text": "0901234567", "score": 0.95}
  ],
  "redacted": "Patient [PERSON_NAME], CCCD [GOVERNMENT_ID], phone [PHONE_NUMBER]"
}
```

## 语言覆盖

与那些在处理东南亚文本时表现不佳的纯英文工具不同，PII Engineer 原生支持 13+ 种语言。GLiNER2 模型在涵盖以下语种的多语言数据上训练：

- **主要语言：** English, Malay, Tamil, Chinese, Indonesian, Vietnamese
- **次要语言：** Thai, Hindi, Bengali, Korean, German, French, Russian

这意味着单一部署即可处理印尼语的客服工单、越南语的病历记录、中文的法律合同和马来语的人力资源文件——无需切换模型或配置。

## 政府证件格式

各国使用不同的证件格式，模型能够识别所有这些格式：

- **Singapore NRIC:** S1234567A（字母 + 7 位数字 + 字母）
- **Vietnam CCCD:** 012345678901（12 位数字）
- **Indonesia NIK:** 3201234567890001（16 位数字）
- **China 身份证:** 110101199001011234（18 位数字）
- **India Aadhaar:** 1234 5678 9012（12 位数字，常带空格）
- **Malaysia MyKad:** 901231-14-1234（12 位数字带连字符）

后处理验证阶段确保检测到的证件号符合预期格式，从而减少误报。

## 实时监控

对于需要持续监控的合规团队，PII Engineer 可作为服务部署，用于扫描：

- 客服聊天消息（存储前检测）
- 数据库导出和备份（传输前检测）
- 日志文件中意外记录的 PII
- 邮件内容中的数据泄露

在 Apple Silicon 上约 150ms、在服务器 CPU 上约 250ms 的延迟下，可实现无感知的实时扫描。

## 自托管 = 数据留在本地

合规场景下的关键优势：PII Engineer 完全运行在你自己的基础设施上，没有数据离开你的网络。这消除了"为了检测 PII 而将 PII 发送到云服务"这一常见悖论——许多 SaaS 扫描工具都存在这个问题。

服务器运行在 CPU 上，无需 GPU，首次运行时自动从 HuggingFace 下载模型。一条 `cargo run` 命令即可获得一个生产就绪的端点。

## 快速开始

```
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
# 模型从 HuggingFace 自动下载
# 服务启动于 http://localhost:8000
```

若需大规模合规扫描，可将其部署在现有的 API 网关之后并集成至数据管道。AGPL-3.0 协议允许在开源系统中免费使用；商业部署可获取商业许可。

源代码：[github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
