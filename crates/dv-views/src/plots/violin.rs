//! Violin plot implementation - shows distribution shape along with quartiles

use egui::{Ui, Color32};
use egui_plot::{Plot, PlotPoints, Line, Polygon, Points, Legend};
use arrow::array::{Float64Array, Int64Array, Array};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;
use super::utils::stats::calculate_quartiles;

/// Configuration for violin plot view
#[derive(Clone)]
pub struct ViolinPlotConfig {
    /// Category column (X-axis)
    pub category_column: Option<String>,
    
    /// Value column (Y-axis)
    pub value_column: String,
    
    /// Width of violins
    pub violin_width: f32,
    
    /// Whether to show box plot inside
    pub show_box: bool,
    
    /// Whether to show individual points
    pub show_points: bool,
    
    /// Point jitter amount
    pub jitter: f32,
    
    /// Whether to show mean
    pub show_mean: bool,
    
    /// Whether to show legend
    pub show_legend: bool,
    
    /// Whether to show grid
    pub show_grid: bool,
    
    /// KDE bandwidth (auto if None)
    pub bandwidth: Option<f32>,
}

impl Default for ViolinPlotConfig {
    fn default() -> Self {
        Self {
            category_column: None,
            value_column: String::new(),
            violin_width: 0.8,
            show_box: true,
            show_points: false,
            jitter: 0.1,
            show_mean: true,
            show_legend: true,
            show_grid: true,
            bandwidth: None,
        }
    }
}

/// Violin plot view
pub struct ViolinPlotView {
    id: SpaceViewId,
    title: String,
    pub config: ViolinPlotConfig,
    
    // State
    cached_data: Option<ViolinData>,
    last_navigation_pos: Option<NavigationPosition>,
}

/// Cached violin plot data
struct ViolinData {
    categories: Vec<String>,
    violins: Vec<ViolinStats>,
}

struct ViolinStats {
    values: Vec<f64>,
    kde_points: Vec<(f64, f64)>, // (value, density)
    quartiles: (f64, f64, f64, f64, f64), // min, q1, median, q3, max
    mean: f64,
}

impl ViolinPlotView {
    /// Create a new violin plot view
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: ViolinPlotConfig::default(),
            cached_data: None,
            last_navigation_pos: None,
        }
    }
    
    /// Fetch violin plot data
    fn fetch_data(&mut self, ctx: &ViewerContext) -> Option<ViolinData> {
        let data_source = ctx.data_source.read();
        let data_source = data_source.as_ref()?;
        
        // Get navigation context
        let nav_context = ctx.navigation.get_context();
        
        // Fetch all data
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
            let stats = self.calculate_violin_stats(&values)?;
            return Some(ViolinData {
                categories: vec!["All Data".to_string()],
                violins: vec![stats],
            });
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
        let mut sorted_categories: Vec<(String, ViolinStats)> = category_values
            .into_iter()
            .filter_map(|(cat, vals)| {
                self.calculate_violin_stats(&vals).map(|stats| (cat, stats))
            })
            .collect();
        
        sorted_categories.sort_by(|a, b| a.0.cmp(&b.0));
        
        Some(ViolinData {
            categories: sorted_categories.iter().map(|(c, _)| c.clone()).collect(),
            violins: sorted_categories.into_iter().map(|(_, s)| s).collect(),
        })
    }
    
    fn calculate_violin_stats(&self, values: &[f64]) -> Option<ViolinStats> {
        if values.is_empty() {
            return None;
        }
        
        let mut sorted_values = values.to_vec();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let (q1, median, q3) = calculate_quartiles(&sorted_values);
        let quartiles = (sorted_values[0], q1, median, q3, sorted_values[sorted_values.len() - 1]);
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        
        // Calculate KDE points
        let kde_points = self.calculate_kde(&sorted_values);
        
        Some(ViolinStats {
            values: values.to_vec(),
            kde_points,
            quartiles,
            mean,
        })
    }
    
    fn calculate_kde(&self, values: &[f64]) -> Vec<(f64, f64)> {
        if values.is_empty() {
            return Vec::new();
        }
        
        let min = values[0];
        let max = values[values.len() - 1];
        
        // Calculate bandwidth using Scott's rule if not specified
        let std_dev = values.iter()
            .map(|&v| v - values.iter().sum::<f64>() / values.len() as f64)
            .map(|d| d * d)
            .sum::<f64>()
            .sqrt() / (values.len() as f64 - 1.0).sqrt();
        
        let bandwidth = self.config.bandwidth.map(|bw| bw as f64).unwrap_or_else(|| {
            1.06 * std_dev * (values.len() as f64).powf(-0.2)
        });
        
        // Generate KDE points
        let num_points = 50;
        let mut kde_points = Vec::new();
        
        for i in 0..=num_points {
            let x = min + (max - min) * i as f64 / num_points as f64;
            let mut density = 0.0;
            
            // Gaussian kernel
            for &value in values {
                let u = (x - value) / bandwidth;
                density += (-0.5 * u * u).exp() / (2.5066282746310002 * bandwidth);
            }
            
            density /= values.len() as f64;
            kde_points.push((x, density));
        }
        
        kde_points
    }
    
    fn draw_violin(&self, plot_ui: &mut egui_plot::PlotUi, x: f64, stats: &ViolinStats, color: Color32) {
        // Find max density for scaling
        let max_density = stats.kde_points.iter()
            .map(|(_, d)| *d)
            .fold(0.0, f64::max);
        
        if max_density == 0.0 {
            return;
        }
        
        let half_width = (self.config.violin_width / 2.0) as f64;
        
        // Create violin shape (both sides)
        let mut violin_points = Vec::new();
        
        // Right side
        for &(y, density) in &stats.kde_points {
            let width = (density / max_density) * half_width;
            violin_points.push([x + width, y]);
        }
        
        // Left side (reverse order)
        for &(y, density) in stats.kde_points.iter().rev() {
            let width = (density / max_density) * half_width;
            violin_points.push([x - width, y]);
        }
        
        // Draw violin
        plot_ui.polygon(
            Polygon::new(PlotPoints::new(violin_points.clone()))
                .fill_color(color.linear_multiply(0.3))
                .stroke(egui::Stroke::new(2.0, color))
        );
        
        // Draw box plot inside if enabled
        if self.config.show_box {
            let box_width = half_width * 0.3;
            let (min, q1, median, q3, max) = stats.quartiles;
            
            // Box
            let box_points = vec![
                [x - box_width, q1],
                [x + box_width, q1],
                [x + box_width, q3],
                [x - box_width, q3],
            ];
            
            plot_ui.polygon(
                Polygon::new(PlotPoints::new(box_points))
                    .fill_color(color.linear_multiply(0.5))
                    .stroke(egui::Stroke::new(1.5, color))
            );
            
            // Median line
            plot_ui.line(
                Line::new(vec![[x - box_width, median], [x + box_width, median]])
                    .color(Color32::WHITE)
                    .width(2.0)
            );
            
            // Whiskers
            plot_ui.line(
                Line::new(vec![[x, q3], [x, max]])
                    .color(color)
                    .width(1.0)
            );
            plot_ui.line(
                Line::new(vec![[x, q1], [x, min]])
                    .color(color)
                    .width(1.0)
            );
        }
        
        // Draw mean if enabled
        if self.config.show_mean {
            plot_ui.points(
                Points::new(vec![[x, stats.mean]])
                    .color(Color32::WHITE)
                    .radius(4.0)
                    .shape(egui_plot::MarkerShape::Diamond)
            );
        }
        
        // Draw individual points if enabled
        if self.config.show_points {
            use rand::prelude::*;
            let mut rng = thread_rng();
            
            let points: Vec<[f64; 2]> = stats.values.iter()
                .map(|&y| {
                    let jitter = (rng.gen::<f64>() - 0.5) * self.config.jitter as f64 * half_width;
                    [x + jitter, y]
                })
                .collect();
            
            plot_ui.points(
                Points::new(points)
                    .color(color.linear_multiply(0.7))
                    .radius(2.0)
                    .shape(egui_plot::MarkerShape::Circle)
            );
        }
    }
}

impl SpaceView for ViolinPlotView {
    fn id(&self) -> &SpaceViewId {
        &self.id
    }
    
    fn display_name(&self) -> &str {
        &self.title
    }
    
    fn view_type(&self) -> &str {
        "ViolinPlotView"
    }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Update data if navigation changed
        let nav_pos = ctx.navigation.get_context().position.clone();
        if self.last_navigation_pos.as_ref() != Some(&nav_pos) {
            self.cached_data = self.fetch_data(ctx);
            self.last_navigation_pos = Some(nav_pos);
        }
        
        // Draw the violin plot
        if let Some(data) = &self.cached_data {
            let plot = Plot::new(format!("{:?}", self.id))
                .legend(Legend::default())
                .show_grid(self.config.show_grid)
                .allow_zoom(true)
                .allow_drag(true)
                .allow_boxed_zoom(true);
            
            plot.show(ui, |plot_ui| {
                for (i, (cat, stats)) in data.categories.iter().zip(&data.violins).enumerate() {
                    let x = i as f64;
                    let color = super::utils::colors::categorical_color(i);
                    
                    self.draw_violin(plot_ui, x, stats, color);
                    
                    // Add to legend
                    plot_ui.points(
                        Points::new(vec![[x, stats.quartiles.2]]) // median
                            .color(color)
                            .radius(0.0) // Hidden point for legend
                            .name(cat)
                    );
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
                ui.label(egui::RichText::new("Configure value column to see violin plot").weak());
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "category_column": self.config.category_column,
            "value_column": self.config.value_column,
            "violin_width": self.config.violin_width,
            "show_box": self.config.show_box,
            "show_points": self.config.show_points,
            "jitter": self.config.jitter,
            "show_mean": self.config.show_mean,
            "show_legend": self.config.show_legend,
            "show_grid": self.config.show_grid,
            "bandwidth": self.config.bandwidth,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(cat_col) = config.get("category_column").and_then(|v| v.as_str()) {
            self.config.category_column = Some(cat_col.to_string());
        }
        if let Some(val_col) = config.get("value_column").and_then(|v| v.as_str()) {
            self.config.value_column = val_col.to_string();
        }
        if let Some(width) = config.get("violin_width").and_then(|v| v.as_f64()) {
            self.config.violin_width = width as f32;
        }
        if let Some(show) = config.get("show_box").and_then(|v| v.as_bool()) {
            self.config.show_box = show;
        }
        if let Some(show) = config.get("show_points").and_then(|v| v.as_bool()) {
            self.config.show_points = show;
        }
        if let Some(jitter) = config.get("jitter").and_then(|v| v.as_f64()) {
            self.config.jitter = jitter as f32;
        }
        if let Some(show) = config.get("show_mean").and_then(|v| v.as_bool()) {
            self.config.show_mean = show;
        }
        if let Some(show) = config.get("show_legend").and_then(|v| v.as_bool()) {
            self.config.show_legend = show;
        }
        if let Some(show) = config.get("show_grid").and_then(|v| v.as_bool()) {
            self.config.show_grid = show;
        }
        if let Some(bw) = config.get("bandwidth").and_then(|v| v.as_f64()) {
            self.config.bandwidth = Some(bw as f32);
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {
        // TODO: Highlight selected violins
    }
    
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {
        // Nothing to update per frame
    }
} 