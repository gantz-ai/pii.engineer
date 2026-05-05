---
title: "Phát Hiện PII Cho Y Tế: Bảo Vệ Dữ Liệu Bệnh Nhân Theo HIPAA và PDPA"
date: "2026-05"
tag: "Healthcare"
description: "Cách PII Engineer xử lý các thách thức đặc thù trong y tế — ghi chú lâm sàng, hồ sơ thuốc, yêu cầu bảo hiểm — đồng thời đáp ứng yêu cầu tuân thủ HIPAA và PDPA thông qua triển khai tự lưu trữ."
---

## Dữ Liệu Y Tế Khác Biệt

Văn bản y tế chứa mật độ thông tin nhận dạng cá nhân cao nhất trong mọi ngành. Một ghi chú lâm sàng đơn lẻ có thể chứa tên đầy đủ bệnh nhân, ngày sinh, số CMND, số điện thoại, địa chỉ và thông tin người thân — tất cả trong vài đoạn văn bản phi cấu trúc.

Thách thức không chỉ là độ chính xác phát hiện. Mà là thực hiện điều đó mà không phơi bày dữ liệu cho các dịch vụ bên thứ ba. Gửi hồ sơ bệnh nhân đến cloud API để phát hiện PII đi ngược lại mục đích bảo vệ chúng.

## PHI: Không Chỉ Là Tên

Quy tắc Bảo mật HIPAA định nghĩa 18 danh mục Thông Tin Y Tế Được Bảo Vệ (PHI). Hầu hết công cụ phát hiện PII chỉ tập trung vào các định danh rõ ràng như tên và số An sinh Xã hội. Nhưng văn bản y tế chứa PHI mà các model NER đa năng bỏ sót:

| Danh mục PHI | Ví dụ trong ghi chú lâm sàng |
|---|---|
| Tên bệnh nhân | "Pt: Sarah Tan Wei Lin" |
| Ngày sinh | "DOB: 15/03/1982" |
| Số định danh chính phủ | "NRIC: S8203451B" (Singapore), "IC: 820315-10-5523" (Malaysia) |
| Số điện thoại | "NOK contact: +65 9123 4567" |
| Địa chỉ | "Discharge to: Blk 123 Ang Mo Kio Ave 4 #05-678" |
| Email | "Follow-up appt confirmation to sarah.tan@gmail.com" |
| Ngày (nhập viện, xuất viện) | "Admitted 12-Mar-2024, discharged 15-Mar-2024" |
| Số hồ sơ y tế | "MRN: 2024-SG-089123" |
| Số tài khoản | "Insurance claim ref: PRU-2024-445566" |

Các model NER tiêu chuẩn gặp khó với văn bản y tế vì:

- **Tên viết tắt** — ghi chú lâm sàng dùng tiền tố "Pt:" hoặc "NOK:", không phải "Dear Mr."
- **Định dạng dày đặc** — nhiều định danh dồn vào các trường có cấu trúc mà không có ngữ cảnh ngôn ngữ tự nhiên
- **Tên đa ngôn ngữ** — y tế Đông Nam Á phục vụ bệnh nhân có tên Trung Quốc, Mã Lai, Tamil và Việt Nam
- **Ngày tháng theo ngữ cảnh** — không phải mọi ngày tháng đều là ngày sinh; ngày nhập viện và ngày thủ thuật cần xử lý khác nhau

## Cách PII Engineer Xử Lý Văn Bản Y Tế

Pipeline 8 giai đoạn của PII Engineer bao gồm xử lý đặc biệt cho các mẫu y tế:

### Phát Hiện Tên Trong Ngữ Cảnh Lâm Sàng

Model GLiNER2 được huấn luyện trên các mẫu ghi chú lâm sàng. Nó nhận dạng tên đứng sau các dấu hiệu đặc thù y tế:

```
Pt: Ahmad bin Ismail           → person_name: "Ahmad bin Ismail"
NOK: Wife - Siti binti Yusof   → person_name: "Siti binti Yusof"
Attending Dr. Rajesh Kumar     → person_name: "Rajesh Kumar"
Referred by: Dr Ng Wei Kiat    → person_name: "Ng Wei Kiat"
```

Giai đoạn chuẩn hóa loại bỏ tiền tố ("Dr.", "Pt:", "Patient") để thực thể phát hiện là tên sạch.

### Số Định Danh Chính Phủ Xuyên Quốc Gia

Hệ thống y tế ở các quốc gia khác nhau sử dụng các loại định danh quốc gia khác nhau:

| Quốc gia | Loại ID | Định dạng | Ví dụ |
|---------|---------|--------|---------|
| Singapore | NRIC/FIN | 1 chữ cái + 7 chữ số + 1 chữ cái | S9012345A |
| Malaysia | MyKad | 6 số - 2 số - 4 số | 820315-10-5523 |
| Indonesia | NIK | 16 chữ số | 3201011234560001 |
| Vietnam | CCCD | 12 chữ số | 024198006789 |
| India | Aadhaar | 4-4-4 chữ số | 2345 6789 0123 |
| Thailand | National ID | 13 chữ số | 1-1001-12345-12-1 |

Giai đoạn xác thực của PII Engineer kiểm tra các thực thể được phát hiện theo định dạng đã biết của mỗi quốc gia, giảm kết quả dương tính giả trên các chuỗi số ngẫu nhiên trong hồ sơ y tế.

### Phân Loại Ngày Tháng

Không phải mọi ngày tháng trong hồ sơ y tế đều là ngày sinh. PII Engineer sử dụng tín hiệu ngữ cảnh:

- "DOB:", "D.O.B", "Born:", "Birth date" → phân loại là `date_of_birth`
- Ngày tháng xuất hiện trong ngữ cảnh nhập viện/xuất viện được phát hiện nhưng có điểm tin cậy thấp hơn
- Ngày tháng rõ ràng trong quá khứ (30+ năm) xuất hiện gần thông tin nhân khẩu học bệnh nhân có độ tin cậy cao hơn

### Văn Bản Thuốc và Chẩn Đoán

Thuật ngữ y khoa tạo nhiễu cho các model NER. Tên thuốc như "Panadol" hoặc "Metformin 500mg" không nên bị gắn cờ là thực thể. Tên bệnh như "Wong's Syndrome" không nên kích hoạt phát hiện tên người.

Giai đoạn lọc của PII Engineer duy trì từ vựng y khoa ngăn các tên thuốc, thủ thuật và tình trạng phổ biến bị phân loại sai thành PII.

## Tự Lưu Trữ: Không PHI Nào Rời Mạng Của Bạn

Đây là quyết định kiến trúc quan trọng cho tuân thủ y tế. PII Engineer chạy hoàn toàn trên hạ tầng của bạn:

```
┌─────────────────────────────────────────────┐
│  Your Network / VPC                          │
│                                              │
│  ┌──────────┐     ┌──────────────────────┐  │
│  │ EHR/EMR  │────▶│  PII Engineer Server │  │
│  │ System   │◀────│  (localhost:8000)     │  │
│  └──────────┘     └──────────────────────┘  │
│                                              │
│  No external API calls. No cloud dependency. │
│  Models loaded from local disk.              │
└─────────────────────────────────────────────┘
```

- **Không cần internet** sau lần tải model đầu tiên
- **Không dữ liệu rời khỏi máy** — suy luận chạy cục bộ trên CPU
- **Không cần thỏa thuận xử lý dữ liệu bên thứ ba**
- **Không cần kiểm toán nhà cung cấp cloud** cho thành phần phát hiện PII
- **Hỗ trợ triển khai cách ly mạng** — sao chép binary và file model qua phương thức truyền an toàn

## Ánh Xạ Tuân Thủ HIPAA

Quy tắc Bảo mật HIPAA yêu cầu biện pháp bảo vệ kỹ thuật cho hệ thống xử lý PHI. Đây là cách triển khai PII Engineer tự lưu trữ ánh xạ đến các yêu cầu chính:

| Yêu cầu HIPAA | Cách PII Engineer Đáp Ứng |
|---|---|
| Kiểm soát Truy cập (§164.312(a)) | Triển khai phía sau lớp xác thực hiện có; API không có xác thực tích hợp theo thiết kế — sử dụng kiểm soát mạng của bạn |
| Kiểm soát Kiểm toán (§164.312(b)) | Server ghi log tất cả request với timestamp; tích hợp với SIEM của bạn |
| Bảo mật Truyền tải (§164.312(e)) | Chạy trên localhost hoặc phía sau reverse proxy TLS; không truyền ra ngoài |
| Tối thiểu Cần thiết (§164.502(b)) | Chỉ phát hiện loại PHI bạn chỉ định qua tham số `labels` |
| Thỏa thuận Đối tác Kinh doanh | Không cần thiết — không bên thứ ba nào xử lý dữ liệu của bạn |

Điểm "Thỏa thuận Đối tác Kinh doanh" rất quan trọng. Khi bạn sử dụng dịch vụ phát hiện PII trên cloud, nhà cung cấp đó trở thành Đối tác Kinh doanh theo HIPAA và phải ký BAA. Với PII Engineer tự lưu trữ, toàn bộ gánh nặng tuân thủ này biến mất.

## Tuân Thủ PDPA (Singapore và Malaysia)

Y tế Đông Nam Á ngày càng chịu sự điều chỉnh của Đạo luật Bảo vệ Dữ liệu Cá nhân — PDPA Singapore (2012) và PDPA Malaysia (2010). Các yêu cầu chính:

### PDPA Singapore

| Nghĩa vụ | Liên quan đến phát hiện PII |
|---|---|
| Đồng ý (s13) | Ẩn danh dữ liệu trước khi sử dụng cho mục đích phụ (nghiên cứu, phân tích) |
| Giới hạn Mục đích (s18) | Chỉ thu thập/xử lý PII cho mục đích đã nêu |
| Bảo vệ (s24) | Biện pháp bảo mật hợp lý cho dữ liệu cá nhân |
| Giới hạn Chuyển giao (s26) | Dữ liệu cá nhân không được chuyển ra ngoài Singapore mà không có bảo vệ đầy đủ |

Nghĩa vụ **Giới hạn Chuyển giao** là lý do phát hiện PII trên cloud gặp vấn đề cho y tế Singapore. Nếu nhà cung cấp phát hiện PII của bạn xử lý dữ liệu tại data center Mỹ/EU, bạn cần đảm bảo bảo vệ đầy đủ theo s26. PII Engineer tự lưu trữ giữ tất cả dữ liệu trong Singapore.

### PDPA Malaysia

| Nguyên tắc | Liên quan |
|---|---|
| Nguyên tắc Bảo mật (s9) | Các bước thực tế để bảo vệ dữ liệu cá nhân khỏi mất mát, lạm dụng, truy cập trái phép |
| Nguyên tắc Lưu giữ (s10) | Không lưu giữ dữ liệu cá nhân lâu hơn mức cần thiết |
| Nguyên tắc Toàn vẹn Dữ liệu (s11) | Đảm bảo dữ liệu cá nhân chính xác và đầy đủ |

## Triển Khai Thực Tế Cho Y Tế

### Kiến Trúc Cho Hệ Thống Bệnh Viện

```python
# Integration with EHR export pipeline
import requests

def deidentify_clinical_note(note_text: str) -> dict:
    """De-identify a clinical note for research use."""
    resp = requests.post("http://pii-engine:8000/api/detect", json={
        "text": note_text,
        "labels": [
            "person_name",
            "government_id",
            "phone_number",
            "email_address",
            "street_address",
            "date_of_birth"
        ]
    })
    entities = resp.json()["entities"]

    # Replace with consistent pseudonyms for research utility
    deidentified = note_text
    name_counter = 0
    name_map = {}

    # Sort reverse for safe replacement
    for entity in sorted(entities, key=lambda e: e["start"], reverse=True):
        if entity["label"] == "person_name":
            if entity["text"] not in name_map:
                name_counter += 1
                name_map[entity["text"]] = f"PATIENT_{name_counter:03d}"
            replacement = name_map[entity["text"]]
        elif entity["label"] == "date_of_birth":
            replacement = "[DOB_REDACTED]"
        elif entity["label"] == "government_id":
            replacement = "[ID_REDACTED]"
        else:
            replacement = f"[{entity['label'].upper()}]"

        deidentified = deidentified[:entity["start"]] + replacement + deidentified[entity["end"]:]

    return {
        "deidentified_text": deidentified,
        "entities_removed": len(entities),
        "entity_types": list(set(e["label"] for e in entities))
    }
```

### Ẩn Danh Hàng Loạt Cho Dataset Nghiên Cứu

```python
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import json

def deidentify_dataset(input_dir: str, output_dir: str):
    """Batch de-identify clinical notes for research release."""
    input_path = Path(input_dir)
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    files = list(input_path.glob("*.txt"))
    print(f"Processing {len(files)} clinical notes...")

    def process_file(filepath):
        text = filepath.read_text(encoding="utf-8")
        result = deidentify_clinical_note(text)
        out_file = output_path / filepath.name
        out_file.write_text(result["deidentified_text"], encoding="utf-8")
        return filepath.name, result["entities_removed"]

    with ThreadPoolExecutor(max_workers=4) as executor:
        results = list(executor.map(process_file, files))

    total_entities = sum(r[1] for r in results)
    print(f"Done. Removed {total_entities} PII entities from {len(files)} notes.")
```

### Ghi Log Kiểm Toán

Để kiểm toán tuân thủ, ghi lại những gì được phát hiện mà không ghi giá trị PII thực tế:

```python
import logging
from datetime import datetime

audit_logger = logging.getLogger("pii_audit")

def detect_and_audit(text: str, document_id: str, user_id: str) -> list[dict]:
    entities = detect_pii(text)

    # Log detection event without the actual PII values
    audit_logger.info(json.dumps({
        "timestamp": datetime.utcnow().isoformat(),
        "document_id": document_id,
        "user_id": user_id,
        "entities_detected": len(entities),
        "entity_types": [e["label"] for e in entities],
        "text_length": len(text)
    }))

    return entities
```

## Độ Chính Xác Trên Văn Bản Y Tế

Chúng tôi đánh giá PII Engineer trên bộ test ghi chú lâm sàng tổng hợp bao gồm các mẫu y tế Singapore, Malaysia và Indonesia:

| Loại thực thể | Precision | Recall | F1 |
|---|---|---|---|
| Tên bệnh nhân (đa ngôn ngữ) | 0.84 | 0.87 | 0.85 |
| Số định danh chính phủ (NRIC/MyKad/NIK) | 0.93 | 0.95 | 0.94 |
| Số điện thoại | 0.96 | 0.97 | 0.97 |
| Ngày sinh | 0.89 | 0.88 | 0.89 |
| Địa chỉ | 0.88 | 0.85 | 0.86 |
| Địa chỉ email | 0.98 | 0.97 | 0.97 |

Tên người là danh mục khó nhất do sự đa dạng của quy ước đặt tên Đông Nam Á — tên phụ danh (bin/binti), tên Trung Quốc nhiều phần, tên Việt Nam ghép. Model xử lý các trường hợp này tốt hơn đáng kể so với phương pháp regex hoặc từ điển.

## Hạn Chế

Minh bạch về những gì PII Engineer không bao quát trong y tế:

- **Số Hồ sơ Y tế** — hệ thống đánh số tùy chỉnh của bệnh viện quá đa dạng cho model chung. Sử dụng quy tắc regex riêng cho định dạng MRN của bạn.
- **Dữ liệu sinh trắc học** — API xử lý văn bản, không phải hình ảnh hay vân tay.
- **Thông tin di truyền** — trình tự DNA hoặc dấu hiệu di truyền không được phát hiện là PII.
- **Định danh ngầm** — "bệnh nhân duy nhất nhập viện thứ Ba với gãy xương đùi" về mặt kỹ thuật có thể tái nhận dạng nhưng không thể phát hiện bằng NER.

Cho những khoảng trống này, kết hợp PII Engineer với các quy tắc đặc thù miền trong pipeline của bạn.

## Bắt Đầu

PII Engineer là mã nguồn mở theo giấy phép AGPL-3.0. Đối với triển khai y tế, kiến trúc tự lưu trữ có nghĩa là đội tuân thủ của bạn chỉ cần kiểm toán hạ tầng của chính mình.

```bash
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
```

- Mã nguồn: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- Models: [huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
