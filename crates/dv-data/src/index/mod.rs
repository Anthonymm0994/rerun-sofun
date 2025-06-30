//! Indexing functionality for efficient data access

// TODO: Implement indexing for large files
// - Row offset index for quick seeking
// - Time-based index for temporal data
// - Column value index for filtering 

use std::sync::Arc;
use parking_lot::RwLock;
use ahash::AHashMap;

/// Index for efficient data navigation
pub struct DataIndex {
    /// Time index mapping timestamps to row indices
    time_index: Arc<RwLock<Vec<(i64, usize)>>>,
    /// Category index mapping category values to row indices
    category_index: Arc<RwLock<AHashMap<String, Vec<usize>>>>,
}

impl DataIndex {
    /// Create a new data index
    pub fn new() -> Self {
        Self {
            time_index: Arc::new(RwLock::new(Vec::new())),
            category_index: Arc::new(RwLock::new(AHashMap::new())),
        }
    }
    
    /// Add a time entry to the index
    pub fn add_time_entry(&self, timestamp: i64, row_index: usize) {
        self.time_index.write().push((timestamp, row_index));
    }
    
    /// Add a category entry to the index
    pub fn add_category_entry(&self, category: String, row_index: usize) {
        self.category_index
            .write()
            .entry(category)
            .or_insert_with(Vec::new)
            .push(row_index);
    }
    
    /// Find row index for a timestamp
    pub fn find_time_row(&self, timestamp: i64) -> Option<usize> {
        let index = self.time_index.read();
        // Binary search for closest timestamp
        match index.binary_search_by_key(&timestamp, |&(t, _)| t) {
            Ok(idx) => Some(index[idx].1),
            Err(idx) => {
                if idx > 0 {
                    Some(index[idx - 1].1)
                } else {
                    None
                }
            }
        }
    }
    
    /// Get all row indices for a category
    pub fn get_category_rows(&self, category: &str) -> Vec<usize> {
        self.category_index
            .read()
            .get(category)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Clear the index
    pub fn clear(&self) {
        self.time_index.write().clear();
        self.category_index.write().clear();
    }
}

impl Default for DataIndex {
    fn default() -> Self {
        Self::new()
    }
} 