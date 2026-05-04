//! Language detection — triggers the Chinese NER model for CJK text.

/// Returns true if `text` contains any CJK Unified Ideograph in BMP or
/// supplementary planes — the trigger to also run the Chinese NER pass.
pub fn has_chinese(text: &str) -> bool {
    text.chars().any(is_cjk)
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3400..=0x4DBF |   // CJK Ext A
        0x4E00..=0x9FFF |   // CJK Unified
        0x20000..=0x2A6DF | // Ext B
        0x2A700..=0x2EBEF | // Ext C-F
        0x30000..=0x3134F   // Ext G
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_only() {
        assert!(!has_chinese("Hello, my name is John."));
    }

    #[test]
    fn mixed() {
        assert!(has_chinese("我的电话是 9123 4567"));
    }

    #[test]
    fn empty() {
        assert!(!has_chinese(""));
    }
}
