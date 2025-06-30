use serde::{Serialize, Deserialize};

/// A position in the navigation space
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NavigationPosition {
    /// Temporal position (timestamp in milliseconds)
    Temporal(i64),
    /// Sequential position (row index)
    Sequential(usize),
    /// Categorical position (category name)
    Categorical(String),
}

impl NavigationPosition {
    /// Get frame number (row index) for the position
    pub fn frame_nr(&self) -> usize {
        match self {
            NavigationPosition::Sequential(idx) => *idx,
            NavigationPosition::Temporal(time) => *time as usize, // Assume 1:1 time to frame for now
            NavigationPosition::Categorical(_) => 0, // TODO: Need category index
        }
    }
}

/// Navigation bounds for a data source
#[derive(Debug, Clone)]
pub struct NavigationBounds<T> {
    pub start: T,
    pub end: T,
}

/// A range in the navigation space
#[derive(Debug, Clone)]
pub struct NavigationRange {
    pub start: NavigationPosition,
    pub end: NavigationPosition,
} 