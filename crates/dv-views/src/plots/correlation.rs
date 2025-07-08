//! Correlation matrix view implementation

use egui::{Ui, Color32, Rect, Pos2, Vec2, FontId, Align2, Stroke};
use arrow::array::{Float64Array, Int64Array, Array};
use arrow::record_batch::RecordBatch;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;

/// Configuration for correlation matrix view
#[derive(Debug, Clone)]
pub struct CorrelationMatrixConfig {
    pub data_source_id: Option<String>,
    pub columns: Vec<String>,
    pub method: CorrelationMethod,
    pub show_values: bool,
    pub color_scheme: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CorrelationMethod {
    Pearson,
    Spearman,
    Kendall,
}

impl Default for CorrelationMatrixConfig {
    fn default() -> Self {
        Self {
            data_source_id: None,
            columns: Vec::new(),
            method: CorrelationMethod::Pearson,
            show_values: true,
            color_scheme: "diverging".to_string(),
        }
    }
}

/// Cached correlation data
struct CorrelationData {
    correlation_matrix: Vec<Vec<f64>>,
    column_names: Vec<String>,
}

/// Correlation matrix view
pub struct CorrelationMatrixView {
    id: SpaceViewId,
    title: String,
    pub config: CorrelationMatrixConfig,
    cached_data: Option<CorrelationData>,
    last_navigation_pos: Option<NavigationPosition>,
}

impl CorrelationMatrixView {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: CorrelationMatrixConfig::default(),
            cached_data: None,
            last_navigation_pos: None,
        }
    }
    
    fn fetch_data(&mut self, ctx: &ViewerContext) -> Option<CorrelationData> {
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
        
        // Get numeric columns
        let mut numeric_columns = Vec::new();
        let mut column_names = Vec::new();
        
        if self.config.columns.is_empty() {
            // Auto-detect numeric columns
            for (i, field) in batch.schema().fields().iter().enumerate() {
                let column = batch.column(i);
                if Self::is_numeric_column(column) {
                    numeric_columns.push(Self::extract_numeric_values(column));
                    column_names.push(field.name().clone());
                }
            }
        } else {
            // Use specified columns
            for col_name in &self.config.columns {
                if let Some(column) = batch.column_by_name(col_name) {
                    let values = Self::extract_numeric_values(column);
                    if !values.is_empty() {
                        numeric_columns.push(values);
                        column_names.push(col_name.clone());
                    }
                }
            }
        }
        
        if numeric_columns.len() < 2 {
            return None;
        }
        
        // Calculate correlation matrix
        let n = numeric_columns.len();
        let mut correlation_matrix = vec![vec![0.0; n]; n];
        
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    correlation_matrix[i][j] = 1.0;
                } else {
                    correlation_matrix[i][j] = Self::calculate_correlation(
                        &numeric_columns[i],
                        &numeric_columns[j],
                        self.config.method
                    );
                }
            }
        }
        
        Some(CorrelationData {
            correlation_matrix,
            column_names,
        })
    }
    
    fn is_numeric_column(column: &dyn Array) -> bool {
        column.as_any().downcast_ref::<Float64Array>().is_some() ||
        column.as_any().downcast_ref::<Int64Array>().is_some() ||
        column.as_any().downcast_ref::<arrow::array::Int32Array>().is_some() ||
        column.as_any().downcast_ref::<arrow::array::Float32Array>().is_some()
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
    
    fn calculate_correlation(x: &[f64], y: &[f64], method: CorrelationMethod) -> f64 {
        if x.len() != y.len() || x.is_empty() {
            return 0.0;
        }
        
        match method {
            CorrelationMethod::Pearson => {
                let n = x.len() as f64;
                let mean_x = x.iter().sum::<f64>() / n;
                let mean_y = y.iter().sum::<f64>() / n;
                
                let mut cov = 0.0;
                let mut var_x = 0.0;
                let mut var_y = 0.0;
                
                for i in 0..x.len() {
                    let dx = x[i] - mean_x;
                    let dy = y[i] - mean_y;
                    cov += dx * dy;
                    var_x += dx * dx;
                    var_y += dy * dy;
                }
                
                if var_x == 0.0 || var_y == 0.0 {
                    0.0
                } else {
                    cov / (var_x.sqrt() * var_y.sqrt())
                }
            }
            CorrelationMethod::Spearman => {
                // Simple rank correlation (simplified implementation)
                // TODO: Implement proper Spearman correlation
                Self::calculate_correlation(x, y, CorrelationMethod::Pearson)
            }
            CorrelationMethod::Kendall => {
                // Simple implementation (simplified)
                // TODO: Implement proper Kendall correlation
                Self::calculate_correlation(x, y, CorrelationMethod::Pearson)
            }
        }
    }
    
    fn get_color_for_correlation(&self, value: f64) -> Color32 {
        let value = value.clamp(-1.0, 1.0);
        
        match self.config.color_scheme.as_str() {
            "diverging" => {
                if value >= 0.0 {
                    // Blue for positive
                    let intensity = (value * 255.0) as u8;
                    Color32::from_rgb(255 - intensity, 255 - intensity, 255)
                } else {
                    // Red for negative
                    let intensity = (-value * 255.0) as u8;
                    Color32::from_rgb(255, 255 - intensity, 255 - intensity)
                }
            }
            "viridis" => {
                let t = (value + 1.0) / 2.0; // Map [-1, 1] to [0, 1]
                let r = (255.0 * (0.267 + 0.003 * t + 1.785 * t * t - 3.876 * t * t * t + 2.291 * t * t * t * t).clamp(0.0, 1.0)) as u8;
                let g = (255.0 * (0.005 + 1.398 * t - 0.725 * t * t).clamp(0.0, 1.0)) as u8;
                let b = (255.0 * (0.329 + 0.876 * t - 0.170 * t * t - 0.363 * t * t * t).clamp(0.0, 1.0)) as u8;
                Color32::from_rgb(r, g, b)
            }
            "grayscale" => {
                let intensity = ((value + 1.0) / 2.0 * 255.0) as u8;
                Color32::from_gray(intensity)
            }
            _ => {
                // Default to diverging color scheme
                Self::get_color_for_correlation(self, value)
            }
        }
    }
}

impl SpaceView for CorrelationMatrixView {
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
    fn view_type(&self) -> &str { "CorrelationMatrixView" }
    
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
            let available_rect = ui.available_rect_before_wrap();
            let size = available_rect.size();
            let n = data.column_names.len();
            
            // Calculate cell size
            let margin = 100.0; // Space for labels
            let cell_size = ((size.x.min(size.y) - margin) / n as f32).min(50.0);
            
            let painter = ui.painter();
            let rect = Rect::from_min_size(
                available_rect.min + Vec2::new(margin, margin),
                Vec2::splat(cell_size * n as f32)
            );
            
            // Draw cells
            for i in 0..n {
                for j in 0..n {
                    let correlation = data.correlation_matrix[i][j];
                    
                    let cell_rect = Rect::from_min_size(
                        rect.min + Vec2::new(j as f32 * cell_size, i as f32 * cell_size),
                        Vec2::splat(cell_size)
                    );
                    
                    let color = self.get_color_for_correlation(correlation);
                    painter.rect_filled(cell_rect, 0.0, color);
                    
                    // Draw value
                    if self.config.show_values && cell_size > 20.0 {
                        let text_color = if correlation.abs() > 0.5 {
                            Color32::WHITE
                        } else {
                            Color32::BLACK
                        };
                        
                        painter.text(
                            cell_rect.center(),
                            Align2::CENTER_CENTER,
                            format!("{:.2}", correlation),
                            FontId::proportional(10.0),
                            text_color,
                        );
                    }
                }
            }
            
            // Draw labels
            for i in 0..n {
                // Column labels (top)
                painter.text(
                    Pos2::new(
                        rect.min.x + (i as f32 + 0.5) * cell_size,
                        rect.min.y - 5.0
                    ),
                    Align2::CENTER_BOTTOM,
                    &data.column_names[i],
                    FontId::proportional(10.0),
                    Color32::GRAY,
                );
                
                // Row labels (left)
                painter.text(
                    Pos2::new(
                        rect.min.x - 5.0,
                        rect.min.y + (i as f32 + 0.5) * cell_size
                    ),
                    Align2::RIGHT_CENTER,
                    &data.column_names[i],
                    FontId::proportional(10.0),
                    Color32::GRAY,
                );
            }
            
            // Draw color scale legend
            let legend_rect = Rect::from_min_size(
                Pos2::new(rect.max.x + 20.0, rect.min.y),
                Vec2::new(20.0, rect.height())
            );
            
            for i in 0..100 {
                let t = i as f32 / 99.0;
                let value = -1.0 + 2.0 * t as f64;
                let color = self.get_color_for_correlation(value);
                
                let y = legend_rect.min.y + t * legend_rect.height();
                painter.line_segment(
                    [
                        Pos2::new(legend_rect.min.x, y),
                        Pos2::new(legend_rect.max.x, y)
                    ],
                    Stroke::new(2.0, color)
                );
            }
            
            // Legend labels
            painter.text(
                Pos2::new(legend_rect.max.x + 5.0, legend_rect.min.y),
                Align2::LEFT_TOP,
                "1.0",
                FontId::proportional(10.0),
                Color32::GRAY,
            );
            
            painter.text(
                Pos2::new(legend_rect.max.x + 5.0, legend_rect.center().y),
                Align2::LEFT_CENTER,
                "0.0",
                FontId::proportional(10.0),
                Color32::GRAY,
            );
            
            painter.text(
                Pos2::new(legend_rect.max.x + 5.0, legend_rect.max.y),
                Align2::LEFT_BOTTOM,
                "-1.0",
                FontId::proportional(10.0),
                Color32::GRAY,
            );
            
            // Info panel
            ui.allocate_space(Vec2::new(0.0, rect.max.y - available_rect.min.y + 20.0));
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(format!("{} variables", n));
                ui.label(format!("Method: {:?}", self.config.method));
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
                ui.label("Need at least 2 numeric columns for correlation matrix");
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "columns": self.config.columns,
            "method": format!("{:?}", self.config.method),
            "show_values": self.config.show_values,
            "color_scheme": self.config.color_scheme,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        // TODO: Load config
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {}
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {}
} 