//! 3D Surface plot implementation
//! Since egui doesn't have native 3D support, we'll create a 2D projection

use egui::{Ui, Color32, Rect, Pos2, Vec2, Painter, Stroke};
use arrow::array::{Float64Array, Int64Array, Array};
use arrow::record_batch::RecordBatch;
use serde_json::{json, Value};

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;

/// Configuration for 3D surface plot
#[derive(Debug, Clone)]
pub struct Surface3DConfig {
    pub data_source_id: Option<String>,
    pub x_column: String,
    pub y_column: String,
    pub z_column: String,
    pub color_scheme: String,
    pub rotation_x: f32,  // X-axis rotation in degrees
    pub rotation_z: f32,  // Z-axis rotation in degrees
    pub show_grid: bool,
    pub show_wireframe: bool,
}

impl Default for Surface3DConfig {
    fn default() -> Self {
        Self {
            data_source_id: None,
            x_column: String::new(),
            y_column: String::new(),
            z_column: String::new(),
            color_scheme: "viridis".to_string(),
            rotation_x: 30.0,
            rotation_z: 45.0,
            show_grid: true,
            show_wireframe: true,
        }
    }
}

/// Cached surface data
struct SurfaceData {
    grid: Vec<Vec<(f64, f64, f64)>>, // Grid of (x, y, z) points
    z_min: f64,
    z_max: f64,
}

/// 3D Surface plot view
pub struct Surface3DPlot {
    id: SpaceViewId,
    title: String,
    pub config: Surface3DConfig,
    cached_data: Option<SurfaceData>,
    last_navigation_pos: Option<NavigationPosition>,
}

impl Surface3DPlot {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: Surface3DConfig::default(),
            cached_data: None,
            last_navigation_pos: None,
        }
    }
    
    fn fetch_data(&mut self, ctx: &ViewerContext) -> Option<SurfaceData> {
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
        
        // Create a simple grid from the data
        // For simplicity, we'll create a scattered point representation
        let mut points: Vec<(f64, f64, f64)> = Vec::new();
        let min_len = x_values.len().min(y_values.len()).min(z_values.len());
        
        for i in 0..min_len {
            points.push((x_values[i], y_values[i], z_values[i]));
        }
        
        // Find unique x and y values to create a grid
        let mut unique_x: Vec<f64> = x_values.clone();
        unique_x.sort_by(|a, b| a.partial_cmp(b).unwrap());
        unique_x.dedup();
        
        let mut unique_y: Vec<f64> = y_values.clone();
        unique_y.sort_by(|a, b| a.partial_cmp(b).unwrap());
        unique_y.dedup();
        
        // Create a grid (for now, just organize points)
        let grid = vec![points]; // Simplified - treat as single row grid
        
        let z_min = z_values.iter().cloned().fold(f64::INFINITY, f64::min);
        let z_max = z_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        Some(SurfaceData {
            grid,
            z_min,
            z_max,
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
    
    fn project_3d_to_2d(&self, x: f64, y: f64, z: f64, bounds: &Rect) -> Pos2 {
        // Simple isometric projection
        let rx = self.config.rotation_x.to_radians();
        let rz = self.config.rotation_z.to_radians();
        
        // Apply rotations
        let x1 = x * (rz.cos() as f64) - y * (rz.sin() as f64);
        let y1 = x * (rz.sin() as f64) + y * (rz.cos() as f64);
        let z1 = z;
        
        let y2 = y1 * (rx.cos() as f64) - z1 * (rx.sin() as f64);
        let z2 = y1 * (rx.sin() as f64) + z1 * (rx.cos() as f64);
        
        // Project to 2D
        let scale = bounds.width().min(bounds.height()) * 0.3;
        let proj_x = bounds.center().x + x1 as f32 * scale;
        let proj_y = bounds.center().y - z2 as f32 * scale + y2 as f32 * scale * 0.5;
        
        Pos2::new(proj_x, proj_y)
    }
    
    fn get_color_for_z(&self, z: f64, z_min: f64, z_max: f64) -> Color32 {
        let t = if z_max > z_min {
            ((z - z_min) / (z_max - z_min)).clamp(0.0, 1.0) as f32
        } else {
            0.5
        };
        
        // Viridis color scheme
        let r = (255.0 * (0.267 + 0.003 * t + 1.785 * t * t - 3.876 * t * t * t + 2.291 * t * t * t * t).clamp(0.0, 1.0)) as u8;
        let g = (255.0 * (0.005 + 1.398 * t - 0.725 * t * t).clamp(0.0, 1.0)) as u8;
        let b = (255.0 * (0.329 + 0.876 * t - 0.170 * t * t - 0.363 * t * t * t).clamp(0.0, 1.0)) as u8;
        Color32::from_rgb(r, g, b)
    }
}

impl SpaceView for Surface3DPlot {
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
    fn view_type(&self) -> &str { "Surface3DPlot" }
    
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
        
        // Controls
        ui.horizontal(|ui| {
            ui.label("Rotation X:");
            ui.add(egui::Slider::new(&mut self.config.rotation_x, -90.0..=90.0).suffix("°"));
            ui.label("Rotation Z:");
            ui.add(egui::Slider::new(&mut self.config.rotation_z, 0.0..=360.0).suffix("°"));
        });
        
        ui.checkbox(&mut self.config.show_wireframe, "Show wireframe");
        ui.checkbox(&mut self.config.show_grid, "Show grid");
        
        let available_rect = ui.available_rect_before_wrap();
        let painter = ui.painter_at(available_rect);
        
        if let Some(data) = &self.cached_data {
            // Draw axes
            if self.config.show_grid {
                let origin = self.project_3d_to_2d(0.0, 0.0, 0.0, &available_rect);
                let x_axis = self.project_3d_to_2d(1.0, 0.0, 0.0, &available_rect);
                let y_axis = self.project_3d_to_2d(0.0, 1.0, 0.0, &available_rect);
                let z_axis = self.project_3d_to_2d(0.0, 0.0, 1.0, &available_rect);
                
                painter.line_segment([origin, x_axis], Stroke::new(2.0, Color32::RED));
                painter.line_segment([origin, y_axis], Stroke::new(2.0, Color32::GREEN));
                painter.line_segment([origin, z_axis], Stroke::new(2.0, Color32::BLUE));
            }
            
            // Draw surface points
            for row in &data.grid {
                for (x, y, z) in row {
                    let pos = self.project_3d_to_2d(
                        (*x - 0.5) * 2.0,  // Normalize to [-1, 1]
                        (*y - 0.5) * 2.0,
                        (*z - data.z_min) / (data.z_max - data.z_min) - 0.5,
                        &available_rect
                    );
                    
                    let color = self.get_color_for_z(*z, data.z_min, data.z_max);
                    painter.circle_filled(pos, 3.0, color);
                    
                    if self.config.show_wireframe {
                        painter.circle_stroke(pos, 3.0, Stroke::new(1.0, Color32::from_gray(100)));
                    }
                }
            }
            
            // Info
            ui.allocate_space(available_rect.size());
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(format!("Points: {}", data.grid[0].len()));
                ui.label(format!("Z range: [{:.2}, {:.2}]", data.z_min, data.z_max));
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
            "color_scheme": self.config.color_scheme,
            "rotation_x": self.config.rotation_x,
            "rotation_z": self.config.rotation_z,
            "show_grid": self.config.show_grid,
            "show_wireframe": self.config.show_wireframe,
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
        if let Some(color_scheme) = config.get("color_scheme").and_then(|v| v.as_str()) {
            self.config.color_scheme = color_scheme.to_string();
        }
        if let Some(rotation_x) = config.get("rotation_x").and_then(|v| v.as_f64()) {
            self.config.rotation_x = rotation_x as f32;
        }
        if let Some(rotation_z) = config.get("rotation_z").and_then(|v| v.as_f64()) {
            self.config.rotation_z = rotation_z as f32;
        }
        if let Some(show_grid) = config.get("show_grid").and_then(|v| v.as_bool()) {
            self.config.show_grid = show_grid;
        }
        if let Some(show_wireframe) = config.get("show_wireframe").and_then(|v| v.as_bool()) {
            self.config.show_wireframe = show_wireframe;
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {}
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {}
} 