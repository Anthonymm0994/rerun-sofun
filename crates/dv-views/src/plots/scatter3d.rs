//! 3D scatter plot implementation inspired by Rerun

use egui::{Ui, Color32, Rect, Pos2, Vec2, Stroke, FontId, Align2, Response, Sense, Key};
use arrow::record_batch::RecordBatch;
use arrow::array::Float64Array;
use serde_json::{json, Value};
use glam::{Vec3, Mat4, Quat};
use std::f32::consts::PI;

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use super::utils::{ColorScheme, viridis_color, plasma_color, categorical_color};

/// 3D scatter plot configuration
#[derive(Debug, Clone)]
pub struct Scatter3DConfig {
    pub data_source_id: Option<String>,
    pub x_column: String,
    pub y_column: String,
    pub z_column: String,
    pub color_column: Option<String>,
    pub size_column: Option<String>,
    pub label_column: Option<String>,
    
    // Rendering options
    pub point_size: f32,
    pub show_axes: bool,
    pub show_grid: bool,
    pub show_labels: bool,
    pub perspective: bool,
    pub color_scheme: ColorScheme,
    
    // Camera settings
    pub auto_rotate: bool,
    pub rotation_speed: f32,
    pub enable_controls: bool,
}

impl Default for Scatter3DConfig {
    fn default() -> Self {
        Self {
            data_source_id: None,
            x_column: String::new(),
            y_column: String::new(),
            z_column: String::new(),
            color_column: None,
            size_column: None,
            label_column: None,
            point_size: 5.0,
            show_axes: true,
            show_grid: true,
            show_labels: true,
            perspective: true,
            color_scheme: ColorScheme::Viridis,
            auto_rotate: false,
            rotation_speed: 0.5,
            enable_controls: true,
        }
    }
}

/// Camera state for 3D view
#[derive(Debug, Clone)]
struct Camera3D {
    position: Vec3,
    target: Vec3,
    up: Vec3,
    fov: f32,
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            position: Vec3::new(5.0, 5.0, 5.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov: 45.0_f32.to_radians(),
            yaw: -PI / 4.0,
            pitch: PI / 6.0,
            distance: 10.0,
        }
    }
}

impl Camera3D {
    fn update_from_angles(&mut self) {
        self.position = Vec3::new(
            self.distance * self.yaw.cos() * self.pitch.cos(),
            self.distance * self.pitch.sin(),
            self.distance * self.yaw.sin() * self.pitch.cos(),
        );
    }
    
    fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }
    
    fn projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov, aspect_ratio, 0.1, 100.0)
    }
}

/// 3D point for rendering
#[derive(Clone)]
struct Point3D {
    position: Vec3,
    color: Color32,
    size: f32,
    label: Option<String>,
    index: usize,
}

/// 3D scatter plot view
pub struct Scatter3DPlot {
    id: SpaceViewId,
    title: String,
    pub config: Scatter3DConfig,
    
    // State
    cached_data: Option<RecordBatch>,
    camera: Camera3D,
    
    // Interaction state
    is_rotating: bool,
    last_mouse_pos: Option<Pos2>,
    selected_point: Option<usize>,
}

impl Scatter3DPlot {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: Scatter3DConfig::default(),
            cached_data: None,
            camera: Camera3D::default(),
            is_rotating: false,
            last_mouse_pos: None,
            selected_point: None,
        }
    }
    
    fn extract_points(&self, batch: &RecordBatch) -> Vec<Point3D> {
        let mut points = Vec::new();
        
        // Find columns
        let x_idx = batch.schema().fields().iter()
            .position(|f| f.name() == &self.config.x_column);
        let y_idx = batch.schema().fields().iter()
            .position(|f| f.name() == &self.config.y_column);
        let z_idx = batch.schema().fields().iter()
            .position(|f| f.name() == &self.config.z_column);
            
        if x_idx.is_none() || y_idx.is_none() || z_idx.is_none() {
            return points;
        }
        
        let x_col = batch.column(x_idx.unwrap());
        let y_col = batch.column(y_idx.unwrap());
        let z_col = batch.column(z_idx.unwrap());
        
        if let (Some(x_array), Some(y_array), Some(z_array)) = (
            x_col.as_any().downcast_ref::<Float64Array>(),
            y_col.as_any().downcast_ref::<Float64Array>(),
            z_col.as_any().downcast_ref::<Float64Array>()
        ) {
            // Extract color values if specified
            let color_array = self.config.color_column.as_ref()
                .and_then(|col_name| {
                    batch.schema().fields().iter()
                        .position(|f| f.name() == col_name)
                        .and_then(|idx| batch.column(idx).as_any().downcast_ref::<Float64Array>())
                });
            
            // Find min/max for color normalization
            let (color_min, color_max) = if let Some(color_arr) = &color_array {
                let mut min = f64::INFINITY;
                let mut max = f64::NEG_INFINITY;
                for i in 0..color_arr.len() {
                    let val = color_arr.value(i);
                    min = min.min(val);
                    max = max.max(val);
                }
                (min, max)
            } else {
                (0.0, 1.0)
            };
            
            // Extract size values if specified
            let size_array = self.config.size_column.as_ref()
                .and_then(|col_name| {
                    batch.schema().fields().iter()
                        .position(|f| f.name() == col_name)
                        .and_then(|idx| batch.column(idx).as_any().downcast_ref::<Float64Array>())
                });
            
            for i in 0..x_array.len() {
                let x = x_array.value(i);
                let y = y_array.value(i);
                let z = z_array.value(i);
                    // Determine color
                    let color = if let Some(color_arr) = &color_array {
                        let val = color_arr.value(i);
                        let normalized = ((val - color_min) / (color_max - color_min)) as f32;
                        match self.config.color_scheme {
                            ColorScheme::Viridis => viridis_color(normalized),
                            ColorScheme::Plasma => plasma_color(normalized),
                            ColorScheme::Categorical => categorical_color(i),
                            _ => Color32::from_rgb(100, 150, 255),
                        }
                    } else {
                        Color32::from_rgb(100, 150, 255)
                    };
                    
                    // Determine size
                    let size = if let Some(size_arr) = &size_array {
                        let val = size_arr.value(i);
                        (val as f32).max(1.0).min(20.0)
                    } else {
                        self.config.point_size
                    };
                    
                    points.push(Point3D {
                        position: Vec3::new(x as f32, y as f32, z as f32),
                        color,
                        size,
                        label: None, // TODO: Extract labels
                        index: i,
                    });
            }
        }
        
        points
    }
    
    fn project_point(&self, point: Vec3, rect: &Rect) -> Option<(Pos2, f32)> {
        let aspect_ratio = rect.width() / rect.height();
        let view_matrix = self.camera.view_matrix();
        let proj_matrix = self.camera.projection_matrix(aspect_ratio);
        let mvp = proj_matrix * view_matrix;
        
        // Transform point
        let transformed = mvp * point.extend(1.0);
        
        // Perspective divide
        if transformed.w <= 0.0 {
            return None; // Behind camera
        }
        
        let ndc = transformed.truncate() / transformed.w;
        
        // Convert to screen coordinates
        let x = (ndc.x + 1.0) * 0.5 * rect.width() + rect.left();
        let y = (1.0 - ndc.y) * 0.5 * rect.height() + rect.top();
        let depth = ndc.z;
        
        Some((Pos2::new(x, y), depth))
    }
    
    fn draw_axes(&self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        
        // Axis lines
        let axes = [
            (Vec3::ZERO, Vec3::X * 3.0, Color32::from_rgb(255, 100, 100), "X"),
            (Vec3::ZERO, Vec3::Y * 3.0, Color32::from_rgb(100, 255, 100), "Y"),
            (Vec3::ZERO, Vec3::Z * 3.0, Color32::from_rgb(100, 100, 255), "Z"),
        ];
        
        for (start, end, color, label) in axes {
            if let (Some((start_2d, _)), Some((end_2d, _))) = (
                self.project_point(start, &rect),
                self.project_point(end, &rect)
            ) {
                painter.line_segment([start_2d, end_2d], Stroke::new(2.0, color));
                painter.text(end_2d, Align2::CENTER_CENTER, label, FontId::proportional(12.0), color);
            }
        }
        
        // Grid
        if self.config.show_grid {
            let grid_color = Color32::from_gray(80);
            let grid_stroke = Stroke::new(0.5, grid_color);
            
            // XZ plane grid
            for i in -5..=5 {
                let x = i as f32;
                if let (Some((start, _)), Some((end, _))) = (
                    self.project_point(Vec3::new(x, 0.0, -5.0), &rect),
                    self.project_point(Vec3::new(x, 0.0, 5.0), &rect)
                ) {
                    painter.line_segment([start, end], grid_stroke);
                }
                
                if let (Some((start, _)), Some((end, _))) = (
                    self.project_point(Vec3::new(-5.0, 0.0, x), &rect),
                    self.project_point(Vec3::new(5.0, 0.0, x), &rect)
                ) {
                    painter.line_segment([start, end], grid_stroke);
                }
            }
        }
    }
    
    fn draw_points(&self, ui: &mut Ui, rect: Rect, points: &[Point3D]) {
        let painter = ui.painter_at(rect);
        
        // Sort points by depth for proper rendering
        let mut sorted_points: Vec<(usize, f32)> = points.iter()
            .enumerate()
            .filter_map(|(i, point)| {
                self.project_point(point.position, &rect)
                    .map(|(_, depth)| (i, depth))
            })
            .collect();
        sorted_points.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        // Draw points back to front
        for (idx, _) in sorted_points {
            let point = &points[idx];
            if let Some((pos_2d, _)) = self.project_point(point.position, &rect) {
                // Draw shadow on ground plane
                if let Some((shadow_pos, _)) = self.project_point(
                    Vec3::new(point.position.x, 0.0, point.position.z), 
                    &rect
                ) {
                    painter.circle_filled(
                        shadow_pos, 
                        point.size * 0.5, 
                        Color32::from_rgba_unmultiplied(0, 0, 0, 30)
                    );
                }
                
                // Draw point
                painter.circle_filled(pos_2d, point.size, point.color);
                
                // Highlight selected point
                if Some(point.index) == self.selected_point {
                    painter.circle_stroke(
                        pos_2d, 
                        point.size + 2.0, 
                        Stroke::new(2.0, Color32::WHITE)
                    );
                }
                
                // Draw label if enabled
                if self.config.show_labels {
                    if let Some(label) = &point.label {
                        painter.text(
                            pos_2d + Vec2::new(point.size + 2.0, 0.0),
                            Align2::LEFT_CENTER,
                            label,
                            FontId::proportional(10.0),
                            Color32::from_gray(200),
                        );
                    }
                }
            }
        }
    }
    
    fn handle_interaction(&mut self, ui: &mut Ui, rect: Rect, points: &[Point3D]) -> Response {
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        
        // Mouse controls
        if self.config.enable_controls {
            if response.dragged_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    if let Some(last_pos) = self.last_mouse_pos {
                        let delta = pos - last_pos;
                        self.camera.yaw -= delta.x * 0.01;
                        self.camera.pitch = (self.camera.pitch - delta.y * 0.01)
                            .clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);
                        self.camera.update_from_angles();
                    }
                    self.last_mouse_pos = Some(pos);
                }
            } else {
                self.last_mouse_pos = None;
            }
            
            // Scroll for zoom
            let scroll_delta = ui.input(|i| i.scroll_delta.y);
            if scroll_delta != 0.0 {
                self.camera.distance = (self.camera.distance * (1.0 - scroll_delta * 0.001))
                    .clamp(2.0, 50.0);
                self.camera.update_from_angles();
            }
            
            // Click to select point
            if response.clicked() {
                if let Some(click_pos) = response.interact_pointer_pos() {
                    let mut closest_point = None;
                    let mut min_dist = 20.0; // Pixel threshold
                    
                    for point in points {
                        if let Some((pos_2d, _)) = self.project_point(point.position, &rect) {
                            let dist = (pos_2d - click_pos).length();
                            if dist < min_dist {
                                min_dist = dist;
                                closest_point = Some(point.index);
                            }
                        }
                    }
                    
                    self.selected_point = closest_point;
                }
            }
        }
        
        // Auto-rotation
        if self.config.auto_rotate {
            self.camera.yaw += self.config.rotation_speed * 0.01;
            self.camera.update_from_angles();
        }
        
        // Keyboard controls
        ui.input(|i| {
            if i.key_pressed(Key::R) {
                self.camera = Camera3D::default();
            }
            if i.key_pressed(Key::Space) {
                self.config.auto_rotate = !self.config.auto_rotate;
            }
        });
        
        response
    }
    
    fn draw_info_panel(&self, ui: &mut Ui, points: &[Point3D]) {
        ui.horizontal(|ui| {
            ui.label("3D Controls:");
            ui.label("🖱 Drag to rotate");
            ui.label("📏 Scroll to zoom");
            ui.label("⌨ R to reset");
            ui.label("⌨ Space to toggle rotation");
            
            ui.separator();
            
            if let Some(selected_idx) = self.selected_point {
                if let Some(point) = points.iter().find(|p| p.index == selected_idx) {
                    ui.label(format!(
                        "Selected: ({:.2}, {:.2}, {:.2})",
                        point.position.x, point.position.y, point.position.z
                    ));
                }
            }
        });
    }
}

impl SpaceView for Scatter3DPlot {
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
    fn view_type(&self) -> &str { "Scatter3DView" }
    
    fn set_data_source(&mut self, source_id: String) {
        self.config.data_source_id = Some(source_id);
        // Clear any cached data
        if let Some(cache_field) = self.as_any_mut().downcast_mut::<Self>() {
            // Reset cached data if the plot has any
        }
    }
    
    fn data_source_id(&self) -> Option<&str> {
        self.config.data_source_id.as_deref()
    }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Update data if needed
        if self.cached_data.is_none() {
            let data_sources = ctx.data_sources.read();
            
            // Get the specific data source for this view
            let data_source = if let Some(source_id) = &self.config.data_source_id {
            data_sources.get(source_id)
            } else {
                (if let Some(source_id) = &self.config.data_source_id {
        data_sources.get(source_id)
    } else {
        data_sources.values().next()
    })
            };
            
            if let Some(source) = data_source {
                let nav_pos = ctx.navigation.get_context().position.clone();
                if let Ok(batch) = ctx.runtime_handle.block_on(source.query_at(&nav_pos)) {
                    self.cached_data = Some(batch);
                }
            }
        }
        
        if let Some(batch) = &self.cached_data {
            let points = self.extract_points(batch);
            
            if points.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("No 3D data available. Please configure X, Y, and Z columns.");
                });
                return;
            }
            
            // Main 3D viewport
            let available_rect = ui.available_rect_before_wrap();
            let viewport_rect = Rect::from_min_size(
                available_rect.left_top(),
                available_rect.size() - Vec2::new(0.0, 30.0) // Leave space for controls
            );
            
            // Background
            ui.painter().rect_filled(
                viewport_rect,
                0.0,
                Color32::from_gray(20)
            );
            
            // Draw 3D scene
            if self.config.show_axes {
                self.draw_axes(ui, viewport_rect);
            }
            self.draw_points(ui, viewport_rect, &points);
            
            // Handle interaction
            let response = self.handle_interaction(ui, viewport_rect, &points);
            
            // Request repaint if animating
            if self.config.auto_rotate {
                ui.ctx().request_repaint();
            }
            
            // Info panel
            self.draw_info_panel(ui, &points);
            
            // Tooltip on hover
            if response.hovered() {
                if let Some(hover_pos) = response.hover_pos() {
                    for point in &points {
                        if let Some((pos_2d, _)) = self.project_point(point.position, &viewport_rect) {
                            if (pos_2d - hover_pos).length() < point.size + 5.0 {
                                response.on_hover_text(format!(
                                    "Point {}: ({:.2}, {:.2}, {:.2})",
                                    point.index, point.position.x, point.position.y, point.position.z
                                ));
                                break;
                            }
                        }
                    }
                }
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Loading 3D data...");
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "x_column": self.config.x_column,
            "y_column": self.config.y_column,
            "z_column": self.config.z_column,
            "color_column": self.config.color_column,
            "size_column": self.config.size_column,
            "label_column": self.config.label_column,
            "point_size": self.config.point_size,
            "show_axes": self.config.show_axes,
            "show_grid": self.config.show_grid,
            "show_labels": self.config.show_labels,
            "perspective": self.config.perspective,
            "color_scheme": format!("{:?}", self.config.color_scheme),
            "auto_rotate": self.config.auto_rotate,
            "rotation_speed": self.config.rotation_speed,
            "camera_yaw": self.camera.yaw,
            "camera_pitch": self.camera.pitch,
            "camera_distance": self.camera.distance,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(x) = config.get("x_column").and_then(|v| v.as_str()) {
            self.config.x_column = x.to_string();
        }
        if let Some(y) = config.get("y_column").and_then(|v| v.as_str()) {
            self.config.y_column = y.to_string();
        }
        if let Some(z) = config.get("z_column").and_then(|v| v.as_str()) {
            self.config.z_column = z.to_string();
        }
        if let Some(yaw) = config.get("camera_yaw").and_then(|v| v.as_f64()) {
            self.camera.yaw = yaw as f32;
        }
        if let Some(pitch) = config.get("camera_pitch").and_then(|v| v.as_f64()) {
            self.camera.pitch = pitch as f32;
        }
        if let Some(distance) = config.get("camera_distance").and_then(|v| v.as_f64()) {
            self.camera.distance = distance as f32;
        }
        self.camera.update_from_angles();
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {}
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {}
} 