---
title: "GLiNER2 vs spaCy vs Presidio: So Sánh Phát Hiện PII Đa Ngôn Ngữ"
date: "2026-05"
tag: "Comparison"
description: "So sánh kỹ thuật ba phương pháp phát hiện PII — dựa trên regex (Presidio), NER thống kê (spaCy), và NER transformer dựa trên span (GLiNER2/PII Engineer). Đánh giá về hỗ trợ đa ngôn ngữ, độ chính xác, độ trễ và độ phức tạp triển khai."
---

## Ba Phương Pháp Phát Hiện PII

Không có một kiến trúc duy nhất nào đúng cho việc phát hiện thông tin nhận dạng cá nhân trong văn bản. Lựa chọn đúng phụ thuộc vào yêu cầu ngôn ngữ, nhu cầu về độ chính xác, ràng buộc hạ tầng và ngân sách bảo trì. Bài viết này so sánh ba phương pháp cơ bản khác nhau:

1. **Microsoft Presidio** — dựa trên quy tắc với mẫu regex và tùy chọn tăng cường NER
2. **spaCy NER** — nhận dạng thực thể có tên bằng thống kê với model riêng cho từng ngôn ngữ
3. **GLiNER2 (PII Engineer)** — NER transformer dựa trên span với phủ sóng đa ngôn ngữ từ một model duy nhất

Mỗi phương pháp có đánh đổi khác nhau. Chúng tôi sẽ chỉ rõ điểm mạnh và điểm yếu của từng phương pháp.

## Khác Biệt Kiến Trúc

### Presidio: Regex + Recognizers

Presidio sử dụng kiến trúc dựa trên recognizer. Mỗi loại PII có một hoặc nhiều "recognizers" — các class Python định nghĩa mẫu regex, danh sách từ chối, hoặc từ ngữ cảnh cho loại thực thể cụ thể.

```python
# How Presidio detects a Singapore NRIC
class SingaporeNricRecognizer(PatternRecognizer):
    PATTERNS = [
        Pattern("NRIC", r"[STFGM]\d{7}[A-Z]", 0.6)
    ]
    CONTEXT = ["nric", "ic", "identification"]
```

Đối với tên và các thực thể không theo mẫu khác, Presidio tùy chọn ủy quyền cho model spaCy hoặc Stanza NER. Nhưng kiến trúc cốt lõi là khớp mẫu.

### spaCy: Phân Loại Token Thống Kê

spaCy huấn luyện model riêng cho từng ngôn ngữ, phân loại từng token bằng parser dựa trên chuyển trạng thái hoặc backbone transformer. Model học các tag BIO (Begin, Inside, Outside) từ dữ liệu huấn luyện có chú thích.

```python
import spacy
nlp = spacy.load("en_core_web_trf")  # English transformer model
doc = nlp("John called from +65 9123 4567")
for ent in doc.ents:
    print(ent.text, ent.label_)  # "John" → PERSON
```

NER của spaCy phát hiện các loại thực thể chung (PERSON, ORG, GPE, DATE) thay vì nhãn đặc thù PII. Bạn cần hậu xử lý để ánh xạ sang danh mục PII và thêm quy tắc cho dữ liệu có cấu trúc như số điện thoại.

### GLiNER2: NER Transformer Dựa Trên Span

GLiNER2 có cách tiếp cận khác. Thay vì phân loại token tuần tự, nó đánh giá tất cả span ứng viên trong văn bản và chấm điểm mỗi span so với embedding loại thực thể. Các loại thực thể được cung cấp tại thời điểm suy luận dưới dạng nhãn ngôn ngữ tự nhiên.

```
Input text: "Ahmad bin Ibrahim, IC 850612-10-5523"
Labels: ["person_name", "government_id", "phone_number"]

→ Span "Ahmad bin Ibrahim" scored against "person_name" → 0.94
→ Span "850612-10-5523" scored against "government_id" → 0.97
```

Kiến trúc model:
- **Encoder**: mDeBERTa-v3-base (280M params) — đa ngôn ngữ theo thiết kế
- **Lớp biểu diễn span**: tạo embedding cho các span ứng viên
- **Classifier**: chấm điểm embedding span so với embedding nhãn

Vì encoder là đa ngôn ngữ (huấn luyện trên 100+ ngôn ngữ), một model duy nhất xử lý tất cả ngôn ngữ mà không cần chuyển đổi model hay pipeline.

## Bảng So Sánh

| Tiêu chí | Presidio | spaCy | PII Engineer (GLiNER2) |
|----------|----------|-------|------------------------|
| **Phương pháp** | Regex + recognizers | BIO tagging thống kê | Transformer dựa trên span |
| **Ngôn ngữ** | Ưu tiên tiếng Anh, quy tắc thủ công cho mỗi ngôn ngữ | Model riêng cho mỗi ngôn ngữ | 13+ ngôn ngữ, một model duy nhất |
| **Thêm ngôn ngữ** | Viết recognizers mới (hàng tuần) | Huấn luyện model mới (cần dữ liệu) | Đã được hỗ trợ nếu nằm trong pretraining mDeBERTa |
| **Chuyên biệt PII** | Có, xây dựng cho PII | Không, NER chung (PERSON, ORG) | Có, huấn luyện trên nhãn PII |
| **Dữ liệu có cấu trúc** (điện thoại, ID) | Mạnh (regex) | Yếu (không thiết kế cho mục này) | Tốt (model + validation) |
| **Dữ liệu phi cấu trúc** (tên) | Yếu nếu không có NER backend | Mạnh cho các ngôn ngữ đã huấn luyện | Mạnh trên tất cả ngôn ngữ |
| **Cần GPU** | Không | Tùy chọn (model transformer hưởng lợi) | Không (ONNX trên CPU) |
| **Độ trễ (điển hình)** | 5-20ms | 50-200ms (transformer) | 150-250ms |
| **Kích thước model** | ~0 (chỉ quy tắc) / 500MB+ (với spaCy) | 400-600MB cho mỗi ngôn ngữ | 620MB tổng (tất cả ngôn ngữ) |
| **Tự lưu trữ** | Có | Có | Có |
| **Độ chính xác PII tiếng Anh** | Cao (quy tắc tinh chỉnh tốt) | Trung bình (không chuyên biệt PII) | Cao |
| **Độ chính xác PII đa ngôn ngữ** | Thấp (quy tắc không tồn tại) | Trung bình (nếu model tồn tại) | Cao |
| **Bảo trì** | Cao (cập nhật quy tắc cho mỗi vùng) | Trung bình (huấn luyện lại cho mỗi ngôn ngữ) | Thấp (một model duy nhất) |

## Hỗ Trợ Đa Ngôn Ngữ: Yếu Tố Khác Biệt Chính

Đây là nơi các phương pháp phân hóa rõ rệt nhất.

### Vấn Đề Ngôn Ngữ Của Presidio

Các regex recognizer của Presidio là đặc thù cho ngôn ngữ và vùng. Mẫu NRIC Singapore hoạt động hoàn hảo — nhưng chỉ cho Singapore. Với mỗi quốc gia mới, ai đó phải:

1. Nghiên cứu định dạng ID
2. Viết mẫu regex
3. Thêm từ ngữ cảnh bằng ngôn ngữ địa phương
4. Kiểm thử với các biến thể thực tế

Đối với tên người trong văn bản không phải tiếng Anh, Presidio dựa vào model NER mà bạn cấu hình. Model spaCy tiếng Anh tích hợp sẽ không phát hiện "Nguyen Thi Lan" hay "Ahmad bin Ibrahim" một cách đáng tin cậy.

Presidio cung cấp recognizers cho khoảng 10 vùng. Nếu dữ liệu của bạn bao gồm văn bản tiếng Việt, tiếng Thái, Bahasa Indonesia, hoặc tiếng Tamil, bạn sẽ phải viết recognizers tùy chỉnh từ đầu.

### Vấn Đề Một-Model-Mỗi-Ngôn-Ngữ Của spaCy

spaCy có model cho nhiều ngôn ngữ, nhưng:

- Không phải tất cả ngôn ngữ đều có model dựa trên transformer (loại chính xác)
- Mỗi model phát hiện loại thực thể khác nhau với lược đồ nhãn khác nhau
- Bạn cần bước phát hiện ngôn ngữ để định tuyến văn bản đến model đúng
- Một số ngôn ngữ (tiếng Mã Lai, tiếng Việt, tiếng Tamil) có model hạn chế hoặc không có model chính thức
- Văn bản trộn mã (tiếng Anh + tiếng Mã Lai trong cùng câu) làm hỏng model đơn ngôn ngữ

Đối với hệ thống xử lý tài liệu từ Singapore, Malaysia, Indonesia, Vietnam và Ấn Độ — bạn sẽ cần 5+ model riêng biệt và một lớp định tuyến.

### Phương Pháp Thống Nhất Của GLiNER2

GLiNER2 sử dụng mDeBERTa-v3-base làm encoder. Model này được pretrain trên dữ liệu CommonCrawl bao phủ 100+ ngôn ngữ sử dụng cùng một bộ từ vựng chia sẻ. Một model duy nhất xử lý:

- English, Chinese (Simplified/Traditional)
- Malay, Indonesian (Bahasa)
- Vietnamese, Thai
- Tamil, Hindi
- Japanese, Korean
- Tagalog, Khmer, Myanmar

Không cần phát hiện ngôn ngữ. Không cần chuyển đổi model. Cùng trọng số xử lý "John Smith" và "Nguyen Thi Lan" và "陈伟" với sự chú ý ngang nhau.

PII Engineer cải thiện thêm với model NER tiếng Trung chuyên dụng (dựa trên BERT với BIO tagging) chạy song song khi phát hiện ký tự CJK, vì văn bản tiếng Trung hưởng lợi từ tokenization cấp ký tự.

## So Sánh Độ Chính Xác

Chúng tôi chạy cả ba hệ thống trên bộ test 500 ví dụ PII đa ngôn ngữ (English, Chinese, Malay, Vietnamese, Indonesian) với chú thích ground truth:

### PII Tiếng Anh

| Hệ thống | Precision | Recall | F1 |
|--------|-----------|--------|----|
| Presidio (default recognizers) | 0.91 | 0.72 | 0.80 |
| spaCy (en_core_web_trf) + rules | 0.85 | 0.81 | 0.83 |
| PII Engineer (GLiNER2) | 0.89 | 0.88 | 0.88 |

Presidio có precision cao trên tiếng Anh vì các mẫu được tinh chỉnh tốt, nhưng recall kém trên tên và địa chỉ không khớp với mẫu mong đợi. Model transformer của spaCy bắt được nhiều tên hơn nhưng phân loại sai một số thực thể. PII Engineer đạt F1 tốt nhất bằng cách kết hợp NER dựa trên span với xác thực định dạng.

### PII Đa Ngôn Ngữ (không phải tiếng Anh)

| Hệ thống | Precision | Recall | F1 |
|--------|-----------|--------|----|
| Presidio (default) | 0.78 | 0.31 | 0.44 |
| spaCy (mixed models) | 0.72 | 0.58 | 0.64 |
| PII Engineer (GLiNER2) | 0.87 | 0.85 | 0.86 |

Khoảng cách nới rộng đáng kể trên văn bản không phải tiếng Anh. Presidio phát hiện các mẫu có cấu trúc (số điện thoại, một số ID) nhưng bỏ sót gần như tất cả tên và địa chỉ trong các ngôn ngữ không phải tiếng Anh. spaCy hoạt động khá tốt khi model tồn tại cho ngôn ngữ đó nhưng giảm trên tiếng Mã Lai và tiếng Việt. PII Engineer duy trì hiệu suất nhất quán trên các ngôn ngữ.

### PII Có Cấu Trúc (Điện thoại, ID, Email)

| Hệ thống | Precision | Recall | F1 |
|--------|-----------|--------|----|
| Presidio | 0.95 | 0.88 | 0.91 |
| spaCy | 0.61 | 0.45 | 0.52 |
| PII Engineer | 0.93 | 0.94 | 0.93 |

Đối với dữ liệu có cấu trúc cao, phương pháp regex của Presidio mạnh — khi mẫu tồn tại cho vùng đó. spaCy không bao giờ được thiết kế để phát hiện số điện thoại hay số định danh chính phủ. PII Engineer kết hợp phát hiện NER với xác thực regex để bắt kịp precision của Presidio đồng thời đạt recall tốt hơn.

## Độ Trễ và Sử Dụng Tài Nguyên

Kiểm thử trên instance cloud 4-vCPU (x86, không GPU):

| Hệ thống | Độ trễ p50 | Độ trễ p99 | Sử dụng RAM | Thời gian khởi động |
|--------|-------------|-------------|-----------|--------------|
| Presidio (không NER) | 3ms | 12ms | 200MB | 2s |
| Presidio (với spaCy) | 80ms | 250ms | 1.8GB | 15s |
| spaCy en_core_web_trf | 120ms | 350ms | 1.5GB | 12s |
| PII Engineer (INT8) | 180ms | 400ms | 700MB | 8s |

Presidio không có NER backend cực kỳ nhanh — chỉ là regex. Nhưng cấu hình đó bỏ sót hầu hết tên người và địa chỉ. Với spaCy được thêm vào, độ trễ tiệm cận PII Engineer.

Độ trễ của PII Engineer cao hơn mỗi request nhưng nó xử lý tất cả ngôn ngữ trong một lần duy nhất. Cấu hình spaCy bao phủ 5 ngôn ngữ sẽ cần 5 model được tải (7.5GB RAM) hoặc hoán đổi model (thêm độ trễ cold-start).

## Độ Phức Tạp Triển Khai

### Presidio

```bash
pip install presidio-analyzer presidio-anonymizer
python -m spacy download en_core_web_lg  # Optional NER backend
```

Presidio chỉ dùng Python. Triển khai yêu cầu môi trường Python, và model spaCy nếu bạn muốn phát hiện tên. Cấu hình thực hiện trong code — bạn khởi tạo recognizers theo chương trình.

Cho production, bạn thường bọc trong Flask/FastAPI service. Presidio cung cấp Docker image nhưng chỉ đi kèm hỗ trợ tiếng Anh.

### spaCy

```bash
pip install spacy
python -m spacy download en_core_web_trf
python -m spacy download zh_core_web_trf  # Per language
```

Mỗi model ngôn ngữ là một bản tải riêng (400-600MB). Bạn cần code ứng dụng để phát hiện ngôn ngữ, định tuyến đến model, hậu xử lý nhãn NER chung thành danh mục PII, và thêm quy tắc cho dữ liệu có cấu trúc.

Không có "phát hiện PII" sẵn dùng — bạn tự xây dựng trên nền NER của spaCy.

### PII Engineer

```bash
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
# Models auto-download on first run (~600MB total)
# API ready at http://localhost:8000
```

Một binary duy nhất, một lần tải model, API sẵn sàng ngay. Không cần Python runtime, không cần cấu hình riêng cho từng ngôn ngữ, không cần code recognizer tùy chỉnh. Docker image có sẵn cho triển khai container.

## Khi Nào Nên Dùng Mỗi Công Cụ

### Dùng Presidio khi:

- Dữ liệu của bạn chủ yếu là tiếng Anh
- Bạn cần độ trễ dưới 10ms
- Bạn có các mẫu PII rõ ràng, thân thiện với regex
- Bạn có nguồn lực kỹ thuật để xây dựng và bảo trì recognizers tùy chỉnh
- Bạn cần kiểm soát chi tiết logic phát hiện cho từng loại thực thể

### Dùng spaCy khi:

- Bạn cần NER chung (người, tổ chức, địa điểm) ngoài PII
- Đội ngũ của bạn đã sử dụng spaCy cho pipeline NLP
- Bạn đang xây dựng giải pháp NLP tùy chỉnh mà phát hiện PII chỉ là một thành phần
- Triển khai đơn ngôn ngữ với ngôn ngữ được hỗ trợ tốt

### Dùng PII Engineer (GLiNER2) khi:

- Dữ liệu của bạn trải rộng nhiều ngôn ngữ (đặc biệt Đông Nam Á)
- Bạn cần phát hiện PII chuyên biệt sẵn dùng ngay
- Bạn muốn một model duy nhất xử lý mọi thứ mà không cần định tuyến ngôn ngữ
- Triển khai tự lưu trữ là yêu cầu bắt buộc
- Bạn không có GPU nhưng cần độ chính xác cấp transformer
- Ngân sách bảo trì hạn chế — một model phục vụ tất cả ngôn ngữ

## Sự Tiết Kiệm Giả Của Regex

Phản ứng phổ biến là "chúng tôi sẽ chỉ viết mẫu regex cho các trường hợp sử dụng của mình." Điều này hoạt động ban đầu nhưng trở nên tốn kém:

1. **Tên người không thể regex.** Không có mẫu nào cho "Nguyen Thi Lan" so với "nguyen thi" (loại rau thơm).
2. **Định dạng địa chỉ khác nhau theo quốc gia.** "Blk 123 Ang Mo Kio Ave 4 #05-678" của Singapore trông hoàn toàn khác "Jl. Sudirman No. 45, Jakarta Selatan 12190" của Indonesia.
3. **Ngữ cảnh quan trọng.** "850612" có thể là ngày, một phần của số IC, hoặc mã bưu chính. Chỉ NER theo ngữ cảnh mới giải quyết được.
4. **Bảo trì tăng tuyến tính.** Mỗi quốc gia hoặc định dạng mới đều cần mẫu mới, kiểm thử mới, trường hợp biên mới.

NER dựa trên transformer học các mẫu này từ dữ liệu. Thêm quốc gia mới có nghĩa là thêm ví dụ huấn luyện, không phải viết regex.

## Ví Dụ Code: Chạy Cả Ba

Cho độc giả muốn benchmark trên dữ liệu riêng:

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

## Kết Luận

Không có công cụ tốt nhất cho mọi trường hợp. Nhưng đối với phát hiện PII đa ngôn ngữ — đặc biệt trong các ngôn ngữ Đông Nam Á — phương pháp transformer dựa trên span được PII Engineer sử dụng giải quyết vấn đề mở rộng cơ bản: một model, tất cả ngôn ngữ, không cần kỹ thuật riêng cho từng vùng.

Nếu dữ liệu của bạn chỉ tiếng Anh và bạn cần độ trễ micro giây, regex của Presidio khó bị đánh bại. Nếu bạn đang xây dựng pipeline NLP rộng hơn và PII chỉ là một thành phần, spaCy cho bạn sự linh hoạt. Nhưng nếu phát hiện PII trên nhiều ngôn ngữ là nhu cầu chính, một hệ thống NER đa ngôn ngữ chuyên dụng sẽ tiết kiệm thời gian kỹ thuật và mang lại độ chính xác tốt hơn.

## Dùng Thử

PII Engineer là mã nguồn mở theo giấy phép AGPL-3.0:

```bash
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
```

- Mã nguồn: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- Models: [huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
