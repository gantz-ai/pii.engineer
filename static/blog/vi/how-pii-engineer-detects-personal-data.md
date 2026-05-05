---
title: "Cách PII Engineer Phát Hiện Dữ Liệu Cá Nhân Trên 13+ Ngôn Ngữ"
date: "2026-05"
tag: "Architecture"
description: "Phân tích chi tiết về engine NER GLiNER2, pipeline hậu xử lý 8 giai đoạn, và cách chúng tôi đạt F1 trên 90% cho 9 loại PII mà không cần GPU."
---

## Thách Thức

Phát hiện thông tin nhận dạng cá nhân (PII) trong văn bản đa ngôn ngữ khó hơn nhiều so với vẻ bề ngoài. Tên người trong tiếng Anh tuân theo những quy tắc khác so với tiếng Trung, tiếng Mã Lai hay tiếng Việt. Giấy tờ tùy thân khác nhau theo từng quốc gia — NRIC của Singapore, NIK của Indonesia, Aadhaar của Ấn Độ, và CCCD của Việt Nam đều có định dạng riêng. Số điện thoại, địa chỉ và ngày tháng cũng được viết khác nhau giữa các nền văn hóa.

Hầu hết các công cụ phát hiện PII đều ưu tiên tiếng Anh, với các ngôn ngữ khác được bổ sung sau. Chúng tôi cần một giải pháp đối xử bình đẳng với tất cả 13+ ngôn ngữ.

## Tổng Quan Kiến Trúc

PII Engineer sử dụng phương pháp hai mô hình:

1. **GLiNER2 Multi-NER** — mô hình NER dựa trên span, xây dựng trên mDeBERTa-v3-base (~280M tham số), tinh chỉnh bằng LoRA cho việc phát hiện PII trên tất cả ngôn ngữ được hỗ trợ
2. **Chinese NER** — bộ phân loại token dựa trên BERT với gắn nhãn BIO, được huấn luyện riêng cho văn bản tiếng Trung vì ký tự CJK đòi hỏi cách tokenization khác

Hệ thống nhận diện ký tự CJK trong đầu vào và tự động gửi đến cả hai mô hình khi phát hiện văn bản tiếng Trung. Với văn bản không có CJK, chỉ mô hình GLiNER2 được chạy.

## GLiNER2: NER Dựa Trên Span

Khác với phương pháp gắn nhãn tuần tự truyền thống (BIO tagging), GLiNER2 sử dụng cách tiếp cận dựa trên span. Thay vì phân loại từng token, mô hình đánh giá các đoạn văn bản ứng viên và chấm điểm chúng dựa trên embedding của loại thực thể.

Mô hình bao gồm 5 thành phần ONNX:

| Thành phần | Kích thước | Vai trò |
|-----------|------|------|
| encoder | 511MB (INT8) | Bộ mã hóa token mDeBERTa-v3-base |
| span_rep | 63MB | Lớp biểu diễn span |
| count_embed | 41MB | Count embedding cho chấm điểm span |
| count_pred | 4.6MB | Head dự đoán count |
| classifier | 4.5MB | Head phân loại cuối cùng |

Chúng tôi phân phối encoder đã lượng tử hóa INT8 (511MB so với 1.1GB FP32), cho tốc độ suy luận nhanh hơn ~15-20% trên CPU x86 với mức suy giảm độ chính xác không đáng kể.

## Pipeline 8 Giai Đoạn

Đầu ra NER thô rất nhiễu. Mô hình có thể đánh dấu "Dr." là tên người, phát hiện số điện thoại không hợp lệ, hoặc trả về các thực thể chồng chéo. Pipeline hậu xử lý của chúng tôi khắc phục những vấn đề này:

### 1. Phân loại lại (Reclassify)

Số điện thoại tiếng Trung thường xuất hiện gần các dấu hiệu như 电话 (điện thoại) hoặc 手机 (di động). Nếu một thực thể chung xuất hiện gần các dấu hiệu này, chúng tôi phân loại lại nó thành `phone_number`.

### 2. Xác thực (Validate)

Mỗi loại thực thể có quy tắc xác thực định dạng riêng. Số điện thoại phải chứa đủ chữ số và không có chữ cái. Giấy tờ tùy thân cần độ dài tối thiểu với ký tự chữ-số. Số hộ chiếu không thể chỉ toàn chữ số. Các định dạng không hợp lệ bị loại bỏ.

### 3. Lọc (Filter)

Chúng tôi duy trì một bộ từ vựng các từ meta — đại từ ("I", "you", "she"), từ chỉ quan hệ gia đình ("mom", "husband"), thuật ngữ y khoa ("doctor", "patient"), và chính các từ nhãn ("name", "phone"). Các thực thể khớp với những từ này sẽ bị lọc ra.

### 4. Chuẩn hóa (Normalize)

Các tiền tố ngữ cảnh như "Patient", "Dr.", "Mr." được loại bỏ khỏi tên người. "Patient Sarah Lim" trở thành "Sarah Lim".

### 5. Phát hiện Email/IP

Phát hiện dựa trên regex cho địa chỉ email và địa chỉ IPv4. Các mẫu này có cấu trúc cao và regex bắt chúng đáng tin cậy hơn NER.

### 6. Ngưỡng (Threshold)

Ngưỡng độ tin cậy riêng cho từng nhãn. Tên người sử dụng ngưỡng thấp hơn (0.25) vì chúng rất đa dạng, trong khi số điện thoại dùng ngưỡng cao hơn (0.30) vì mô hình tự tin hơn với các mẫu có cấu trúc.

### 7. Loại trùng (Dedup)

Các thực thể chồng chéo cùng nhãn được loại trùng — span dài hơn được giữ lại. Các thực thể chồng chéo khác nhãn đều được giữ.

### 8. Hợp nhất (Merge)

Các thực thể liền kề cùng nhãn, cách nhau 3 ký tự trở xuống, được hợp nhất. "John" + " " + "Doe" trở thành "John Doe".

## Hiệu Suất

| Nhãn | Precision | Recall | F1 |
|-------|-----------|--------|----|
| person_name | 0.808 | 0.838 | 0.823 |
| phone_number | 0.962 | 0.975 | 0.968 |
| government_id | 0.902 | 0.938 | 0.920 |
| street_address | 0.903 | 0.891 | 0.897 |
| date_of_birth | 0.901 | 0.901 | 0.901 |
| email_address | 0.974 | 0.966 | 0.970 |
| passport_number | 0.808 | 0.812 | 0.810 |
| license_plate | 0.837 | 0.847 | 0.842 |
| bank_account_number | 0.879 | 0.906 | 0.892 |
| **Trung bình** | | | **0.902** |

## Độ Trễ

Trên MacBook M-series với encoder FP32, suy luận thông thường khoảng ~150ms mỗi request. Trên Xeon 4-vCPU với encoder INT8, khoảng ~250ms. Toàn bộ pipeline chạy trên CPU — không cần GPU.

Server tự động tải mô hình từ HuggingFace khi chạy lần đầu, khởi động các phiên ONNX, và khóa trọng số mô hình trong RAM để tránh swap.

## Dùng Thử

PII Engineer là mã nguồn mở theo giấy phép AGPL-3.0. Bắt đầu chỉ với một lệnh:

```
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
# Mô hình tự động tải về — http://localhost:8000
```

Mã nguồn: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)

Mô hình: [huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
