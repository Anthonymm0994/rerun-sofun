//! Null value handling for data loading

use serde::{Serialize, Deserialize};

/// Null value configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NullConfig {
    /// Patterns to treat as null
    pub patterns: Vec<String>,
    
    /// Whether to trim whitespace before checking
    pub trim_whitespace: bool,
    
    /// Case sensitive matching
    pub case_sensitive: bool,
}

impl Default for NullConfig {
    fn default() -> Self {
        Self {
            patterns: vec![
                String::new(),      // Empty string
                "-".to_string(),
                "N/A".to_string(),
                "n/a".to_string(),
                "null".to_string(),
                "NULL".to_string(),
                "None".to_string(),
                "none".to_string(),
            ],
            trim_whitespace: true,
            case_sensitive: false,
        }
    }
}

impl NullConfig {
    /// Check if a value should be treated as null
    pub fn is_null(&self, value: &str) -> bool {
        let test_value = if self.trim_whitespace {
            value.trim()
        } else {
            value
        };
        
        self.patterns.iter().any(|pattern| {
            if self.case_sensitive {
                test_value == pattern
            } else {
                test_value.eq_ignore_ascii_case(pattern)
            }
        })
    }
    
    /// Add a null pattern
    pub fn add_pattern(&mut self, pattern: String) {
        if !self.patterns.contains(&pattern) {
            self.patterns.push(pattern);
        }
    }
    
    /// Remove a null pattern
    pub fn remove_pattern(&mut self, pattern: &str) {
        self.patterns.retain(|p| p != pattern);
    }
    
    /// Clear all patterns
    pub fn clear_patterns(&mut self) {
        self.patterns.clear();
    }
} 