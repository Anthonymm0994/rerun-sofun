use arrow::datatypes::{Schema, Field, DataType};
use std::collections::HashMap;

/// Schema detector for analyzing data and determining column types
pub struct SchemaDetector {
    sample_size: usize,
}

/// Information about a detected schema
#[derive(Debug, Clone)]
pub struct SchemaInfo {
    pub schema: Schema,
    pub column_stats: HashMap<String, ColumnStats>,
    pub suggested_navigation_column: Option<String>,
}

/// Statistics about a column
#[derive(Debug, Clone)]
pub struct ColumnStats {
    pub null_count: usize,
    pub distinct_count: usize,
    pub is_sorted: bool,
    pub is_unique: bool,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
}

impl SchemaDetector {
    /// Create a new schema detector
    pub fn new() -> Self {
        Self {
            sample_size: 1000,
        }
    }
    
    /// Set the sample size for detection
    pub fn with_sample_size(mut self, size: usize) -> Self {
        self.sample_size = size;
        self
    }
    
    /// Detect schema from sample data
    pub fn detect_from_samples(&self, headers: &[String], samples: &[Vec<String>]) -> SchemaInfo {
        let mut fields = Vec::new();
        let mut column_stats = HashMap::new();
        let mut suggested_navigation_column = None;
        
        for (col_idx, header) in headers.iter().enumerate() {
            let (data_type, stats) = self.analyze_column(samples, col_idx);
            
            // Check if this could be a good navigation column
            if suggested_navigation_column.is_none() {
                if Self::is_good_navigation_column(&data_type, &stats) {
                    suggested_navigation_column = Some(header.clone());
                }
            }
            
            fields.push(Field::new(header, data_type, stats.null_count > 0));
            column_stats.insert(header.clone(), stats);
        }
        
        SchemaInfo {
            schema: Schema::new(fields),
            column_stats,
            suggested_navigation_column,
        }
    }
    
    /// Analyze a single column
    fn analyze_column(&self, samples: &[Vec<String>], col_idx: usize) -> (DataType, ColumnStats) {
        let mut null_count = 0;
        let mut values = Vec::new();
        let mut is_int = true;
        let mut is_float = true;
        let mut is_timestamp = true;
        let mut is_bool = true;
        
        // Collect non-null values and check types
        for row in samples {
            if let Some(value) = row.get(col_idx) {
                if value.is_empty() {
                    null_count += 1;
                } else {
                    values.push(value.clone());
                    
                    // Type checks
                    if is_int && value.parse::<i64>().is_err() {
                        is_int = false;
                    }
                    if is_float && value.parse::<f64>().is_err() {
                        is_float = false;
                    }
                    if is_timestamp && !Self::looks_like_timestamp(value) {
                        is_timestamp = false;
                    }
                    if is_bool && !matches!(value.to_lowercase().as_str(), "true" | "false" | "0" | "1") {
                        is_bool = false;
                    }
                }
            } else {
                null_count += 1;
            }
        }
        
        // Determine data type
        let data_type = if is_bool {
            DataType::Boolean
        } else if is_timestamp {
            DataType::Timestamp(arrow::datatypes::TimeUnit::Millisecond, None)
        } else if is_int {
            DataType::Int64
        } else if is_float {
            DataType::Float64
        } else {
            DataType::Utf8
        };
        
        // Calculate statistics
        let distinct_count = {
            let mut unique = std::collections::HashSet::new();
            for v in &values {
                unique.insert(v.clone());
            }
            unique.len()
        };
        
        let is_unique = distinct_count == values.len();
        let is_sorted = Self::check_sorted(&values, &data_type);
        
        let (min_value, max_value) = if !values.is_empty() {
            let min = values.iter().min().cloned();
            let max = values.iter().max().cloned();
            (min, max)
        } else {
            (None, None)
        };
        
        let stats = ColumnStats {
            null_count,
            distinct_count,
            is_sorted,
            is_unique,
            min_value,
            max_value,
        };
        
        (data_type, stats)
    }
    
    /// Check if values look like timestamps
    fn looks_like_timestamp(value: &str) -> bool {
        // Common timestamp patterns
        value.contains('-') || value.contains('/') || value.contains(':') ||
        value.parse::<i64>().map(|v| v > 1000000000 && v < 2000000000).unwrap_or(false)
    }
    
    /// Check if values are sorted
    fn check_sorted(values: &[String], data_type: &DataType) -> bool {
        if values.len() < 2 {
            return true;
        }
        
        match data_type {
            DataType::Int64 => {
                let parsed: Vec<_> = values.iter()
                    .filter_map(|v| v.parse::<i64>().ok())
                    .collect();
                parsed.windows(2).all(|w| w[0] <= w[1])
            }
            DataType::Float64 => {
                let parsed: Vec<_> = values.iter()
                    .filter_map(|v| v.parse::<f64>().ok())
                    .collect();
                parsed.windows(2).all(|w| w[0] <= w[1])
            }
            _ => false,
        }
    }
    
    /// Check if this column would make a good navigation column
    fn is_good_navigation_column(data_type: &DataType, stats: &ColumnStats) -> bool {
        match data_type {
            DataType::Timestamp(_, _) => true,
            DataType::Int64 | DataType::Float64 => stats.is_sorted && stats.is_unique,
            _ => false,
        }
    }
}

impl Default for SchemaDetector {
    fn default() -> Self {
        Self::new()
    }
} 