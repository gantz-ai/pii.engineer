//! Post-processing pipeline for NER entities.

use std::collections::HashMap;

use crate::gliner::Entity;
use crate::labels::{
    canonicalize, chinese_phone_marker_present, context_prefixes, meta_words,
};

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub review_threshold: f32,
    pub label_thresholds: HashMap<String, f32>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            review_threshold: 0.25,
            label_thresholds: HashMap::new(),
        }
    }
}

pub fn default_labels() -> Vec<String> {
    crate::labels::LABELS.iter().map(|s| s.to_string()).collect()
}

pub fn run(mut entities: Vec<Entity>, text: &str, cfg: &PipelineConfig) -> Vec<Entity> {
    entities = reclassify(entities);
    entities = validate_format(entities);
    entities = meta_filter(entities);
    entities = normalize(entities);
    entities = meta_filter(entities);
    entities = expand_emails(entities, text);
    entities.extend(detect_emails(text, &entities));
    entities.extend(detect_ip_addresses(text));
    entities = threshold(entities, cfg);
    entities = dedup(entities);
    entities = merge_adjacent(entities, text);
    entities
}

// 1. reclassify

fn reclassify(entities: Vec<Entity>) -> Vec<Entity> {
    entities
        .into_iter()
        .map(reclassify_chinese_phone)
        .collect()
}

fn reclassify_chinese_phone(mut e: Entity) -> Entity {
    let has_marker = chinese_phone_marker_present(&e.text);
    let digits = e.text.chars().filter(|c| c.is_ascii_digit()).count();
    if has_marker && (7..=12).contains(&digits) {
        e.label = "contact number".to_string();
        e.score = e.score.max(0.35);
    }
    e
}

// 1b. format validation — reject entities that don't match their label's expected pattern

fn validate_format(entities: Vec<Entity>) -> Vec<Entity> {
    entities.into_iter().filter(is_valid_for_label).collect()
}

fn is_valid_for_label(e: &Entity) -> bool {
    let t = e.text.trim();
    let digits_only: String = t.chars().filter(|c| c.is_ascii_digit()).collect();
    let digit_count = digits_only.len();

    match e.label.as_str() {
        "government_id" => {
            (6..=20).contains(&t.len())
                && t.chars().any(|c| c.is_ascii_digit())
                && t.chars().all(|c| c.is_ascii_alphanumeric() || " -".contains(c))
        }
        "phone_number" => {
            (7..=15).contains(&digit_count)
                && t.chars().all(|c| c.is_ascii_digit() || " -+()".contains(c))
        }
        "email_address" => t.contains('@') && t.contains('.'),
        "passport_number" => {
            (6..=12).contains(&t.len())
                && t.chars().any(|c| c.is_ascii_alphabetic())
                && t.chars().any(|c| c.is_ascii_digit())
        }
        "license_plate" => {
            (3..=15).contains(&t.len())
                && t.chars().any(|c| c.is_ascii_alphanumeric())
        }
        "bank_account_number" => {
            (8..=20).contains(&digit_count)
                && t.chars().all(|c| c.is_ascii_digit() || c == ' ' || c == '-')
        }
        "date_of_birth" => {
            digit_count >= 4
                && t.chars().any(|c| c.is_ascii_digit())
        }
        // person, street_address — no strict format, always pass
        _ => true,
    }
}

// 2. meta filter

fn clean_for_meta(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .trim_matches(|c: char| {
            c.is_whitespace() || ".,;:!?()[]{}，。；：".contains(c)
        })
        .to_string()
}

fn meta_filter(entities: Vec<Entity>) -> Vec<Entity> {
    let words = meta_words();
    entities
        .into_iter()
        .filter(|e| {
            let trimmed = e.text.trim();
            !trimmed.is_empty() && !words.contains(clean_for_meta(trimmed).as_str())
        })
        .collect()
}

// 3. normalize (canonicalize labels + strip context prefix/suffix)

const STRIP_CHARS: &[char] = &[
    ' ', '\t', '.', ',', ';', ':', '!', '?', '\u{3000}', '，', '。', '；', '：',
];

fn is_word_boundary(b: u8) -> bool {
    !b.is_ascii_alphanumeric() && b != b'_'
}

fn strip_context_prefix(mut e: Entity) -> Entity {
    let prefixes = context_prefixes();
    for _ in 0..5 {
        let lowered = e.text.to_lowercase();
        let mut matched = false;
        for prefix in prefixes {
            if lowered.starts_with(prefix) {
                let after = lowered.as_bytes().get(prefix.len());
                if after.is_some_and(|&b| !is_word_boundary(b)) {
                    continue;
                }
                let remaining = &e.text[prefix.len()..];
                let stripped = remaining.trim_start_matches(STRIP_CHARS);
                if !stripped.is_empty() {
                    let offset = e.text.len() - stripped.len();
                    e.start += offset;
                    e.text = stripped.to_string();
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            break;
        }
    }
    e
}

fn strip_context_suffix(mut e: Entity) -> Entity {
    let suffixes = context_prefixes();
    for _ in 0..5 {
        let lowered = e.text.to_lowercase();
        let mut matched = false;
        for suffix in suffixes {
            if lowered.ends_with(suffix) {
                let cut = lowered.len() - suffix.len();
                if cut > 0 {
                    let before = lowered.as_bytes().get(cut - 1);
                    if before.is_some_and(|&b| !is_word_boundary(b)) {
                        continue;
                    }
                }
                let remaining = &e.text[..e.text.len() - suffix.len()];
                let stripped = remaining.trim_end_matches(STRIP_CHARS);
                if !stripped.is_empty() {
                    let removed = e.text.len() - stripped.len();
                    e.end -= removed;
                    e.text = stripped.to_string();
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            break;
        }
    }
    e
}

fn normalize(entities: Vec<Entity>) -> Vec<Entity> {
    entities
        .into_iter()
        .map(|mut e| {
            if let Some(c) = canonicalize(&e.label) {
                e.label = c.to_string();
            }
            e
        })
        .map(strip_context_prefix)
        .map(strip_context_suffix)
        .collect()
}

// 5. email expansion + detection

fn find_email_at(text: &str, pos: usize) -> Option<(usize, usize)> {
    let at_pos = text[pos..].find('@').map(|i| i + pos)?;
    let local_start = text[..at_pos]
        .bytes()
        .rposition(|b| b == b' ' || b == b'\t' || b == b'\n' || b == b',' || b == b';' || b == b'(' || b == b'<')
        .map(|i| i + 1)
        .unwrap_or(0);
    let local = &text[local_start..at_pos];
    if local.is_empty() || !local.bytes().all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b)) {
        return None;
    }
    let domain_bytes = &text.as_bytes()[at_pos + 1..];
    let mut domain_end = 0;
    while domain_end < domain_bytes.len()
        && (domain_bytes[domain_end].is_ascii_alphanumeric() || domain_bytes[domain_end] == b'.' || domain_bytes[domain_end] == b'-')
    {
        domain_end += 1;
    }
    let domain = &text[at_pos + 1..at_pos + 1 + domain_end];
    if !domain.contains('.') || domain.len() < 3 || domain.ends_with('.') || domain.starts_with('.') {
        return None;
    }
    Some((local_start, at_pos + 1 + domain_end))
}

fn expand_emails(entities: Vec<Entity>, text: &str) -> Vec<Entity> {
    entities
        .into_iter()
        .map(|mut e| {
            if let Some((start, end)) = find_email_at(text, e.start) {
                if start <= e.start && end >= e.end {
                    e.start = start;
                    e.end = end;
                    e.text = text[start..end].to_string();
                    e.label = "email_address".to_string();
                    e.score = e.score.max(0.8);
                }
            }
            e
        })
        .collect()
}

fn detect_emails(text: &str, existing: &[Entity]) -> Vec<Entity> {
    let mut results = Vec::new();
    let mut search_from = 0;
    while let Some(at_rel) = text[search_from..].find('@') {
        let at_pos = search_from + at_rel;
        if let Some((start, end)) = find_email_at(text, at_pos.saturating_sub(64).max(search_from)) {
            let covered = existing.iter().any(|e| e.start <= start && e.end >= end);
            if !covered {
                results.push(Entity {
                    start,
                    end,
                    text: text[start..end].to_string(),
                    label: "email_address".to_string(),
                    score: 0.9,
                });
            }
            search_from = end;
        } else {
            search_from = at_pos + 1;
        }
    }
    results
}

// 6. threshold

fn threshold(entities: Vec<Entity>, cfg: &PipelineConfig) -> Vec<Entity> {
    entities.into_iter().filter(|e| {
        let t = cfg.label_thresholds.get(&e.label)
            .copied()
            .unwrap_or(cfg.review_threshold);
        e.score >= t
    }).collect()
}

// 7. dedup overlapping spans

fn dedup(mut entities: Vec<Entity>) -> Vec<Entity> {
    entities.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then((b.end - b.start).cmp(&(a.end - a.start)))
            .then(b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal))
    });

    let mut out: Vec<Entity> = Vec::with_capacity(entities.len());
    for e in entities {
        if let Some(last) = out.last_mut() {
            if e.start < last.end {
                let cur_len = last.end - last.start;
                let new_len = e.end - e.start;
                let same_label = e.label == last.label;
                let replace = (same_label && new_len > cur_len)
                    || (!same_label && e.score > last.score * 1.3);
                if replace {
                    *last = e;
                }
                continue;
            }
        }
        out.push(e);
    }
    out
}

// 8. regex-based IP address detection

fn detect_ip_addresses(text: &str) -> Vec<Entity> {
    let mut results = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i].is_ascii_digit() {
            if let Some((ip, end)) = try_parse_ipv4(text, i) {
                let before = if i > 0 { bytes[i - 1] } else { b' ' };
                let after = if end < len { bytes[end] } else { b' ' };
                if !before.is_ascii_alphanumeric() && before != b'.' && !after.is_ascii_digit() && after != b'.' {
                    results.push(Entity {
                        start: i,
                        end,
                        text: ip.to_string(),
                        label: "ip_address".to_string(),
                        score: 1.0,
                    });
                    i = end;
                    continue;
                }
            }
        }
        i += 1;
    }
    results
}

fn try_parse_ipv4(text: &str, start: usize) -> Option<(&str, usize)> {
    let rest = &text[start..];
    let mut octets = 0;
    let mut pos = 0;

    for _ in 0..4 {
        let digit_start = pos;
        while pos < rest.len() && rest.as_bytes()[pos].is_ascii_digit() {
            pos += 1;
        }
        let digit_len = pos - digit_start;
        if digit_len == 0 || digit_len > 3 {
            return None;
        }
        let octet: u16 = rest[digit_start..pos].parse().ok()?;
        if octet > 255 {
            return None;
        }
        if digit_len > 1 && rest.as_bytes()[digit_start] == b'0' {
            return None;
        }
        octets += 1;
        if octets < 4 {
            if pos >= rest.len() || rest.as_bytes()[pos] != b'.' {
                return None;
            }
            pos += 1;
        }
    }

    if octets == 4 {
        Some((&rest[..pos], start + pos))
    } else {
        None
    }
}

// 9. merge adjacent same-label spans separated by ≤ 2 chars

fn merge_adjacent(entities: Vec<Entity>, text: &str) -> Vec<Entity> {
    let mut merged: Vec<Entity> = Vec::with_capacity(entities.len());
    for e in entities {
        if let Some(last) = merged.last_mut() {
            let gap_len = e.start.saturating_sub(last.end);
            if last.label == e.label && gap_len <= 2 && e.start >= last.end {
                let gap = &text[last.end..e.start];
                last.text = format!("{}{}{}", last.text, gap, e.text);
                last.end = e.end;
                last.score = last.score.max(e.score);
                continue;
            }
        }
        merged.push(e);
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(start: usize, end: usize, text: &str, label: &str, score: f32) -> Entity {
        Entity { start, end, text: text.to_string(), label: label.to_string(), score }
    }

    fn default_cfg() -> PipelineConfig {
        PipelineConfig { review_threshold: 0.25, label_thresholds: HashMap::new() }
    }

    // ── format validation ──────────────────────────────────────────

    #[test]
    fn valid_government_id() {
        let e = entity(0, 9, "S1234567A", "government_id", 0.9);
        assert!(is_valid_for_label(&e));
    }

    #[test]
    fn reject_short_government_id() {
        let e = entity(0, 3, "S12", "government_id", 0.9);
        assert!(!is_valid_for_label(&e));
    }

    #[test]
    fn reject_government_id_no_digits() {
        let e = entity(0, 8, "ABCDEFGH", "government_id", 0.9);
        assert!(!is_valid_for_label(&e));
    }

    #[test]
    fn valid_phone_number() {
        let e = entity(0, 13, "+65 9123 4567", "phone_number", 0.9);
        assert!(is_valid_for_label(&e));
    }

    #[test]
    fn reject_short_phone() {
        let e = entity(0, 4, "1234", "phone_number", 0.9);
        assert!(!is_valid_for_label(&e));
    }

    #[test]
    fn reject_phone_with_letters() {
        let e = entity(0, 10, "012-ABC-45", "phone_number", 0.9);
        assert!(!is_valid_for_label(&e));
    }

    #[test]
    fn valid_email() {
        let e = entity(0, 16, "john@example.com", "email_address", 0.9);
        assert!(is_valid_for_label(&e));
    }

    #[test]
    fn reject_email_no_at() {
        let e = entity(0, 15, "john.example.com", "email_address", 0.9);
        assert!(!is_valid_for_label(&e));
    }

    #[test]
    fn valid_passport() {
        let e = entity(0, 9, "E12345678", "passport_number", 0.9);
        assert!(is_valid_for_label(&e));
    }

    #[test]
    fn reject_passport_digits_only() {
        let e = entity(0, 9, "123456789", "passport_number", 0.9);
        assert!(!is_valid_for_label(&e));
    }

    #[test]
    fn valid_bank_account() {
        let e = entity(0, 14, "1234 5678 9012", "bank_account_number", 0.9);
        assert!(is_valid_for_label(&e));
    }

    #[test]
    fn reject_short_bank_account() {
        let e = entity(0, 5, "12345", "bank_account_number", 0.9);
        assert!(!is_valid_for_label(&e));
    }

    #[test]
    fn valid_license_plate() {
        let e = entity(0, 8, "SBA1234A", "license_plate", 0.9);
        assert!(is_valid_for_label(&e));
    }

    #[test]
    fn valid_dob() {
        let e = entity(0, 10, "1990-05-15", "date_of_birth", 0.9);
        assert!(is_valid_for_label(&e));
    }

    #[test]
    fn reject_dob_too_few_digits() {
        let e = entity(0, 3, "May", "date_of_birth", 0.9);
        assert!(!is_valid_for_label(&e));
    }

    #[test]
    fn person_name_always_valid() {
        let e = entity(0, 8, "John Doe", "person_name", 0.9);
        assert!(is_valid_for_label(&e));
    }

    // ── meta filter ────────────────────────────────────────────────

    #[test]
    fn meta_filter_removes_label_words() {
        let entities = vec![entity(0, 4, "name", "person_name", 0.9)];
        let result = meta_filter(entities);
        assert!(result.is_empty());
    }

    #[test]
    fn meta_filter_removes_pronouns() {
        let entities = vec![entity(0, 3, "you", "person_name", 0.9)];
        let result = meta_filter(entities);
        assert!(result.is_empty());
    }

    #[test]
    fn meta_filter_keeps_real_names() {
        let entities = vec![entity(0, 8, "John Doe", "person_name", 0.9)];
        let result = meta_filter(entities);
        assert_eq!(result.len(), 1);
    }

    // ── email detection ────────────────────────────────────────────

    #[test]
    fn detect_email_in_text() {
        let text = "Contact john@example.com for details";
        let existing: Vec<Entity> = vec![];
        let emails = detect_emails(text, &existing);
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].text, "john@example.com");
        assert_eq!(emails[0].start, 8);
        assert_eq!(emails[0].end, 24);
    }

    #[test]
    fn detect_multiple_emails() {
        let text = "Email a@b.co or c@d.io";
        let emails = detect_emails(text, &[]);
        assert_eq!(emails.len(), 2);
    }

    #[test]
    fn no_email_without_domain() {
        let text = "user@localhost is not valid";
        let emails = detect_emails(text, &[]);
        assert!(emails.is_empty());
    }

    #[test]
    fn skip_already_covered_email() {
        let text = "Email john@example.com here";
        let existing = vec![entity(6, 22, "john@example.com", "email_address", 0.9)];
        let emails = detect_emails(text, &existing);
        assert!(emails.is_empty());
    }

    // ── IP address detection ───────────────────────────────────────

    #[test]
    fn detect_ipv4() {
        let text = "Server at 192.168.1.1 is down";
        let ips = detect_ip_addresses(text);
        assert_eq!(ips.len(), 1);
        assert_eq!(ips[0].text, "192.168.1.1");
    }

    #[test]
    fn reject_octet_over_255() {
        let text = "Value 999.999.999.999 here";
        let ips = detect_ip_addresses(text);
        assert!(ips.is_empty());
    }

    #[test]
    fn reject_leading_zero_octet() {
        let text = "Value 01.02.03.04 here";
        let ips = detect_ip_addresses(text);
        assert!(ips.is_empty());
    }

    #[test]
    fn detect_multiple_ips() {
        let text = "From 10.0.0.1 to 10.0.0.2";
        let ips = detect_ip_addresses(text);
        assert_eq!(ips.len(), 2);
    }

    // ── threshold ──────────────────────────────────────────────────

    #[test]
    fn threshold_filters_low_scores() {
        let cfg = PipelineConfig {
            review_threshold: 0.5,
            label_thresholds: HashMap::new(),
        };
        let entities = vec![
            entity(0, 4, "John", "person_name", 0.8),
            entity(5, 9, "weak", "person_name", 0.3),
        ];
        let result = threshold(entities, &cfg);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "John");
    }

    #[test]
    fn threshold_uses_label_specific() {
        let mut thresholds = HashMap::new();
        thresholds.insert("phone_number".to_string(), 0.9);
        let cfg = PipelineConfig { review_threshold: 0.25, label_thresholds: thresholds };
        let entities = vec![
            entity(0, 12, "+65 91234567", "phone_number", 0.85),
        ];
        let result = threshold(entities, &cfg);
        assert!(result.is_empty());
    }

    // ── dedup ──────────────────────────────────────────────────────

    #[test]
    fn dedup_overlapping_same_label_keeps_longer() {
        let entities = vec![
            entity(0, 4, "John", "person_name", 0.9),
            entity(0, 8, "John Doe", "person_name", 0.85),
        ];
        let result = dedup(entities);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "John Doe");
    }

    #[test]
    fn dedup_non_overlapping_keeps_both() {
        let entities = vec![
            entity(0, 4, "John", "person_name", 0.9),
            entity(10, 22, "+65 91234567", "phone_number", 0.95),
        ];
        let result = dedup(entities);
        assert_eq!(result.len(), 2);
    }

    // ── merge adjacent ─────────────────────────────────────────────

    #[test]
    fn merge_adjacent_same_label() {
        let text = "John A. Doe lives here";
        let entities = vec![
            entity(0, 4, "John", "person_name", 0.8),
            entity(5, 11, "A. Doe", "person_name", 0.85),
        ];
        let result = merge_adjacent(entities, text);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "John A. Doe");
        assert_eq!(result[0].score, 0.85);
    }

    #[test]
    fn no_merge_different_labels() {
        let text = "John 91234567";
        let entities = vec![
            entity(0, 4, "John", "person_name", 0.9),
            entity(5, 13, "91234567", "phone_number", 0.9),
        ];
        let result = merge_adjacent(entities, text);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn no_merge_large_gap() {
        let text = "John    Doe";
        let entities = vec![
            entity(0, 4, "John", "person_name", 0.9),
            entity(8, 11, "Doe", "person_name", 0.9),
        ];
        let result = merge_adjacent(entities, text);
        assert_eq!(result.len(), 2);
    }

    // ── chinese phone reclassification ─────────────────────────────

    #[test]
    fn reclassify_chinese_phone_marker() {
        let e = entity(0, 14, "电话 91234567", "phone_number", 0.3);
        let result = reclassify_chinese_phone(e);
        assert_eq!(result.label, "contact number");
        assert!(result.score >= 0.35);
    }

    #[test]
    fn no_reclassify_without_marker() {
        let e = entity(0, 8, "91234567", "phone_number", 0.9);
        let result = reclassify_chinese_phone(e);
        assert_eq!(result.label, "phone_number");
    }

    // ── full pipeline ──────────────────────────────────────────────

    #[test]
    fn pipeline_end_to_end() {
        let text = "John Doe called from +65 9123 4567, email john@test.com";
        let entities = vec![
            entity(0, 8, "John Doe", "person_name", 0.9),
            entity(21, 34, "+65 9123 4567", "phone_number", 0.95),
        ];
        let result = run(entities, text, &default_cfg());
        assert!(result.len() >= 2);
        let labels: Vec<&str> = result.iter().map(|e| e.label.as_str()).collect();
        assert!(labels.contains(&"person_name"));
        assert!(labels.contains(&"phone_number"));
        assert!(labels.contains(&"email_address"));
    }

    #[test]
    fn pipeline_filters_meta_words() {
        let text = "patient name is phone number";
        let entities = vec![
            entity(0, 7, "patient", "person_name", 0.9),
            entity(16, 28, "phone number", "phone_number", 0.9),
        ];
        let result = run(entities, text, &default_cfg());
        assert!(result.is_empty());
    }

    #[test]
    fn pipeline_detects_ip_addresses() {
        let text = "Server 10.0.0.1 is running";
        let result = run(vec![], text, &default_cfg());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].label, "ip_address");
        assert_eq!(result[0].text, "10.0.0.1");
    }

    #[test]
    fn pipeline_rejects_invalid_formats() {
        let text = "ID is ABC and phone is 123";
        let entities = vec![
            entity(6, 9, "ABC", "government_id", 0.9),
            entity(23, 26, "123", "phone_number", 0.9),
        ];
        let result = run(entities, text, &default_cfg());
        assert!(result.is_empty());
    }

    // ── context prefix/suffix stripping ────────────────────────────

    #[test]
    fn strip_patient_prefix() {
        let e = entity(0, 16, "patient John Doe", "person_name", 0.9);
        let result = strip_context_prefix(e);
        assert_eq!(result.text, "John Doe");
    }

    #[test]
    fn strip_title_prefix() {
        let e = entity(0, 11, "mr John Doe", "person_name", 0.9);
        let result = strip_context_prefix(e);
        assert_eq!(result.text, "John Doe");
    }

    // ── default labels ─────────────────────────────────────────────

    #[test]
    fn default_labels_returns_all() {
        let labels = default_labels();
        assert_eq!(labels.len(), 9);
        assert!(labels.contains(&"person_name".to_string()));
        assert!(labels.contains(&"phone_number".to_string()));
        assert!(labels.contains(&"government_id".to_string()));
        assert!(labels.contains(&"email_address".to_string()));
    }
}

