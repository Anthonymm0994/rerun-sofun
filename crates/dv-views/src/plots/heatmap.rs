//! Heatmap implementation for 2D data visualization

use egui::{Ui, Color32, Rect, pos2, vec2, Stroke, Sense, TextStyle};
use arrow::array::{Float64Array, Int64Array, Array};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;
use super::utils::colors::ColorScheme as UtilColorScheme;

/// Configuration for heatmap view
#[derive(Clone)]
pub struct HeatmapConfig {
    pub data_source_id: String,
    /// X-axis column
    pub x_column: String,
    
    /// Y-axis column
    pub y_column: String,
    
    /// Value column
    pub value_column: String,
    
    /// Aggregation method
    pub aggregation: AggregationMethod,
    
    /// Color scheme
    pub color_scheme: ColorScheme,
    
    /// Whether to show values in cells
    pub show_values: bool,
    
    /// Cell size
    pub cell_size: f32,
    
    /// Whether to show colorbar
    pub show_colorbar: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AggregationMethod {
    Sum,
    Mean,
    Count,
    Min,
    Max,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ColorScheme {
    Viridis,
    Plasma,
    Inferno,
    Magma,
    CoolWarm,
    RedBlue,
}

impl Default for HeatmapConfig {
    fn default() -> Self {
        Self {
            data_source_id: String::new(),
            x_column: String::new(),
            y_column: String::new(),
            value_column: String::new(),
            aggregation: AggregationMethod::Mean,
            color_scheme: ColorScheme::Viridis,
            show_values: false,
            cell_size: 50.0,
            show_colorbar: true,
        }
    }
}

/// Heatmap view
pub struct HeatmapView {
    id: SpaceViewId,
    title: String,
    pub config: HeatmapConfig,
    
    // State
    cached_data: Option<HeatmapData>,
    last_navigation_pos: Option<NavigationPosition>,
}

/// Cached heatmap data
struct HeatmapData {
    x_labels: Vec<String>,
    y_labels: Vec<String>,
    values: Vec<Vec<Option<f64>>>,
    min_value: f64,
    max_value: f64,
}

impl HeatmapView {
    /// Create a new heatmap view
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: HeatmapConfig::default(),
            cached_data: None,
            last_navigation_pos: None,
        }
    }
    
    /// Fetch heatmap data
    fn fetch_data(&mut self, ctx: &ViewerContext) -> Option<HeatmapData> {
        let data_sources = ctx.data_sources.read();

        let data_source = data_sources.values().next();
        let data_source = data_source.as_ref()?;
        
        // Get navigation context
        let nav_context = ctx.navigation.get_context();
        
        // Fetch all data
        let range = dv_core::navigation::NavigationRange {
            start: dv_core::navigation::NavigationPosition::Sequential(0),
            end: dv_core::navigation::NavigationPosition::Sequential(nav_context.total_rows),
        };
        
        let data = ctx.runtime_handle.block_on(data_source.query_range(&range)).ok()?;
        
        // Extract columns
        let x_column = data.column_by_name(&self.config.x_column)?;
        let y_column = data.column_by_name(&self.config.y_column)?;
        let value_column = data.column_by_name(&self.config.value_column)?;
        
        // Convert to strings for labels
        let x_values: Vec<String> = (0..x_column.len())
            .map(|i| arrow::util::display::array_value_to_string(x_column, i).unwrap_or_default())
            .collect();
        let y_values: Vec<String> = (0..y_column.len())
            .map(|i| arrow::util::display::array_value_to_string(y_column, i).unwrap_or_default())
            .collect();
        
        // Extract numeric values
        let values: Vec<f64> = if let Some(float_array) = value_column.as_any().downcast_ref::<Float64Array>() {
            (0..float_array.len()).filter_map(|i| {
                if float_array.is_null(i) { None } else { Some(float_array.value(i)) }
            }).collect()
        } else if let Some(int_array) = value_column.as_any().downcast_ref::<Int64Array>() {
            (0..int_array.len()).filter_map(|i| {
                if int_array.is_null(i) { None } else { Some(int_array.value(i) as f64) }
            }).collect()
        } else {
            return None;
        };
        
        // Create unique label lists
        let mut x_labels: Vec<String> = x_values.clone();
        x_labels.sort();
        x_labels.dedup();
        
        let mut y_labels: Vec<String> = y_values.clone();
        y_labels.sort();
        y_labels.dedup();
        
        // Create grid
        let mut grid: HashMap<(String, String), Vec<f64>> = HashMap::new();
        for ((x, y), val) in x_values.iter().zip(y_values.iter()).zip(values.iter()) {
            grid.entry((x.clone(), y.clone())).or_insert_with(Vec::new).push(*val);
        }
        
        // Aggregate values
        let mut matrix = vec![vec![None; x_labels.len()]; y_labels.len()];
        let mut min_value = f64::INFINITY;
        let mut max_value = f64::NEG_INFINITY;
        
        for (y_idx, y_label) in y_labels.iter().enumerate() {
            for (x_idx, x_label) in x_labels.iter().enumerate() {
                if let Some(vals) = grid.get(&(x_label.clone(), y_label.clone())) {
                    let aggregated = match self.config.aggregation {
                        AggregationMethod::Sum => vals.iter().sum(),
                        AggregationMethod::Mean => vals.iter().sum::<f64>() / vals.len() as f64,
                        AggregationMethod::Count => vals.len() as f64,
                        AggregationMethod::Min => vals.iter().cloned().fold(f64::INFINITY, f64::min),
                        AggregationMethod::Max => vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                    };
                    
                    matrix[y_idx][x_idx] = Some(aggregated);
                    min_value = min_value.min(aggregated);
                    max_value = max_value.max(aggregated);
                }
            }
        }
        
        Some(HeatmapData {
            x_labels,
            y_labels,
            values: matrix,
            min_value,
            max_value,
        })
    }
    
    fn value_to_color(&self, value: f64, min: f64, max: f64) -> Color32 {
        let t = if max > min { 
            ((value - min) / (max - min)) as f32
        } else { 
            0.5 
        };
        
        match self.config.color_scheme {
            ColorScheme::Viridis => {
                super::utils::colors::viridis_color(t)
            }
            ColorScheme::Plasma => {
                super::utils::colors::plasma_color(t)
            }
            ColorScheme::CoolWarm | ColorScheme::RedBlue => {
                super::utils::colors::diverging_color(t)
            }
            _ => {
                // Fallback gradient
                let r = (255.0 * t) as u8;
                let b = (255.0 * (1.0 - t)) as u8;
                Color32::from_rgb(r, 128, b)
            }
        }
    }
}

impl SpaceView for HeatmapView {
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
        "HeatmapView"
    }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Update data if navigation changed
        let nav_pos = ctx.navigation.get_context().position.clone();
        if self.last_navigation_pos.as_ref() != Some(&nav_pos) {
            self.cached_data = self.fetch_data(ctx);
            self.last_navigation_pos = Some(nav_pos);
        }
        
        // Draw the heatmap
        if let Some(data) = &self.cached_data {
            egui::ScrollArea::both().show(ui, |ui| {
                let cell_size = self.config.cell_size;
                let margin = 60.0; // Space for labels
                
                // Calculate total size
                let width = data.x_labels.len() as f32 * cell_size + margin;
                let height = data.y_labels.len() as f32 * cell_size + margin;
                
                let (response, painter) = ui.allocate_painter(
                    vec2(width, height),
                    Sense::hover()
                );
                
                let rect = response.rect;
                let origin = rect.min + vec2(margin, margin);
                
                // Draw cells
                for (y_idx, y_label) in data.y_labels.iter().enumerate() {
                    for (x_idx, x_label) in data.x_labels.iter().enumerate() {
                        let cell_rect = Rect::from_min_size(
                            origin + vec2(x_idx as f32 * cell_size, y_idx as f32 * cell_size),
                            vec2(cell_size, cell_size)
                        );
                        
                        if let Some(value) = data.values[y_idx][x_idx] {
                            let color = self.value_to_color(value, data.min_value, data.max_value);
                            painter.rect_filled(cell_rect, 0.0, color);
                            
                            // Show value if enabled
                            if self.config.show_values {
                                let text = format!("{:.1}", value);
                                let text_color = if value > (data.min_value + data.max_value) / 2.0 {
                                    Color32::BLACK
                                } else {
                                    Color32::WHITE
                                };
                                painter.text(
                                    cell_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    text,
                                    TextStyle::Small.resolve(ui.style()),
                                    text_color
                                );
                            }
                        } else {
                            // Empty cell
                            painter.rect_filled(cell_rect, 0.0, Color32::from_gray(40));
                        }
                        
                        // Cell border
                        painter.rect_stroke(cell_rect, 0.0, Stroke::new(0.5, Color32::from_gray(80)));
                    }
                    
                    // Y labels
                    painter.text(
                        pos2(rect.min.x + margin - 5.0, origin.y + y_idx as f32 * cell_size + cell_size / 2.0),
                        egui::Align2::RIGHT_CENTER,
                        y_label,
                        TextStyle::Small.resolve(ui.style()),
                        ui.style().visuals.text_color()
                    );
                }
                
                // X labels (rotated)
                for (x_idx, x_label) in data.x_labels.iter().enumerate() {
                    let pos = origin + vec2(x_idx as f32 * cell_size + cell_size / 2.0, -5.0);
                    painter.text(
                        pos,
                        egui::Align2::CENTER_BOTTOM,
                        x_label,
                        TextStyle::Small.resolve(ui.style()),
                        ui.style().visuals.text_color()
                    );
                }
                
                // Colorbar
                if self.config.show_colorbar {
                    let colorbar_rect = Rect::from_min_size(
                        pos2(rect.max.x - 40.0, origin.y),
                        vec2(20.0, data.y_labels.len() as f32 * cell_size)
                    );
                    
                    // Draw gradient
                    let steps = 50;
                    let step_height = colorbar_rect.height() / steps as f32;
                    for i in 0..steps {
                        let t = i as f32 / (steps - 1) as f32;
                        let value = data.min_value + (t as f64) * (data.max_value - data.min_value);
                        let color = self.value_to_color(value, data.min_value, data.max_value);
                        
                        let step_rect = Rect::from_min_size(
                            colorbar_rect.min + vec2(0.0, (steps - 1 - i) as f32 * step_height),
                            vec2(colorbar_rect.width(), step_height)
                        );
                        painter.rect_filled(step_rect, 0.0, color);
                    }
                    
                    // Colorbar labels
                    painter.text(
                        pos2(colorbar_rect.max.x + 5.0, colorbar_rect.min.y),
                        egui::Align2::LEFT_TOP,
                        format!("{:.1}", data.max_value),
                        TextStyle::Small.resolve(ui.style()),
                        ui.style().visuals.text_color()
                    );
                    
                    painter.text(
                        pos2(colorbar_rect.max.x + 5.0, colorbar_rect.max.y),
                        egui::Align2::LEFT_BOTTOM,
                        format!("{:.1}", data.min_value),
                        TextStyle::Small.resolve(ui.style()),
                        ui.style().visuals.text_color()
                    );
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
                ui.label(egui::RichText::new("Configure X, Y and value columns to see heatmap").weak());
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "x_column": self.config.x_column,
            "y_column": self.config.y_column,
            "value_column": self.config.value_column,
            "aggregation": match self.config.aggregation {
                AggregationMethod::Sum => "sum",
                AggregationMethod::Mean => "mean",
                AggregationMethod::Count => "count",
                AggregationMethod::Min => "min",
                AggregationMethod::Max => "max",
            },
            "color_scheme": match self.config.color_scheme {
                ColorScheme::Viridis => "viridis",
                ColorScheme::Plasma => "plasma",
                ColorScheme::Inferno => "inferno",
                ColorScheme::Magma => "magma",
                ColorScheme::CoolWarm => "cool_warm",
                ColorScheme::RedBlue => "red_blue",
            },
            "show_values": self.config.show_values,
            "cell_size": self.config.cell_size,
            "show_colorbar": self.config.show_colorbar,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(x) = config.get("x_column").and_then(|v| v.as_str()) {
            self.config.x_column = x.to_string();
        }
        if let Some(y) = config.get("y_column").and_then(|v| v.as_str()) {
            self.config.y_column = y.to_string();
        }
        if let Some(val) = config.get("value_column").and_then(|v| v.as_str()) {
            self.config.value_column = val.to_string();
        }
        if let Some(agg) = config.get("aggregation").and_then(|v| v.as_str()) {
            self.config.aggregation = match agg {
                "sum" => AggregationMethod::Sum,
                "mean" => AggregationMethod::Mean,
                "count" => AggregationMethod::Count,
                "min" => AggregationMethod::Min,
                "max" => AggregationMethod::Max,
                _ => AggregationMethod::Mean,
            };
        }
        if let Some(scheme) = config.get("color_scheme").and_then(|v| v.as_str()) {
            self.config.color_scheme = match scheme {
                "viridis" => ColorScheme::Viridis,
                "plasma" => ColorScheme::Plasma,
                "inferno" => ColorScheme::Inferno,
                "magma" => ColorScheme::Magma,
                "cool_warm" => ColorScheme::CoolWarm,
                "red_blue" => ColorScheme::RedBlue,
                _ => ColorScheme::Viridis,
            };
        }
        if let Some(show) = config.get("show_values").and_then(|v| v.as_bool()) {
            self.config.show_values = show;
        }
        if let Some(size) = config.get("cell_size").and_then(|v| v.as_f64()) {
            self.config.cell_size = size as f32;
        }
        if let Some(show) = config.get("show_colorbar").and_then(|v| v.as_bool()) {
            self.config.show_colorbar = show;
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {
        // TODO: Highlight selected cells
    }
    
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {
        // Nothing to update per frame
    }
} 