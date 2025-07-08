//! Contour plot implementation

use egui::{Ui, Color32, Pos2, Vec2, Rect};
use egui_plot::{Plot, PlotPoints, Line, Legend};
use arrow::array::{Float64Array, Int64Array, Array};
use arrow::record_batch::RecordBatch;
use serde_json::{json, Value};

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;

/// Configuration for contour plot
#[derive(Debug, Clone)]
pub struct ContourConfig {
    pub data_source_id: Option<String>,
    pub x_column: String,
    pub y_column: String,
    pub z_column: String,
    pub levels: usize,
    pub color_scheme: String,
}

impl Default for ContourConfig {
    fn default() -> Self {
        Self {
            data_source_id: None,
            x_column: String::new(),
            y_column: String::new(),
            z_column: String::new(),
            levels: 10,
            color_scheme: "viridis".to_string(),
        }
    }
}

/// Cached contour data
struct ContourData {
    contours: Vec<ContourLine>,
}

struct ContourLine {
    level: f64,
    points: Vec<(f64, f64)>,
    color: Color32,
}

/// Contour plot view
pub struct ContourPlot {
    id: SpaceViewId,
    title: String,
    pub config: ContourConfig,
    cached_data: Option<ContourData>,
    last_navigation_pos: Option<NavigationPosition>,
}

impl ContourPlot {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: ContourConfig::default(),
            cached_data: None,
            last_navigation_pos: None,
        }
    }
    
    fn fetch_data(&mut self, ctx: &ViewerContext) -> Option<ContourData> {
        if self.config.x_column.is_empty() || 
           self.config.y_column.is_empty() || 
           self.config.z_column.is_empty() {
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
        let y_col = batch.column_by_name(&self.config.y_column)?;
        let z_col = batch.column_by_name(&self.config.z_column)?;
        
        // Extract numeric values with null handling
        let x_values: Vec<f64> = Self::extract_numeric_values(x_col);
        let y_values: Vec<f64> = Self::extract_numeric_values(y_col);
        let z_values: Vec<f64> = Self::extract_numeric_values(z_col);
        
        if x_values.is_empty() || y_values.is_empty() || z_values.is_empty() {
            return None;
        }
        
        // Create coordinate triples
        let coords: Vec<(f64, f64, f64)> = x_values.into_iter()
            .zip(y_values.into_iter())
            .zip(z_values.into_iter())
            .map(|((x, y), z)| (x, y, z))
            .collect();
        
        if coords.is_empty() {
            return None;
        }
        
        // Generate contour lines
        let mut contours = Vec::new();
        let z_min = coords.iter().map(|(_, _, z)| z).cloned().fold(f64::INFINITY, f64::min);
        let z_max = coords.iter().map(|(_, _, z)| z).cloned().fold(f64::NEG_INFINITY, f64::max);
        
        for i in 0..self.config.levels {
            let level = z_min + (z_max - z_min) * (i as f64) / (self.config.levels as f64);
            let color = Self::viridis_color((i as f32) / (self.config.levels as f32));
            
            // Find points near this contour level
            let mut points = Vec::new();
            for &(x, y, z) in &coords {
                if (z - level).abs() < (z_max - z_min) / (self.config.levels as f64 * 2.0) {
                    points.push((x, y));
                }
            }
            
            if !points.is_empty() {
                contours.push(ContourLine { level, points, color });
            }
        }
        
        Some(ContourData { contours })
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
    
    fn viridis_color(t: f32) -> Color32 {
        let t = t.clamp(0.0, 1.0);
        let r = (255.0 * (0.267 + 0.003 * t + 1.785 * t * t - 3.876 * t * t * t + 2.291 * t * t * t * t).clamp(0.0, 1.0)) as u8;
        let g = (255.0 * (0.005 + 1.398 * t - 0.725 * t * t).clamp(0.0, 1.0)) as u8;
        let b = (255.0 * (0.329 + 0.876 * t - 0.170 * t * t - 0.363 * t * t * t).clamp(0.0, 1.0)) as u8;
        Color32::from_rgb(r, g, b)
    }
}

impl SpaceView for ContourPlot {
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
    fn view_type(&self) -> &str { "ContourPlot" }
    
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
            // Create a contour visualization using egui_plot
            Plot::new(format!("{:?}_contour", self.id))
                .legend(Legend::default())
                .show_grid(true)
                .auto_bounds(egui::Vec2b::new(true, true))
                .show(ui, |plot_ui| {
                    // Draw contour lines
                    for contour in &data.contours {
                        if !contour.points.is_empty() {
                            let points: PlotPoints = contour.points.iter()
                                .map(|&(x, y)| [x, y])
                                .collect();
                            
                            let line = Line::new(points)
                                .color(contour.color)
                                .width(2.0)
                                .name(format!("Level {:.2}", contour.level));
                            
                            plot_ui.line(line);
                        }
                    }
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
                if self.config.x_column.is_empty() || 
                   self.config.y_column.is_empty() || 
                   self.config.z_column.is_empty() {
                    ui.label("Please configure X, Y, and Z columns");
                }
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "data_source_id": self.config.data_source_id,
            "x_column": self.config.x_column,
            "y_column": self.config.y_column,
            "z_column": self.config.z_column,
            "levels": self.config.levels,
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
        if let Some(y_col) = config.get("y_column").and_then(|v| v.as_str()) {
            self.config.y_column = y_col.to_string();
        }
        if let Some(z_col) = config.get("z_column").and_then(|v| v.as_str()) {
            self.config.z_column = z_col.to_string();
        }
        if let Some(levels) = config.get("levels").and_then(|v| v.as_u64()) {
            self.config.levels = levels as usize;
        }
        if let Some(color_scheme) = config.get("color_scheme").and_then(|v| v.as_str()) {
            self.config.color_scheme = color_scheme.to_string();
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {}
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {}
} 