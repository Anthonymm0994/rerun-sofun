use serde::{Serialize, Deserialize};

mod engine;
mod position;
mod subscriber;

pub use engine::NavigationEngine;
pub use position::{NavigationPosition, NavigationBounds, NavigationRange};
pub use subscriber::NavigationSubscriber;

/// Navigation modes supported by the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NavigationMode {
    /// Time-based navigation (timestamps)
    Temporal,
    /// Sequential navigation (row indices)
    Sequential,
    /// Categorical navigation (discrete values)
    Categorical { 
        categories: Vec<String>,
    },
}

/// Navigation specification for data sources
#[derive(Debug, Clone)]
pub struct NavigationSpec {
    pub mode: NavigationMode,
    pub total_rows: usize,
    pub temporal_bounds: Option<(i64, i64)>,
    pub categories: Option<Vec<String>>,
}

/// Context passed to views during navigation updates
#[derive(Debug, Clone)]
pub struct NavigationContext {
    pub mode: NavigationMode,
    pub position: NavigationPosition,
    pub selection_range: Option<NavigationRange>,
    pub total_rows: usize,
} 