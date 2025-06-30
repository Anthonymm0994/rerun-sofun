//! Statistics view implementation
// TODO: Update to use SpaceView trait

/*
use egui::{Ui, Grid};
use arrow::record_batch::RecordBatch;
use arrow::array::{Float64Array, Int64Array, ArrayRef};
use arrow::compute::{min, max, sum};
use serde::{Serialize, Deserialize};
use dv_core::navigation::NavigationContext;
use crate::{View, ViewConfig};

/// Configuration for statistics views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsConfig {
    pub show_mean: bool,
    pub show_median: bool,
    pub show_std_dev: bool,
    pub show_min_max: bool,
    pub show_count: bool,
    pub show_null_count: bool,
    pub show_unique_count: bool,
}

impl Default for StatsConfig {
    fn default() -> Self {
        Self {
            show_mean: true,
            show_median: true,
            show_std_dev: true,
            show_min_max: true,
            show_count: true,
            show_null_count: true,
            show_unique_count: false,
        }
    }
}

/// Statistics view that displays summary statistics for columns
pub struct StatsView {
    id: String,
    name: String,
    config: StatsConfig,
    stats_cache: Vec<ColumnStats>,
}

#[derive(Clone, Debug)]
struct ColumnStats {
    name: String,
    count: usize,
    null_count: usize,
    unique_count: Option<usize>,
    mean: Option<f64>,
    median: Option<f64>,
    std_dev: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
}

impl StatsView {
    /// Create a new statistics view
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            config: StatsConfig::default(),
            stats_cache: Vec::new(),
        }
    }
    
    /// Calculate statistics for a numeric array
    fn calculate_numeric_stats(array: &ArrayRef) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
        if let Some(float_array) = array.as_any().downcast_ref::<Float64Array>() {
            let values: Vec<f64> = float_array.iter().filter_map(|v| v).collect();
            
            if values.is_empty() {
                return (None, None, None, None, None);
            }
            
            let count = values.len() as f64;
            let mean = values.iter().sum::<f64>() / count;
            
            let variance = values.iter()
                .map(|v| (v - mean).powi(2))
                .sum::<f64>() / count;
            let std_dev = variance.sqrt();
            
            let mut sorted = values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            
            let median = if sorted.len() % 2 == 0 {
                (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
            } else {
                sorted[sorted.len() / 2]
            };
            
            let min_val = sorted.first().cloned();
            let max_val = sorted.last().cloned();
            
            (Some(mean), Some(median), Some(std_dev), min_val, max_val)
        } else if let Some(int_array) = array.as_any().downcast_ref::<Int64Array>() {
            let values: Vec<f64> = int_array.iter().filter_map(|v| v.map(|i| i as f64)).collect();
            
            if values.is_empty() {
                return (None, None, None, None, None);
            }
            
            let count = values.len() as f64;
            let mean = values.iter().sum::<f64>() / count;
            
            let variance = values.iter()
                .map(|v| (v - mean).powi(2))
                .sum::<f64>() / count;
            let std_dev = variance.sqrt();
            
            let mut sorted = values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            
            let median = if sorted.len() % 2 == 0 {
                (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
            } else {
                sorted[sorted.len() / 2]
            };
            
            let min_val = sorted.first().cloned();
            let max_val = sorted.last().cloned();
            
            (Some(mean), Some(median), Some(std_dev), min_val, max_val)
        } else {
            (None, None, None, None, None)
        }
    }
}

impl View for StatsView {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
    
    fn update(&mut self, data: &RecordBatch, _context: &NavigationContext) {
        self.stats_cache.clear();
        
        for (idx, field) in data.schema().fields().iter().enumerate() {
            let column = data.column(idx);
            let null_count = column.null_count();
            
            let (mean, median, std_dev, min_val, max_val) = Self::calculate_numeric_stats(column);
            
            self.stats_cache.push(ColumnStats {
                name: field.name().clone(),
                count: column.len(),
                null_count,
                unique_count: None, // TODO: Calculate unique count
                mean,
                median,
                std_dev,
                min: min_val,
                max: max_val,
            });
        }
    }
    
    fn render(&mut self, ui: &mut Ui) {
        if self.stats_cache.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No statistics to display");
            });
            return;
        }
        
        egui::ScrollArea::vertical()
            .id_source(&self.id)
            .show(ui, |ui| {
                for stats in &self.stats_cache {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            ui.heading(&stats.name);
                            
                            Grid::new(format!("{}_stats", stats.name))
                                .num_columns(2)
                                .spacing([40.0, 4.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    if self.config.show_count {
                                        ui.label("Count:");
                                        ui.label(stats.count.to_string());
                                        ui.end_row();
                                    }
                                    
                                    if self.config.show_null_count {
                                        ui.label("Null Count:");
                                        ui.label(stats.null_count.to_string());
                                        ui.end_row();
                                    }
                                    
                                    if self.config.show_mean {
                                        ui.label("Mean:");
                                        ui.label(stats.mean.map_or("N/A".to_string(), |v| format!("{:.2}", v)));
                                        ui.end_row();
                                    }
                                    
                                    if self.config.show_median {
                                        ui.label("Median:");
                                        ui.label(stats.median.map_or("N/A".to_string(), |v| format!("{:.2}", v)));
                                        ui.end_row();
                                    }
                                    
                                    if self.config.show_std_dev {
                                        ui.label("Std Dev:");
                                        ui.label(stats.std_dev.map_or("N/A".to_string(), |v| format!("{:.2}", v)));
                                        ui.end_row();
                                    }
                                    
                                    if self.config.show_min_max {
                                        ui.label("Min:");
                                        ui.label(stats.min.map_or("N/A".to_string(), |v| format!("{:.2}", v)));
                                        ui.end_row();
                                        
                                        ui.label("Max:");
                                        ui.label(stats.max.map_or("N/A".to_string(), |v| format!("{:.2}", v)));
                                        ui.end_row();
                                    }
                                });
                        });
                    });
                    
                    ui.add_space(8.0);
                }
            });
    }
    
    fn config(&self) -> ViewConfig {
        ViewConfig::Stats(self.config.clone())
    }
    
    fn set_config(&mut self, config: ViewConfig) {
        if let ViewConfig::Stats(stats_config) = config {
            self.config = stats_config;
        }
    }
    
    fn on_zoom(&mut self, _factor: f32, _center: Option<egui::Pos2>) {
        // Stats don't zoom
    }
    
    fn on_selection(&mut self, _start: egui::Pos2, _end: egui::Pos2) {
        // Stats don't have selection
    }
}
*/ 