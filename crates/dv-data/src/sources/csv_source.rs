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
use crate::memory::{MemoryManager, estimate_batch_memory};

/// Performance tuning constants
const MAX_SAMPLE_ROWS: usize = 5000;  // Increased for better type detection
const CHUNK_SIZE: usize = 10000;      // Rows per chunk for efficient memory usage
const MAX_CACHED_CHUNKS: usize = 50;  // Maximum chunks to keep in memory
const PREFETCH_CHUNKS: usize = 2;     // Number of chunks to prefetch

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
    /// Cache for loaded data
    cache: Arc<RwLock<DataCache>>,
    /// Detected time column (reserved for future use)
    _time_column: Option<String>,
    /// Row offsets for seeking - stores byte offset for every CHUNK_SIZE rows
    row_offsets: Vec<u64>,
    /// Memory manager
    memory_manager: Arc<MemoryManager>,
}

/// Data cache for chunk-based loading
struct DataCache {
    chunks: AHashMap<usize, RecordBatch>,
    max_chunks: usize,
    /// LRU tracking for cache eviction
    access_order: Vec<usize>,
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
            cache: Arc::new(RwLock::new(DataCache {
                chunks: AHashMap::new(),
                max_chunks: 100,
                access_order: Vec::new(),
            })),
            _time_column: None,
            row_offsets: row_offsets,
            memory_manager: Arc::new(MemoryManager::new()),
        })
    }
    
    /// Analyze the CSV file to detect schema and build index
    async fn analyze_file(path: &Path) -> Result<(Schema, usize, Vec<u64>), DataError> {
        tokio::task::spawn_blocking({
            let path = path.to_path_buf();
            move || {
                let file = File::open(&path)?;
                let file_size = file.metadata()?.len();
                let mut reader = BufReader::new(file);
                let mut csv_reader = ReaderBuilder::new()
                    .has_headers(true)
                    .from_reader(&mut reader);
                
                // Get headers
                let headers = csv_reader.headers()?.clone();
                
                // Sample rows for type detection
                let mut sample_rows = Vec::new();
                let mut row_offsets = vec![0u64];
                
                // Read samples for type detection
                for (idx, result) in csv_reader.records().enumerate() {
                    let record = result?;
                    
                    // Store sample rows for type detection (up to MAX_SAMPLE_ROWS)
                    if sample_rows.len() < MAX_SAMPLE_ROWS {
                    sample_rows.push(record.iter().map(|s| s.to_string()).collect::<Vec<_>>());
                }
                    
                    // Track byte offsets every CHUNK_SIZE rows for efficient seeking
                    if (idx + 1) % CHUNK_SIZE == 0 {
                        // Approximate offset based on file position
                        let current_offset = (file_size * (idx as u64 + 1)) / (sample_rows.len() as u64 + idx as u64 + 1);
                        row_offsets.push(current_offset);
                    }
                }
                
                // Count total rows
                let total_rows = sample_rows.len() + csv_reader.records().count();
                
                // Detect column types
                let fields = headers.iter().enumerate().map(|(idx, name)| {
                    let data_type = Self::detect_column_type(&sample_rows, idx);
                    Field::new(name, data_type, true)
                }).collect::<Vec<_>>();
                
                let schema = Schema::new(fields);
                
                Ok((schema, total_rows, row_offsets))
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
    
    /// Read a chunk of data from the CSV file with caching
    async fn read_chunk(&self, start_row: usize, num_rows: usize) -> Result<RecordBatch, DataError> {
        let chunk_id = start_row / CHUNK_SIZE;
        
        // Check cache first
        {
            let mut cache = self.cache.write();
            if let Some(batch) = cache.chunks.get(&chunk_id) {
                // Clone the batch to avoid borrowing issues
                let batch_clone = batch.clone();
                
                // Update LRU
                cache.access_order.retain(|&id| id != chunk_id);
                cache.access_order.push(chunk_id);
                
                // Extract the requested range from the cached chunk
                let chunk_start = chunk_id * CHUNK_SIZE;
                let offset_in_chunk = start_row - chunk_start;
                let available_in_chunk = batch_clone.num_rows() - offset_in_chunk;
                let rows_to_take = num_rows.min(available_in_chunk);
                
                return Ok(batch_clone.slice(offset_in_chunk, rows_to_take));
            }
        }
        
        // Not in cache, load from file
        let path = self.path.clone();
        let schema = self.schema.clone();
        let chunk_start = chunk_id * CHUNK_SIZE;
        let chunk_rows = CHUNK_SIZE.min(self.row_count - chunk_start);
        
        let batch = tokio::task::spawn_blocking(move || {
            Self::read_chunk_from_file(&path, schema, chunk_start, chunk_rows)
        }).await.map_err(|e| DataError::Other(e.to_string()))??;
        
        // Estimate memory usage of the new batch
        let batch_memory = estimate_batch_memory(&batch);
        
        // Store in cache with memory-aware eviction
        {
            let mut cache = self.cache.write();
            
            // Check if we need to evict based on memory pressure
            let current_memory: usize = cache.chunks.values()
                .map(estimate_batch_memory)
                .sum();
            
            // Update memory stats
            self.memory_manager.update_cache_memory(
                current_memory + batch_memory,
                cache.chunks.len() + 1
            );
            
            // Evict if necessary
            if self.memory_manager.should_evict() || cache.chunks.len() >= cache.max_chunks {
                // Evict based on LRU and memory usage
                let chunks_to_evict = if cache.access_order.len() > 0 {
                    // Calculate how many chunks to evict
                    let target_evict = ((cache.chunks.len() + 1).saturating_sub(cache.max_chunks / 2)).max(1);
                    
                    // Get the least recently used chunks
                    let evict_count = target_evict.min(cache.access_order.len());
                    let chunks_to_remove: Vec<usize> = cache.access_order
                        .drain(..evict_count)
                        .collect();
                    
                    chunks_to_remove
                } else {
                    vec![]
                };
                
                // Remove evicted chunks
                for chunk_id in chunks_to_evict {
                    cache.chunks.remove(&chunk_id);
                }
            }
            
            cache.chunks.insert(chunk_id, batch.clone());
            cache.access_order.push(chunk_id);
            
            // Update memory stats after eviction
            let new_memory: usize = cache.chunks.values()
                .map(estimate_batch_memory)
                .sum();
            self.memory_manager.update_cache_memory(new_memory, cache.chunks.len());
        }
        
        // Extract the requested range
        let offset_in_chunk = start_row - chunk_start;
        let available_in_chunk = batch.num_rows() - offset_in_chunk;
        let rows_to_take = num_rows.min(available_in_chunk);
        
        Ok(batch.slice(offset_in_chunk, rows_to_take))
    }
    
    /// Read a chunk directly from file
    fn read_chunk_from_file(path: &Path, schema: Arc<Schema>, start_row: usize, num_rows: usize) -> Result<RecordBatch, DataError> {
        let file = File::open(path)?;
            let reader = BufReader::new(file);
            let mut csv_reader = ReaderBuilder::new()
                .has_headers(true)
                .from_reader(reader);
            
            // Skip to start row
        for _ in 0..=start_row {  // +1 for header
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
                DataType::Boolean => {
                    let mut builder = BooleanBuilder::new();
                    for row in &row_data {
                        if let Some(value) = row.get(col_idx) {
                            if value.is_empty() {
                                builder.append_null();
                            } else {
                                let lower = value.to_lowercase();
                                if lower == "true" || lower == "1" || lower == "yes" {
                                    builder.append_value(true);
                                } else if lower == "false" || lower == "0" || lower == "no" {
                                    builder.append_value(false);
                                } else {
                                    builder.append_null();
                                }
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
        
        // Query a smaller window for better performance
        let window_size = CHUNK_SIZE / 2;
        let start = row_idx.saturating_sub(window_size / 2);
        let end = (row_idx + window_size / 2).min(self.row_count);
        
        self.read_chunk(start, end - start).await.map_err(|e| e.into())
    }
    
    async fn query_range(&self, range: &NavigationRange) -> anyhow::Result<RecordBatch> {
        let (start, end) = match (&range.start, &range.end) {
            (NavigationPosition::Sequential(s), NavigationPosition::Sequential(e)) => (*s, *e),
            _ => return Err(DataError::InvalidPosition.into()),
        };
        
        // For large ranges, limit to a reasonable size
        let max_range = CHUNK_SIZE * 2;
        let actual_end = end.min(start + max_range);
        
        self.read_chunk(start, actual_end - start).await.map_err(|e| e.into())
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