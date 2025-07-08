//! Histogram implementation

use egui::{Ui, Color32};
use egui_plot::{Plot, Bar, BarChart, Line, PlotPoints, Legend};
use arrow::array::{Float64Array, Int64Array, Array};
use serde_json::{json, Value};
use statrs::statistics::Statistics;

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;

/// Configuration for histogram view
#[derive(Clone)]
pub struct HistogramConfig {
    /// Column to create histogram from
    pub column: String,
    
    /// Number of bins
    pub num_bins: usize,
    
    /// Bin strategy
    pub bin_strategy: BinStrategy,
    
    /// Whether to show density curve
    pub show_density: bool,
    
    /// Whether to show normal distribution overlay
    pub show_normal: bool,
    
    /// Bar color
    pub bar_color: Color32,
    
    /// Whether to show grid
    pub show_grid: bool,
    
    /// Whether to show statistics
    pub show_stats: bool,
    
    /// Whether to use log scale on Y axis
    pub log_y: bool,
    
    /// Data source ID
    pub data_source_id: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum BinStrategy {
    Fixed,      // Fixed number of bins
    Sturges,    // Sturges' rule
    Scott,      // Scott's rule
    FreedmanDiaconis, // Freedman-Diaconis rule
    SquareRoot, // Square root rule
}

impl Default for HistogramConfig {
    fn default() -> Self {
        Self {
            column: String::new(),
            num_bins: 30,
            bin_strategy: BinStrategy::Fixed,
            show_density: false,
            show_normal: false,
            bar_color: Color32::from_rgb(92, 140, 97),
            show_grid: true,
            show_stats: true,
            log_y: false,
            data_source_id: None,
        }
    }
}

/// Histogram view
pub struct HistogramView {
    id: SpaceViewId,
    title: String,
    pub config: HistogramConfig,
    
    // State
    cached_data: Option<HistogramData>,
    last_navigation_pos: Option<NavigationPosition>,
}

/// Cached histogram data
struct HistogramData {
    bins: Vec<Bin>,
    statistics: DataStatistics,
    density_curve: Option<Vec<(f64, f64)>>,
}

struct Bin {
    start: f64,
    end: f64,
    count: usize,
    density: f64,
}

struct DataStatistics {
    mean: f64,
    std_dev: f64,
    min: f64,
    max: f64,
    count: usize,
}

impl HistogramView {
    /// Create a new histogram view
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: HistogramConfig::default(),
            cached_data: None,
            last_navigation_pos: None,
        }
    }
    
    /// Fetch histogram data based on the current navigation context
    fn fetch_histogram_data(&self, ctx: &ViewerContext) -> Option<HistogramData> {
        if self.config.column.is_empty() {
            return None;
        }
        
        let data_sources = ctx.data_sources.read();
        
        // Get the specific data source for this view or fallback to first available
        let data_source = if let Some(source_id) = &self.config.data_source_id {
            data_sources.get(source_id)
        } else {
            data_sources.values().next()
        }?;
        
        // Query data
        let nav_pos = ctx.navigation.get_context().position.clone();
        let batch = ctx.runtime_handle.block_on(
            data_source.query_at(&nav_pos)
        ).ok()?;
        
        // Get the column
        let column = batch.column_by_name(&self.config.column)?;
        let values: Vec<f64> = if let Some(float_array) = column.as_any().downcast_ref::<Float64Array>() {
            (0..float_array.len()).filter_map(|i| {
                if float_array.is_null(i) { None } else { Some(float_array.value(i)) }
            }).collect()
        } else if let Some(int_array) = column.as_any().downcast_ref::<Int64Array>() {
            (0..int_array.len()).filter_map(|i| {
                if int_array.is_null(i) { None } else { Some(int_array.value(i) as f64) }
            }).collect()
        } else if let Some(int_array) = column.as_any().downcast_ref::<arrow::array::Int32Array>() {
            (0..int_array.len()).filter_map(|i| {
                if int_array.is_null(i) { None } else { Some(int_array.value(i) as f64) }
            }).collect()
        } else if let Some(float_array) = column.as_any().downcast_ref::<arrow::array::Float32Array>() {
            (0..float_array.len()).filter_map(|i| {
                if float_array.is_null(i) { None } else { Some(float_array.value(i) as f64) }
            }).collect()
        } else {
            return None;
        };
        
        if values.is_empty() {
            return None;
        }
        
        // Calculate statistics
        let stats = DataStatistics {
            mean: values.iter().sum::<f64>() / values.len() as f64,
            std_dev: {
                let mean = values.iter().sum::<f64>() / values.len() as f64;
                let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
                variance.sqrt()
            },
            min: values.iter().copied().fold(f64::INFINITY, f64::min),
            max: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            count: values.len(),
        };
        
        // Determine number of bins
        let num_bins = match self.config.bin_strategy {
            BinStrategy::Fixed => self.config.num_bins,
            BinStrategy::Sturges => (1.0 + (values.len() as f64).log2()).ceil() as usize,
            BinStrategy::Scott => {
                let h = 3.5 * stats.std_dev / (values.len() as f64).powf(1.0/3.0);
                ((stats.max - stats.min) / h).ceil() as usize
            }
            BinStrategy::FreedmanDiaconis => {
                // Calculate quartiles manually
                let mut sorted_values = values.clone();
                sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let n = sorted_values.len();
                let q1 = sorted_values[n / 4];
                let q3 = sorted_values[3 * n / 4];
                let iqr = q3 - q1;
                let h = 2.0 * iqr / (values.len() as f64).powf(1.0/3.0);
                ((stats.max - stats.min) / h).ceil() as usize
            }
            BinStrategy::SquareRoot => (values.len() as f64).sqrt().ceil() as usize,
        }.max(1);
        
        // Create bins
        let bin_width = (stats.max - stats.min) / num_bins as f64;
        let mut bins = Vec::new();
        
        for i in 0..num_bins {
            let start = stats.min + i as f64 * bin_width;
            let end = start + bin_width;
            
            let count = values.iter()
                .filter(|&&v| {
                    if i == num_bins - 1 {
                        v >= start && v <= end
                    } else {
                        v >= start && v < end
                    }
                })
                .count();
            
            let density = count as f64 / (values.len() as f64 * bin_width);
            
            bins.push(Bin { start, end, count, density });
        }
        
        // Calculate density curve if enabled
        let density_curve = if self.config.show_density {
            Some(self.calculate_kde(&values, &stats))
        } else {
            None
        };
        
        Some(HistogramData { bins, statistics: stats, density_curve })
    }
    
    fn calculate_kde(&self, values: &[f64], stats: &DataStatistics) -> Vec<(f64, f64)> {
        // Simple Gaussian kernel density estimation
        let bandwidth = 1.06 * stats.std_dev * (values.len() as f64).powf(-0.2);
        let num_points = 100;
        let mut curve = Vec::new();
        
        for i in 0..=num_points {
            let x = stats.min + (stats.max - stats.min) * i as f64 / num_points as f64;
            let mut density = 0.0;
            
            for &value in values {
                let u = (x - value) / bandwidth;
                density += (-0.5_f64 * u * u).exp() / (2.5066282746310002_f64 * bandwidth);
            }
            
            density /= values.len() as f64;
            curve.push((x, density));
        }
        
        curve
    }
}

impl SpaceView for HistogramView {
    fn id(&self) -> SpaceViewId {
        self.id
    }
    
    fn display_name(&self) -> &str {
        &self.title
    }
    
    fn view_type(&self) -> &str {
        "HistogramView"
    }
    
    fn title(&self) -> &str {
        &self.title
    }
    
    fn set_data_source(&mut self, source_id: String) {
        self.config.data_source_id = Some(source_id);
        self.cached_data = None;
    }
    
    fn data_source_id(&self) -> Option<&str> {
        self.config.data_source_id.as_deref()
    }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        
        // Update data if navigation changed or if we have no cached data
        let nav_pos = ctx.navigation.get_context().position.clone();
        if self.cached_data.is_none() || self.last_navigation_pos.as_ref() != Some(&nav_pos) {
            self.cached_data = self.fetch_histogram_data(ctx);
            self.last_navigation_pos = Some(nav_pos);
        }
        
        // Draw the histogram
        if let Some(data) = &self.cached_data {
            // Show statistics if enabled
            if self.config.show_stats {
                ui.horizontal(|ui| {
                    ui.label(format!("Count: {}", data.statistics.count));
                    ui.separator();
                    ui.label(format!("Mean: {:.2}", data.statistics.mean));
                    ui.separator();
                    ui.label(format!("Std Dev: {:.2}", data.statistics.std_dev));
                    ui.separator();
                    ui.label(format!("Min: {:.2}", data.statistics.min));
                    ui.separator();
                    ui.label(format!("Max: {:.2}", data.statistics.max));
                });
                ui.add_space(4.0);
            }
            
            let plot = Plot::new(format!("{:?}", self.id))
                .legend(Legend::default())
                .show_grid(self.config.show_grid)
                .allow_zoom(true)
                .allow_drag(true)
                .allow_boxed_zoom(true)
                .x_axis_label(&self.config.column)
                .y_axis_label(if self.config.show_density { "Density" } else { "Count" });
            
            plot.show(ui, |plot_ui| {
                // Draw histogram bars
                let mut bars = Vec::new();
                for bin in &data.bins {
                    let center = (bin.start + bin.end) / 2.0;
                    let height = if self.config.show_density { bin.density } else { bin.count as f64 };
                    let width = bin.end - bin.start;
                    
                    bars.push(
                        Bar::new(center, height)
                            .width(width)
                            .fill(self.config.bar_color.linear_multiply(0.7))
                    );
                }
                
                plot_ui.bar_chart(
                    BarChart::new(bars)
                        .color(self.config.bar_color)
                        .name("Histogram")
                );
                
                // Draw density curve if enabled
                if let Some(curve) = &data.density_curve {
                    let points: Vec<[f64; 2]> = curve.iter()
                        .map(|&(x, y)| [x, y])
                        .collect();
                    
                    plot_ui.line(
                        Line::new(PlotPoints::new(points))
                            .color(Color32::from_rgb(255, 100, 100))
                            .width(2.0)
                            .name("Density")
                    );
                }
                
                // Draw normal distribution overlay if enabled
                if self.config.show_normal {
                    let num_points = 100;
                    let mut normal_curve = Vec::new();
                    
                    for i in 0..=num_points {
                        let x = data.statistics.min + (data.statistics.max - data.statistics.min) * i as f64 / num_points as f64;
                        let z = (x - data.statistics.mean) / data.statistics.std_dev;
                        let y = (-0.5 * z * z).exp() / (data.statistics.std_dev * 2.5066282746310002);
                        normal_curve.push([x, y]);
                    }
                    
                    plot_ui.line(
                        Line::new(PlotPoints::new(normal_curve))
                            .color(Color32::from_rgb(100, 100, 255))
                            .width(2.0)
                            .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                            .name("Normal")
                    );
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
                ui.label(egui::RichText::new("Configure column to see histogram").weak());
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "column": self.config.column,
            "num_bins": self.config.num_bins,
            "bin_strategy": match self.config.bin_strategy {
                BinStrategy::Fixed => "fixed",
                BinStrategy::Sturges => "sturges",
                BinStrategy::Scott => "scott",
                BinStrategy::FreedmanDiaconis => "freedman_diaconis",
                BinStrategy::SquareRoot => "square_root",
            },
            "show_density": self.config.show_density,
            "show_normal": self.config.show_normal,
            "bar_color": [
                self.config.bar_color.r(),
                self.config.bar_color.g(),
                self.config.bar_color.b(),
            ],
            "show_grid": self.config.show_grid,
            "show_stats": self.config.show_stats,
            "log_y": self.config.log_y,
            "data_source_id": self.config.data_source_id,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(col) = config.get("column").and_then(|v| v.as_str()) {
            self.config.column = col.to_string();
        }
        if let Some(bins) = config.get("num_bins").and_then(|v| v.as_u64()) {
            self.config.num_bins = bins as usize;
        }
        if let Some(strategy) = config.get("bin_strategy").and_then(|v| v.as_str()) {
            self.config.bin_strategy = match strategy {
                "sturges" => BinStrategy::Sturges,
                "scott" => BinStrategy::Scott,
                "freedman_diaconis" => BinStrategy::FreedmanDiaconis,
                "square_root" => BinStrategy::SquareRoot,
                _ => BinStrategy::Fixed,
            };
        }
        if let Some(show_density) = config.get("show_density").and_then(|v| v.as_bool()) {
            self.config.show_density = show_density;
        }
        if let Some(show_normal) = config.get("show_normal").and_then(|v| v.as_bool()) {
            self.config.show_normal = show_normal;
        }
        if let Some(color) = config.get("bar_color").and_then(|v| v.as_array()) {
            if color.len() == 3 {
                if let (Some(r), Some(g), Some(b)) = (
                    color[0].as_u64(),
                    color[1].as_u64(),
                    color[2].as_u64()
                ) {
                    self.config.bar_color = Color32::from_rgb(r as u8, g as u8, b as u8);
                }
            }
        }
        if let Some(show_grid) = config.get("show_grid").and_then(|v| v.as_bool()) {
            self.config.show_grid = show_grid;
        }
        if let Some(show_stats) = config.get("show_stats").and_then(|v| v.as_bool()) {
            self.config.show_stats = show_stats;
        }
        if let Some(log_y) = config.get("log_y").and_then(|v| v.as_bool()) {
            self.config.log_y = log_y;
        }
        if let Some(data_source_id) = config.get("data_source_id").and_then(|v| v.as_str()) {
            self.config.data_source_id = Some(data_source_id.to_string());
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {
        // TODO: Highlight selected bins
    }
    
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {
        // Nothing to update per frame
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
} 