//! Core functionality for the data visualization platform
//! 
//! This crate provides the fundamental abstractions and state management
//! for the visualization system.

pub mod events;
pub mod navigation;
pub mod state;
pub mod sync;
pub mod notes;

// Re-export commonly used types
pub use navigation::{
    NavigationEngine, NavigationMode, NavigationPosition, 
    NavigationContext, NavigationSubscriber,
};
pub use state::{AppState, AppSettings, SpaceViewId, HoveredData, ViewerContext, TimeControl, FrameTime};
pub use data::DataSource;

// Placeholder modules that will be implemented in other crates
pub mod data {
    use std::sync::Arc;
    use crate::navigation::{NavigationSpec, NavigationPosition, NavigationRange};
    
    /// Trait for data sources
    #[async_trait::async_trait]
    pub trait DataSource: Send + Sync {
        /// Get the schema of this data source
        async fn schema(&self) -> Arc<arrow::datatypes::Schema>;
        
        /// Get the navigation specification
        async fn navigation_spec(&self) -> anyhow::Result<NavigationSpec>;
        
        /// Query data at a specific position
        async fn query_at(&self, position: &NavigationPosition) -> anyhow::Result<arrow::record_batch::RecordBatch>;
        
        /// Query data for a range
        async fn query_range(&self, range: &NavigationRange) -> anyhow::Result<arrow::record_batch::RecordBatch>;
        
        /// Query all data
        async fn query_all(&self) -> anyhow::Result<arrow::record_batch::RecordBatch>;
        
        /// Get total row count
        async fn row_count(&self) -> anyhow::Result<usize>;
        
        /// Get the source name/path
        fn source_name(&self) -> &str;
    }
}

pub mod templates {
    /// Template structure (placeholder)
    #[derive(Debug, Clone)]
    pub struct Template {
        pub id: String,
        pub name: String,
    }
} 