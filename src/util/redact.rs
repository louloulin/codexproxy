//! Sensitive Data Redaction Utilities
//! 
//! This module provides functions for redacting sensitive information
//! from strings, such as API keys, tokens, and passwords.

/// Redact sensitive information from a string
pub fn redact_sensitive(input: &str) -> String {
    let mut result = input.to_string();
    
    // Redact sk-* API keys (but do sk-ant-* first to catch anthropic keys)
    // Pattern: sk-ant-* (Anthropic keys)
    if let Ok(re) = regex::Regex::new("sk-ant-[a-zA-Z0-9_-]{10,}") {
        result = re.replace_all(&result, "sk-ant-<redacted>").to_string();
    }
    
    // Pattern: sk-* (generic API keys)
    if let Ok(re) = regex::Regex::new("sk-[a-zA-Z0-9_-]{10,}") {
        result = re.replace_all(&result, "sk-<redacted>").to_string();
    }
    
    // Redact Bearer tokens
    if let Ok(re) = regex::Regex::new(r#"(Bearer\s+)([^\s"'`,\x00-\x1F]+)"#) {
        result = re.replace_all(&result, "$1{redacted}").to_string();
        result = result.replace("{redacted}", "<redacted>");
    }
    
    // Try to parse as JSON and redact known sensitive fields
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&result) {
        if let Some(redacted) = redact_json_values(&json) {
            if let Ok(json_str) = serde_json::to_string(&redacted) {
                result = json_str;
            }
        }
    }
    
    result
}

/// Recursively redact sensitive values in JSON
fn redact_json_values(json: &serde_json::Value) -> Option<serde_json::Value> {
    match json {
        serde_json::Value::Object(map) => {
            let sensitive_fields = [
                "api_key", "apiKey", "apikey",
                "authorization", "Authorization",
                "access_token", "accessToken", 
                "password", "Password",
                "secret", "Secret",
                "token", "Token",
            ];
            
            let mut new_map = serde_json::Map::new();
            for (key, value) in map {
                if sensitive_fields.iter().any(|f| key.eq_ignore_ascii_case(f)) {
                    new_map.insert(key.clone(), serde_json::Value::String("<redacted>".to_string()));
                } else if let Some(redacted) = redact_json_values(value) {
                    new_map.insert(key.clone(), redacted);
                } else {
                    new_map.insert(key.clone(), value.clone());
                }
            }
            Some(serde_json::Value::Object(new_map))
        }
        serde_json::Value::Array(arr) => {
            let redacted: Vec<_> = arr.iter().filter_map(redact_json_values).collect();
            if redacted.len() == arr.len() {
                Some(serde_json::Value::Array(redacted))
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // mimo2codex aligned tests from redact.test.ts

    #[test]
    fn test_scrubs_bearer_tokens() {
        let input = r#"{"headers":"Authorization: Bearer sk-ABCDEFG123456789xyz"}"#;
        let result = redact_sensitive(input);
        assert!(!result.contains("sk-ABCDEFG123456789xyz"));
        assert!(result.contains("Bearer <redacted>"));
    }

    #[test]
    fn test_scrubs_sk_style_keys_embedded_in_text() {
        let input = "hi sk-abcdefghij1234567890 there";
        let result = redact_sensitive(input);
        assert_eq!(result, "hi sk-<redacted> there");
    }

    #[test]
    fn test_scrubs_sk_ant_anthropic_keys_before_generic() {
        let input = "key=sk-ant-abc1234567890XYZ end";
        let result = redact_sensitive(input);
        assert!(result.contains("sk-ant-<redacted>"));
        assert!(!result.contains("abc1234567890XYZ"));
    }

    #[test]
    fn test_handles_authorization_access_token_password_fields() {
        let input = r#"{"authorization":"foo","access_token":"bar","password":"pw"}"#;
        let result = redact_sensitive(input);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["authorization"], "<redacted>");
        assert_eq!(parsed["access_token"], "<redacted>");
        assert_eq!(parsed["password"], "<redacted>");
    }

    #[test]
    fn test_leaves_non_sensitive_content_untouched() {
        let input = r#"{"messages":[{"role":"user","content":"hello world"}]}"#;
        let result = redact_sensitive(input);
        // Parse both and compare JSON values (order may differ)
        let input_json: serde_json::Value = serde_json::from_str(input).unwrap();
        let result_json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(input_json, result_json);
    }

    #[test]
    fn test_returns_empty_input_as_is() {
        let result = redact_sensitive("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_redacts_multiple_sk_keys() {
        let input = "sk-key1-12345678 and sk-key2-87654321";
        let result = redact_sensitive(input);
        assert!(result.contains("sk-<redacted>"));
        assert!(!result.contains("key1"));
        assert!(!result.contains("key2"));
    }
}
