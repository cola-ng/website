use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionReport {
    pub text: String,
    pub replacements: usize,
}

/// Removes common secrets before an observation reaches storage or a remote provider.
#[must_use]
pub fn redact_sensitive_text(input: &str) -> RedactionReport {
    let patterns = [
        (r"(?i)\b[\w.+-]+@[\w.-]+\.[a-z]{2,}\b", "[email]"),
        (r"\b(?:\d[ -]*?){13,19}\b", "[number]"),
        (
            r"(?i)\b(?:api[_-]?key|token|password)\s*[:=]\s*\S+",
            "[secret]",
        ),
    ];
    let mut text = input.to_owned();
    let mut replacements = 0;
    for (pattern, replacement) in patterns {
        let regex = Regex::new(pattern).expect("static redaction regex must compile");
        replacements += regex.find_iter(&text).count();
        text = regex.replace_all(&text, replacement).into_owned();
    }
    RedactionReport { text, replacements }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_credentials_and_contact_data() {
        let report = redact_sensitive_text(
            "mail me at dev@example.com; api_key=abc123; card 4111 1111 1111 1111",
        );
        assert_eq!(report.replacements, 3);
        assert!(!report.text.contains("abc123"));
        assert!(!report.text.contains("dev@example.com"));
        assert!(!report.text.contains("4111"));
    }
}
