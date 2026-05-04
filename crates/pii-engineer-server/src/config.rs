//! Server configuration. Env prefix: `PII_ENGINEER_`.

use std::collections::HashMap;
use std::env;

#[derive(Debug, Clone)]
pub struct Settings {
    pub host: String,
    pub port: u16,
    pub log_level: String,

    pub gliner_models: Vec<String>,
    pub chinese_ner_model: String,
    pub auto_redact_threshold: f32,
    pub review_threshold: f32,
    pub raw_threshold: f32,
    pub label_thresholds: HashMap<String, f32>,

    pub max_text_length: usize,
    pub rate_limit_rpm: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 8000,
            log_level: "info".into(),

            gliner_models: vec!["models/PII-Engineer-Multi-NER-v2.1".into()],
            chinese_ner_model: "models/PII-Engineer-Chinese-NER-v1.0".into(),

            auto_redact_threshold: 0.6,
            review_threshold: 0.25,
            raw_threshold: 0.08,
            label_thresholds: default_label_thresholds(),

            max_text_length: 50_000,
            rate_limit_rpm: 120,
        }
    }
}

impl Settings {
    pub fn from_env() -> Self {
        let mut s = Self::default();

        if let Ok(v) = env::var("PII_ENGINEER_HOST") { s.host = v; }
        if let Ok(v) = env::var("PII_ENGINEER_PORT").or_else(|_| env::var("PORT")) {
            if let Ok(n) = v.parse() { s.port = n; }
        }
        if let Ok(v) = env::var("PII_ENGINEER_LOG_LEVEL") { s.log_level = v; }

        if let Ok(v) = env::var("GLINER_MODELS") {
            s.gliner_models = split_csv(&v);
        }
        if let Ok(v) = env::var("CHINESE_NER_MODEL") { s.chinese_ner_model = v; }

        if let Ok(v) = env::var("PII_ENGINEER_AUTO_REDACT_THRESHOLD") {
            if let Ok(n) = v.parse() { s.auto_redact_threshold = n; }
        }
        if let Ok(v) = env::var("PII_ENGINEER_REVIEW_THRESHOLD") {
            if let Ok(n) = v.parse() { s.review_threshold = n; }
        }
        if let Ok(v) = env::var("PII_ENGINEER_LABEL_THRESHOLDS") {
            for pair in v.split(',') {
                if let Some((label, thresh)) = pair.split_once(':') {
                    if let Ok(t) = thresh.trim().parse::<f32>() {
                        s.label_thresholds.insert(label.trim().to_string(), t);
                    }
                }
            }
        }
        if let Ok(v) = env::var("PII_ENGINEER_RATE_LIMIT_RPM") {
            if let Ok(n) = v.parse() { s.rate_limit_rpm = n; }
        }
        s
    }
}

fn default_label_thresholds() -> HashMap<String, f32> {
    HashMap::from([
        ("person_name".into(),         0.25),
        ("phone_number".into(),        0.30),
        ("government_id".into(),       0.20),
        ("street_address".into(),      0.25),
        ("date_of_birth".into(),       0.25),
        ("email_address".into(),       0.30),
        ("passport_number".into(),     0.25),
        ("license_plate".into(),       0.25),
        ("bank_account_number".into(), 0.25),
    ])
}

fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}
