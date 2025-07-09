//! View components for the data visualization application

pub mod plots;
pub mod space_view;
pub mod stats;
pub mod tables;
pub mod time_series_view;
pub mod viewport;
pub mod export;

mod polar_view;

// Re-export all components
pub use space_view::{SpaceView, SpaceViewId, SpaceViewConfig, SelectionState};
pub use viewport::{Viewport, GridLayoutConfig, GridCell};
pub use time_series_view::{TimeSeriesView, TimeSeriesConfig};
pub use tables::{TableView, TableConfig};
pub use stats::SummaryStatsView;
pub use polar_view::PolarPlotView;

// Re-export from dv_core
pub use dv_core::{ViewerContext, TimeControl, HoveredData, FrameTime, NavigationEngine}; 