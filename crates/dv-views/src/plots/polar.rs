//! Polar plot visualization

use std::sync::Arc;
use egui::{Ui, Pos2, Vec2, Color32, Stroke, Rect};
use egui_plot::{Plot, PlotPoints, Line, Legend, MarkerShape, Points};
use dv_core::ViewerContext;
use arrow::array::{Float64Array, Array};
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;

/// Configuration for polar plot
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PolarPlotConfig {
    pub data_source_id: Option<String>,
    pub angle_column: String,
    pub radius_column: String,
    pub category_column: Option<String>,
    pub angle_in_degrees: bool,
    pub show_grid: bool,
    pub show_legend: bool,
}

impl Default for PolarPlotConfig {
    fn default() -> Self {
        Self {
            data_source_id: None,
            angle_column: String::new(),
            radius_column: String::new(),
            category_column: None,
            angle_in_degrees: true,
            show_grid: true,
            show_legend: true,
        }
    }
}

/// Polar plot visualization
pub struct PolarPlot {
    pub id: uuid::Uuid,
    pub title: String,
    pub config: PolarPlotConfig,
}

impl PolarPlot {
    pub fn new(id: uuid::Uuid, title: String) -> Self {
        Self {
            id,
            title,
            config: PolarPlotConfig::default(),
        }
    }
    
    pub fn ui(&mut self, ui: &mut Ui, viewer_context: &ViewerContext) {
        // Configuration panel
        egui::CollapsingHeader::new("Configuration")
            .default_open(false)
            .show(ui, |ui| {
                self.show_config(ui, viewer_context);
            });
        
        ui.separator();
        
        // Main plot area
        let data_sources = viewer_context.data_sources.read();
        let data_source = if let Some(source_id) = &self.config.data_source_id {
            data_sources.get(source_id)
        } else {
            data_sources.values().next()
        };
        
        if let Some(data_source) = data_source {
            let schema = viewer_context.runtime_handle.block_on(data_source.schema());
            
            if self.config.angle_column.is_empty() || self.config.radius_column.is_empty() {
                ui.label("Please configure angle and radius columns");
                return;
            }
            
            // Get current data
            let nav_pos = viewer_context.navigation.get_context().position.clone();
            match viewer_context.runtime_handle.block_on(data_source.query_at(&nav_pos)) {
                Ok(batch) => {
                    self.render_polar_plot(ui, &batch, &schema);
                }
                Err(e) => {
                    ui.colored_label(Color32::RED, format!("Error loading data: {}", e));
                }
            }
        } else {
            ui.label("No data source loaded");
        }
    }
    
    fn show_config(&mut self, ui: &mut Ui, viewer_context: &ViewerContext) {
        let data_sources = viewer_context.data_sources.read();
        let data_source = if let Some(source_id) = &self.config.data_source_id {
            data_sources.get(source_id)
        } else {
            data_sources.values().next()
        };
        
        if let Some(data_source) = data_source {
            let schema = viewer_context.runtime_handle.block_on(data_source.schema());
            
            // Get numeric columns
            let numeric_columns: Vec<String> = schema.fields()
                .iter()
                .filter(|f| matches!(f.data_type(), 
                    arrow::datatypes::DataType::Float64 | 
                    arrow::datatypes::DataType::Float32 | 
                    arrow::datatypes::DataType::Int64 | 
                    arrow::datatypes::DataType::Int32))
                .map(|f| f.name().clone())
                .collect();
            
            // Get categorical columns
            let categorical_columns: Vec<String> = schema.fields()
                .iter()
                .filter(|f| matches!(f.data_type(), arrow::datatypes::DataType::Utf8))
                .map(|f| f.name().clone())
                .collect();
            
            ui.horizontal(|ui| {
                ui.label("Angle:");
                egui::ComboBox::from_id_source(format!("polar_angle_{}", self.id))
                    .selected_text(&self.config.angle_column)
                    .show_ui(ui, |ui| {
                        for col in &numeric_columns {
                            ui.selectable_value(&mut self.config.angle_column, col.clone(), col);
                        }
                    });
            });
            
            ui.horizontal(|ui| {
                ui.label("Radius:");
                egui::ComboBox::from_id_source(format!("polar_radius_{}", self.id))
                    .selected_text(&self.config.radius_column)
                    .show_ui(ui, |ui| {
                        for col in &numeric_columns {
                            ui.selectable_value(&mut self.config.radius_column, col.clone(), col);
                        }
                    });
            });
            
            ui.horizontal(|ui| {
                ui.label("Category:");
                let current = self.config.category_column.as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("None");
                
                egui::ComboBox::from_id_source(format!("polar_category_{}", self.id))
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(self.config.category_column.is_none(), "None").clicked() {
                            self.config.category_column = None;
                        }
                        for col in &categorical_columns {
                            let selected = self.config.category_column.as_ref() == Some(col);
                            if ui.selectable_label(selected, col).clicked() {
                                self.config.category_column = Some(col.clone());
                            }
                        }
                    });
            });
            
            ui.separator();
            
            ui.checkbox(&mut self.config.angle_in_degrees, "Angles in degrees");
            ui.checkbox(&mut self.config.show_grid, "Show grid");
            ui.checkbox(&mut self.config.show_legend, "Show legend");
        }
    }
    
    fn render_polar_plot(&self, ui: &mut Ui, batch: &RecordBatch, _schema: &Arc<Schema>) {
        // Get angle and radius data
        let angle_array = match batch.column_by_name(&self.config.angle_column) {
            Some(col) => col,
            None => {
                ui.label(format!("Column '{}' not found", self.config.angle_column));
                return;
            }
        };
        
        let radius_array = match batch.column_by_name(&self.config.radius_column) {
            Some(col) => col,
            None => {
                ui.label(format!("Column '{}' not found", self.config.radius_column));
                return;
            }
        };
        
        // Convert to f64 arrays
        let angles = match angle_array.as_any().downcast_ref::<Float64Array>() {
            Some(arr) => arr,
            None => {
                ui.label(format!("Column '{}' is not numeric", self.config.angle_column));
                return;
            }
        };
        
        let radii = match radius_array.as_any().downcast_ref::<Float64Array>() {
            Some(arr) => arr,
            None => {
                ui.label(format!("Column '{}' is not numeric", self.config.radius_column));
                return;
            }
        };
        
        // Convert polar to cartesian coordinates
        let mut points = Vec::new();
        for i in 0..angles.len().min(radii.len()) {
            if angles.is_valid(i) && radii.is_valid(i) {
                let mut angle = angles.value(i);
                if self.config.angle_in_degrees {
                    angle = angle.to_radians();
                }
                let radius = radii.value(i);
                
                let x = radius * angle.cos();
                let y = radius * angle.sin();
                points.push([x, y]);
            }
        }
        
        // Create plot
        let plot = Plot::new(format!("polar_plot_{}", self.id))
            .data_aspect(1.0) // Keep aspect ratio 1:1 for circular plot
            .legend(Legend::default())
            .show_axes([self.config.show_grid, self.config.show_grid]);
        
        let response = plot.show(ui, |plot_ui| {
            // Add polar grid if enabled
            if self.config.show_grid {
                // Draw concentric circles
                let max_radius = points.iter()
                    .map(|p| (p[0] * p[0] + p[1] * p[1]).sqrt())
                    .fold(0.0f64, f64::max);
                
                let num_circles = 5;
                for i in 1..=num_circles {
                    let r = max_radius * (i as f64) / (num_circles as f64);
                    let circle_points: Vec<[f64; 2]> = (0..=360)
                        .map(|deg| {
                            let angle = (deg as f64).to_radians();
                            [r * angle.cos(), r * angle.sin()]
                        })
                        .collect();
                    
                    plot_ui.line(
                        Line::new(PlotPoints::new(circle_points))
                            .color(Color32::from_gray(60))
                            .width(0.5)
                            .name(format!("r={:.1}", r))
                    );
                }
                
                // Draw radial lines
                for angle_deg in (0..360).step_by(30) {
                    let angle = (angle_deg as f64).to_radians();
                    let line_points = vec![
                        [0.0, 0.0],
                        [max_radius * angle.cos(), max_radius * angle.sin()]
                    ];
                    
                    plot_ui.line(
                        Line::new(PlotPoints::new(line_points))
                            .color(Color32::from_gray(60))
                            .width(0.5)
                            .name(format!("{}°", angle_deg))
                    );
                }
            }
            
            // Plot data points
            if self.config.category_column.is_none() {
                // Single series
                plot_ui.points(
                    Points::new(PlotPoints::new(points))
                        .color(Color32::from_rgb(100, 150, 250))
                        .radius(5.0)
                        .shape(MarkerShape::Circle)
                        .name(&self.title)
                );
            } else {
                // TODO: Implement categorized polar plot
                plot_ui.points(
                    Points::new(PlotPoints::new(points))
                        .color(Color32::from_rgb(100, 150, 250))
                        .radius(5.0)
                        .shape(MarkerShape::Circle)
                        .name(&self.title)
                );
            }
        });
    }
} 