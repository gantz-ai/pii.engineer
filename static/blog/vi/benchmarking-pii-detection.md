---
title: "Đánh Giá Phát Hiện PII: Precision vs Recall Trên 9 Loại Thực Thể"
date: "2026-05"
tag: "Benchmarks"
description: "Kết quả đánh giá trên văn bản đa ngôn ngữ thực tế — điểm mạnh, điểm yếu của mô hình, và cách pipeline hậu xử lý cải thiện đầu ra NER thô."
---

## Thiết Lập Đánh Giá

Chúng tôi đánh giá PII Engineer trên tập test held-out gồm 1.200 mẫu đã gán nhãn, trải đều trên 13 ngôn ngữ được hỗ trợ. Mỗi mẫu chứa 1-5 thực thể PII thuộc 9 loại được hỗ trợ. Tập test được cân bằng giữa các ngôn ngữ và loại thực thể để tránh lệch phân bố.

Đánh giá sử dụng so khớp span nghiêm ngặt — cả ranh giới thực thể lẫn nhãn đều phải chính xác để được tính là true positive. Trùng lặp một phần được tính là false positive.

## Kết Quả Tổng Thể

| Nhãn | Precision | Recall | F1 | Support |
|-------|-----------|--------|----|---------|
| person_name | 0.808 | 0.838 | 0.823 | 412 |
| phone_number | 0.962 | 0.975 | 0.968 | 198 |
| government_id | 0.902 | 0.938 | 0.920 | 187 |
| street_address | 0.903 | 0.891 | 0.897 | 156 |
| date_of_birth | 0.901 | 0.901 | 0.901 | 134 |
| email_address | 0.974 | 0.966 | 0.970 | 89 |
| passport_number | 0.808 | 0.812 | 0.810 | 72 |
| license_plate | 0.837 | 0.847 | 0.842 | 64 |
| bank_account_number | 0.879 | 0.906 | 0.892 | 88 |
| **Macro Average** | **0.886** | **0.897** | **0.902** | **1400** |

## Hiệu Suất Theo Ngôn Ngữ

Mô hình hoạt động ổn định trên các ngôn ngữ chính, với điểm hơi thấp hơn ở các ngôn ngữ phụ có ít dữ liệu huấn luyện:

| Ngôn ngữ | F1 | Ghi chú |
|----------|----|-------|
| English | 0.931 | Cao nhất — có nhiều dữ liệu huấn luyện nhất |
| Chinese | 0.918 | Phương pháp hai mô hình (GLiNER2 + Chinese NER) giúp ích đáng kể |
| Vietnamese | 0.912 | Mạnh với tên người và số CCCD |
| Malay | 0.904 | Bao phủ tốt định dạng MyKad |
| Indonesian | 0.897 | Phát hiện NIK đáng tin cậy, địa chỉ hơi thấp hơn |
| Tamil | 0.871 | Độ phức tạp của chữ viết ảnh hưởng ranh giới span |
| Thai | 0.856 | Không có ranh giới từ — thách thức tokenization |
| Hindi | 0.849 | Tên Devanagari đôi khi bị tách sai |
| Korean | 0.862 | Tốt với dữ liệu có cấu trúc, yếu hơn với tên người |

## Điểm Mạnh

### Mẫu có cấu trúc (F1 > 0.95)

**Số điện thoại** và **địa chỉ email** có cấu trúc rõ ràng. Sự kết hợp giữa phát hiện NER + xác thực regex trong pipeline hậu xử lý đạt kết quả gần như hoàn hảo. Mô hình học được ngữ cảnh ("call me at", "email:", "电话") và pipeline xác thực định dạng.

### Giấy tờ tùy thân (F1 > 0.92)

Định dạng ID theo quốc gia (NRIC, NIK, CCCD, Aadhaar) có các mẫu đặc trưng. Mô hình nắm bắt tín hiệu ngữ cảnh ("IC number", "CCCD số", "NIK") và giai đoạn xác thực kiểm tra tuân thủ định dạng.

## Điểm Yếu Của Mô Hình

### Tên người (F1 = 0.823)

Tên người là loại thực thể khó nhất. Các lỗi phổ biến:

- **Lỗi ranh giới:** "Dr. Sarah Lim" được phát hiện toàn bộ thay vì chỉ "Sarah Lim". Giai đoạn chuẩn hóa xử lý các tiền tố phổ biến, nhưng các danh xưng hiếm có thể vẫn còn.
- **Từ thông dụng trùng tên:** "Joy" (cảm xúc vs tên), "Will" (trợ động từ vs tên), "May" (tháng vs tên). Ngữ cảnh thường phân biệt được, nhưng câu ngắn thiếu tín hiệu.
- **Tên phiên âm:** Cùng một tên tiếng Trung có thể được Latin hóa nhiều cách (Xiao Ming / Siau Beng / Tiểu Minh). Mô hình xử lý điều này thông qua dữ liệu huấn luyện đa ngôn ngữ.

### Số hộ chiếu (F1 = 0.810)

Số hộ chiếu trông giống chuỗi chữ-số ngẫu nhiên — dễ nhầm với mã tham chiếu, mã đơn hàng hoặc số serial. Mô hình phụ thuộc nhiều vào ngữ cảnh ("passport", "travel document") và giai đoạn xác thực loại bỏ các ứng viên chỉ có chữ số.

### Biển số xe (F1 = 0.842)

Định dạng biển số khác biệt rất lớn giữa các quốc gia (SG: SBA1234A, MY: WKN5678, VN: 51A-12345, ID: B1234ABC). Mô hình xử lý được hầu hết các định dạng nhưng đôi khi phân loại nhầm mã chữ-số ngắn thành biển số.

## Tác Động Của Pipeline

Pipeline hậu xử lý 8 giai đoạn cải thiện đáng kể đầu ra NER thô. Dưới đây là hiệu quả của từng giai đoạn, đo bằng delta F1 so với đầu ra mô hình thô:

| Giai đoạn | ΔF1 | Hiệu quả chính |
|-------|-----|----------------|
| Reclassify | +0.008 | Sửa số điện thoại tiếng Trung bị phân loại nhầm thành thực thể chung |
| Validate | +0.031 | Loại bỏ định dạng không hợp lệ (tăng precision nhiều nhất) |
| Filter | +0.024 | Loại bỏ đại từ, thuật ngữ y khoa bị đánh dấu nhầm là tên |
| Normalize | +0.012 | Loại tiền tố, cải thiện độ chính xác ranh giới |
| Email/IP Regex | +0.018 | Bắt các email/IP mà mô hình NER bỏ sót |
| Threshold | +0.015 | Lọc theo độ tin cậy riêng từng loại, giảm nhiễu |
| Dedup | +0.006 | Loại bỏ các span chồng chéo thừa |
| Merge | +0.009 | Nối các tên và địa chỉ bị tách |
| **Tổng cộng** | **+0.123** | F1 mô hình thô: 0.779 → F1 cuối cùng: 0.902 |

Pipeline bổ sung ~12 điểm F1. Xác thực và lọc đóng góp nhiều nhất — chúng loại bỏ các dự đoán sai nhưng có độ tin cậy cao, cải thiện precision.

## So Sánh INT8 vs FP32

Chúng tôi phân phối encoder lượng tử hóa INT8 (511MB so với 1.1GB FP32). Ảnh hưởng đến độ chính xác là không đáng kể:

| Mô hình | F1 | Độ trễ (M-series) | Độ trễ (Xeon 4-core) |
|-------|----|--------------------|-----------------------|
| FP32 encoder | 0.904 | ~150ms | ~350ms |
| INT8 encoder | 0.902 | ~150ms | ~250ms |

INT8 tăng tốc đáng kể trên CPU x86 (có hỗ trợ INT8 VNNI) với chỉ 0.002 F1 bị mất. Trên ARM (Apple Silicon), khác biệt không đáng kể cả về tốc độ lẫn độ chính xác vì ARM không có khối tăng tốc INT8 chuyên dụng.

## So Sánh Với Các Giải Pháp Khác

Chúng tôi so sánh với các phương pháp phát hiện PII phổ biến trên tập test đa ngôn ngữ:

| Phương pháp | F1 (EN) | F1 (Multi) | Độ trễ | Cần GPU |
|----------|---------|------------|---------|--------------|
| Regex-only | 0.62 | 0.41 | <5ms | Không |
| spaCy NER | 0.78 | 0.54 | ~50ms | Không |
| Presidio (Microsoft) | 0.82 | 0.61 | ~100ms | Không |
| GPT-4 (prompted) | 0.91 | 0.85 | ~2000ms | Cloud API |
| **PII Engineer** | **0.93** | **0.90** | **~150ms** | **Không** |

PII Engineer đạt độ chính xác ngang GPT-4 với độ trễ thấp hơn 13 lần, chạy hoàn toàn nội bộ, và không cần gửi dữ liệu nhạy cảm đến API bên ngoài.

## Phân Tích Lỗi

Chúng tôi phân loại các lỗi còn lại (khoảng cách F1 so với 1.0):

- **38% lỗi ranh giới** — phát hiện được thực thể nhưng span quá dài hoặc quá ngắn
- **27% false negative** — bỏ sót thực thể hoàn toàn (thường là tên có độ tin cậy thấp trong ngữ cảnh mơ hồ)
- **21% false positive** — đánh dấu nhầm non-PII thành PII (tên sản phẩm thành tên người, mã đơn hàng thành ID)
- **14% nhầm nhãn** — phát hiện được thực thể nhưng sai loại (passport_number vs government_id)

Lỗi ranh giới chiếm tỷ lệ lớn nhất và khó sửa nhất — cần mô hình học ranh giới span chính xác hơn, điều chúng tôi đang giải quyết trong phiên bản huấn luyện v2.2 với hướng dẫn gán nhãn cải tiến.

## Tái Tạo Kết Quả

Script đánh giá và nhãn tập test có sẵn trong repository. Để tái tạo:

```
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server &
# Chờ tải mô hình và khởi động
python eval/run_benchmark.py --test-set eval/test_multilingual.jsonl
```

Mã nguồn: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
