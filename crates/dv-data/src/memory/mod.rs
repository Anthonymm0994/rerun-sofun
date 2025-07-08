//! Memory management utilities for handling large datasets

use std::sync::Arc;
use parking_lot::RwLock;

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total memory allocated for data (bytes)
    pub data_memory: usize,
    /// Total memory allocated for caches (bytes) 
    pub cache_memory: usize,
    /// Number of cached chunks
    pub cached_chunks: usize,
    /// Maximum memory limit (bytes)
    pub memory_limit: usize,
}

impl Default for MemoryStats {
    fn default() -> Self {
        Self {
            data_memory: 0,
            cache_memory: 0,
            cached_chunks: 0,
            // Default to 1GB limit
            memory_limit: 1024 * 1024 * 1024,
        }
    }
}

/// Global memory manager
pub struct MemoryManager {
    stats: Arc<RwLock<MemoryStats>>,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(MemoryStats::default())),
        }
    }
    
    /// Get current memory stats
    pub fn stats(&self) -> MemoryStats {
        self.stats.read().clone()
    }
    
    /// Update data memory usage
    pub fn update_data_memory(&self, bytes: usize) {
        let mut stats = self.stats.write();
        stats.data_memory = bytes;
    }
    
    /// Update cache memory usage
    pub fn update_cache_memory(&self, bytes: usize, chunks: usize) {
        let mut stats = self.stats.write();
        stats.cache_memory = bytes;
        stats.cached_chunks = chunks;
    }
    
    /// Check if we should evict cache based on memory usage
    pub fn should_evict(&self) -> bool {
        let stats = self.stats.read();
        let total_memory = stats.data_memory + stats.cache_memory;
        total_memory > stats.memory_limit
    }
    
    /// Set memory limit
    pub fn set_memory_limit(&self, limit_mb: usize) {
        let mut stats = self.stats.write();
        stats.memory_limit = limit_mb * 1024 * 1024;
    }
}

/// Estimate memory usage of a RecordBatch
pub fn estimate_batch_memory(batch: &arrow::record_batch::RecordBatch) -> usize {
    let mut total_bytes = 0;
    
    for column in batch.columns() {
        // Base data size
        total_bytes += column.get_array_memory_size();
        
        // Add overhead for null bitmap if present
        if column.null_count() > 0 {
            total_bytes += (batch.num_rows() + 7) / 8; // 1 bit per row
        }
    }
    
    // Add some overhead for metadata
    total_bytes += std::mem::size_of::<arrow::record_batch::RecordBatch>();
    total_bytes += batch.schema().fields().len() * 64; // Estimate for field metadata
    
    total_bytes
}

/// Memory-aware cache eviction policy
pub trait CacheEvictionPolicy {
    /// Determine which chunks to evict given current memory pressure
    fn chunks_to_evict(&self, cache_info: &[(usize, usize, u64)]) -> Vec<usize>;
}

/// LRU (Least Recently Used) eviction policy
pub struct LruEvictionPolicy {
    target_chunks: usize,
}

impl LruEvictionPolicy {
    pub fn new(target_chunks: usize) -> Self {
        Self { target_chunks }
    }
}

impl CacheEvictionPolicy for LruEvictionPolicy {
    fn chunks_to_evict(&self, cache_info: &[(usize, usize, u64)]) -> Vec<usize> {
        // Sort by last access time (oldest first)
        let mut sorted: Vec<_> = cache_info.to_vec();
        sorted.sort_by_key(|&(_, _, access_time)| access_time);
        
        // Evict oldest chunks
        sorted.iter()
            .take(self.target_chunks)
            .map(|&(chunk_id, _, _)| chunk_id)
            .collect()
    }
} 