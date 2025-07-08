//! File configuration for data loading

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use arrow::datatypes::DataType;

use super::null_handling::NullConfig;

/// Data type override that can be serialized
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SerializableDataType {
    Boolean,
    Int32,
    Int64,
    Float32,
    Float64,
    Utf8,
    Date32,
    Date64,
    Timestamp,
}

impl From<SerializableDataType> for DataType {
    fn from(sdt: SerializableDataType) -> Self {
        match sdt {
            SerializableDataType::Boolean => DataType::Boolean,
            SerializableDataType::Int32 => DataType::Int32,
            SerializableDataType::Int64 => DataType::Int64,
            SerializableDataType::Float32 => DataType::Float32,
            SerializableDataType::Float64 => DataType::Float64,
            SerializableDataType::Utf8 => DataType::Utf8,
            SerializableDataType::Date32 => DataType::Date32,
            SerializableDataType::Date64 => DataType::Date64,
            SerializableDataType::Timestamp => DataType::Timestamp(arrow::datatypes::TimeUnit::Millisecond, None),
        }
    }
}

impl From<DataType> for SerializableDataType {
    fn from(dt: DataType) -> Self {
        match dt {
            DataType::Boolean => SerializableDataType::Boolean,
            DataType::Int32 => SerializableDataType::Int32,
            DataType::Int64 => SerializableDataType::Int64,
            DataType::Float32 => SerializableDataType::Float32,
            DataType::Float64 => SerializableDataType::Float64,
            DataType::Utf8 => SerializableDataType::Utf8,
            DataType::Date32 => SerializableDataType::Date32,
            DataType::Date64 => SerializableDataType::Date64,
            DataType::Timestamp(_, _) => SerializableDataType::Timestamp,
            _ => SerializableDataType::Utf8, // Default to string for unsupported types
        }
    }
}

/// Configuration for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConfig {
    /// Path to the file
    pub path: PathBuf,
    
    /// File type
    pub file_type: FileType,
    
    /// Header line number (0-indexed) for CSV files
    pub header_line: usize,
    
    /// Selected columns (column names for CSV, table names for SQLite)
    pub selected_columns: HashSet<String>,
    
    /// Column type overrides
    pub column_types: HashMap<String, SerializableDataType>,
    
    /// Null handling configuration
    pub null_config: NullConfig,
    
    /// Sample size for type inference
    pub sample_size: usize,
    
    /// Whether the file has been loaded
    pub is_loaded: bool,
    
    /// Preview lines cache
    pub preview_lines: Option<Vec<Vec<String>>>,
    
    /// Detected column names
    pub detected_columns: Vec<String>,
}

/// File type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Csv,
    Sqlite,
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            file_type: FileType::Csv,
            header_line: 0,
            selected_columns: HashSet::new(),
            column_types: HashMap::new(),
            null_config: NullConfig::default(),
            sample_size: 1000,
            is_loaded: false,
            preview_lines: None,
            detected_columns: Vec::new(),
        }
    }
}

impl FileConfig {
    /// Create a new file configuration
    pub fn new(path: PathBuf, file_type: FileType) -> Self {
        Self {
            path,
            file_type,
            header_line: 0,
            selected_columns: HashSet::new(),
            column_types: HashMap::new(),
            sample_size: 1000,
            null_config: NullConfig::default(),
            is_loaded: false,
            preview_lines: None,
            detected_columns: Vec::new(),
        }
    }
    
    /// Get the file name
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    }
    
    /// Check if configuration has changed
    pub fn has_changed(&self, other: &FileConfig) -> bool {
        self.header_line != other.header_line
            || self.selected_columns != other.selected_columns
            || self.column_types != other.column_types
    }
    
    /// Get column type with override
    pub fn get_column_type(&self, column: &str, detected_type: &DataType) -> DataType {
        self.column_types
            .get(column)
            .cloned()
            .map(|sdt| sdt.into())
            .unwrap_or_else(|| detected_type.clone())
    }
    
    /// Check if a value should be treated as null
    pub fn is_null_value(&self, value: &str) -> bool {
        self.null_config.patterns.iter().any(|pattern| value == pattern)
    }
}

/// File configuration manager
#[derive(Debug, Clone, Default)]
pub struct FileConfigManager {
    /// Configurations for each file
    pub configs: HashMap<PathBuf, FileConfig>,
    
    /// Currently active file
    pub active_file: Option<PathBuf>,
}

impl FileConfigManager {
    /// Create a new file configuration manager
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a file configuration
    pub fn add_file(&mut self, config: FileConfig) {
        let path = config.path.clone();
        self.configs.insert(path.clone(), config);
        
        // Set as active if it's the first file
        if self.active_file.is_none() {
            self.active_file = Some(path);
        }
    }
    
    /// Get the active file configuration
    pub fn active_config(&self) -> Option<&FileConfig> {
        self.active_file
            .as_ref()
            .and_then(|path| self.configs.get(path))
    }
    
    /// Get mutable active file configuration
    pub fn active_config_mut(&mut self) -> Option<&mut FileConfig> {
        self.active_file
            .as_ref()
            .and_then(|path| self.configs.get_mut(path))
    }
    
    /// Set active file
    pub fn set_active_file(&mut self, path: PathBuf) {
        if self.configs.contains_key(&path) {
            self.active_file = Some(path);
        }
    }
    
    /// Get all file names
    pub fn file_names(&self) -> Vec<String> {
        self.configs
            .values()
            .map(|config| config.file_name())
            .collect()
    }
    
    /// Remove a file configuration
    pub fn remove_file(&mut self, path: &PathBuf) {
        self.configs.remove(path);
        
        // Update active file if needed
        if self.active_file.as_ref() == Some(path) {
            self.active_file = self.configs.keys().next().cloned();
        }
    }
} 