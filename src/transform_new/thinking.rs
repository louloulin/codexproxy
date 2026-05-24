//! Inline Think Tag Extraction

/// State machine for tracking think tag boundaries
#[derive(Debug, Clone)]
pub struct ThinkSplitter {
    buffer: String,
    in_think: bool,
    reasoning_buffer: String,
}

impl ThinkSplitter {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            in_think: false,
            reasoning_buffer: String::new(),
        }
    }
    
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.in_think = false;
        self.reasoning_buffer.clear();
    }
    
    pub fn process_chunk(&mut self, chunk: &str) -> (String, Option<String>, bool) {
        let mut combined = std::mem::take(&mut self.buffer);
        combined.push_str(chunk);
        
        let mut result_content = String::new();
        let mut reasoning_parts: Vec<String> = Vec::new();
        
        let mut remaining = combined.as_str();
        
        while let Some(start_pos) = remaining.find("<think>") {
            result_content.push_str(&remaining[..start_pos]);
            
            let after_open = &remaining[start_pos + 4..];
            if let Some(end_pos) = after_open.find("</think>") {
                let think_content = &after_open[..end_pos].trim();
                if !think_content.is_empty() {
                    reasoning_parts.push(think_content.to_string());
                }
                // Skip past the closing tag (6 characters: <think> + )
                remaining = &after_open[end_pos + 6..];
                // Complete the thinking block
                self.in_think = false;
            } else {
                // No closing tag found, buffer everything including the opening tag
                self.buffer = remaining[start_pos..].to_string();
                self.in_think = true;
                break;
            }
        }
        
        if !self.in_think {
            // Only append remaining content if we're not in a thinking block
            result_content.push_str(remaining);
            self.buffer.clear();
        }
        
        let reasoning = if reasoning_parts.is_empty() {
            None
        } else {
            self.reasoning_buffer.push_str(&reasoning_parts.join("\n"));
            self.reasoning_buffer.push('\n');
            Some(self.reasoning_buffer.trim().to_string())
        };
        
        let is_complete = !self.in_think && self.buffer.is_empty();
        
        (result_content, reasoning, is_complete)
    }
    
    pub fn finalize(&mut self) -> Option<String> {
        let result = if self.reasoning_buffer.is_empty() {
            None
        } else {
            Some(self.reasoning_buffer.trim().to_string())
        };
        self.reset();
        result
    }
}

impl Default for ThinkSplitter {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple function to extract think tags from content
pub fn extract_inline_think(content: &str) -> (String, Option<String>) {
    let mut result = String::new();
    let mut reasoning_parts: Vec<String> = Vec::new();
    
    let mut remaining = content;
    
    while let Some(start_pos) = remaining.find("<think>") {
        result.push_str(&remaining[..start_pos]);
        
        let after_open = &remaining[start_pos + 4..];
        if let Some(end_pos) = after_open.find("</think>") {
            let think_content = &after_open[..end_pos].trim();
            if !think_content.is_empty() {
                reasoning_parts.push(think_content.to_string());
            }
            remaining = &after_open[end_pos + 6..];
        } else {
            break;
        }
    }
    
    result.push_str(remaining);
    
    let reasoning = if reasoning_parts.is_empty() {
        None
    } else {
        Some(reasoning_parts.join("\n"))
    };
    
    (result, reasoning)
}

/// Check if content contains complete think tags
pub fn contains_think_tags(content: &str) -> bool {
    content.contains("<think>") && content.contains("</think>")
}

/// Count complete think tag pairs
pub fn count_think_tags(content: &str) -> usize {
    content.matches("<think>").count()
}

/// Extract think content only
pub fn extract_think_content(content: &str) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut remaining = content;
    
    while let Some(start_pos) = remaining.find("<think>") {
        let after_open = &remaining[start_pos + 4..];
        if let Some(end_pos) = after_open.find("</think>") {
            let think_content = &after_open[..end_pos].trim();
            if !think_content.is_empty() {
                parts.push(think_content.to_string());
            }
            remaining = &after_open[end_pos + 6..];
        } else {
            break;
        }
    }
    
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_think_tags() {
        let content = "Normal text without think tags.";
        let (result, reasoning) = extract_inline_think(content);
        assert_eq!(result, content);
        assert!(reasoning.is_none());
    }

    #[test]
    fn test_unclosed_think_tag() {
        let content = "<think>This is unclosed";
        let (result, reasoning) = extract_inline_think(content);
        assert!(result.contains("<think>"));
        assert!(reasoning.is_none());
    }

    #[test]
    fn test_think_splitter_partial() {
        let mut splitter = ThinkSplitter::new();
        
        // First chunk - opening tag only (no closing tag)
        let (result1, _, is_complete1) = splitter.process_chunk("<think>unclosed");
        assert!(result1.is_empty());
        assert!(!is_complete1);
        
        // Second chunk - closing tag completes the thinking block
        // The combined content should be "<think>unclosed</think> normal text"
        let (result2, reasoning, is_complete2) = splitter.process_chunk("</think> normal text");
        assert!(result2.contains("normal"));
        assert!(result2.contains("text"));
        assert!(is_complete2);
        assert!(reasoning.is_some());
        let reasoning_str = reasoning.unwrap();
        assert!(reasoning_str.contains("unclosed"));
    }

    #[test]
    fn test_complete_think_in_single_chunk() {
        let mut splitter = ThinkSplitter::new();
        
        let (result, reasoning, is_complete) = splitter.process_chunk("<think>thinking content</think> and more text");
        assert!(result.contains("and more text"));
        assert!(!result.contains("<think>"));
        assert!(!result.contains("</think>"));
        assert!(reasoning.is_some());
        assert!(reasoning.unwrap().contains("thinking content"));
        assert!(is_complete);
    }
}
