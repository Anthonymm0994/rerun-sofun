//! Stream graph visualization
//! Used for showing stacked area charts that flow around a central axis

use egui::{Ui, Color32};
use egui_plot::{Plot, PlotPoints, Polygon, Legend};
use arrow::array::{Float64Array, Int64Array, StringArray, Array};
use arrow::record_batch::RecordBatch;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;

/// Configuration for stream graph
#[derive(Debug, Clone)]
pub struct StreamConfig {
    pub data_source_id: Option<String>,
    pub x_column: String,         // Time or sequence column
    pub category_column: String,  // Category/series column
    pub value_column: String,     // Value column
    pub baseline: StreamBaseline,
    pub interpolation: String,
    pub color_scheme: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StreamBaseline {
    Zero,        // Stack from zero
    Wiggle,      // Minimize wiggle
    Centered,    // Center around middle
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            data_source_id: None,
            x_column: String::new(),
            category_column: String::new(),
            value_column: String::new(),
            baseline: StreamBaseline::Wiggle,
            interpolation: "smooth".to_string(),
            color_scheme: "categorical".to_string(),
        }
    }
}

/// Cached stream data
struct StreamData {
    series: Vec<StreamSeries>,
    x_values: Vec<f64>,
}

struct StreamSeries {
    name: String,
    values: Vec<f64>,
    lower_bounds: Vec<f64>,
    upper_bounds: Vec<f64>,
    color: Color32,
}

/// Stream graph view
pub struct StreamGraph {
    id: SpaceViewId,
    title: String,
    pub config: StreamConfig,
    cached_data: Option<StreamData>,
    last_navigation_pos: Option<NavigationPosition>,
}

impl StreamGraph {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: StreamConfig::default(),
            cached_data: None,
            last_navigation_pos: None,
        }
    }
    
    fn fetch_data(&mut self, ctx: &ViewerContext) -> Option<StreamData> {
        if self.config.x_column.is_empty() || 
           self.config.category_column.is_empty() || 
           self.config.value_column.is_empty() {
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
        
        // Extract columns
        let x_col = batch.column_by_name(&self.config.x_column)?;
        let cat_col = batch.column_by_name(&self.config.category_column)?;
        let val_col = batch.column_by_name(&self.config.value_column)?;
        
        // Extract values
        let x_values_raw = Self::extract_numeric_values(x_col);
        let categories = Self::extract_string_values(cat_col);
        let values = Self::extract_numeric_values(val_col);
        
        if x_values_raw.is_empty() || categories.is_empty() || values.is_empty() {
            return None;
        }
        
        // Group by x value and category
        let mut data_map: HashMap<String, HashMap<i64, f64>> = HashMap::new();
        let mut unique_x_values = std::collections::BTreeSet::new();
        
        for i in 0..x_values_raw.len().min(categories.len()).min(values.len()) {
            let x = x_values_raw[i] as i64; // Convert to integer for grouping
            let cat = &categories[i];
            let val = values[i];
            
            unique_x_values.insert(x);
            data_map.entry(cat.clone())
                .or_insert_with(HashMap::new)
                .insert(x, val);
        }
        
        let x_values: Vec<f64> = unique_x_values.into_iter().map(|x| x as f64).collect();
        let unique_categories: Vec<String> = data_map.keys().cloned().collect();
        
        // Create series data
        let mut series_data = Vec::new();
        for cat in &unique_categories {
            let cat_data = &data_map[cat];
            let values: Vec<f64> = x_values.iter()
                .map(|&x| *cat_data.get(&(x as i64)).unwrap_or(&0.0))
                .collect();
            series_data.push((cat.clone(), values));
        }
        
        // Calculate stream layout
        let (series, _) = self.calculate_stream_layout(series_data, &x_values);
        
        Some(StreamData { series, x_values })
    }
    
    fn calculate_stream_layout(&self, series_data: Vec<(String, Vec<f64>)>, x_values: &[f64]) -> (Vec<StreamSeries>, Vec<f64>) {
        let n_series = series_data.len();
        let n_points = x_values.len();
        
        if n_series == 0 || n_points == 0 {
            return (Vec::new(), Vec::new());
        }
        
        // Initialize bounds
        let mut all_series = Vec::new();
        let mut baseline = vec![0.0; n_points];
        
        // Calculate baselines based on method
        match self.config.baseline {
            StreamBaseline::Zero => {
                // Stack from zero - baseline stays at 0
            }
            StreamBaseline::Centered => {
                // Calculate total at each point and center
                for i in 0..n_points {
                    let total: f64 = series_data.iter().map(|(_, vals)| vals[i]).sum();
                    baseline[i] = -total / 2.0;
                }
            }
            StreamBaseline::Wiggle => {
                // Minimize wiggle (simplified version)
                // This is a simplified implementation of the wiggle algorithm
                for i in 0..n_points {
                    let total: f64 = series_data.iter().map(|(_, vals)| vals[i]).sum();
                    baseline[i] = -total / 2.0;
                }
            }
        }
        
        // Stack series
        let mut current_baseline = baseline.clone();
        
        for (idx, (name, values)) in series_data.into_iter().enumerate() {
            let mut lower_bounds = Vec::new();
            let mut upper_bounds = Vec::new();
            
            for i in 0..n_points {
                lower_bounds.push(current_baseline[i]);
                upper_bounds.push(current_baseline[i] + values[i]);
                current_baseline[i] += values[i];
            }
            
            let color = Self::categorical_color(idx);
            
            all_series.push(StreamSeries {
                name,
                values,
                lower_bounds,
                upper_bounds,
                color,
            });
        }
        
        (all_series, baseline)
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
    
    fn extract_string_values(array: &dyn Array) -> Vec<String> {
        if let Some(str_array) = array.as_any().downcast_ref::<StringArray>() {
            (0..str_array.len()).map(|i| {
                if str_array.is_null(i) { 
                    "null".to_string() 
                } else { 
                    str_array.value(i).to_string() 
                }
            }).collect()
        } else {
            // Try to convert other types to string
            (0..array.len()).map(|i| {
                arrow::util::display::array_value_to_string(array, i).unwrap_or_else(|_| "null".to_string())
            }).collect()
        }
    }
    
    fn categorical_color(idx: usize) -> Color32 {
        const COLORS: &[Color32] = &[
            Color32::from_rgb(31, 119, 180),   // Blue
            Color32::from_rgb(255, 127, 14),   // Orange
            Color32::from_rgb(44, 160, 44),    // Green
            Color32::from_rgb(214, 39, 40),    // Red
            Color32::from_rgb(148, 103, 189),  // Purple
            Color32::from_rgb(140, 86, 75),    // Brown
            Color32::from_rgb(227, 119, 194),  // Pink
            Color32::from_rgb(127, 127, 127),  // Gray
            Color32::from_rgb(188, 189, 34),   // Olive
            Color32::from_rgb(23, 190, 207),   // Cyan
        ];
        COLORS[idx % COLORS.len()]
    }
}

impl SpaceView for StreamGraph {
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
    fn view_type(&self) -> &str { "StreamGraph" }
    
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
            Plot::new(format!("{:?}_stream", self.id))
                .legend(Legend::default())
                .show_grid(true)
                .auto_bounds(egui::Vec2b::new(true, true))
                .show(ui, |plot_ui| {
                    // Draw each stream as a polygon
                    for series in &data.series {
                        if series.values.is_empty() {
                            continue;
                        }
                        
                        // Create polygon points for the stream
                        let mut polygon_points = Vec::new();
                        
                        // Top edge (forward)
                        for i in 0..data.x_values.len() {
                            polygon_points.push([data.x_values[i], series.upper_bounds[i]]);
                        }
                        
                        // Bottom edge (backward)
                        for i in (0..data.x_values.len()).rev() {
                            polygon_points.push([data.x_values[i], series.lower_bounds[i]]);
                        }
                        
                        let polygon = Polygon::new(PlotPoints::new(polygon_points))
                            .fill_color(series.color.linear_multiply(0.8))
                            .color(series.color)
                            .width(1.0)
                            .name(&series.name);
                        
                        plot_ui.polygon(polygon);
                    }
                });
                
            // Info panel
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(format!("Series: {}", data.series.len()));
                ui.label(format!("Points: {}", data.x_values.len()));
                ui.label(format!("Baseline: {:?}", self.config.baseline));
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
                if self.config.x_column.is_empty() || 
                   self.config.category_column.is_empty() || 
                   self.config.value_column.is_empty() {
                    ui.label("Please configure X, Category, and Value columns");
                }
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "data_source_id": self.config.data_source_id,
            "x_column": self.config.x_column,
            "category_column": self.config.category_column,
            "value_column": self.config.value_column,
            "baseline": match self.config.baseline {
                StreamBaseline::Zero => "zero",
                StreamBaseline::Wiggle => "wiggle",
                StreamBaseline::Centered => "centered",
            },
            "interpolation": self.config.interpolation,
            "color_scheme": self.config.color_scheme,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(data_source_id) = config.get("data_source_id").and_then(|v| v.as_str()) {
            self.config.data_source_id = Some(data_source_id.to_string());
        }
        if let Some(x_col) = config.get("x_column").and_then(|v| v.as_str()) {
            self.config.x_column = x_col.to_string();
        }
        if let Some(cat_col) = config.get("category_column").and_then(|v| v.as_str()) {
            self.config.category_column = cat_col.to_string();
        }
        if let Some(val_col) = config.get("value_column").and_then(|v| v.as_str()) {
            self.config.value_column = val_col.to_string();
        }
        if let Some(baseline) = config.get("baseline").and_then(|v| v.as_str()) {
            self.config.baseline = match baseline {
                "zero" => StreamBaseline::Zero,
                "centered" => StreamBaseline::Centered,
                _ => StreamBaseline::Wiggle,
            };
        }
        if let Some(interp) = config.get("interpolation").and_then(|v| v.as_str()) {
            self.config.interpolation = interp.to_string();
        }
        if let Some(color_scheme) = config.get("color_scheme").and_then(|v| v.as_str()) {
            self.config.color_scheme = color_scheme.to_string();
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {}
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {}
} 