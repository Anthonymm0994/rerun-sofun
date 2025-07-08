//! Sunburst chart for hierarchical data visualization

use egui::{Ui, Color32, Rect, Pos2, Vec2, Stroke, FontId, Align2, Shape, Response, Sense, Rounding};
use arrow::record_batch::RecordBatch;
use arrow::array::{Float64Array, StringArray, Int64Array};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use super::utils::{ColorScheme, categorical_color, diverging_color};

/// Sunburst configuration  
#[derive(Debug, Clone)]
pub struct SunburstConfig {
    pub hierarchy_columns: Vec<String>,
    pub value_column: Option<String>,
    
    // Visual options
    pub inner_radius_ratio: f32,
    pub color_scheme: ColorScheme,
    pub show_labels: bool,
    pub label_threshold: f32,
    pub show_values: bool,
    pub show_tooltip: bool,
    
    // Interaction
    pub enable_zoom: bool,
    pub animate_transitions: bool,
    pub highlight_on_hover: bool,
}

impl Default for SunburstConfig {
    fn default() -> Self {
        Self {
            hierarchy_columns: Vec::new(),
            value_column: None,
            inner_radius_ratio: 0.3,
            color_scheme: ColorScheme::Categorical,
            show_labels: true,
            label_threshold: 0.03,
            show_values: false,
            show_tooltip: true,
            enable_zoom: true,
            animate_transitions: true,
            highlight_on_hover: true,
        }
    }
}

#[derive(Clone, Debug)]
struct SunburstNode {
    name: String,
    value: f64,
    children: Vec<SunburstNode>,
    color: Color32,
    depth: usize,
    angle_start: f64,
    angle_end: f64,
    parent_path: Vec<String>,
}

impl SunburstNode {
    fn new(name: String, depth: usize, parent_path: Vec<String>) -> Self {
        Self {
            name,
            value: 0.0,
            children: Vec::new(),
            color: Color32::WHITE,
            depth,
            angle_start: 0.0,
            angle_end: 0.0,
            parent_path,
        }
    }
    
    fn add_child(&mut self, child: SunburstNode) {
        self.children.push(child);
    }
    
    fn calculate_value(&mut self) {
        if self.children.is_empty() {
            // Leaf node - value already set
            return;
        }
        
        // Internal node - sum children
        self.value = 0.0;
        for child in &mut self.children {
            child.calculate_value();
            self.value += child.value;
        }
    }
    
    fn assign_angles(&mut self, start_angle: f64, end_angle: f64) {
        self.angle_start = start_angle;
        self.angle_end = end_angle;
        
        if self.children.is_empty() || self.value == 0.0 {
            return;
        }
        
        let angle_range = end_angle - start_angle;
        let mut current_angle = start_angle;
        
        for child in &mut self.children {
            let child_angle_range = (child.value / self.value) * angle_range;
            child.assign_angles(current_angle, current_angle + child_angle_range);
            current_angle += child_angle_range;
        }
    }
    
    fn assign_colors(&mut self, color_scheme: &ColorScheme, index: &mut usize) {
        match color_scheme {
            ColorScheme::Categorical => {
                if self.depth == 1 {
                    // Top level gets distinct colors
                    self.color = categorical_color(*index);
                    *index += 1;
                } else if !self.children.is_empty() {
                    // Inherit parent color with slight variation
                    self.color = self.parent_color_variation();
                }
            }
            ColorScheme::Sequential => {
                let normalized = (*index as f32) / 20.0;
                self.color = diverging_color(normalized);
                *index += 1;
            }
            _ => {
                self.color = categorical_color(*index);
                *index += 1;
            }
        }
        
        for child in &mut self.children {
            child.assign_colors(color_scheme, index);
        }
    }
    
    fn parent_color_variation(&self) -> Color32 {
        // Lighten or darken parent color based on depth
        let factor = 1.0 + (self.depth as f32 - 1.0) * 0.1;
        Color32::from_rgba_unmultiplied(
            (self.color.r() as f32 * factor).min(255.0) as u8,
            (self.color.g() as f32 * factor).min(255.0) as u8,
            (self.color.b() as f32 * factor).min(255.0) as u8,
            self.color.a(),
        )
    }
    
    fn find_node_at_angle(&self, angle: f64, radius_ratio: f32, depth: usize) -> Option<&SunburstNode> {
        if angle >= self.angle_start && angle <= self.angle_end {
            if depth == self.depth {
                return Some(self);
            }
            
            for child in &self.children {
                if let Some(node) = child.find_node_at_angle(angle, radius_ratio, depth) {
                    return Some(node);
                }
            }
        }
        None
    }
}

/// Sunburst chart view
pub struct SunburstChart {
    id: SpaceViewId,
    title: String,
    pub config: SunburstConfig,
    
    // State
    cached_data: Option<RecordBatch>,
    root: Option<SunburstNode>,
    
    // Interaction state
    hovered_node: Option<Vec<String>>,
    selected_path: Vec<String>,
    zoom_level: usize,
    animation_progress: f32,
}

impl SunburstChart {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: SunburstConfig::default(),
            cached_data: None,
            root: None,
            hovered_node: None,
            selected_path: Vec::new(),
            zoom_level: 0,
            animation_progress: 1.0,
        }
    }
    
    fn build_hierarchy(&mut self, batch: &RecordBatch) {
        if self.config.hierarchy_columns.is_empty() {
            return;
        }
        
        // Extract value column data if specified
        let value_array = self.config.value_column.as_ref()
            .and_then(|col_name| {
                batch.schema().fields().iter()
                    .position(|f| f.name() == col_name)
                    .and_then(|idx| {
                        let col = batch.column(idx);
                        if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                            Some(arr)
                        } else if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                            // Convert Int64 to Float64
                            let values: Vec<Option<f64>> = (0..arr.len())
                                .map(|i| Some(arr.value(i) as f64))
                                .collect();
                            None // TODO: Handle this conversion properly
                        } else {
                            None
                        }
                    })
            });
        
        // Create root node
        let mut root = SunburstNode::new("Root".to_string(), 0, Vec::new());
        
        // Process each row
        for row_idx in 0..batch.num_rows() {
            let mut current_node = &mut root;
            let mut path = Vec::new();
            
            // Navigate/create path through hierarchy
            for (depth, col_name) in self.config.hierarchy_columns.iter().enumerate() {
                if let Some(col_idx) = batch.schema().fields().iter()
                    .position(|f| f.name() == col_name) {
                    
                    let col = batch.column(col_idx);
                    let node_name = if let Some(str_array) = col.as_any().downcast_ref::<StringArray>() {
                        str_array.value(row_idx).to_string()
                    } else {
                        "Unknown".to_string()
                    };
                    
                    path.push(node_name.clone());
                    
                    // Find or create child node
                    let child_exists = current_node.children.iter()
                        .position(|child| child.name == node_name);
                    
                    if let Some(child_idx) = child_exists {
                        // Navigate to existing child
                        // This is tricky with mutable references, so we'll rebuild if needed
                    } else {
                        // Create new child
                        let mut new_child = SunburstNode::new(
                            node_name.clone(),
                            depth + 1,
                            path.clone()
                        );
                        
                        // If this is a leaf node, set its value
                        if depth == self.config.hierarchy_columns.len() - 1 {
                            let value = value_array
                                .map(|arr| arr.value(row_idx))
                                .unwrap_or(1.0);
                            new_child.value = value;
                        }
                        
                        current_node.add_child(new_child);
                    }
                }
            }
        }
        
        // Calculate values for internal nodes
        root.calculate_value();
        
        // Assign angles
        root.assign_angles(0.0, 2.0 * std::f64::consts::PI);
        
        // Assign colors
        let mut color_index = 0;
        root.assign_colors(&self.config.color_scheme, &mut color_index);
        
        self.root = Some(root);
    }
    
    fn draw_sunburst(&self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        let center = rect.center();
        let radius = rect.width().min(rect.height()) / 2.0 * 0.9;
        
        if let Some(root) = &self.root {
            // Draw from inside out
            let max_depth = self.get_max_depth(root);
            let ring_width = radius * (1.0 - self.config.inner_radius_ratio) / max_depth as f32;
            
            // Use queue for breadth-first traversal
            let mut queue = VecDeque::new();
            queue.push_back(root);
            
            while let Some(node) = queue.pop_front() {
                if node.depth > 0 && node.value > 0.0 {
                    let inner_radius = self.config.inner_radius_ratio * radius + (node.depth - 1) as f32 * ring_width;
                    let outer_radius = inner_radius + ring_width;
                    
                    // Check if node is in selected path for zoom
                    let is_zoomed = self.zoom_level > 0 && 
                        self.selected_path.len() >= node.depth &&
                        node.parent_path[..node.depth.min(self.selected_path.len())] == self.selected_path[..node.depth.min(self.selected_path.len())];
                    
                    if self.zoom_level == 0 || is_zoomed {
                        // Draw arc
                        self.draw_arc(
                            &painter,
                            center,
                            inner_radius,
                            outer_radius,
                            node.angle_start,
                            node.angle_end,
                            node.color,
                            node.name.clone(),
                            node.value,
                            &node.parent_path
                        );
                    }
                }
                
                // Add children to queue
                for child in &node.children {
                    queue.push_back(child);
                }
            }
        }
    }
    
    fn draw_arc(
        &self,
        painter: &egui::Painter,
        center: Pos2,
        inner_radius: f32,
        outer_radius: f32,
        start_angle: f64,
        end_angle: f64,
        color: Color32,
        label: String,
        value: f64,
        path: &[String]
    ) {
        let segments = ((end_angle - start_angle) * 180.0 / std::f64::consts::PI).max(8.0) as usize;
        let mut vertices = Vec::new();
        
        // Create arc vertices
        for i in 0..=segments {
            let angle = start_angle + (end_angle - start_angle) * (i as f64 / segments as f64);
            let cos = angle.cos() as f32;
            let sin = angle.sin() as f32;
            
            vertices.push(center + Vec2::new(inner_radius * cos, inner_radius * sin));
            vertices.push(center + Vec2::new(outer_radius * cos, outer_radius * sin));
        }
        
        // Draw filled arc
        for i in 0..segments {
            let idx = i * 2;
            let quad = vec![
                vertices[idx],
                vertices[idx + 1],
                vertices[idx + 3],
                vertices[idx + 2],
            ];
            
            let is_hovered = self.hovered_node.as_ref()
                .map(|h| h == path)
                .unwrap_or(false);
            
            let fill_color = if is_hovered {
                Color32::from_rgba_unmultiplied(
                    color.r().saturating_add(30),
                    color.g().saturating_add(30),
                    color.b().saturating_add(30),
                    color.a()
                )
            } else {
                color
            };
            
            painter.add(Shape::convex_polygon(
                quad,
                fill_color,
                Stroke::new(1.0, Color32::from_gray(240))
            ));
        }
        
        // Draw label if large enough
        let angle_span = (end_angle - start_angle) as f32;
        let arc_length = angle_span * (inner_radius + outer_radius) / 2.0;
        
        if self.config.show_labels && angle_span > self.config.label_threshold {
            let mid_angle = (start_angle + end_angle) / 2.0;
            let label_radius = (inner_radius + outer_radius) / 2.0;
            let label_pos = center + Vec2::new(
                label_radius * mid_angle.cos() as f32,
                label_radius * mid_angle.sin() as f32
            );
            
            // Rotate text for better readability
            let rotation = if mid_angle > std::f64::consts::PI / 2.0 && mid_angle < 3.0 * std::f64::consts::PI / 2.0 {
                mid_angle as f32 + std::f32::consts::PI
            } else {
                mid_angle as f32
            };
            
            let text = if self.config.show_values {
                format!("{}: {:.0}", label, value)
            } else {
                label
            };
            
            // Simple text without rotation for now
            painter.text(
                label_pos,
                Align2::CENTER_CENTER,
                text,
                FontId::proportional(10.0),
                Color32::from_gray(20),
            );
        }
    }
    
    fn get_max_depth(&self, node: &SunburstNode) -> usize {
        if node.children.is_empty() {
            node.depth
        } else {
            node.children.iter()
                .map(|child| self.get_max_depth(child))
                .max()
                .unwrap_or(node.depth)
        }
    }
    
    fn handle_interaction(&mut self, ui: &mut Ui, rect: Rect) -> Response {
        let response = ui.allocate_rect(rect, Sense::click());
        
        if let Some(hover_pos) = response.hover_pos() {
            let center = rect.center();
            let dx = hover_pos.x - center.x;
            let dy = hover_pos.y - center.y;
            let distance = (dx * dx + dy * dy).sqrt();
            let angle = dy.atan2(dx) as f64;
            let normalized_angle = if angle < 0.0 { angle + 2.0 * std::f64::consts::PI } else { angle };
            
            let radius = rect.width().min(rect.height()) / 2.0 * 0.9;
            let ring_width = radius * (1.0 - self.config.inner_radius_ratio) / 10.0; // Approximate
            
            // Find which ring we're in
            if distance > self.config.inner_radius_ratio * radius {
                let depth = ((distance - self.config.inner_radius_ratio * radius) / ring_width) as usize + 1;
                
                // Find node at this angle and depth
                if let Some(root) = &self.root {
                    if let Some(node) = root.find_node_at_angle(normalized_angle, self.config.inner_radius_ratio, depth) {
                        self.hovered_node = Some(node.parent_path.clone());
                        
                        // Show tooltip
                        let mut tooltip = node.parent_path.join(" → ");
                        tooltip.push_str(&format!("\nValue: {:.1}", node.value));
                        response.clone().on_hover_text(tooltip);
                    } else {
                        self.hovered_node = None;
                    }
                }
            } else {
                self.hovered_node = None;
            }
        }
        
        // Handle click for zoom
        if response.clicked() && self.config.enable_zoom {
            if let Some(hovered) = &self.hovered_node {
                if self.selected_path == *hovered {
                    // Click on same node - zoom out
                    if self.zoom_level > 0 {
                        self.zoom_level -= 1;
                        if self.zoom_level == 0 {
                            self.selected_path.clear();
                        } else {
                            self.selected_path.pop();
                        }
                    }
                } else {
                    // Zoom in
                    self.selected_path = hovered.clone();
                    self.zoom_level = hovered.len();
                }
                self.animation_progress = 0.0;
            }
        }
        
        response
    }
    
    fn draw_breadcrumb(&self, ui: &mut Ui) {
        if !self.selected_path.is_empty() {
            ui.horizontal(|ui| {
                ui.label("📍");
                ui.label("Root");
                for segment in &self.selected_path {
                    ui.label("→");
                    if ui.link(segment).clicked() {
                        // TODO: Zoom to this level
                    }
                }
            });
        }
    }
}

impl SpaceView for SunburstChart {
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
    fn view_type(&self) -> &str { "SunburstView" }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Update data if needed
        if self.cached_data.is_none() {
            let data_sources = ctx.data_sources.read();

            let data_source = data_sources.values().next();
            if let Some(source) = data_source.as_ref() {
                let nav_pos = ctx.navigation.get_context().position.clone();
                if let Ok(batch) = ctx.runtime_handle.block_on(source.query_at(&nav_pos)) {
                    self.cached_data = Some(batch.clone());
                    self.build_hierarchy(&batch);
                }
            }
        }
        
        if self.root.is_some() {
            // Breadcrumb navigation
            if self.config.enable_zoom {
                self.draw_breadcrumb(ui);
            }
            
            // Configuration
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.config.show_labels, "Labels");
                ui.checkbox(&mut self.config.show_values, "Values");
                ui.checkbox(&mut self.config.enable_zoom, "Enable Zoom");
                
                ui.separator();
                ui.label("Inner radius:");
                ui.add(egui::Slider::new(&mut self.config.inner_radius_ratio, 0.0..=0.5));
            });
            
            // Main visualization
            let available_rect = ui.available_rect_before_wrap();
            self.draw_sunburst(ui, available_rect);
            self.handle_interaction(ui, available_rect);
            
            // Animate transitions
            if self.config.animate_transitions && self.animation_progress < 1.0 {
                self.animation_progress += ui.input(|i| i.stable_dt) * 3.0;
                self.animation_progress = self.animation_progress.min(1.0);
                ui.ctx().request_repaint();
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Select hierarchy columns to create a sunburst chart.");
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "hierarchy_columns": self.config.hierarchy_columns,
            "value_column": self.config.value_column,
            "inner_radius_ratio": self.config.inner_radius_ratio,
            "show_labels": self.config.show_labels,
            "show_values": self.config.show_values,
            "enable_zoom": self.config.enable_zoom,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(columns) = config.get("hierarchy_columns").and_then(|v| v.as_array()) {
            self.config.hierarchy_columns = columns.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {}
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {}
} 