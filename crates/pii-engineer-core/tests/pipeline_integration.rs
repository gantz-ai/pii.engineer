use std::collections::HashMap;
use pii_engineer_core::{run_pipeline, Entity, PipelineConfig};
use pii_engineer_core::labels::{canonicalize, LABELS, LABEL_DESCRIPTIONS};
use pii_engineer_core::pipeline::default_labels;
use pii_engineer_core::lang::has_chinese;

fn entity(start: usize, end: usize, text: &str, label: &str, score: f32) -> Entity {
    Entity { start, end, text: text.to_string(), label: label.to_string(), score }
}

fn cfg() -> PipelineConfig {
    PipelineConfig { review_threshold: 0.25, label_thresholds: HashMap::new() }
}

// ── Pipeline: real-world PII scenarios ─────────────────────────────

#[test]
fn english_person_and_phone() {
    let text = "John Doe called from +65 9123 4567";
    let entities = vec![
        entity(0, 8, "John Doe", "person_name", 0.9),
        entity(21, 34, "+65 9123 4567", "phone_number", 0.95),
    ];
    let result = run_pipeline(entities, text, &cfg());
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].label, "person_name");
    assert_eq!(result[1].label, "phone_number");
}

#[test]
fn nric_detection() {
    let text = "NRIC: S1234567A";
    let entities = vec![entity(6, 15, "S1234567A", "government_id", 0.95)];
    let result = run_pipeline(entities, text, &cfg());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].text, "S1234567A");
}

#[test]
fn email_detected_by_pipeline() {
    let text = "Please email support@pii.engineer for help";
    let result = run_pipeline(vec![], text, &cfg());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].label, "email_address");
    assert_eq!(result[0].text, "support@pii.engineer");
}

#[test]
fn ip_address_detected_by_pipeline() {
    let text = "Access logs show 203.0.113.42 and 10.0.0.1";
    let result = run_pipeline(vec![], text, &cfg());
    assert_eq!(result.len(), 2);
    assert!(result.iter().all(|e| e.label == "ip_address"));
}

#[test]
fn mixed_pii_scenario() {
    let text = "Patient Tan Ah Kow (S8012345B) DOB 15/03/1980 lives at 123 Orchard Rd, email tan@mail.com";
    let entities = vec![
        entity(8, 18, "Tan Ah Kow", "person_name", 0.92),
        entity(20, 29, "S8012345B", "government_id", 0.97),
        entity(35, 45, "15/03/1980", "date_of_birth", 0.88),
        entity(55, 69, "123 Orchard Rd", "street_address", 0.85),
    ];
    let result = run_pipeline(entities, text, &cfg());
    let labels: Vec<&str> = result.iter().map(|e| e.label.as_str()).collect();
    assert!(labels.contains(&"person_name"));
    assert!(labels.contains(&"government_id"));
    assert!(labels.contains(&"date_of_birth"));
    assert!(labels.contains(&"street_address"));
    assert!(labels.contains(&"email_address"));
}

#[test]
fn malay_name_and_id() {
    let text = "Nama: Ahmad bin Abdullah, No KP: 900515-10-1234";
    let entities = vec![
        entity(6, 24, "Ahmad bin Abdullah", "person_name", 0.88),
        entity(33, 48, "900515-10-1234", "government_id", 0.91),
    ];
    let result = run_pipeline(entities, text, &cfg());
    assert_eq!(result.len(), 2);
}

#[test]
fn pipeline_strips_context_prefix() {
    let text = "patient John Doe has an appointment";
    let entities = vec![entity(0, 16, "patient John Doe", "person_name", 0.85)];
    let result = run_pipeline(entities, text, &cfg());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].text, "John Doe");
}

#[test]
fn pipeline_filters_pronouns() {
    let text = "i told she that you called me";
    let entities = vec![
        entity(0, 1, "i", "person_name", 0.4),
        entity(7, 10, "she", "person_name", 0.38),
        entity(16, 19, "you", "person_name", 0.36),
        entity(27, 29, "me", "person_name", 0.35),
    ];
    let result = run_pipeline(entities, text, &cfg());
    assert!(result.is_empty());
}

#[test]
fn pipeline_rejects_bad_formats() {
    let text = "ID: ABC, Phone: hello, Passport: 12345678901234567890";
    let entities = vec![
        entity(4, 7, "ABC", "government_id", 0.9),
        entity(16, 21, "hello", "phone_number", 0.9),
        entity(33, 53, "12345678901234567890", "passport_number", 0.9),
    ];
    let result = run_pipeline(entities, text, &cfg());
    assert!(result.is_empty());
}

#[test]
fn pipeline_deduplicates_overlapping() {
    let text = "John Doe lives at 123 Main Street";
    let entities = vec![
        entity(0, 4, "John", "person_name", 0.8),
        entity(0, 8, "John Doe", "person_name", 0.85),
    ];
    let result = run_pipeline(entities, text, &cfg());
    let names: Vec<&Entity> = result.iter().filter(|e| e.label == "person_name").collect();
    assert_eq!(names.len(), 1);
    assert_eq!(names[0].text, "John Doe");
}

#[test]
fn empty_text_returns_empty() {
    let result = run_pipeline(vec![], "", &cfg());
    assert!(result.is_empty());
}

#[test]
fn no_pii_text() {
    let text = "The quick brown fox jumps over the lazy dog";
    let result = run_pipeline(vec![], text, &cfg());
    assert!(result.is_empty());
}

// ── Labels ─────────────────────────────────────────────────────────

#[test]
fn all_nine_labels_present() {
    assert_eq!(LABELS.len(), 9);
    let expected = [
        "person_name", "phone_number", "government_id", "street_address",
        "date_of_birth", "email_address", "passport_number",
        "license_plate", "bank_account_number",
    ];
    for label in &expected {
        assert!(LABELS.contains(label), "missing label: {label}");
    }
}

#[test]
fn default_labels_matches_labels() {
    let defaults = default_labels();
    assert_eq!(defaults.len(), LABELS.len());
    for label in LABELS {
        assert!(defaults.contains(&label.to_string()));
    }
}

#[test]
fn every_label_has_description() {
    for label in LABELS {
        let found = LABEL_DESCRIPTIONS.iter().any(|(l, _)| l == label);
        assert!(found, "no description for {label}");
    }
}

#[test]
fn canonicalize_roundtrips() {
    for label in LABELS {
        assert_eq!(canonicalize(label), Some(*label));
    }
}

#[test]
fn canonicalize_southeast_asian_ids() {
    assert_eq!(canonicalize("nric"), Some("government_id"));
    assert_eq!(canonicalize("cccd"), Some("government_id"));
    assert_eq!(canonicalize("nik"), Some("government_id"));
    assert_eq!(canonicalize("aadhaar"), Some("government_id"));
    assert_eq!(canonicalize("ktp"), Some("government_id"));
}

// ── Language detection ─────────────────────────────────────────────

#[test]
fn chinese_text_detected() {
    assert!(has_chinese("我的电话是 9123 4567"));
    assert!(has_chinese("请联系张三"));
}

#[test]
fn non_chinese_text() {
    assert!(!has_chinese("Hello World"));
    assert!(!has_chinese("Nama saya Ahmad"));
    assert!(!has_chinese("こんにちは")); // Japanese hiragana, not CJK unified
}

#[test]
fn mixed_chinese_english() {
    assert!(has_chinese("My name is 李明 and I live in Singapore"));
}

// ── Model loading (requires model files) ───────────────────────────

#[test]
fn model_load_missing_dir_returns_error() {
    let result = pii_engineer_core::GlinerSpanModel::load("/nonexistent/path");
    assert!(result.is_err());
}
