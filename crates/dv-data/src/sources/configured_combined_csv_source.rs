//! Combined CSV data source with configuration support

use std::sync::Arc;
use async_trait::async_trait;
use arrow::datatypes::{Schema, Field};
use arrow::record_batch::RecordBatch;
use arrow::array::{ArrayRef, StringArray};
use dv_core::navigation::{NavigationPosition, NavigationSpec, NavigationMode, NavigationRange};
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
        let combined_schema = Self::merge_schemas(&sources)?;
        
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
        })
    }
    
    /// Merge schemas from multiple sources
    fn merge_schemas(sources: &[ConfiguredCsvSource]) -> Result<Schema, DataError> {
        if sources.is_empty() {
            return Err(DataError::Other("No sources to merge".to_string()));
        }
        
        // Start with the first schema
        let mut fields: Vec<Field> = sources[0].schema.fields()
            .iter()
            .map(|f| f.as_ref().clone())
            .collect();
        
        // Add a source file column
        fields.insert(0, Field::new("_source_file", arrow::datatypes::DataType::Utf8, false));
        
        // For now, we'll require all CSV files to have the same schema
        // In the future, we could do schema reconciliation
        for (idx, source) in sources.iter().enumerate().skip(1) {
            if source.schema.fields().len() != sources[0].schema.fields().len() {
                return Err(DataError::SchemaDetection(
                    format!("Schema mismatch: file {} has different number of columns", idx)
                ));
            }
            
            // Check field names match
            for (i, field) in source.schema.fields().iter().enumerate() {
                if field.name() != sources[0].schema.fields()[i].name() {
                    return Err(DataError::SchemaDetection(
                        format!("Schema mismatch: column '{}' in file {} doesn't match", field.name(), idx)
                    ));
                }
            }
        }
        
        Ok(Schema::new(fields))
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
                
                let mut batch = source.query_range(&local_range).await?;
                
                // Add source file column
                let source_name = source.source_name();
                let num_rows = batch.num_rows();
                let source_array: ArrayRef = Arc::new(
                    StringArray::from(vec![source_name; num_rows])
                );
                
                // Create new batch with source column
                let mut columns = vec![source_array];
                columns.extend_from_slice(batch.columns());
                
                batch = RecordBatch::try_new(self.schema.clone(), columns)?;
                batches.push(batch);
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