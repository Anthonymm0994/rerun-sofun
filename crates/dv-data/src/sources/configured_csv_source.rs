//! CSV data source with configuration support

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
use tracing::info;

use dv_core::navigation::{NavigationSpec, NavigationPosition, NavigationRange, NavigationMode};

use crate::{DataError, config::FileConfig};

/// Performance tuning constants (same as csv_source.rs)
const MAX_SAMPLE_ROWS: usize = 5000;
const CHUNK_SIZE: usize = 10000;
const MAX_CACHED_CHUNKS: usize = 50;

/// CSV data source with configuration support
pub struct ConfiguredCsvSource {
    /// File configuration
    config: FileConfig,
    
    /// Schema of the CSV file
    pub schema: Arc<Schema>,
    
    /// Row count
    pub row_count: usize,
    
    /// Navigation spec
    navigation_spec: NavigationSpec,
    
    /// Cache for loaded data
    cache: Arc<RwLock<DataCache>>,
    
    /// Source name
    source_name: String,
}

/// Data cache for chunk-based loading
struct DataCache {
    chunks: AHashMap<usize, RecordBatch>,
    max_chunks: usize,
    /// LRU tracking for cache eviction
    access_order: Vec<usize>,
}

impl ConfiguredCsvSource {
    /// Create a new configured CSV source
    pub async fn new(config: FileConfig) -> Result<Self, DataError> {
        info!("Creating ConfiguredCsvSource for {:?}", config.path);
        
        let source_name = config.file_name();
        
        // Validate configuration
        if config.selected_columns.is_empty() {
            return Err(DataError::Other("No columns selected".to_string()));
        }
        
        // Analyze the file with configuration
        let (schema, row_count) = Self::analyze_file(&config).await?;
        
        // Determine navigation spec
        let navigation_spec = Self::determine_navigation(&schema, &config).await?;
        
        Ok(Self {
            config,
            source_name,
            schema: Arc::new(schema),
            row_count,
            navigation_spec,
            cache: Arc::new(RwLock::new(DataCache {
                chunks: AHashMap::new(),
                max_chunks: MAX_CACHED_CHUNKS,
                access_order: Vec::new(),
            })),
        })
    }
    
    /// Analyze the CSV file with configuration
    async fn analyze_file(config: &FileConfig) -> Result<(Schema, usize), DataError> {
        tokio::task::spawn_blocking({
            let config = config.clone();
            move || {
                let file = File::open(&config.path)?;
                let mut reader = BufReader::new(file);
                
                // Skip to header line
                let mut csv_reader = ReaderBuilder::new()
                    .has_headers(false)
                    .from_reader(&mut reader);
                
                // Skip lines before header
                for _ in 0..config.header_line {
                    let mut record = csv::StringRecord::new();
                    csv_reader.read_record(&mut record)?;
                }
                
                // Read header
                let mut header_record = csv::StringRecord::new();
                csv_reader.read_record(&mut header_record)?;
                let headers: Vec<String> = header_record.iter()
                    .map(|s| s.to_string())
                    .collect();
                
                // Sample rows for type detection
                let mut sample_rows = Vec::new();
                for _ in 0..config.sample_size.min(MAX_SAMPLE_ROWS) {
                    let mut record = csv::StringRecord::new();
                    if csv_reader.read_record(&mut record)? {
                        sample_rows.push(record.iter().map(|s| s.to_string()).collect::<Vec<_>>());
                    } else {
                        break;
                    }
                }
                
                // Build schema based on selected columns
                let mut fields = Vec::new();
                for (idx, header) in headers.iter().enumerate() {
                    if config.selected_columns.contains(header) {
                        let data_type = config.column_types.get(header)
                            .cloned()
                            .map(|sdt| sdt.into())
                            .unwrap_or_else(|| Self::detect_column_type(&sample_rows, idx, &config));
                        fields.push(Field::new(header, data_type, true));
                    }
                }
                
                let schema = Schema::new(fields);
                
                // Count total rows
                let mut row_count = sample_rows.len();
                for _ in csv_reader.records() {
                    row_count += 1;
                }
                
                Ok((schema, row_count))
            }
        }).await.map_err(|e| DataError::SchemaDetection(e.to_string()))?
    }
    
    /// Detect column type from sample data
    fn detect_column_type(samples: &[Vec<String>], col_idx: usize, config: &FileConfig) -> DataType {
        let mut is_int = true;
        let mut is_float = true;
        let mut is_bool = true;
        let mut is_timestamp = true;
        
        for row in samples {
            if let Some(value) = row.get(col_idx) {
                if value.is_empty() || config.is_null_value(value) {
                    continue;
                }
                
                // Try parsing as boolean
                if is_bool {
                    let lower = value.to_lowercase();
                    if lower != "true" && lower != "false" && lower != "1" && lower != "0" {
                        is_bool = false;
                    }
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
        
        if is_bool {
            DataType::Boolean
        } else if is_timestamp {
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
        value.contains('-') || value.contains('/') || value.contains(':') ||
        value.parse::<i64>().map(|v| v > 1000000000).unwrap_or(false)
    }
    
    /// Determine navigation mode based on schema
    async fn determine_navigation(schema: &Schema, _config: &FileConfig) -> Result<NavigationSpec, DataError> {
        let mut has_timestamp = false;
        for field in schema.fields() {
            if matches!(field.data_type(), DataType::Timestamp(_, _)) {
                has_timestamp = true;
                break;
            }
        }
        
        if has_timestamp {
            Ok(NavigationSpec {
                mode: NavigationMode::Temporal,
                total_rows: 0,
                temporal_bounds: Some((0, 86400000)),
                categories: None,
            })
        } else {
            Ok(NavigationSpec {
                mode: NavigationMode::Sequential,
                total_rows: 0,
                temporal_bounds: None,
                categories: None,
            })
        }
    }
    
    /// Read a chunk of data from the CSV file
    async fn read_chunk(&self, start_row: usize, num_rows: usize) -> Result<RecordBatch, DataError> {
        let config = self.config.clone();
        let schema = self.schema.clone();
        
        tokio::task::spawn_blocking(move || {
            let file = File::open(&config.path)?;
            let reader = BufReader::new(file);
            let mut csv_reader = ReaderBuilder::new()
                .has_headers(false)
                .from_reader(reader);
            
            // Skip to start row (including header)
            for _ in 0..(config.header_line + 1 + start_row) {
                let mut record = csv::StringRecord::new();
                csv_reader.read_record(&mut record)?;
            }
            
            // Read the requested rows
            let mut row_data: Vec<Vec<Option<String>>> = Vec::new();
            
            for _ in 0..num_rows {
                let mut record = csv::StringRecord::new();
                if csv_reader.read_record(&mut record)? {
                    let row: Vec<Option<String>> = record.iter()
                        .map(|s| {
                            if config.is_null_value(s) {
                                None
                            } else {
                                Some(s.to_string())
                            }
                        })
                        .collect();
                    row_data.push(row);
                } else {
                    break;
                }
            }
            
            // Get column indices for selected columns
            let header_record = {
                let file = File::open(&config.path)?;
                let mut reader = BufReader::new(file);
                let mut csv_reader = ReaderBuilder::new()
                    .has_headers(false)
                    .from_reader(&mut reader);
                
                // Skip to header
                for _ in 0..config.header_line {
                    let mut record = csv::StringRecord::new();
                    csv_reader.read_record(&mut record)?;
                }
                
                let mut header = csv::StringRecord::new();
                csv_reader.read_record(&mut header)?;
                header.iter().map(|s| s.to_string()).collect::<Vec<_>>()
            };
            
            // Build arrow arrays for each selected column
            let mut columns: Vec<ArrayRef> = Vec::new();
            
            for field in schema.fields() {
                if let Some(col_idx) = header_record.iter().position(|h| h == field.name()) {
                    let array: ArrayRef = match field.data_type() {
                        DataType::Boolean => {
                            let mut builder = BooleanBuilder::new();
                            for row in &row_data {
                                if let Some(Some(value)) = row.get(col_idx) {
                                    let lower = value.to_lowercase();
                                    if lower == "true" || lower == "1" {
                                        builder.append_value(true);
                                    } else if lower == "false" || lower == "0" {
                                        builder.append_value(false);
                                    } else {
                                        builder.append_null();
                                    }
                                } else {
                                    builder.append_null();
                                }
                            }
                            Arc::new(builder.finish())
                        }
                        DataType::Int64 => {
                            let mut builder = Int64Builder::new();
                            for row in &row_data {
                                if let Some(Some(value)) = row.get(col_idx) {
                                    if let Ok(v) = value.parse::<i64>() {
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
                                if let Some(Some(value)) = row.get(col_idx) {
                                    if let Ok(v) = value.parse::<f64>() {
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
                                if let Some(Some(value)) = row.get(col_idx) {
                                    builder.append_value(value);
                                } else {
                                    builder.append_null();
                                }
                            }
                            Arc::new(builder.finish())
                        }
                        DataType::Timestamp(_, _) => {
                            let mut builder = TimestampMillisecondBuilder::new();
                            for row in &row_data {
                                if let Some(Some(value)) = row.get(col_idx) {
                                    if let Ok(v) = value.parse::<i64>() {
                                        builder.append_value(v);
                                    } else {
                                        // TODO: Proper date parsing
                                        builder.append_null();
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
                                if let Some(Some(value)) = row.get(col_idx) {
                                    builder.append_value(value);
                                } else {
                                    builder.append_null();
                                }
                            }
                            Arc::new(builder.finish())
                        }
                    };
                    
                    columns.push(array);
                } else {
                    return Err(DataError::SchemaDetection(
                        format!("Column '{}' not found in CSV", field.name())
                    ));
                }
            }
            
            RecordBatch::try_new(schema, columns).map_err(|e| e.into())
        }).await.map_err(|e| DataError::SchemaDetection(e.to_string()))?
    }
}

#[async_trait]
impl dv_core::data::DataSource for ConfiguredCsvSource {
    async fn schema(&self) -> Arc<Schema> {
        self.schema.clone()
    }
    
    async fn navigation_spec(&self) -> anyhow::Result<NavigationSpec> {
        let mut spec = self.navigation_spec.clone();
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
                return Err(DataError::InvalidPosition.into());
            }
        };
        
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
        &self.source_name
    }
} 