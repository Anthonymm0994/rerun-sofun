//! Application state management

use uuid::Uuid;
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;

pub type SpaceViewId = Uuid;

/// Application-wide state
pub struct AppState {
    pub settings: AppSettings,
}

/// Application settings
pub struct AppSettings {
    pub dark_mode: bool,
    pub auto_save: bool,
    pub show_navigation_bar: bool,
    pub show_stats_panel: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            dark_mode: true,
            auto_save: true,
            show_navigation_bar: true,
            show_stats_panel: true,
        }
    }
}

/// Hovered data information
#[derive(Default, Clone, Debug)]
pub struct HoveredData {
    pub x: f64,
    pub y: f64,
    pub column: String,
    pub view_id: Option<SpaceViewId>,
    pub point_index: Option<usize>,
}

/// Time control state
#[derive(Clone, Debug)]
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
#[derive(Default, Clone, Debug)]
pub struct FrameTime {
    pub avg_frame_ms: f64,
    pub max_frame_ms: f64,
}

/// Context passed to views during rendering
#[derive(Clone)]
pub struct ViewerContext {
    /// Map of data sources by their unique ID (filename)
    pub data_sources: Arc<RwLock<HashMap<String, Box<dyn crate::DataSource>>>>,
    
    /// Navigation engine
    pub navigation: Arc<crate::navigation::NavigationEngine>,
    
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