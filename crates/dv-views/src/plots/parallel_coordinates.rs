//! Parallel coordinates plot for high-dimensional data visualization

use egui::{Ui, Color32, Rect, Pos2, Vec2, Stroke, FontId, Align2, Shape, Response, Sense, Rounding};
use arrow::record_batch::RecordBatch;
use arrow::array::{Array, Float64Array, StringArray, Int64Array};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use ndarray::{Array1, Array2};

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use super::utils::{ColorScheme, categorical_color, viridis_color, plasma_color};

/// Parallel coordinates configuration
#[derive(Debug, Clone)]
pub struct ParallelCoordinatesConfig {
    pub data_source_id: Option<String>,
    pub columns: Vec<String>,
    pub color_column: Option<String>,
    pub group_column: Option<String>,
    
    // Scaling options
    pub scale_type: ScaleType,
    pub show_outliers: bool,
    pub outlier_threshold: f64,
    
    // Visual options
    pub line_width: f32,
    pub line_opacity: u8,
    pub highlight_opacity: u8,
    pub color_scheme: ColorScheme,
    pub show_axes: bool,
    pub show_ticks: bool,
    pub show_grid: bool,
    pub show_distributions: bool,
    
    // Interaction
    pub enable_brushing: bool,
    pub enable_reordering: bool,
    pub highlight_on_hover: bool,
    pub bundle_lines: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScaleType {
    Linear,
    Normalized,  // 0-1
    Standardized, // z-score
    Logarithmic,
}

impl Default for ParallelCoordinatesConfig {
    fn default() -> Self {
        Self {
            data_source_id: None,
            columns: Vec::new(),
            color_column: None,
            group_column: None,
            scale_type: ScaleType::Normalized,
            show_outliers: true,
            outlier_threshold: 3.0,
            line_width: 1.0,
            line_opacity: 128,
            highlight_opacity: 255,
            color_scheme: ColorScheme::Categorical,
            show_axes: true,
            show_ticks: true,
            show_grid: true,
            show_distributions: true,
            enable_brushing: true,
            enable_reordering: true,
            highlight_on_hover: true,
            bundle_lines: false,
        }
    }
}

/// Axis information
#[derive(Clone, Debug)]
struct Axis {
    name: String,
    position: f32,
    min_value: f64,
    max_value: f64,
    mean: f64,
    std_dev: f64,
    brush_min: Option<f64>,
    brush_max: Option<f64>,
    distribution: Vec<f64>,
}

impl Axis {
    fn scale_value(&self, value: f64, scale_type: &ScaleType) -> f64 {
        match scale_type {
            ScaleType::Linear => {
                (value - self.min_value) / (self.max_value - self.min_value)
            }
            ScaleType::Normalized => {
                (value - self.min_value) / (self.max_value - self.min_value)
            }
            ScaleType::Standardized => {
                if self.std_dev > 0.0 {
                    (value - self.mean) / self.std_dev / 6.0 + 0.5 // Map ±3σ to 0-1
                } else {
                    0.5
                }
            }
            ScaleType::Logarithmic => {
                if value > 0.0 && self.min_value > 0.0 && self.max_value > 0.0 {
                    (value.ln() - self.min_value.ln()) / (self.max_value.ln() - self.min_value.ln())
                } else {
                    0.0
                }
            }
        }
    }
    
    fn unscale_value(&self, scaled: f64, scale_type: &ScaleType) -> f64 {
        match scale_type {
            ScaleType::Linear | ScaleType::Normalized => {
                scaled * (self.max_value - self.min_value) + self.min_value
            }
            ScaleType::Standardized => {
                (scaled - 0.5) * 6.0 * self.std_dev + self.mean
            }
            ScaleType::Logarithmic => {
                if self.min_value > 0.0 && self.max_value > 0.0 {
                    (scaled * (self.max_value.ln() - self.min_value.ln()) + self.min_value.ln()).exp()
                } else {
                    self.min_value
                }
            }
        }
    }
}

/// Data line
#[derive(Clone, Debug)]
struct DataLine {
    values: Vec<Option<f64>>,
    scaled_values: Vec<Option<f64>>,
    color: Color32,
    group: Option<String>,
    is_outlier: bool,
    index: usize,
}

/// Parallel coordinates view
pub struct ParallelCoordinatesPlot {
    id: SpaceViewId,
    title: String,
    pub config: ParallelCoordinatesConfig,
    
    // State
    cached_data: Option<RecordBatch>,
    axes: Vec<Axis>,
    lines: Vec<DataLine>,
    
    // Interaction state
    hovered_line: Option<usize>,
    selected_lines: HashSet<usize>,
    dragging_axis: Option<usize>,
    brushing_axis: Option<usize>,
    axis_order: Vec<usize>,
}

impl ParallelCoordinatesPlot {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: ParallelCoordinatesConfig::default(),
            cached_data: None,
            axes: Vec::new(),
            lines: Vec::new(),
            hovered_line: None,
            selected_lines: HashSet::new(),
            dragging_axis: None,
            brushing_axis: None,
            axis_order: Vec::new(),
        }
    }
    
    fn extract_data(&mut self, batch: &RecordBatch) {
        self.axes.clear();
        self.lines.clear();
        
        // Use all numeric columns if none specified
        if self.config.columns.is_empty() {
            for field in batch.schema().fields() {
                if matches!(field.data_type(), arrow::datatypes::DataType::Float64 | 
                                             arrow::datatypes::DataType::Int64) {
                    self.config.columns.push(field.name().to_string());
                }
            }
        }
        
        // Extract data for each axis
        let mut axis_data: Vec<Vec<f64>> = Vec::new();
        let mut valid_columns = Vec::new();
        
        for col_name in &self.config.columns {
            if let Some(col_idx) = batch.schema().fields().iter()
                .position(|f| f.name() == col_name) {
                
                let col = batch.column(col_idx);
                if let Some(float_array) = col.as_any().downcast_ref::<Float64Array>() {
                    let values: Vec<f64> = (0..float_array.len())
                        .map(|i| float_array.value(i))
                        .collect();
                    
                    if !values.is_empty() {
                        axis_data.push(values);
                        valid_columns.push(col_name.clone());
                    }
                }
            }
        }
        
        // Calculate statistics for each axis
        for (i, (col_name, values)) in valid_columns.iter().zip(&axis_data).enumerate() {
            let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let variance = values.iter()
                .map(|v| (v - mean).powi(2))
                .sum::<f64>() / values.len() as f64;
            let std_dev = variance.sqrt();
            
            // Calculate distribution for violin plot
            let mut distribution = vec![0.0; 50];
            for &value in values {
                let bin = ((value - min) / (max - min) * 49.0) as usize;
                let bin = bin.min(49);
                distribution[bin] += 1.0;
            }
            
            // Normalize distribution
            let max_count = distribution.iter().fold(0.0_f64, |a, &b| a.max(b));
            if max_count > 0.0 {
                for d in &mut distribution {
                    *d /= max_count;
                }
            }
            
            self.axes.push(Axis {
                name: col_name.clone(),
                position: i as f32 / (valid_columns.len() - 1).max(1) as f32,
                min_value: min,
                max_value: max,
                mean,
                std_dev,
                brush_min: None,
                brush_max: None,
                distribution,
            });
        }
        
        // Initialize axis order
        self.axis_order = (0..self.axes.len()).collect();
        
        // Extract color/group information
        let color_array = self.config.color_column.as_ref()
            .and_then(|col_name| {
                batch.schema().fields().iter()
                    .position(|f| f.name() == col_name)
                    .and_then(|idx| batch.column(idx).as_any().downcast_ref::<Float64Array>())
            });
            
        let group_array = self.config.group_column.as_ref()
            .and_then(|col_name| {
                batch.schema().fields().iter()
                    .position(|f| f.name() == col_name)
                    .and_then(|idx| batch.column(idx).as_any().downcast_ref::<StringArray>())
            });
        
        // Build lines from row data
        let num_rows = batch.num_rows();
        for row_idx in 0..num_rows {
            let mut values = Vec::new();
            let mut scaled_values = Vec::new();
            let mut all_valid = true;
            
            // Initialize color calculation outside the column loop
            let color = if let Some(color_arr) = &color_array {
                let color_val = color_arr.value(row_idx);
                let normalized = (color_val / 100.0) as f32; // TODO: proper normalization
                match self.config.color_scheme {
                    ColorScheme::Viridis => viridis_color(normalized),
                    ColorScheme::Plasma => plasma_color(normalized),
                    _ => categorical_color(row_idx),
                }
            } else {
                categorical_color(row_idx)
            };
            
            for col_name in &valid_columns {
                if let Some(col_idx) = batch.schema().fields().iter()
                    .position(|f| f.name() == col_name) {
                    
                    let col = batch.column(col_idx);
                    if let Some(float_array) = col.as_any().downcast_ref::<Float64Array>() {
                        let value = float_array.value(row_idx);
                        values.push(Some(value));
                        
                        // Find corresponding axis and scale value
                        if let Some(axis_idx) = self.axes.iter().position(|a| a.name == *col_name) {
                            let scaled = self.axes[axis_idx].scale_value(value, &self.config.scale_type);
                            scaled_values.push(Some(scaled));
                        } else {
                            scaled_values.push(None);
                        }
                    }
                }
            }
            
            if all_valid || values.iter().filter(|v| v.is_some()).count() >= 2 {
                // Get group
                let group = group_array
                    .map(|arr| arr.value(row_idx).to_string());
                
                // Check if outlier
                let is_outlier = if self.config.show_outliers {
                    scaled_values.iter().any(|&v| {
                        v.map(|val| val < 0.0 || val > 1.0).unwrap_or(false)
                    })
                } else {
                    false
                };
                
                self.lines.push(DataLine {
                    values,
                    scaled_values,
                    color,
                    group,
                    is_outlier,
                    index: row_idx,
                });
            }
        }
    }
    
    fn draw_axes(&self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        
        for &axis_idx in &self.axis_order {
            let axis = &self.axes[axis_idx];
            let x = rect.left() + axis.position * rect.width();
            
            // Draw axis line
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(1.0, Color32::from_gray(200))
            );
            
            // Draw axis label
            painter.text(
                Pos2::new(x, rect.top() - 10.0),
                Align2::CENTER_BOTTOM,
                &axis.name,
                FontId::proportional(12.0),
                Color32::from_gray(200),
            );
            
            // Draw ticks and values
            if self.config.show_ticks {
                for i in 0..=5 {
                    let y = rect.top() + (i as f32 / 5.0) * rect.height();
                    let value = axis.unscale_value(1.0 - i as f64 / 5.0, &self.config.scale_type);
                    
                    // Tick mark
                    painter.line_segment(
                        [Pos2::new(x - 3.0, y), Pos2::new(x + 3.0, y)],
                        Stroke::new(1.0, Color32::from_gray(180))
                    );
                    
                    // Value label
                    painter.text(
                        Pos2::new(x - 5.0, y),
                        Align2::RIGHT_CENTER,
                        format!("{:.1}", value),
                        FontId::proportional(9.0),
                        Color32::from_gray(150),
                    );
                }
            }
            
            // Draw distribution violin if enabled
            if self.config.show_distributions {
                let violin_width = 20.0;
                let mut points_left = Vec::new();
                let mut points_right = Vec::new();
                
                for (i, &density) in axis.distribution.iter().enumerate() {
                    let y = rect.top() + (i as f32 / (axis.distribution.len() - 1) as f32) * rect.height();
                    let width = density * violin_width;
                    points_left.push(Pos2::new(x - width as f32, y));
                    points_right.push(Pos2::new(x + width as f32, y));
                }
                
                // Combine points for closed shape
                let mut violin_points = points_left;
                violin_points.extend(points_right.into_iter().rev());
                
                if violin_points.len() > 2 {
                    painter.add(Shape::convex_polygon(
                        violin_points,
                        Color32::from_rgba_unmultiplied(100, 100, 200, 30),
                        Stroke::new(0.5, Color32::from_rgba_unmultiplied(100, 100, 200, 60))
                    ));
                }
            }
            
            // Draw brush if active
            if let (Some(min), Some(max)) = (axis.brush_min, axis.brush_max) {
                let min_scaled = axis.scale_value(min, &self.config.scale_type);
                let max_scaled = axis.scale_value(max, &self.config.scale_type);
                
                let brush_top = rect.top() + (1.0 - max_scaled as f32) * rect.height();
                let brush_bottom = rect.top() + (1.0 - min_scaled as f32) * rect.height();
                
                let brush_rect = Rect::from_min_max(
                    Pos2::new(x - 10.0, brush_top),
                    Pos2::new(x + 10.0, brush_bottom)
                );
                
                painter.rect_filled(
                    brush_rect,
                    Rounding::ZERO,
                    Color32::from_rgba_unmultiplied(255, 200, 0, 50)
                );
                painter.rect_stroke(
                    brush_rect,
                    Rounding::ZERO,
                    Stroke::new(2.0, Color32::from_rgb(255, 200, 0))
                );
            }
        }
    }
    
    fn draw_lines(&self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        
        // Draw lines in layers: unselected, selected, hovered
        let layers = [
            (false, false, self.config.line_opacity),
            (true, false, self.config.highlight_opacity),
            (false, true, self.config.highlight_opacity),
        ];
        
        for (is_selected, is_hovered, opacity) in layers {
            for (line_idx, line) in self.lines.iter().enumerate() {
                let line_selected = self.selected_lines.contains(&line_idx);
                let line_hovered = self.hovered_line == Some(line_idx);
                
                if line_selected != is_selected || line_hovered != is_hovered {
                    continue;
                }
                
                // Check if line passes all brushes
                let passes_brushes = self.line_passes_brushes(line);
                if !passes_brushes && !line_selected && !line_hovered {
                    continue;
                }
                
                // Build path
                let mut points = Vec::new();
                for (i, &axis_idx) in self.axis_order.iter().enumerate() {
                    if let Some(scaled_value) = line.scaled_values.get(axis_idx).and_then(|&v| v) {
                        let axis = &self.axes[axis_idx];
                        let x = rect.left() + axis.position * rect.width();
                        let y = rect.top() + (1.0 - scaled_value as f32) * rect.height();
                        points.push(Pos2::new(x, y));
                    }
                }
                
                if points.len() >= 2 {
                    let color = if line.is_outlier && self.config.show_outliers {
                        Color32::from_rgba_unmultiplied(255, 100, 100, opacity)
                    } else {
                        Color32::from_rgba_unmultiplied(
                            line.color.r(),
                            line.color.g(),
                            line.color.b(),
                            opacity
                        )
                    };
                    
                    let width = if line_hovered || line_selected {
                        self.config.line_width * 2.0
                    } else {
                        self.config.line_width
                    };
                    
                    // Draw polyline
                    for window in points.windows(2) {
                        painter.line_segment(
                            [window[0], window[1]],
                            Stroke::new(width, color)
                        );
                    }
                    
                    // Draw points at axes
                    if line_hovered {
                        for point in &points {
                            painter.circle_filled(*point, 3.0, color);
                            painter.circle_stroke(*point, 3.0, Stroke::new(1.0, Color32::WHITE));
                        }
                    }
                }
            }
        }
    }
    
    fn line_passes_brushes(&self, line: &DataLine) -> bool {
        for (axis_idx, axis) in self.axes.iter().enumerate() {
            if let (Some(brush_min), Some(brush_max)) = (axis.brush_min, axis.brush_max) {
                if let Some(Some(value)) = line.values.get(axis_idx) {
                    if *value < brush_min || *value > brush_max {
                        return false;
                    }
                }
            }
        }
        true
    }
    
    fn handle_interaction(&mut self, ui: &mut Ui, rect: Rect) -> Response {
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        
        if let Some(hover_pos) = response.hover_pos() {
            // Check for axis hover/drag
            if self.config.enable_reordering {
                for (i, &axis_idx) in self.axis_order.iter().enumerate() {
                    let axis = &self.axes[axis_idx];
                    let x = rect.left() + axis.position * rect.width();
                    
                    if (hover_pos.x - x).abs() < 10.0 {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        
                        if response.drag_started() {
                            self.dragging_axis = Some(i);
                        }
                    }
                }
            }
            
            // Check for brush interaction
            if self.config.enable_brushing && response.drag_started() {
                for &axis_idx in &self.axis_order {
                    let axis = &self.axes[axis_idx];
                    let x = rect.left() + axis.position * rect.width();
                    
                    if (hover_pos.x - x).abs() < 20.0 {
                        self.brushing_axis = Some(axis_idx);
                        let value = 1.0 - (hover_pos.y - rect.top()) / rect.height();
                        let real_value = axis.unscale_value(value as f64, &self.config.scale_type);
                        self.axes[axis_idx].brush_min = Some(real_value);
                        self.axes[axis_idx].brush_max = Some(real_value);
                        break;
                    }
                }
            }
            
            // Update brush while dragging
            if let Some(axis_idx) = self.brushing_axis {
                if response.dragged() {
                    let axis = &self.axes[axis_idx];
                    let value = 1.0 - (hover_pos.y - rect.top()) / rect.height();
                    let real_value = axis.unscale_value(value as f64, &self.config.scale_type);
                    
                    if let Some(min) = self.axes[axis_idx].brush_min {
                        self.axes[axis_idx].brush_min = Some(min.min(real_value));
                        self.axes[axis_idx].brush_max = Some(min.max(real_value));
                    }
                }
            }
            
            // Find hovered line
            if self.config.highlight_on_hover && self.dragging_axis.is_none() && self.brushing_axis.is_none() {
                self.hovered_line = None;
                let mut min_dist = 10.0; // Pixel threshold
                
                for (line_idx, line) in self.lines.iter().enumerate() {
                    // Check distance to line segments
                    for window in self.axis_order.windows(2) {
                        let axis1_idx = window[0];
                        let axis2_idx = window[1];
                        
                        if let (Some(Some(val1)), Some(Some(val2))) = (
                            line.scaled_values.get(axis1_idx),
                            line.scaled_values.get(axis2_idx)
                        ) {
                            let axis1 = &self.axes[axis1_idx];
                            let axis2 = &self.axes[axis2_idx];
                            
                            let p1 = Pos2::new(
                                rect.left() + axis1.position * rect.width(),
                                rect.top() + (1.0 - *val1 as f32) * rect.height()
                            );
                            let p2 = Pos2::new(
                                rect.left() + axis2.position * rect.width(),
                                rect.top() + (1.0 - *val2 as f32) * rect.height()
                            );
                            
                            // Distance from point to line segment
                            let dist = distance_to_segment(hover_pos, p1, p2);
                            if dist < min_dist {
                                min_dist = dist;
                                self.hovered_line = Some(line_idx);
                            }
                        }
                    }
                }
            }
        }
        
        // Handle drag end
        if response.drag_released() {
            self.dragging_axis = None;
            self.brushing_axis = None;
        }
        
        // Handle axis reordering while dragging
        if let Some(dragging_idx) = self.dragging_axis {
            if response.dragged() {
                if let Some(hover_pos) = response.hover_pos() {
                    // Find new position
                    let rel_x = (hover_pos.x - rect.left()) / rect.width();
                    let new_idx = (rel_x * self.axes.len() as f32) as usize;
                    let new_idx = new_idx.clamp(0, self.axes.len() - 1);
                    
                    if new_idx != dragging_idx {
                        // Reorder axes
                        let dragged_axis = self.axis_order.remove(dragging_idx);
                        self.axis_order.insert(new_idx, dragged_axis);
                        self.dragging_axis = Some(new_idx);
                        
                        // Update positions
                        for (i, &axis_idx) in self.axis_order.iter().enumerate() {
                            self.axes[axis_idx].position = i as f32 / (self.axes.len() - 1).max(1) as f32;
                        }
                    }
                }
            }
        }
        
        // Handle click for selection
        if response.clicked() && self.hovered_line.is_some() {
            let line_idx = self.hovered_line.unwrap();
            if self.selected_lines.contains(&line_idx) {
                self.selected_lines.remove(&line_idx);
            } else {
                if !ui.input(|i| i.modifiers.ctrl) {
                    self.selected_lines.clear();
                }
                self.selected_lines.insert(line_idx);
            }
        }
        
        // Show tooltip
        if let Some(line_idx) = self.hovered_line {
            if let Some(line) = self.lines.get(line_idx) {
                let mut tooltip = String::new();
                for (i, &axis_idx) in self.axis_order.iter().enumerate() {
                    let axis = &self.axes[axis_idx];
                    if let Some(Some(value)) = line.values.get(axis_idx) {
                        if i > 0 {
                            tooltip.push_str("\n");
                        }
                        tooltip.push_str(&format!("{}: {:.2}", axis.name, value));
                    }
                }
                if let Some(group) = &line.group {
                    tooltip.push_str(&format!("\nGroup: {}", group));
                }
                response.clone().on_hover_text(tooltip);
            }
        }
        
        // Context menu
        let response = response.context_menu(|ui| {
            if ui.button("Clear Brushes").clicked() {
                for axis in &mut self.axes {
                    axis.brush_min = None;
                    axis.brush_max = None;
                }
                ui.close_menu();
            }
            
            if ui.button("Clear Selection").clicked() {
                self.selected_lines.clear();
                ui.close_menu();
            }
            
            if ui.button("Reset Axis Order").clicked() {
                self.axis_order = (0..self.axes.len()).collect();
                let num_axes = self.axes.len();
                for (i, axis) in self.axes.iter_mut().enumerate() {
                    axis.position = i as f32 / (num_axes - 1).max(1) as f32;
                }
                ui.close_menu();
            }
        });
        
        response
    }
    
    fn draw_info_panel(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(format!("Dimensions: {}", self.axes.len()));
            ui.label(format!("Data points: {}", self.lines.len()));
            
            let selected_count = self.selected_lines.len();
            if selected_count > 0 {
                ui.label(format!("Selected: {}", selected_count));
            }
            
            let brushed_axes = self.axes.iter().filter(|a| a.brush_min.is_some()).count();
            if brushed_axes > 0 {
                ui.label(format!("Brushed axes: {}", brushed_axes));
            }
            
            ui.separator();
            ui.label("🖱 Drag axes to reorder • Drag on axis to brush • Click lines to select");
        });
    }
}

// Helper function to calculate distance from point to line segment
fn distance_to_segment(point: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let ap = point - a;
    let ab_squared = ab.x * ab.x + ab.y * ab.y;
    
    if ab_squared == 0.0 {
        return ap.length();
    }
    
    let t = ((ap.x * ab.x + ap.y * ab.y) / ab_squared).clamp(0.0, 1.0);
    let projection = a + ab * t;
    (point - projection).length()
}

impl SpaceView for ParallelCoordinatesPlot {
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
    fn view_type(&self) -> &str { "ParallelCoordinatesView" }
    
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

            let data_source = if let Some(source_id) = &self.config.data_source_id {
        data_sources.get(source_id)
    } else {
        data_sources.values().next()
    };
            if let Some(source) = data_source.as_ref() {
                let nav_pos = ctx.navigation.get_context().position.clone();
                if let Ok(batch) = ctx.runtime_handle.block_on(source.query_at(&nav_pos)) {
                    self.cached_data = Some(batch.clone());
                    self.extract_data(&batch);
                }
            }
        }
        
        if self.cached_data.is_some() && !self.axes.is_empty() {
            // Main plot area
            let available_rect = ui.available_rect_before_wrap();
            let plot_rect = Rect::from_min_size(
                available_rect.left_top() + Vec2::new(50.0, 30.0),
                available_rect.size() - Vec2::new(100.0, 60.0)
            );
            
            // Draw
            self.draw_axes(ui, plot_rect);
            self.draw_lines(ui, plot_rect);
            
            // Handle interaction
            self.handle_interaction(ui, plot_rect);
            
            // Info panel
            self.draw_info_panel(ui);
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No numeric data available for parallel coordinates plot.");
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "columns": self.config.columns,
            "color_column": self.config.color_column,
            "group_column": self.config.group_column,
            "scale_type": format!("{:?}", self.config.scale_type),
            "line_width": self.config.line_width,
            "color_scheme": format!("{:?}", self.config.color_scheme),
            "show_distributions": self.config.show_distributions,
            "axis_order": self.axis_order,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(columns) = config.get("columns").and_then(|v| v.as_array()) {
            self.config.columns = columns.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
        }
        if let Some(order) = config.get("axis_order").and_then(|v| v.as_array()) {
            self.axis_order = order.iter()
                .filter_map(|v| v.as_u64())
                .map(|n| n as usize)
                .collect();
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {}
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {}
} 