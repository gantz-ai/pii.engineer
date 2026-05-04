//! Canonical PII label scheme and meta vocabulary.

use once_cell::sync::Lazy;
use std::collections::HashSet;

pub const LABELS: &[&str] = &[
    "person_name",
    "phone_number",
    "government_id",
    "street_address",
    "date_of_birth",
    "email_address",
    "passport_number",
    "license_plate",
    "bank_account_number",
];

pub const PROMPT_LABELS: &[&str] = LABELS;

pub const LABEL_DESCRIPTIONS: &[(&str, &str)] = &[
    ("person_name", "full name of a real person including first name and family name"),
    ("phone_number", "telephone or mobile phone number with country or area code"),
    ("government_id", "national identification number such as NRIC MyKad IC Aadhaar CCCD NIK Thai ID Citizen ID"),
    ("street_address", "physical street address including building number street name city or district"),
    ("date_of_birth", "date of birth of a person in any date format"),
    ("email_address", "email address with username and domain name"),
    ("passport_number", "passport travel document number with country prefix"),
    ("license_plate", "vehicle license plate number"),
    ("bank_account_number", "bank account number or routing number for financial transactions"),
];

pub const CHINESE_PHONE_MARKERS: &[&str] = &["电话", "手机", "号码", "联系"];

pub fn chinese_phone_marker_present(s: &str) -> bool {
    CHINESE_PHONE_MARKERS.iter().any(|m| s.contains(m))
}

pub fn canonicalize(label: &str) -> Option<&'static str> {
    let lower = label.to_lowercase();
    for canonical in LABELS {
        if &lower == canonical {
            return Some(canonical);
        }
    }
    match lower.as_str() {
        "person" | "person name" | "patient name" | "full name" | "姓名" => Some("person_name"),
        "phone number" | "mobile number" | "contact number" | "联系电话" | "电话" | "手机" => Some("phone_number"),
        "national id" | "identification number" | "id number" | "identity card number"
        | "nric" | "cccd" | "thai_id" | "nik" | "aadhaar"
        | "citizen id" | "can cuoc cong dan" | "căn cước công dân" | "cmnd"
        | "บัตรประชาชน" | "thai national id" | "thai citizen id" | "เลขบัตรประชาชน"
        | "nomor induk kependudukan" | "ktp" | "kartu tanda penduduk"
        | "aadhar" | "aadhar card" | "aadhaar card" | "aadhaar number" | "आधार" | "aadhaar no"
        | "身份证号码" => Some("government_id"),
        "mailing_address" | "mailing address" | "address" | "home address" | "地址" | "住址" => Some("street_address"),
        "date of birth" | "birthday" | "出生日期" | "生日" => Some("date_of_birth"),
        "email" | "email address" | "电子邮件" => Some("email_address"),
        "passport" | "passport number" => Some("passport_number"),
        "license plate" | "vehicle plate" | "车牌" | "車牌" => Some("license_plate"),
        "bank account" | "account number" | "银行账户" => Some("bank_account_number"),
        _ => None,
    }
}

static META_WORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set: HashSet<&'static str> = HashSet::new();
    for variant in PROMPT_LABELS {
        for word in variant.split_whitespace() {
            set.insert(word);
        }
        set.insert(*variant);
    }
    for canonical in LABELS {
        set.insert(*canonical);
    }
    // All label variants (including ones not in PROMPT_LABELS) for filtering
    for w in &[
        "person", "patient", "full", "name", "姓名",
        "phone", "mobile", "联系电话", "电话", "手机",
        "national", "identification", "身份证号码",
        "address", "home", "mailing", "地址", "住址",
        "birthday", "出生日期", "生日",
        "电子邮件",
        "passport", "vehicle", "车牌", "車牌",
        "银行账户", "account",
    ] {
        set.insert(*w);
    }
    for w in &[
        "dob", "d.o.b", "d.o.b.", "id", "no", "no.", "number", "contact",
        "new", "old", "current", "previous", "on file",
        "tomorrow", "yesterday", "today", "tonight",
        "morning", "afternoon", "evening", "noon", "midnight",
        "emergency", "urgent", "primary", "secondary", "main", "standard",
        "doctor", "nurse", "patient", "clinic", "hospital", "lab",
        "mdm", "madam", "sir",
        "n/a", "na", "n.a.", "tbd", "tbc", "unknown", "anonymous", "none",
        "周一", "周二", "周三", "周四", "周五", "周六", "周日",
        "星期一", "星期二", "星期三", "星期四", "星期五", "星期六", "星期日",
        "上午", "下午", "晚上", "中午",
        "i", "you", "he", "she", "we", "they", "it",
        "me", "my", "your", "his", "her", "their", "our", "us",
        "i'm", "i've", "you're", "we're", "they're",
        "我", "你", "他", "她", "我们", "你们", "他们", "它",
        "mom", "dad", "mum", "mother", "father", "parent", "parents",
        "husband", "wife", "spouse", "son", "daughter", "kid", "kids",
        "child", "children", "sister", "brother", "sibling",
        "grandma", "grandpa", "grandmother", "grandfather",
        "uncle", "aunt", "cousin", "nephew", "niece",
        "friend", "colleague", "neighbour", "neighbor",
        "someone", "anyone", "everyone", "nobody", "people",
        "妈妈", "爸爸", "母亲", "父亲", "儿子", "女儿",
        "妻子", "丈夫", "哥哥", "弟弟", "姐姐", "妹妹",
        "爷爷", "奶奶", "外公", "外婆",
    ] {
        set.insert(*w);
    }
    set
});

pub fn meta_words() -> &'static HashSet<&'static str> {
    &META_WORDS
}

static CONTEXT_PREFIXES: Lazy<Vec<String>> = Lazy::new(|| {
    let mut v: HashSet<String> = META_WORDS.iter().map(|s| s.to_string()).collect();
    for w in &[
        "patient", "doctor", "dr", "mr", "mrs", "ms",
        "born", "aged", "called",
        "患者", "医生", "大夫", "先生", "女士",
        "我的", "他的", "她的", "你的",
        "是", "叫", "住在", "住址", "电话是", "生日是",
        "名字是", "名字", "姓名是", "号码是", "号码",
        "身份证号码是", "身份证是", "手机号码是",
    ] {
        v.insert((*w).to_string());
    }
    let mut out: Vec<String> = v.into_iter().collect();
    out.sort_by_key(|b| std::cmp::Reverse(b.len()));
    out
});

pub fn context_prefixes() -> &'static [String] {
    &CONTEXT_PREFIXES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_exact_match() {
        assert_eq!(canonicalize("person_name"), Some("person_name"));
        assert_eq!(canonicalize("phone_number"), Some("phone_number"));
        assert_eq!(canonicalize("government_id"), Some("government_id"));
    }

    #[test]
    fn canonicalize_case_insensitive() {
        assert_eq!(canonicalize("Person_Name"), Some("person_name"));
        assert_eq!(canonicalize("PHONE_NUMBER"), Some("phone_number"));
    }

    #[test]
    fn canonicalize_aliases() {
        assert_eq!(canonicalize("person"), Some("person_name"));
        assert_eq!(canonicalize("full name"), Some("person_name"));
        assert_eq!(canonicalize("nric"), Some("government_id"));
        assert_eq!(canonicalize("aadhaar"), Some("government_id"));
        assert_eq!(canonicalize("email"), Some("email_address"));
        assert_eq!(canonicalize("passport"), Some("passport_number"));
        assert_eq!(canonicalize("address"), Some("street_address"));
        assert_eq!(canonicalize("birthday"), Some("date_of_birth"));
    }

    #[test]
    fn canonicalize_chinese_aliases() {
        assert_eq!(canonicalize("姓名"), Some("person_name"));
        assert_eq!(canonicalize("电话"), Some("phone_number"));
        assert_eq!(canonicalize("身份证号码"), Some("government_id"));
        assert_eq!(canonicalize("地址"), Some("street_address"));
        assert_eq!(canonicalize("出生日期"), Some("date_of_birth"));
        assert_eq!(canonicalize("车牌"), Some("license_plate"));
    }

    #[test]
    fn canonicalize_unknown() {
        assert_eq!(canonicalize("random_label"), None);
        assert_eq!(canonicalize(""), None);
    }

    #[test]
    fn chinese_phone_markers() {
        assert!(chinese_phone_marker_present("电话 91234567"));
        assert!(chinese_phone_marker_present("手机号码"));
        assert!(!chinese_phone_marker_present("phone 91234567"));
    }

    #[test]
    fn meta_words_contains_labels() {
        let words = meta_words();
        assert!(words.contains("person_name"));
        assert!(words.contains("phone_number"));
        assert!(words.contains("name"));
        assert!(words.contains("phone"));
    }

    #[test]
    fn meta_words_contains_pronouns() {
        let words = meta_words();
        assert!(words.contains("i"));
        assert!(words.contains("you"));
        assert!(words.contains("he"));
        assert!(words.contains("she"));
    }

    #[test]
    fn meta_words_contains_family_terms() {
        let words = meta_words();
        assert!(words.contains("mom"));
        assert!(words.contains("dad"));
        assert!(words.contains("husband"));
        assert!(words.contains("wife"));
    }

    #[test]
    fn context_prefixes_sorted_longest_first() {
        let prefixes = context_prefixes();
        for window in prefixes.windows(2) {
            assert!(window[0].len() >= window[1].len());
        }
    }

    #[test]
    fn all_labels_have_descriptions() {
        for label in LABELS {
            let has_desc = LABEL_DESCRIPTIONS.iter().any(|(l, _)| l == label);
            assert!(has_desc, "missing description for {label}");
        }
    }
}
