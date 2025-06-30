//! Query result caching for performance

// TODO: Implement caching layer
// - LRU cache for recent queries
// - Memory-mapped cache for large datasets
// - Query result deduplication 

//! Data caching layer

use std::sync::Arc;
use arrow::record_batch::RecordBatch;
use parking_lot::RwLock;
use ahash::AHashMap;

/// Data cache for storing frequently accessed data
pub struct DataCache {
    /// Cached chunks indexed by chunk ID
    chunks: Arc<RwLock<AHashMap<usize, RecordBatch>>>,
    /// Maximum number of chunks to cache
    max_chunks: usize,
}

impl DataCache {
    /// Create a new data cache
    pub fn new(max_chunks: usize) -> Self {
        Self {
            chunks: Arc::new(RwLock::new(AHashMap::new())),
            max_chunks,
        }
    }
    
    /// Get a chunk from cache
    pub fn get(&self, chunk_id: usize) -> Option<RecordBatch> {
        self.chunks.read().get(&chunk_id).cloned()
    }
    
    /// Put a chunk in cache
    pub fn put(&self, chunk_id: usize, batch: RecordBatch) {
        let mut chunks = self.chunks.write();
        
        // Simple LRU eviction if at capacity
        if chunks.len() >= self.max_chunks && !chunks.contains_key(&chunk_id) {
            // Remove a random chunk (proper LRU would track access times)
            if let Some(key) = chunks.keys().next().cloned() {
                chunks.remove(&key);
            }
        }
        
        chunks.insert(chunk_id, batch);
    }
    
    /// Clear the cache
    pub fn clear(&self) {
        self.chunks.write().clear();
    }
} 