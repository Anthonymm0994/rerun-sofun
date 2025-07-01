//! View system for data visualization platform

mod space_view;
mod viewport;
mod time_series_view;
pub mod plots;
mod tables;
mod stats;

pub use space_view::{SpaceView, SpaceViewId, SpaceViewConfig, SelectionState};
pub use viewport::{Viewport, GridLayoutConfig, GridCell};
pub use time_series_view::{TimeSeriesView, TimeSeriesConfig};
pub use tables::{TableView, TableConfig};
pub use plots::{ScatterPlotView, ScatterPlotConfig, BarChartView, BarChartConfig};
pub use stats::SummaryStatsView;

use std::sync::Arc;
use parking_lot::RwLock;
use dv_core::{
    data::DataSource,
    navigation::NavigationEngine,
};

/// Hovered data information
#[derive(Default, Clone)]
pub struct HoveredData {
    pub x: f64,
    pub y: f64,
    pub column: String,
    pub view_id: Option<SpaceViewId>,
    pub point_index: Option<usize>,
}

/// Time control state
#[derive(Clone)]
pub struct TimeControl {
    pub playing: bool,
    pub speed: f64,
    pub looping: bool,
}

impl Default for TimeControl {
    fn default() -> Self {
        Self {
            playing: false,
            speed: 1.0,
            looping: false,
        }
    }
}

/// Frame timing information
#[derive(Default, Clone)]
pub struct FrameTime {
    pub avg_frame_ms: f64,
    pub max_frame_ms: f64,
}

/// Context passed to views during rendering
#[derive(Clone)]
pub struct ViewerContext {
    /// Current data source
    pub data_source: Arc<RwLock<Option<Box<dyn DataSource>>>>,
    
    /// Navigation engine
    pub navigation: Arc<NavigationEngine>,
    
    /// Time control state
    pub time_control: Arc<RwLock<TimeControl>>,
    
    /// Currently hovered data
    pub hovered_data: Arc<RwLock<HoveredData>>,
    
    /// Frame timing
    pub frame_time: Arc<RwLock<FrameTime>>,
    
    /// Tokio runtime handle
    pub runtime_handle: tokio::runtime::Handle,
    
    /// Views that share time axis
    pub time_axis_views: Arc<RwLock<Vec<SpaceViewId>>>,
} 