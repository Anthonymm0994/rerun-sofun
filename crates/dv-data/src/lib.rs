//! Data handling and sources for the visualization platform

pub mod cache;
pub mod index;
pub mod schema;
pub mod sources;
pub mod config;
pub mod memory;

use arrow::error::ArrowError;
use tokio::task::JoinError;
use thiserror::Error;

// Re-exports
pub use cache::DataCache;
pub use index::DataIndex;
pub use sources::{CsvSource, SqliteSource, ConfiguredCsvSource};

/// Errors that can occur in data operations
#[derive(Error, Debug)]
pub enum DataError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Arrow error: {0}")]
    Arrow(ArrowError),
    
    #[error("CSV parsing error: {0}")]
    Csv(String),
    
    #[error("SQLite error: {0}")]
    Sqlite(String),
    
    #[error("Schema detection error: {0}")]
    SchemaDetection(String),
    
    #[error("Invalid navigation position")]
    InvalidPosition,
    
    #[error("Join error: {0}")]
    Join(#[from] JoinError),
    
    #[error("Other error: {0}")]
    Other(String),
}

impl From<csv::Error> for DataError {
    fn from(error: csv::Error) -> Self {
        match error.kind() {
            csv::ErrorKind::Io(io_err) => DataError::Io(std::io::Error::new(io_err.kind(), error.to_string())),
            _ => DataError::Csv(error.to_string()),
        }
    }
}

impl From<ArrowError> for DataError {
    fn from(error: ArrowError) -> Self {
        DataError::Arrow(error)
    }
}

/// Type inference result for a single column
#[derive(Debug, Clone)]
pub struct TypeInferenceResult {
    pub name: String,
    pub data_type: arrow::datatypes::DataType,
    pub sample_values: Vec<String>,
    pub null_count: usize,
    pub row_count: usize,
}

/// Column info for file preview
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: arrow::datatypes::DataType,
} 