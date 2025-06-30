//! Data layer for loading and managing data sources
//! 
//! This crate provides implementations for various data sources
//! including CSV files and SQLite databases.

pub mod sources;
pub mod cache;
pub mod index;
pub mod schema;

// Re-export commonly used types
pub use sources::{CsvSource, SqliteSource};
pub use cache::DataCache;
pub use index::DataIndex;

use thiserror::Error;

/// Errors that can occur in the data layer
#[derive(Error, Debug)]
pub enum DataError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    
    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow::error::ArrowError),
    
    #[error("Schema detection failed: {0}")]
    SchemaDetection(String),
    
    #[error("Invalid position for navigation")]
    InvalidPosition,
    
    #[error("Data source error: {0}")]
    Other(String),
} 