//! Distribution plot implementation

use egui::{Ui, Color32};
use egui_plot::{Plot, PlotPoints, Line, Points, Legend, BarChart, Bar};
use arrow::array::{Float64Array, Int64Array, Array};
use arrow::record_batch::RecordBatch;
use serde_json::{json, Value};

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;

/// Configuration for distribution plot
#[derive(Debug, Clone)]
pub struct DistributionConfig {
    pub data_source_id: Option<String>,
    pub column: String,
    pub plot_type: DistributionPlotType,
    pub bins: usize,
    pub show_kde: bool,
    pub show_rug: bool,
    pub show_mean: bool,
    pub show_median: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistributionPlotType {
    Histogram,
    KDE,
    Both,
}

impl Default for DistributionConfig {
    fn default() -> Self {
        Self {
            data_source_id: None,
            column: String::new(),
            plot_type: DistributionPlotType::Both,
            bins: 30,
            show_kde: true,
            show_rug: false,
            show_mean: true,
            show_median: true,
        }
    }
}

/// Cached distribution data
struct DistributionData {
    values: Vec<f64>,
    histogram_bins: Vec<(f64, f64, usize)>, // (start, end, count)
    kde_points: Vec<(f64, f64)>, // (x, density)
    mean: f64,
    median: f64,
    std_dev: f64,
}

/// Distribution plot view
pub struct DistributionPlot {
    id: SpaceViewId,
    title: String,
    pub config: DistributionConfig,
    cached_data: Option<DistributionData>,
    last_navigation_pos: Option<NavigationPosition>,
}

impl DistributionPlot {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: DistributionConfig::default(),
            cached_data: None,
            last_navigation_pos: None,
        }
    }
    
    fn fetch_data(&mut self, ctx: &ViewerContext) -> Option<DistributionData> {
        if self.config.column.is_empty() {
            return None;
        }
        
        let data_sources = ctx.data_sources.read();
        let data_source = if let Some(source_id) = &self.config.data_source_id {
            data_sources.get(source_id)
        } else {
            data_sources.values().next()
        }?;
        
        // Fetch data
        let nav_context = ctx.navigation.get_context();
        let range = dv_core::navigation::NavigationRange {
            start: dv_core::navigation::NavigationPosition::Sequential(0),
            end: dv_core::navigation::NavigationPosition::Sequential(nav_context.total_rows),
        };
        
        let batch = ctx.runtime_handle.block_on(
            data_source.query_range(&range)
        ).ok()?;
        
        // Extract column
        let column = batch.column_by_name(&self.config.column)?;
        
        // Extract numeric values with null handling
        let values: Vec<f64> = Self::extract_numeric_values(column);
        
        if values.is_empty() {
            return None;
        }
        
        // Calculate statistics
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let mut sorted_values = values.clone();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = sorted_values[sorted_values.len() / 2];
        let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();
        
        // Create histogram
        let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let bin_width = (max_val - min_val) / self.config.bins as f64;
        
        let mut histogram_bins = vec![(0.0, 0.0, 0); self.config.bins];
        for i in 0..self.config.bins {
            let start = min_val + i as f64 * bin_width;
            let end = start + bin_width;
            let count = values.iter().filter(|&&v| v >= start && v < end).count();
            histogram_bins[i] = (start, end, count);
        }
        
        // Simple KDE (kernel density estimation) - using Gaussian kernel
        let kde_points = if self.config.show_kde {
            let bandwidth = 1.06 * std_dev * (values.len() as f64).powf(-0.2);
            let kde_range = max_val - min_val;
            let kde_min = min_val - kde_range * 0.1;
            let kde_max = max_val + kde_range * 0.1;
            let kde_steps = 100;
            
            (0..kde_steps).map(|i| {
                let x = kde_min + (kde_max - kde_min) * i as f64 / (kde_steps - 1) as f64;
                let density: f64 = values.iter().map(|&v| {
                    let u = (x - v) / bandwidth;
                    (-0.5 * u * u).exp() / (bandwidth * (2.0 * std::f64::consts::PI).sqrt())
                }).sum::<f64>() / values.len() as f64;
                (x, density)
            }).collect()
        } else {
            Vec::new()
        };
        
        Some(DistributionData {
            values,
            histogram_bins,
            kde_points,
            mean,
            median,
            std_dev,
        })
    }
    
    fn extract_numeric_values(array: &dyn Array) -> Vec<f64> {
        if let Some(float_array) = array.as_any().downcast_ref::<Float64Array>() {
            (0..float_array.len()).filter_map(|i| {
                if float_array.is_null(i) { None } else { Some(float_array.value(i)) }
            }).collect()
        } else if let Some(int_array) = array.as_any().downcast_ref::<Int64Array>() {
            (0..int_array.len()).filter_map(|i| {
                if int_array.is_null(i) { None } else { Some(int_array.value(i) as f64) }
            }).collect()
        } else if let Some(int_array) = array.as_any().downcast_ref::<arrow::array::Int32Array>() {
            (0..int_array.len()).filter_map(|i| {
                if int_array.is_null(i) { None } else { Some(int_array.value(i) as f64) }
            }).collect()
        } else if let Some(float_array) = array.as_any().downcast_ref::<arrow::array::Float32Array>() {
            (0..float_array.len()).filter_map(|i| {
                if float_array.is_null(i) { None } else { Some(float_array.value(i) as f64) }
            }).collect()
        } else {
            Vec::new()
        }
    }
}

impl SpaceView for DistributionPlot {
    fn id(&self) -> SpaceViewId { self.id }
    fn title(&self) -> &str {
        &self.title
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn display_name(&self) -> &str { &self.title }
    fn view_type(&self) -> &str { "DistributionPlot" }
    
    fn set_data_source(&mut self, source_id: String) {
        self.config.data_source_id = Some(source_id);
        self.cached_data = None;
    }
    
    fn data_source_id(&self) -> Option<&str> {
        self.config.data_source_id.as_deref()
    }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Update data if needed
        let nav_pos = ctx.navigation.get_context().position.clone();
        if self.cached_data.is_none() || self.last_navigation_pos.as_ref() != Some(&nav_pos) {
            self.cached_data = self.fetch_data(ctx);
            self.last_navigation_pos = Some(nav_pos);
        }
        
        if let Some(data) = &self.cached_data {
            Plot::new(format!("{:?}_distribution", self.id))
                .legend(Legend::default())
                .show_grid(true)
                .auto_bounds(egui::Vec2b::new(true, true))
                .show(ui, |plot_ui| {
                    // Draw histogram
                    if self.config.plot_type == DistributionPlotType::Histogram || 
                       self.config.plot_type == DistributionPlotType::Both {
                        let bars: Vec<Bar> = data.histogram_bins.iter().enumerate().map(|(i, &(start, end, count))| {
                            let center = (start + end) / 2.0;
                            let width = end - start;
                            let height = count as f64 / (data.values.len() as f64 * width);
                            Bar::new(center, height)
                                .width(width)
                                .fill(Color32::from_rgba_unmultiplied(92, 140, 97, 128))
                                .name(if i == 0 { "Histogram" } else { "" })
                        }).collect();
                        
                        plot_ui.bar_chart(BarChart::new(bars));
                    }
                    
                    // Draw KDE
                    if self.config.show_kde && !data.kde_points.is_empty() {
                        let kde_line = Line::new(PlotPoints::new(
                            data.kde_points.iter().map(|&(x, y)| [x, y]).collect()
                        ))
                        .color(Color32::from_rgb(31, 119, 180))
                        .width(2.0)
                        .name("KDE");
                        
                        plot_ui.line(kde_line);
                    }
                    
                    // Draw mean line
                    if self.config.show_mean {
                        let mean_line = Line::new(PlotPoints::new(vec![
                            [data.mean, 0.0],
                            [data.mean, data.kde_points.iter().map(|(_, y)| y).cloned().fold(0.0, f64::max)]
                        ]))
                        .color(Color32::RED)
                        .width(2.0)
                        .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                        .name(format!("Mean: {:.2}", data.mean));
                        
                        plot_ui.line(mean_line);
                    }
                    
                    // Draw median line
                    if self.config.show_median {
                        let median_line = Line::new(PlotPoints::new(vec![
                            [data.median, 0.0],
                            [data.median, data.kde_points.iter().map(|(_, y)| y).cloned().fold(0.0, f64::max)]
                        ]))
                        .color(Color32::GREEN)
                        .width(2.0)
                        .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                        .name(format!("Median: {:.2}", data.median));
                        
                        plot_ui.line(median_line);
                    }
                    
                    // Draw rug plot
                    if self.config.show_rug && data.values.len() < 1000 {
                        let rug_points: Vec<[f64; 2]> = data.values.iter()
                            .map(|&x| [x, 0.0])
                            .collect();
                        
                        let rug = Points::new(rug_points)
                            .color(Color32::from_rgba_unmultiplied(128, 128, 128, 64))
                            .radius(1.0)
                            .name("Data points");
                        
                        plot_ui.points(rug);
                    }
                });
                
            // Statistics panel
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(format!("N: {}", data.values.len()));
                ui.label(format!("Mean: {:.2}", data.mean));
                ui.label(format!("Median: {:.2}", data.median));
                ui.label(format!("Std Dev: {:.2}", data.std_dev));
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
                if self.config.column.is_empty() {
                    ui.label("Please configure a column");
                }
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "data_source_id": self.config.data_source_id,
            "column": self.config.column,
            "plot_type": match self.config.plot_type {
                DistributionPlotType::Histogram => "histogram",
                DistributionPlotType::KDE => "kde",
                DistributionPlotType::Both => "both",
            },
            "bins": self.config.bins,
            "show_kde": self.config.show_kde,
            "show_rug": self.config.show_rug,
            "show_mean": self.config.show_mean,
            "show_median": self.config.show_median,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(data_source_id) = config.get("data_source_id").and_then(|v| v.as_str()) {
            self.config.data_source_id = Some(data_source_id.to_string());
        }
        if let Some(column) = config.get("column").and_then(|v| v.as_str()) {
            self.config.column = column.to_string();
        }
        if let Some(plot_type) = config.get("plot_type").and_then(|v| v.as_str()) {
            self.config.plot_type = match plot_type {
                "histogram" => DistributionPlotType::Histogram,
                "kde" => DistributionPlotType::KDE,
                _ => DistributionPlotType::Both,
            };
        }
        if let Some(bins) = config.get("bins").and_then(|v| v.as_u64()) {
            self.config.bins = bins as usize;
        }
        if let Some(show_kde) = config.get("show_kde").and_then(|v| v.as_bool()) {
            self.config.show_kde = show_kde;
        }
        if let Some(show_rug) = config.get("show_rug").and_then(|v| v.as_bool()) {
            self.config.show_rug = show_rug;
        }
        if let Some(show_mean) = config.get("show_mean").and_then(|v| v.as_bool()) {
            self.config.show_mean = show_mean;
        }
        if let Some(show_median) = config.get("show_median").and_then(|v| v.as_bool()) {
            self.config.show_median = show_median;
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {}
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {}
} 