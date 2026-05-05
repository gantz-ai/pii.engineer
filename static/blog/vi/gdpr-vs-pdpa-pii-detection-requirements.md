---
title: "GDPR vs PDPA: So Sánh Yêu Cầu Phát Hiện PII"
date: "2026-05"
tag: "Compliance"
description: "So sánh song song giữa GDPR của EU và các quy định PDPA tại Đông Nam Á — thế nào được coi là dữ liệu cá nhân, yêu cầu phát hiện khác nhau ra sao, và PII Engineer ánh xạ vào cả hai khuôn khổ như thế nào."
---

## Hai Khuôn Khổ, Một Vấn Đề

Các tổ chức hoạt động tại cả châu Âu và Đông Nam Á đối mặt với các quy định bảo mật chồng chéo nhưng khác biệt. GDPR (General Data Protection Regulation) của EU và PDPA (Personal Data Protection Act) của Singapore đều yêu cầu tổ chức phải xác định, bảo vệ và quản lý dữ liệu cá nhân — nhưng chúng định nghĩa phạm vi, nghĩa vụ và hình phạt khác nhau.

Phát hiện PII tự động cần tính đến cả hai khuôn khổ. Một hệ thống chỉ tinh chỉnh cho GDPR sẽ bỏ sót các mã định danh Đông Nam Á. Một hệ thống xây dựng cho PDPA có thể không bao phủ định nghĩa "dữ liệu cá nhân" rộng hơn theo GDPR.

## Thế Nào Được Coi Là Dữ Liệu Cá Nhân?

### Định Nghĩa của GDPR (Điều 4)

> "Bất kỳ thông tin nào liên quan đến một thể nhân đã được xác định hoặc có thể xác định được."

Định nghĩa này cố tình rộng. Nó bao gồm:

- Mã định danh trực tiếp: tên, số chứng minh, email
- Mã định danh gián tiếp: dữ liệu vị trí, địa chỉ IP, cookie ID
- Danh mục đặc biệt: nguồn gốc chủng tộc/dân tộc, quan điểm chính trị, dữ liệu sức khỏe, dữ liệu sinh trắc học, xu hướng tính dục

Phạm vi của GDPR mở rộng đến bất kỳ dữ liệu nào *có thể* xác định một người, ngay cả khi kết hợp với dữ liệu khác.

### Định Nghĩa của PDPA (Phần 2)

> "Dữ liệu, dù đúng hay không, về một cá nhân có thể được xác định từ dữ liệu đó; hoặc từ dữ liệu đó và các thông tin khác mà tổ chức có hoặc có khả năng có quyền truy cập."

PDPA hẹp hơn trong thực tế:

- Tập trung vào dữ liệu xác định một cá nhân *cụ thể*
- Không có khái niệm "danh mục đặc biệt" rõ ràng (mặc dù Do Not Call registry bổ sung các quy tắc riêng cho điện thoại)
- Thông tin liên lạc kinh doanh thường được loại trừ khi sử dụng cho mục đích kinh doanh

### Khác Biệt Thực Tế

| Khía cạnh | GDPR | PDPA (Singapore) |
|---|---|---|
| Phạm vi | Bất kỳ dữ liệu nào có thể xác định một người | Dữ liệu xác định một người cụ thể |
| Địa chỉ IP | Dữ liệu cá nhân | Không được quy định rõ ràng |
| Cookie ID | Dữ liệu cá nhân (với ePrivacy) | Thường không phải dữ liệu cá nhân |
| Danh thiếp | Dữ liệu cá nhân | Loại trừ cho mục đích kinh doanh |
| Người đã mất | Không bao gồm | Không bao gồm |
| Dữ liệu ẩn danh | Không phải dữ liệu cá nhân nếu thực sự ẩn danh | Không phải dữ liệu cá nhân nếu thực sự ẩn danh |
| Dữ liệu giả danh | Vẫn là dữ liệu cá nhân | Tùy thuộc vào rủi ro tái nhận dạng |

## So Sánh Các Nghĩa Vụ Chính

### Sự Đồng Ý

| | GDPR | PDPA |
|---|---|---|
| Mặc định | Yêu cầu opt-in | Yêu cầu đồng ý (ngầm định hoặc rõ ràng) |
| Lợi ích hợp pháp | Có — có thể xử lý mà không cần đồng ý | Không có tương đương — cần đồng ý hoặc ngoại lệ |
| Rút lại | Phải dễ dàng như khi cho đồng ý | Phải có khả năng rút lại |
| Trẻ em | Dưới 16 tuổi cần sự đồng ý của phụ huynh | Không có ngưỡng tuổi cụ thể |

### Thông Báo Vi Phạm Dữ Liệu

| | GDPR | PDPA |
|---|---|---|
| Thông báo cơ quan | 72 giờ | "Sớm nhất có thể" (cập nhật 2021: 3 ngày dương lịch) |
| Thông báo cá nhân | Khi có "rủi ro cao" đối với quyền lợi | Khi có khả năng "thiệt hại đáng kể" |
| Ngưỡng | Đánh giá dựa trên rủi ro | 500+ cá nhân hoặc thiệt hại đáng kể |

### Chuyển Dữ Liệu Xuyên Biên Giới

| | GDPR | PDPA |
|---|---|---|
| Quy tắc mặc định | Cấm trừ khi có bảo vệ đầy đủ | Cấm trừ khi có bảo vệ tương đương |
| Cơ chế | Quyết định tương đương, SCC, BCR | Thỏa thuận hợp đồng, binding corporate rules |
| Mối quan tâm chính | Dữ liệu chuyển đến các nước ngoài EU | Dữ liệu chuyển ra ngoài Singapore |

## Loại Entity PII: Bản Đồ Bao Phủ

PII Engineer phát hiện 9 loại entity. Dưới đây là cách mỗi loại ánh xạ vào yêu cầu GDPR và PDPA:

| Loại Entity | Liên quan đến GDPR | Liên quan đến PDPA | Ưu tiên phát hiện |
|---|---|---|---|
| `person_name` | Mã định danh trực tiếp (Art. 4) | Mã định danh trực tiếp (s2) | Quan trọng cho cả hai |
| `government_id` | Số chứng minh quốc gia (Art. 87) | NRIC (advisory guidelines) | Quan trọng cho cả hai |
| `phone_number` | Dữ liệu liên lạc | DNC Registry (Part IX) | Cao cho cả hai |
| `email_address` | Mã định danh trực tiếp | Mã định danh trực tiếp | Cao cho cả hai |
| `street_address` | Dữ liệu vị trí (Recital 30) | Mã định danh trực tiếp | Cao cho cả hai |
| `date_of_birth` | Mã định danh gián tiếp | Mã định danh gián tiếp | Trung bình — rủi ro cao hơn khi kết hợp |
| `passport_number` | Tài liệu du lịch (Art. 87) | Giấy tờ do chính phủ cấp | Quan trọng cho cả hai |
| `bank_account_number` | Dữ liệu tài chính | Dữ liệu tài chính | Cao cho cả hai |
| `license_plate` | Mã định danh gián tiếp (có thể truy ngược chủ sở hữu) | Mã định danh gián tiếp | Trung bình cho cả hai |

### Khoảng Trống Riêng của GDPR

Định nghĩa rộng hơn của GDPR đồng nghĩa với việc một số loại dữ liệu cá nhân nằm ngoài khả năng phát hiện NER tiêu chuẩn:

- **Địa chỉ IP** — không giống entity, nhưng là dữ liệu cá nhân theo GDPR. Sử dụng regex pattern kết hợp với NER.
- **Cookie identifier** — chuỗi kỹ thuật, không phải entity ngôn ngữ tự nhiên
- **Dữ liệu di truyền/sinh trắc** — danh mục đặc biệt, cần phát hiện chuyên biệt theo lĩnh vực
- **Dữ liệu vị trí** — tọa độ GPS, cell tower ID. PII Engineer phát hiện địa chỉ đường phố nhưng không phát hiện tọa độ thô.

### Trọng Tâm Riêng của PDPA

PDPA của Singapore nhấn mạnh đặc biệt vào:

- **Số NRIC** — Singapore đã ban hành advisory guidelines hạn chế việc thu thập NRIC. PII Engineer xác thực định dạng NRIC (chữ cái + 7 chữ số + chữ cái checksum).
- **Số điện thoại** — Do Not Call (DNC) Registry khiến việc phát hiện số điện thoại đặc biệt quan trọng cho tuân thủ marketing.

## Chiến Lược Phát Hiện Theo Quy Định

### Cho Tuân Thủ GDPR

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

### Cho Tuân Thủ PDPA

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

### Cho Tuân Thủ Kép (GDPR + PDPA)

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

## Tự Triển Khai: Con Đường Tắt Tuân Thủ

Cả GDPR và PDPA đều áp đặt hạn chế về chuyển dữ liệu xuyên biên giới. Sử dụng dịch vụ phát hiện PII trên cloud tạo ra một sự kiện chuyển dữ liệu — bạn đang gửi dữ liệu cá nhân đến bên thứ ba để xử lý.

Tự triển khai PII Engineer loại bỏ hoàn toàn vấn đề này:

| Mối quan tâm tuân thủ | Dịch vụ PII trên Cloud | Tự triển khai PII Engineer |
|---|---|---|
| Thỏa thuận bên xử lý dữ liệu GDPR | Bắt buộc (Art. 28) | Không cần |
| Trung gian dữ liệu PDPA | Phải ràng buộc bằng hợp đồng | Không áp dụng |
| Đánh giá chuyển dữ liệu xuyên biên giới | Bắt buộc nếu dịch vụ ngoài phạm vi quyền hạn | Không có chuyển dữ liệu |
| Đánh giá tác động xử lý dữ liệu | Có thể bắt buộc (Art. 35) | Đơn giản hóa — không có xử lý bên ngoài |
| Quyền kiểm toán nhà cung cấp | Phải đàm phán | Không áp dụng |

## Mẫu PII Theo Khu Vực

Model đa ngôn ngữ của PII Engineer xử lý các mẫu dữ liệu cá nhân khác biệt trên khắp EU và ASEAN:

| Khu vực | Mẫu tên | Định dạng ID | Định dạng địa chỉ |
|---|---|---|---|
| EU (Germany) | "Hans Müller" | Personalausweisnr | Straße + PLZ + Stadt |
| EU (France) | "Jean-Pierre Dubois" | Numéro de sécurité sociale | Rue + Code postal + Ville |
| Singapore | "Tan Ah Kow" / "S. Ramasamy" | NRIC: S9012345A | Blk + Street + #unit |
| Malaysia | "Ahmad bin Ismail" | MyKad: 820315-10-5523 | Jalan + Taman + Poskod |
| Vietnam | "Nguyen Thi Lan" | CCCD: 024198006789 | Số + Đường + Quận |
| Indonesia | "Budi Santoso" | NIK: 3201011234560001 | Jalan + RT/RW + Kecamatan |

Model GLiNER2 xử lý tất cả các mẫu này mà không cần cấu hình riêng theo ngôn ngữ. Một lệnh gọi API duy nhất phát hiện PII bất kể ngôn ngữ hay định dạng khu vực trong văn bản đầu vào.

## Khuyến Nghị

1. **Bắt đầu với phát hiện đầy đủ** — sử dụng tất cả 9 loại entity. Lọc kết quả ở downstream dễ hơn việc bỏ sót entity.
2. **Ánh xạ vào nghĩa vụ cụ thể của bạn** — không phải mọi entity được phát hiện đều yêu cầu cách xử lý giống nhau. Danh mục đặc biệt của GDPR cần bảo vệ mạnh hơn dữ liệu cá nhân tiêu chuẩn.
3. **Tài liệu hóa phạm vi phát hiện** — cả GDPR (Art. 30) và PDPA (s24) đều yêu cầu chứng minh các biện pháp bảo vệ dữ liệu. Một pipeline phát hiện PII được tài liệu hóa sẽ củng cố thế trận tuân thủ của bạn.
4. **Tự triển khai cho đơn giản** — loại bỏ mối quan hệ bên xử lý dữ liệu sẽ xóa bỏ toàn bộ một danh mục nghĩa vụ tuân thủ.

## Mã Nguồn

PII Engineer là mã nguồn mở theo giấy phép AGPL-3.0:

- Repository: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- Models: [huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
