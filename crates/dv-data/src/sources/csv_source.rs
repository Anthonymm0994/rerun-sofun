use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::fs::File;
use std::io::BufReader;
use async_trait::async_trait;
use arrow::datatypes::{Schema, Field, DataType};
use arrow::record_batch::RecordBatch;
use arrow::array::*;
use csv::ReaderBuilder;
use parking_lot::RwLock;
use ahash::AHashMap;
use dv_core::navigation::{NavigationSpec, NavigationPosition, NavigationRange, NavigationMode};
use crate::DataError;

/// CSV data source for loading and querying CSV files
pub struct CsvSource {
    /// Path to the CSV file
    path: PathBuf,
    /// Schema of the CSV file
    pub schema: Arc<Schema>,
    /// Row count
    pub row_count: usize,
    /// Navigation spec
    navigation_spec: NavigationSpec,
    /// Cache for loaded data (reserved for future use)
    _cache: Arc<RwLock<DataCache>>,
    /// Detected time column (reserved for future use)
    _time_column: Option<String>,
    /// Row offsets for seeking (reserved for future use)
    _row_offsets: Vec<u64>, // Byte offsets for each row
}

/// Data cache for chunk-based loading (reserved for future use)
struct DataCache {
    _chunks: AHashMap<usize, RecordBatch>,
    _max_chunks: usize,
}

impl CsvSource {
    /// Create a new CSV source from a file path
    pub async fn new(path: PathBuf) -> Result<Self, DataError> {
        // First, analyze the file
        let (schema, row_count, row_offsets) = Self::analyze_file(&path).await?;
        
        // Determine navigation spec
        let navigation_spec = Self::determine_navigation(&schema, &path).await?;
        
        Ok(Self {
            path,
            schema: Arc::new(schema),
            row_count: row_count,
            navigation_spec,
            _cache: Arc::new(RwLock::new(DataCache {
                _chunks: AHashMap::new(),
                _max_chunks: 100,
            })),
            _time_column: None,
            _row_offsets: row_offsets,
        })
    }
    
    /// Analyze the CSV file to detect schema and build index
    async fn analyze_file(path: &Path) -> Result<(Schema, usize, Vec<u64>), DataError> {
        tokio::task::spawn_blocking({
            let path = path.to_path_buf();
            move || {
                let file = File::open(&path)?;
                let mut reader = BufReader::new(file);
                let mut csv_reader = ReaderBuilder::new()
                    .has_headers(true)
                    .from_reader(&mut reader);
                
                // Get headers
                let headers = csv_reader.headers()?.clone();
                
                // Sample first 1000 rows to detect types
                let mut sample_rows = Vec::new();
                let row_offsets = vec![0u64];
                
                for result in csv_reader.records().take(1000) {
                    let record = result?;
                    sample_rows.push(record.iter().map(|s| s.to_string()).collect::<Vec<_>>());
                }
                
                // Detect column types
                let fields = headers.iter().enumerate().map(|(idx, name)| {
                    let data_type = Self::detect_column_type(&sample_rows, idx);
                    Field::new(name, data_type, true)
                }).collect::<Vec<_>>();
                
                let schema = Schema::new(fields);
                
                // Count total rows (we've already read sample_rows.len())
                let mut row_count = sample_rows.len();
                for _ in csv_reader.records() {
                    row_count += 1;
                }
                
                Ok((schema, row_count, row_offsets))
            }
        }).await.map_err(|e| DataError::SchemaDetection(e.to_string()))?
    }
    
    /// Detect column type from sample data
    fn detect_column_type(samples: &[Vec<String>], col_idx: usize) -> DataType {
        let mut is_int = true;
        let mut is_float = true;
        let mut is_timestamp = true;
        
        for row in samples {
            if let Some(value) = row.get(col_idx) {
                if value.is_empty() {
                    continue;
                }
                
                // Try parsing as integer
                if is_int && value.parse::<i64>().is_err() {
                    is_int = false;
                }
                
                // Try parsing as float
                if is_float && value.parse::<f64>().is_err() {
                    is_float = false;
                }
                
                // Try parsing as timestamp (simple check)
                if is_timestamp && !Self::looks_like_timestamp(value) {
                    is_timestamp = false;
                }
            }
        }
        
        if is_timestamp {
            DataType::Timestamp(arrow::datatypes::TimeUnit::Millisecond, None)
        } else if is_int {
            DataType::Int64
        } else if is_float {
            DataType::Float64
        } else {
            DataType::Utf8
        }
    }
    
    /// Simple heuristic to check if a string looks like a timestamp
    fn looks_like_timestamp(value: &str) -> bool {
        // Check for common timestamp patterns
        value.contains('-') || value.contains('/') || value.contains(':') ||
        value.parse::<i64>().map(|v| v > 1000000000).unwrap_or(false) // Unix timestamp
    }
    
    /// Determine navigation mode based on schema
    async fn determine_navigation(schema: &Schema, _path: &Path) -> Result<NavigationSpec, DataError> {
        // Look for timestamp columns
        let mut has_timestamp = false;
        for field in schema.fields() {
            if matches!(field.data_type(), DataType::Timestamp(_, _)) {
                has_timestamp = true;
                break;
            }
        }
        
        if has_timestamp {
            // Found timestamp column - use temporal navigation
            Ok(NavigationSpec {
                mode: NavigationMode::Temporal,
                total_rows: 0, // Will be updated later
                temporal_bounds: Some((0, 86400000)), // Default to 24 hours
                categories: None,
            })
        } else {
            // Default to sequential navigation
            Ok(NavigationSpec {
                mode: NavigationMode::Sequential,
                total_rows: 0, // Will be updated later
                temporal_bounds: None,
                categories: None,
            })
        }
    }
    
    /// Read a chunk of data from the CSV file
    async fn read_chunk(&self, start_row: usize, num_rows: usize) -> Result<RecordBatch, DataError> {
        let path = self.path.clone();
        let schema = self.schema.clone();
        
        tokio::task::spawn_blocking(move || {
            let file = File::open(&path)?;
            let reader = BufReader::new(file);
            let mut csv_reader = ReaderBuilder::new()
                .has_headers(true)
                .from_reader(reader);
            
            // Skip to start row
            for _ in 0..start_row {
                csv_reader.records().next();
            }
            
            // Read the requested rows
            let mut columns: Vec<ArrayRef> = Vec::new();
            let mut row_data: Vec<Vec<String>> = Vec::new();
            
            for (i, result) in csv_reader.records().enumerate() {
                if i >= num_rows {
                    break;
                }
                
                let record = result?;
                row_data.push(record.iter().map(|s| s.to_string()).collect());
            }
            
            // Build arrow arrays for each column
            for (col_idx, field) in schema.fields().iter().enumerate() {
                let array: ArrayRef = match field.data_type() {
                    DataType::Int64 => {
                        let mut builder = Int64Builder::new();
                        for row in &row_data {
                            if let Some(value) = row.get(col_idx) {
                                if value.is_empty() {
                                    builder.append_null();
                                } else if let Ok(v) = value.parse::<i64>() {
                                    builder.append_value(v);
                                } else {
                                    builder.append_null();
                                }
                            } else {
                                builder.append_null();
                            }
                        }
                        Arc::new(builder.finish())
                    }
                    DataType::Float64 => {
                        let mut builder = Float64Builder::new();
                        for row in &row_data {
                            if let Some(value) = row.get(col_idx) {
                                if value.is_empty() {
                                    builder.append_null();
                                } else if let Ok(v) = value.parse::<f64>() {
                                    builder.append_value(v);
                                } else {
                                    builder.append_null();
                                }
                            } else {
                                builder.append_null();
                            }
                        }
                        Arc::new(builder.finish())
                    }
                    DataType::Utf8 => {
                        let mut builder = StringBuilder::new();
                        for row in &row_data {
                            if let Some(value) = row.get(col_idx) {
                                if value.is_empty() {
                                    builder.append_null();
                                } else {
                                    builder.append_value(value);
                                }
                            } else {
                                builder.append_null();
                            }
                        }
                        Arc::new(builder.finish())
                    }
                    DataType::Timestamp(_, _) => {
                        // Simple timestamp parsing - would need proper implementation
                        let mut builder = TimestampMillisecondBuilder::new();
                        for row in &row_data {
                            if let Some(value) = row.get(col_idx) {
                                if value.is_empty() {
                                    builder.append_null();
                                } else if let Ok(v) = value.parse::<i64>() {
                                    builder.append_value(v);
                                } else {
                                    // Try parsing as date string
                                    builder.append_null(); // TODO: Proper date parsing
                                }
                            } else {
                                builder.append_null();
                            }
                        }
                        Arc::new(builder.finish())
                    }
                    _ => {
                        // Default to string
                        let mut builder = StringBuilder::new();
                        for row in &row_data {
                            if let Some(value) = row.get(col_idx) {
                                builder.append_value(value);
                            } else {
                                builder.append_null();
                            }
                        }
                        Arc::new(builder.finish())
                    }
                };
                
                columns.push(array);
            }
            
            RecordBatch::try_new(schema, columns).map_err(|e| e.into())
        }).await.map_err(|e| DataError::SchemaDetection(e.to_string()))?
    }
}

#[async_trait]
impl dv_core::data::DataSource for CsvSource {
    async fn schema(&self) -> Arc<Schema> {
        self.schema.clone()
    }
    
    async fn navigation_spec(&self) -> anyhow::Result<NavigationSpec> {
        let mut spec = self.navigation_spec.clone();
        
        // Update with actual row count
        spec.total_rows = self.row_count;
        
        Ok(spec)
    }
    
    async fn query_at(&self, position: &NavigationPosition) -> anyhow::Result<RecordBatch> {
        let row_idx = match position {
            NavigationPosition::Sequential(idx) => *idx,
            NavigationPosition::Temporal(_) => {
                // TODO: Implement time-based indexing
                0
            }
            NavigationPosition::Categorical(_) => {
                // CSV doesn't support categorical navigation
                return Err(DataError::InvalidPosition.into());
            }
        };
        
        // Query a window around the position
        let window_size = 1000;
        let start = row_idx.saturating_sub(window_size / 2);
        let end = (row_idx + window_size / 2).min(self.row_count);
        
        self.read_chunk(start, end - start).await.map_err(|e| e.into())
    }
    
    async fn query_range(&self, range: &NavigationRange) -> anyhow::Result<RecordBatch> {
        let (start, end) = match (&range.start, &range.end) {
            (NavigationPosition::Sequential(s), NavigationPosition::Sequential(e)) => (*s, *e),
            _ => return Err(DataError::InvalidPosition.into()),
        };
        
        self.read_chunk(start, end - start).await.map_err(|e| e.into())
    }
    
    async fn row_count(&self) -> anyhow::Result<usize> {
        Ok(self.row_count)
    }
    
    fn source_name(&self) -> &str {
        self.path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.csv")
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    // ... existing code ...
} 