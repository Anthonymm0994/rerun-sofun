//! Box plot implementation for statistical data visualization

use egui::{Ui, Color32};
use egui_plot::{Plot, PlotPoints, Points, Line, Legend, Polygon};
use arrow::array::{Float64Array, Int64Array, Array};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;
use super::utils::stats::calculate_quartiles;

/// Configuration for box plot
#[derive(Debug, Clone)]
pub struct BoxPlotConfig {
    /// Data source ID
    pub data_source_id: Option<String>,
    
    /// Value column to plot
    pub value_column: String,
    
    /// Optional grouping column for multiple box plots
    pub category_column: Option<String>,
    
    /// Whether to show outliers
    pub show_outliers: bool,
    
    /// Whether to show mean marker
    pub show_mean: bool,
    
    /// Box width
    pub box_width: f32,
    
    /// Whether to show legend
    pub show_legend: bool,
    
    /// Whether to show grid
    pub show_grid: bool,
    
    /// Orientation (vertical or horizontal)
    pub vertical: bool,
}

impl Default for BoxPlotConfig {
    fn default() -> Self {
        Self {
            data_source_id: None,
            value_column: String::new(),
            category_column: None,
            show_outliers: true,
            show_mean: true,
            box_width: 0.5,
            show_legend: true,
            show_grid: true,
            vertical: true,
        }
    }
}

/// Box plot view
pub struct BoxPlotView {
    id: SpaceViewId,
    title: String,
    pub config: BoxPlotConfig,
    
    // State
    cached_data: Option<BoxPlotData>,
    last_navigation_pos: Option<NavigationPosition>,
}

/// Cached box plot data
struct BoxPlotData {
    categories: Vec<String>,
    box_stats: Vec<BoxStats>,
}

#[derive(Debug, Clone)]
struct BoxStats {
    min: f64,
    q1: f64,
    median: f64,
    q3: f64,
    max: f64,
    mean: f64,
    outliers: Vec<f64>,
}

impl BoxPlotView {
    /// Create a new box plot view
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: BoxPlotConfig::default(),
            cached_data: None,
            last_navigation_pos: None,
        }
    }
    
    /// Fetch box plot data
    fn fetch_data(&mut self, ctx: &ViewerContext) -> Option<Vec<BoxPlotData>> {
        let data_sources = ctx.data_sources.read();
        
        // Get the specific data source for this view or fallback to first available
        let data_source = if let Some(source_id) = &self.config.data_source_id {
            data_sources.get(source_id)
        } else {
            (if let Some(source_id) = &self.config.data_source_id {
        data_sources.get(source_id)
    } else {
        data_sources.values().next()
    })
        }?;
        
        // Get navigation context
        let nav_context = ctx.navigation.get_context();
        
        // Fetch all data for statistics
        let range = dv_core::navigation::NavigationRange {
            start: dv_core::navigation::NavigationPosition::Sequential(0),
            end: dv_core::navigation::NavigationPosition::Sequential(nav_context.total_rows),
        };
        
        let data = ctx.runtime_handle.block_on(data_source.query_range(&range)).ok()?;
        
        // Extract value column
        let val_column = data.column_by_name(&self.config.value_column)?;
        let values: Vec<f64> = if let Some(float_array) = val_column.as_any().downcast_ref::<Float64Array>() {
            (0..float_array.len()).filter_map(|i| {
                if float_array.is_null(i) { None } else { Some(float_array.value(i)) }
            }).collect()
        } else if let Some(int_array) = val_column.as_any().downcast_ref::<Int64Array>() {
            (0..int_array.len()).filter_map(|i| {
                if int_array.is_null(i) { None } else { Some(int_array.value(i) as f64) }
            }).collect()
        } else {
            return None;
        };
        
        // If no category column, treat all data as one category
        if self.config.category_column.is_none() {
            let stats = self.calculate_box_stats(&values)?;
            return Some(vec![BoxPlotData {
                categories: vec!["All Data".to_string()],
                box_stats: vec![stats],
            }]);
        }
        
        // Group by category
        let cat_column = data.column_by_name(self.config.category_column.as_ref()?)?;
        let categories: Vec<String> = (0..cat_column.len())
            .map(|i| arrow::util::display::array_value_to_string(cat_column, i).unwrap_or_default())
            .collect();
        
        // Group values by category
        let mut category_values: HashMap<String, Vec<f64>> = HashMap::new();
        for (cat, val) in categories.iter().zip(values.iter()) {
            category_values.entry(cat.clone()).or_insert_with(Vec::new).push(*val);
        }
        
        // Calculate statistics for each category
        let mut sorted_categories: Vec<(String, BoxStats)> = category_values
            .into_iter()
            .filter_map(|(cat, vals)| {
                self.calculate_box_stats(&vals).map(|stats| (cat, stats))
            })
            .collect();
        
        sorted_categories.sort_by(|a, b| a.0.cmp(&b.0));
        
        Some(vec![BoxPlotData {
            categories: sorted_categories.iter().map(|(c, _)| c.clone()).collect(),
            box_stats: sorted_categories.into_iter().map(|(_, s)| s).collect(),
        }])
    }
    
    fn calculate_box_stats(&self, values: &[f64]) -> Option<BoxStats> {
        if values.is_empty() {
            return None;
        }
        
        let mut sorted_values = values.to_vec();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let (q1, median, q3) = calculate_quartiles(&sorted_values);
        let min = sorted_values[0];
        let max = sorted_values[sorted_values.len() - 1];
        
        // Calculate mean
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        
        // Find outliers using IQR method
        let iqr = q3 - q1;
        let lower_fence = q1 - 1.5 * iqr;
        let upper_fence = q3 + 1.5 * iqr;
        
        let outliers: Vec<f64> = values.iter()
            .filter(|&&v| v < lower_fence || v > upper_fence)
            .copied()
            .collect();
        
        // Adjust min/max to whisker ends (non-outlier extremes)
        let whisker_min = values.iter()
            .filter(|&&v| v >= lower_fence)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(min);
            
        let whisker_max = values.iter()
            .filter(|&&v| v <= upper_fence)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(max);
        
        Some(BoxStats {
            min: whisker_min,
            q1,
            median,
            q3,
            max: whisker_max,
            mean,
            outliers,
        })
    }
    
    fn draw_box(&self, plot_ui: &mut egui_plot::PlotUi, x: f64, stats: &BoxStats, color: Color32) {
        let half_width = (self.config.box_width / 2.0) as f64;
        
        if self.config.vertical {
            // Draw box (Q1 to Q3)
            let box_points = vec![
                [x - half_width, stats.q1],
                [x + half_width, stats.q1],
                [x + half_width, stats.q3],
                [x - half_width, stats.q3],
            ];
            plot_ui.polygon(
                Polygon::new(PlotPoints::new(box_points))
                    .fill_color(color.linear_multiply(0.3))
                    .stroke(egui::Stroke::new(2.0, color))
            );
            
            // Draw median line
            plot_ui.line(
                Line::new(vec![[x - half_width, stats.median], [x + half_width, stats.median]])
                    .color(color)
                    .width(3.0)
            );
            
            // Draw whiskers
            plot_ui.line(
                Line::new(vec![[x, stats.q3], [x, stats.max]])
                    .color(color)
                    .width(1.5)
            );
            plot_ui.line(
                Line::new(vec![[x, stats.q1], [x, stats.min]])
                    .color(color)
                    .width(1.5)
            );
            
            // Draw whisker caps
            let cap_width = half_width * 0.5;
            plot_ui.line(
                Line::new(vec![[x - cap_width, stats.max], [x + cap_width, stats.max]])
                    .color(color)
                    .width(1.5)
            );
            plot_ui.line(
                Line::new(vec![[x - cap_width, stats.min], [x + cap_width, stats.min]])
                    .color(color)
                    .width(1.5)
            );
            
            // Draw mean if enabled
            if self.config.show_mean {
                plot_ui.points(
                    Points::new(vec![[x, stats.mean]])
                        .color(color)
                        .radius(4.0)
                        .shape(egui_plot::MarkerShape::Diamond)
                );
            }
            
            // Draw outliers if enabled
            if self.config.show_outliers && !stats.outliers.is_empty() {
                let outlier_points: Vec<[f64; 2]> = stats.outliers.iter()
                    .map(|&y| [x, y])
                    .collect();
                plot_ui.points(
                    Points::new(outlier_points)
                        .color(color.linear_multiply(0.7))
                        .radius(3.0)
                        .shape(egui_plot::MarkerShape::Circle)
                );
            }
        }
    }
}

impl SpaceView for BoxPlotView {
    fn id(&self) -> SpaceViewId {
        self.id
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn display_name(&self) -> &str {
        &self.title
    }
    
    fn view_type(&self) -> &str {
        "BoxPlotView"
    }
    
    fn set_data_source(&mut self, source_id: String) {
        self.config.data_source_id = Some(source_id);
        self.cached_data = None;
    }
    
    fn data_source_id(&self) -> Option<&str> {
        self.config.data_source_id.as_deref()
    }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        
        // Update data if navigation changed
        let nav_pos = ctx.navigation.get_context().position.clone();
        if self.last_navigation_pos.as_ref() != Some(&nav_pos) {
            self.cached_data = self.fetch_data(ctx).and_then(|v| v.into_iter().next());
            self.last_navigation_pos = Some(nav_pos);
        }
        
        // Draw the box plot
        if let Some(data) = &self.cached_data {
            let plot = Plot::new(format!("{:?}", self.id))
                .legend(Legend::default())
                .show_grid(self.config.show_grid)
                .allow_zoom(true)
                .allow_drag(true)
                .allow_boxed_zoom(true);
            
            plot.show(ui, |plot_ui| {
                for (i, (cat, stats)) in data.categories.iter().zip(&data.box_stats).enumerate() {
                    let color = super::utils::colors::categorical_color(i);
                    
                    self.draw_box(plot_ui, i as f64, stats, color);
                    
                    // Add to legend
                    plot_ui.points(
                        Points::new(vec![[i as f64, stats.median]])
                            .color(color)
                            .radius(0.0) // Hidden point for legend
                            .name(cat)
                    );
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
                ui.label(egui::RichText::new("Configure value column to see box plot").weak());
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "data_source_id": self.config.data_source_id,
            "value_column": self.config.value_column,
            "category_column": self.config.category_column,
            "show_outliers": self.config.show_outliers,
            "show_mean": self.config.show_mean,
            "box_width": self.config.box_width,
            "show_legend": self.config.show_legend,
            "show_grid": self.config.show_grid,
            "vertical": self.config.vertical,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(data_source_id) = config.get("data_source_id").and_then(|v| v.as_str()) {
            self.config.data_source_id = Some(data_source_id.to_string());
        }
        if let Some(val_col) = config.get("value_column").and_then(|v| v.as_str()) {
            self.config.value_column = val_col.to_string();
        }
        if let Some(cat_col) = config.get("category_column").and_then(|v| v.as_str()) {
            self.config.category_column = Some(cat_col.to_string());
        }
        if let Some(show_outliers) = config.get("show_outliers").and_then(|v| v.as_bool()) {
            self.config.show_outliers = show_outliers;
        }
        if let Some(show_mean) = config.get("show_mean").and_then(|v| v.as_bool()) {
            self.config.show_mean = show_mean;
        }
        if let Some(box_width) = config.get("box_width").and_then(|v| v.as_f64()) {
            self.config.box_width = box_width as f32;
        }
        if let Some(show_legend) = config.get("show_legend").and_then(|v| v.as_bool()) {
            self.config.show_legend = show_legend;
        }
        if let Some(show_grid) = config.get("show_grid").and_then(|v| v.as_bool()) {
            self.config.show_grid = show_grid;
        }
        if let Some(vertical) = config.get("vertical").and_then(|v| v.as_bool()) {
            self.config.vertical = vertical;
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {
        // TODO: Highlight selected boxes
    }
    
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {
        // Nothing to update per frame
    }
} 