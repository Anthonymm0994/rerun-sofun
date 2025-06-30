//! SQLite data source implementation

use std::path::{Path, PathBuf};
use std::sync::Arc;
use async_trait::async_trait;
use arrow::array::{
    ArrayRef, ArrayBuilder, 
    Float64Builder,
    Int64Builder,
    StringBuilder,
    BooleanBuilder
};
use arrow::datatypes::{Schema, Field, DataType};
use arrow::record_batch::RecordBatch;
use dv_core::navigation::{NavigationPosition, NavigationSpec, NavigationMode, NavigationRange};
use rusqlite::{Connection, types::ValueRef};
use crate::DataError;

/// SQLite data source implementation
pub struct SqliteSource {
    path: PathBuf,
    table_name: String,
    schema: Arc<Schema>,
    row_count: usize,
    time_column: Option<String>,
}

impl SqliteSource {
    /// Create a new SQLite data source
    pub async fn new<P: AsRef<Path>>(path: P, table_name: String) -> Result<Self, DataError> {
        let path = path.as_ref().to_path_buf();
        
        // Open connection to detect schema
        let conn = Connection::open(&path)
            .map_err(|e| DataError::Other(format!("Failed to open SQLite database: {}", e)))?;
        
        // Get schema from table
        let schema = Self::detect_schema(&conn, &table_name)?;
        
        // Count rows
        let row_count = Self::count_rows(&conn, &table_name)?;
        
        // Detect time column if any
        let time_column = Self::detect_time_column(&schema);
        
        Ok(Self {
            path,
            table_name,
            schema: Arc::new(schema),
            row_count,
            time_column,
        })
    }
    
    /// Detect schema from SQLite table
    fn detect_schema(conn: &Connection, table_name: &str) -> Result<Schema, DataError> {
        let query = format!("PRAGMA table_info({})", table_name);
        let mut stmt = conn.prepare(&query)
            .map_err(|e| DataError::Other(format!("Failed to get table info: {}", e)))?;
        
        let mut fields = Vec::new();
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,  // column name
                row.get::<_, String>(2)?,  // data type
                row.get::<_, i32>(3)? == 0 // not null
            ))
        }).map_err(|e| DataError::Other(format!("Failed to query table info: {}", e)))?;
        
        for row_result in rows {
            let (name, sqlite_type, nullable) = row_result
                .map_err(|e| DataError::Other(format!("Failed to read column info: {}", e)))?;
            
            let arrow_type = match sqlite_type.to_uppercase().as_str() {
                "INTEGER" => DataType::Int64,
                "REAL" | "FLOAT" | "DOUBLE" => DataType::Float64,
                "TEXT" | "VARCHAR" => DataType::Utf8,
                "BOOLEAN" => DataType::Boolean,
                "DATE" | "DATETIME" | "TIMESTAMP" => DataType::Utf8, // Parse as string for now
                _ => DataType::Utf8, // Default to string
            };
            
            fields.push(Field::new(&name, arrow_type, nullable));
        }
        
        if fields.is_empty() {
            return Err(DataError::SchemaDetection(format!("Table '{}' has no columns", table_name)));
        }
        
        Ok(Schema::new(fields))
    }
    
    /// Count rows in table
    fn count_rows(conn: &Connection, table_name: &str) -> Result<usize, DataError> {
        let query = format!("SELECT COUNT(*) FROM {}", table_name);
        let count: i64 = conn.query_row(&query, [], |row| row.get(0))
            .map_err(|e| DataError::Other(format!("Failed to count rows: {}", e)))?;
        Ok(count as usize)
    }
    
    /// Detect time column if any
    fn detect_time_column(schema: &Schema) -> Option<String> {
        for field in schema.fields() {
            let name_lower = field.name().to_lowercase();
            if name_lower.contains("time") || name_lower.contains("date") || 
               name_lower == "timestamp" || name_lower == "created" {
                return Some(field.name().clone());
            }
        }
        None
    }
    
    /// Query data with limit and offset
    async fn query_data(&self, limit: usize, offset: usize) -> Result<RecordBatch, DataError> {
        let conn = Connection::open(&self.path)
            .map_err(|e| DataError::Other(format!("Failed to open database: {}", e)))?;
        
        let query = format!(
            "SELECT * FROM {} LIMIT {} OFFSET {}",
            self.table_name, limit, offset
        );
        
        let mut stmt = conn.prepare(&query)
            .map_err(|e| DataError::Other(format!("Failed to prepare query: {}", e)))?;
        
        // Initialize column builders
        let mut builders: Vec<Box<dyn ArrayBuilder>> = self.schema.fields()
            .iter()
            .map(|field| match field.data_type() {
                DataType::Int64 => Box::new(Int64Builder::new()) as Box<dyn ArrayBuilder>,
                DataType::Float64 => Box::new(Float64Builder::new()) as Box<dyn ArrayBuilder>,
                DataType::Utf8 => Box::new(StringBuilder::new()) as Box<dyn ArrayBuilder>,
                DataType::Boolean => Box::new(BooleanBuilder::new()) as Box<dyn ArrayBuilder>,
                _ => Box::new(StringBuilder::new()) as Box<dyn ArrayBuilder>,
            })
            .collect();
        
        // Execute query and build arrays
        let mut rows = stmt.query([])
            .map_err(|e| DataError::Other(format!("Failed to execute query: {}", e)))?;
        
        while let Some(row) = rows.next()
            .map_err(|e| DataError::Other(format!("Failed to fetch row: {}", e)))? {
            
            for (col_idx, field) in self.schema.fields().iter().enumerate() {
                let value = row.get_ref(col_idx)
                    .map_err(|e| DataError::Other(format!("Failed to get column value: {}", e)))?;
                
                match (field.data_type(), &mut builders[col_idx]) {
                    (DataType::Int64, builder) => {
                        let builder = builder.as_any_mut().downcast_mut::<Int64Builder>().unwrap();
                        match value {
                            ValueRef::Integer(i) => builder.append_value(i),
                            ValueRef::Null => builder.append_null(),
                            _ => builder.append_null(),
                        }
                    }
                    (DataType::Float64, builder) => {
                        let builder = builder.as_any_mut().downcast_mut::<Float64Builder>().unwrap();
                        match value {
                            ValueRef::Real(f) => builder.append_value(f),
                            ValueRef::Integer(i) => builder.append_value(i as f64),
                            ValueRef::Null => builder.append_null(),
                            _ => builder.append_null(),
                        }
                    }
                    (DataType::Boolean, builder) => {
                        let builder = builder.as_any_mut().downcast_mut::<BooleanBuilder>().unwrap();
                        match value {
                            ValueRef::Integer(i) => builder.append_value(i != 0),
                            ValueRef::Null => builder.append_null(),
                            _ => builder.append_null(),
                        }
                    }
                    (_, builder) => {
                        let builder = builder.as_any_mut().downcast_mut::<StringBuilder>().unwrap();
                        match value {
                            ValueRef::Text(s) => {
                                let text = std::str::from_utf8(s).unwrap_or("");
                                builder.append_value(text);
                            }
                            ValueRef::Null => builder.append_null(),
                            ValueRef::Integer(i) => builder.append_value(i.to_string()),
                            ValueRef::Real(f) => builder.append_value(f.to_string()),
                            _ => builder.append_null(),
                        }
                    }
                }
            }
        }
        
        // Build final arrays
        let arrays: Vec<ArrayRef> = builders.into_iter()
            .map(|mut builder| builder.finish())
            .collect();
        
        RecordBatch::try_new(self.schema.clone(), arrays)
            .map_err(|e| DataError::Arrow(e))
    }
}

#[async_trait]
impl dv_core::data::DataSource for SqliteSource {
    async fn schema(&self) -> Arc<Schema> {
        self.schema.clone()
    }
    
    async fn navigation_spec(&self) -> anyhow::Result<NavigationSpec> {
        Ok(NavigationSpec {
            mode: if self.time_column.is_some() {
                NavigationMode::Temporal
            } else {
                NavigationMode::Sequential
            },
            total_rows: self.row_count,
            temporal_bounds: None, // TODO: Could query min/max of time column
            categories: None,
        })
    }
    
    async fn query_at(&self, position: &NavigationPosition) -> anyhow::Result<RecordBatch> {
        let row_idx = match position {
            NavigationPosition::Sequential(idx) => *idx,
            NavigationPosition::Temporal(_) => {
                // TODO: Implement time-based querying
                0
            }
            NavigationPosition::Categorical(_) => {
                return Err(DataError::InvalidPosition.into());
            }
        };
        
        // Query a window around the position
        let window_size = 1000;
        let start = row_idx.saturating_sub(window_size / 2);
        
        self.query_data(window_size, start).await.map_err(|e| e.into())
    }
    
    async fn query_range(&self, range: &NavigationRange) -> anyhow::Result<RecordBatch> {
        let (start, end) = match (&range.start, &range.end) {
            (NavigationPosition::Sequential(s), NavigationPosition::Sequential(e)) => (*s, *e),
            _ => return Err(DataError::InvalidPosition.into()),
        };
        
        let count = end.saturating_sub(start);
        self.query_data(count, start).await.map_err(|e| e.into())
    }
    
    async fn row_count(&self) -> anyhow::Result<usize> {
        Ok(self.row_count)
    }
    
    fn source_name(&self) -> &str {
        self.path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.db")
    }
} 