//! Plot view implementations
// TODO: Update to use SpaceView trait

// Basic 2D plots
pub mod scatter;
pub mod line;
pub mod bar;
pub mod histogram;
pub mod box_plot;
pub mod heatmap;
pub mod violin;

// Statistical plots
pub mod anomaly;
pub mod correlation;
pub mod distribution;

// 3D plots
pub mod scatter3d;
pub mod surface3d;
pub mod contour;

// Multi-dimensional plots
pub mod parallel_coordinates;
pub mod radar;

// Hierarchical and flow plots
pub mod sankey;
pub mod treemap;
pub mod sunburst;
pub mod network;

// Geographic plots
pub mod geo;

// Time series analysis
pub mod time_analysis;

// Financial plots
pub mod candlestick;

// Utility plots
pub mod stream;

// Utilities
pub mod utils;

// Re-exports
pub use scatter::{ScatterPlotView, ScatterPlotConfig};
pub use line::{LinePlotView, LinePlotConfig};
pub use bar::{BarChartView, BarChartConfig};
pub use histogram::{HistogramView, HistogramConfig};
pub use box_plot::{BoxPlotView, BoxPlotConfig};
pub use heatmap::{HeatmapView, HeatmapConfig};
pub use violin::{ViolinPlotView, ViolinPlotConfig};
pub use anomaly::{AnomalyDetectionView, AnomalyDetectionConfig};
pub use correlation::{CorrelationMatrixView, CorrelationMatrixConfig};
pub use distribution::{DistributionPlot, DistributionConfig};
pub use scatter3d::{Scatter3DPlot as Scatter3DView, Scatter3DConfig};
pub use surface3d::{Surface3DView, Surface3DConfig};
pub use contour::{ContourPlot, ContourConfig};
pub use parallel_coordinates::{ParallelCoordinatesPlot as ParallelCoordinatesView, ParallelCoordinatesConfig};
pub use radar::{RadarChart, RadarChartConfig};
pub use sankey::{SankeyDiagram, SankeyConfig as SankeyDiagramConfig};
pub use treemap::{TreemapView, TreemapConfig};
pub use sunburst::{SunburstChart, SunburstConfig as SunburstChartConfig};
pub use network::{NetworkGraph, NetworkGraphConfig};
pub use geo::{GeoPlot, GeoPlotConfig};
pub use time_analysis::{TimeAnalysisPlot, TimeAnalysisConfig};
pub use candlestick::{CandlestickChart, CandlestickConfig};
pub use stream::{StreamGraph, StreamGraphConfig}; 