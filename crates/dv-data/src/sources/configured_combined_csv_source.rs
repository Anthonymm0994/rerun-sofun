//! Combined CSV data source with configuration support

use std::sync::Arc;
use std::collections::HashMap;
use async_trait::async_trait;
use arrow::datatypes::{Schema, Field, DataType};
use arrow::record_batch::RecordBatch;
use arrow::array::{ArrayRef, StringArray, NullArray};
use dv_core::navigation::{NavigationPosition, NavigationSpec, NavigationMode, NavigationRange};
use dv_core::data::DataSource;
use crate::{DataError, config::FileConfig};
use super::configured_csv_source::ConfiguredCsvSource;

/// Combined CSV data source with configuration support
pub struct ConfiguredCombinedCsvSource {
    /// Individual configured CSV sources
    sources: Vec<ConfiguredCsvSource>,
    
    /// Combined schema
    schema: Arc<Schema>,
    
    /// Total row count
    total_rows: usize,
    
    /// Source file for each row range
    row_ranges: Vec<(usize, usize, usize)>, // (start, end, source_index)
    
    /// Mapping from column name to index in combined schema
    column_mapping: HashMap<String, usize>,
}

impl ConfiguredCombinedCsvSource {
    /// Create a new combined CSV source from configured files
    pub async fn new(configs: Vec<FileConfig>) -> Result<Self, DataError> {
        if configs.is_empty() {
            return Err(DataError::Other("No CSV files provided".to_string()));
        }
        
        // Load all configured CSV sources
        let mut sources = Vec::new();
        for config in configs {
            let source = ConfiguredCsvSource::new(config).await?;
            sources.push(source);
        }
        
        // Merge schemas
        let (combined_schema, column_mapping) = Self::merge_schemas(&sources)?;
        
        // Calculate row ranges
        let mut row_ranges = Vec::new();
        let mut current_row = 0;
        for (idx, source) in sources.iter().enumerate() {
            let source_rows = source.row_count;
            row_ranges.push((current_row, current_row + source_rows, idx));
            current_row += source_rows;
        }
        
        let total_rows = current_row;
        
        Ok(Self {
            sources,
            schema: Arc::new(combined_schema),
            total_rows,
            row_ranges,
            column_mapping,
        })
    }
    
    /// Merge schemas from multiple sources creating a union schema
    fn merge_schemas(sources: &[ConfiguredCsvSource]) -> Result<(Schema, HashMap<String, usize>), DataError> {
        if sources.is_empty() {
            return Err(DataError::Other("No sources to merge".to_string()));
        }
        
        // Collect all unique columns from all sources
        let mut all_fields: HashMap<String, Field> = HashMap::new();
        let mut column_mapping: HashMap<String, usize> = HashMap::new();
        
        // Add source file column first
        let source_field = Field::new("_source_file", DataType::Utf8, false);
        all_fields.insert("_source_file".to_string(), source_field);
        column_mapping.insert("_source_file".to_string(), 0);
        
        // Process each source schema
        for source in sources {
            for field in source.schema.fields() {
                let field_name = field.name().clone();
                if !all_fields.contains_key(&field_name) {
                    // New field - add it
                    all_fields.insert(field_name.clone(), field.as_ref().clone());
                } else {
                    // Existing field - check compatibility
                    let existing_field = &all_fields[&field_name];
                    if existing_field.data_type() != field.data_type() {
                        // Try to find a compatible type (e.g., promote to string)
                        let compatible_type = Self::find_compatible_type(existing_field.data_type(), field.data_type());
                        let updated_field = Field::new(&field_name, compatible_type, true); // Make nullable for safety
                        all_fields.insert(field_name.clone(), updated_field);
                    }
                }
            }
        }
        
        // Create ordered field list and update column mapping
        let mut fields: Vec<Field> = Vec::new();
        
        // Add source file column first
        fields.push(all_fields["_source_file"].clone());
        
        // Add other fields in alphabetical order for consistency
        let mut other_fields: Vec<_> = all_fields.iter()
            .filter(|(name, _)| *name != "_source_file")
            .collect();
        other_fields.sort_by_key(|(name, _)| *name);
        
        for (idx, (name, field)) in other_fields.iter().enumerate() {
            fields.push((*field).clone());
            column_mapping.insert((*name).clone(), idx + 1); // +1 for source file column
        }
        
        Ok((Schema::new(fields), column_mapping))
    }
    
    /// Find a compatible data type for two different types
    fn find_compatible_type(type1: &DataType, type2: &DataType) -> DataType {
        match (type1, type2) {
            // If types are the same, return it
            (t1, t2) if t1 == t2 => t1.clone(),
            // For different numeric types, promote to string for simplicity
            (DataType::Int64, DataType::Float64) | (DataType::Float64, DataType::Int64) => DataType::Utf8,
            // For any other mismatches, use string
            _ => DataType::Utf8,
        }
    }
    
    /// Create a record batch with the combined schema from a source batch
    fn align_batch_to_schema(&self, batch: RecordBatch, source_idx: usize) -> Result<RecordBatch, DataError> {
        let source = &self.sources[source_idx];
        let source_name = source.source_name();
        let num_rows = batch.num_rows();
        
        // Create columns for the combined schema
        let mut columns: Vec<ArrayRef> = Vec::new();
        
        // Add source file column
        let source_array: ArrayRef = Arc::new(StringArray::from(vec![source_name; num_rows]));
        columns.push(source_array);
        
        // Add other columns in schema order
        for field in self.schema.fields().iter().skip(1) { // Skip source file column
            let field_name = field.name();
            
            // Find this column in the source batch
            if let Some(source_field_idx) = source.schema.fields().iter().position(|f| f.name() == field_name) {
                // Column exists in source - use it
                columns.push(batch.column(source_field_idx).clone());
            } else {
                // Column doesn't exist in source - create null array
                let null_array: ArrayRef = Arc::new(NullArray::new(num_rows));
                columns.push(null_array);
                }
            }
        
        RecordBatch::try_new(self.schema.clone(), columns)
            .map_err(|e| DataError::Other(format!("Failed to create aligned batch: {}", e)))
    }
    
    /// Find which source and local row index for a given global row
    #[allow(dead_code)]
    fn find_source_for_row(&self, row: usize) -> Option<(usize, usize)> {
        for &(start, end, source_idx) in &self.row_ranges {
            if row >= start && row < end {
                return Some((source_idx, row - start));
            }
        }
        None
    }
}

#[async_trait]
impl dv_core::data::DataSource for ConfiguredCombinedCsvSource {
    async fn schema(&self) -> Arc<Schema> {
        self.schema.clone()
    }
    
    async fn navigation_spec(&self) -> anyhow::Result<NavigationSpec> {
        Ok(NavigationSpec {
            mode: NavigationMode::Sequential,
            total_rows: self.total_rows,
            temporal_bounds: None,
            categories: None,
        })
    }
    
    async fn query_at(&self, position: &NavigationPosition) -> anyhow::Result<RecordBatch> {
        let row_idx = match position {
            NavigationPosition::Sequential(idx) => *idx,
            _ => return Err(DataError::InvalidPosition.into()),
        };
        
        // Query a window around the position
        let window_size = 1000;
        let start = row_idx.saturating_sub(window_size / 2);
        let end = (start + window_size).min(self.total_rows);
        
        self.query_range(&NavigationRange {
            start: NavigationPosition::Sequential(start),
            end: NavigationPosition::Sequential(end),
        }).await
    }
    
    async fn query_range(&self, range: &NavigationRange) -> anyhow::Result<RecordBatch> {
        let (start, end) = match (&range.start, &range.end) {
            (NavigationPosition::Sequential(s), NavigationPosition::Sequential(e)) => (*s, *e),
            _ => return Err(DataError::InvalidPosition.into()),
        };
        
        // For simplicity, we'll query each source that overlaps the range
        // and combine the results. In production, this could be optimized.
        let mut batches = Vec::new();
        
        for &(range_start, range_end, source_idx) in &self.row_ranges {
            if range_end <= start || range_start >= end {
                continue; // No overlap
            }
            
            let source = &self.sources[source_idx];
            let local_start = start.saturating_sub(range_start);
            let local_end = (end - range_start).min(source.row_count);
            
            if local_start < local_end {
                let local_range = NavigationRange {
                    start: NavigationPosition::Sequential(local_start),
                    end: NavigationPosition::Sequential(local_end),
                };
                
                let batch = source.query_range(&local_range).await?;
                let aligned_batch = self.align_batch_to_schema(batch, source_idx)?;
                batches.push(aligned_batch);
            }
        }
        
        // Combine all batches
        if batches.is_empty() {
            return Ok(RecordBatch::new_empty(self.schema.clone()));
        }
        
        arrow::compute::concat_batches(&self.schema, &batches).map_err(|e| e.into())
    }
    

    
    async fn row_count(&self) -> anyhow::Result<usize> {
        Ok(self.total_rows)
    }
    
    fn source_name(&self) -> &str {
        "Combined CSV"
    }
} 