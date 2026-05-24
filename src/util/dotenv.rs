use std::collections::HashMap;

/// Result of parsing dotenv content
#[derive(Debug, Clone, PartialEq)]
pub struct DotenvParseResult {
    /// Successfully parsed entries
    pub entries: Vec<DotenvEntry>,
    /// Lines that were skipped (invalid syntax)
    pub skipped: Vec<SkippedLine>,
}

/// A parsed dotenv entry
#[derive(Debug, Clone, PartialEq)]
pub struct DotenvEntry {
    pub key: String,
    pub value: String,
}

/// A line that was skipped with reason
#[derive(Debug, Clone, PartialEq)]
pub struct SkippedLine {
    pub line: String,
    pub reason: String,
}

/// Parses dotenv file content
/// 
/// # Algorithm (mirrors mimo2codex)
///
/// 1. Normalize line endings (CRLF → LF)
/// 2. Split into lines
/// 3. For each line:
///    - Strip leading whitespace
///    - Skip empty lines
///    - Skip comment lines (#)
///    - Remove `export ` prefix if present
///    - Split on first `=`
///    - Validate key (alphanumeric + underscore, must start with letter)
///    - Strip paired quotes from value
///    - Add to entries or skipped list
pub fn parse_dotenv(content: &str) -> DotenvParseResult {
    let mut entries = Vec::new();
    let mut skipped = Vec::new();
    
    // Normalize line endings
    let normalized = content.replace("\r\n", "\n");
    
    for line in normalized.lines() {
        let trimmed = line.trim();
        
        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }
        
        // Skip comment lines
        if trimmed.starts_with('#') {
            continue;
        }
        
        // Remove export prefix
        let no_export = if trimmed.starts_with("export ") {
            &trimmed[7..]
        } else {
            trimmed
        };
        
        // Find first '='
        if let Some(eq_pos) = no_export.find('=') {
            let key = no_export[..eq_pos].trim();
            let mut value = no_export[eq_pos + 1..].to_string();
            
            // Validate key
            if !is_valid_key(key) {
                skipped.push(SkippedLine {
                    line: trimmed.to_string(),
                    reason: "invalid key".to_string(),
                });
                continue;
            }
            
            // Strip paired quotes
            value = strip_quotes(&value);
            
            entries.push(DotenvEntry {
                key: key.to_string(),
                value,
            });
        } else {
            // No '=' found - skip this line
            skipped.push(SkippedLine {
                line: trimmed.to_string(),
                reason: "no '=' found".to_string(),
            });
        }
    }
    
    DotenvParseResult { entries, skipped }
}

/// Loads dotenv file into the provided env map
/// 
/// Returns list of keys that were loaded
pub fn load_dotenv_file(path: &str, env: &mut HashMap<String, String>) -> Vec<String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    
    let result = parse_dotenv(&content);
    let mut loaded = Vec::new();
    
    for entry in result.entries {
        env.insert(entry.key.clone(), entry.value);
        loaded.push(entry.key);
    }
    
    loaded
}

/// Validates a dotenv key
/// 
/// Valid keys:
/// - Start with a letter or underscore
/// - Contain only letters, digits, and underscores
fn is_valid_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }
    
    let mut chars = key.chars();
    let first = chars.next().unwrap();
    
    // First char must be letter or underscore
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    
    // Rest must be alphanumeric or underscore
    for c in chars {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return false;
        }
    }
    
    true
}

/// Strips paired surrounding quotes (double or single)
/// Only strips if both opening and closing quotes are present
fn strip_quotes(value: &str) -> String {
    let trimmed = value.trim();
    
    if trimmed.len() >= 2 {
        // Check for paired double quotes
        if trimmed.starts_with('"') && trimmed.ends_with('"') {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
        // Check for paired single quotes
        if trimmed.starts_with('\'') && trimmed.ends_with('\'') {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }
    
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parses_plain_key_value_lines() {
        let result = parse_dotenv("FOO=bar\nBAZ=qux");
        
        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.entries[0].key, "FOO");
        assert_eq!(result.entries[0].value, "bar");
        assert_eq!(result.entries[1].key, "BAZ");
        assert_eq!(result.entries[1].value, "qux");
        assert!(result.skipped.is_empty());
    }

    #[test]
    fn test_strips_paired_surrounding_quotes() {
        let result = parse_dotenv("A=\"hello\"\nB='world'\nC=\"mixed'end\nD=plain");
        
        assert_eq!(result.entries.len(), 4);
        assert_eq!(result.entries[0].key, "A");
        assert_eq!(result.entries[0].value, "hello");  // double quotes stripped
        assert_eq!(result.entries[1].key, "B");
        assert_eq!(result.entries[1].value, "world"); // single quotes stripped
        assert_eq!(result.entries[2].key, "C");
        // unpaired - kept with literal quote
        assert_eq!(result.entries[2].value, "\"mixed'end");
        assert_eq!(result.entries[3].key, "D");
        assert_eq!(result.entries[3].value, "plain");
    }

    #[test]
    fn test_does_not_expand_variables() {
        let content = concat!(
            "PATH_LIKE=$HOME/bin\n",
            r#"NEWLINE="a\\nb""#
        );
        let result = parse_dotenv(content);
        
        assert_eq!(result.entries[0].value, "$HOME/bin");
        assert_eq!(result.entries[1].value, r#"a\\nb"#);
    }

    #[test]
    fn test_skips_comments_and_blank_lines() {
        let result = parse_dotenv("# top comment\n\nFOO=1\n   # indented comment\n\nBAR=2\n");
        
        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.entries[0].key, "FOO");
        assert_eq!(result.entries[1].key, "BAR");
    }

    #[test]
    fn test_tolerates_export_prefix() {
        let result = parse_dotenv("export MIMO_API_KEY=sk-xxx\nexport  TWO=two");
        
        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.entries[0].key, "MIMO_API_KEY");
        assert_eq!(result.entries[0].value, "sk-xxx");
        assert_eq!(result.entries[1].key, "TWO");
        assert_eq!(result.entries[1].value, "two");
    }

    #[test]
    fn test_tolerates_windows_crlf() {
        let result = parse_dotenv("FOO=bar\r\nBAZ=qux\r\n");
        
        let keys: Vec<_> = result.entries.iter().map(|e| e.key.clone()).collect();
        assert_eq!(keys, vec!["FOO", "BAZ"]);
        
        let values: Vec<_> = result.entries.iter().map(|e| e.value.clone()).collect();
        assert_eq!(values, vec!["bar", "qux"]);
    }

    #[test]
    fn test_rejects_invalid_key_names() {
        let result = parse_dotenv("1FOO=bad\nBAR-BAZ=bad\nGOOD_1=ok");
        
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].key, "GOOD_1");
        assert_eq!(result.skipped.len(), 2);
        assert!(result.skipped[0].reason.contains("invalid key"));
        assert!(result.skipped[1].reason.contains("invalid key"));
    }

    #[test]
    fn test_skips_lines_without_equals() {
        let result = parse_dotenv("FOO=1\nnonsenseline\nBAR=2");
        
        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.skipped.len(), 1);
    }

    #[test]
    fn test_allows_equals_in_value() {
        let result = parse_dotenv("KEY=a=b=c");
        
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].value, "a=b=c");
    }

    #[test]
    fn test_strips_leading_whitespace() {
        let result = parse_dotenv("   FOO=1\n\tBAR=2");
        
        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.entries[0].key, "FOO");
        assert_eq!(result.entries[1].key, "BAR");
    }

    #[test]
    fn test_load_dotenv_file_overwrites() {
        use std::io::Write;
        use tempfile::NamedTempFile;
        
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "FOO=newvalue").unwrap();
        writeln!(file, "NEW=appears").unwrap();
        let path = file.path().to_str().unwrap();
        
        let mut env: HashMap<String, String> = [
            ("FOO".to_string(), "old".to_string()),
            ("UNRELATED".to_string(), "keep".to_string()),
        ].into_iter().collect();
        
        let loaded = load_dotenv_file(path, &mut env);
        
        assert!(loaded.contains(&"FOO".to_string()));
        assert!(loaded.contains(&"NEW".to_string()));
        assert_eq!(env.get("FOO").unwrap(), "newvalue");
        assert_eq!(env.get("NEW").unwrap(), "appears");
        assert_eq!(env.get("UNRELATED").unwrap(), "keep");
    }
}