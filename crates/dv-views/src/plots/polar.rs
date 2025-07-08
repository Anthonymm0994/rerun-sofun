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
    
    pub fn config_mut(&mut self) -> &mut PolarPlotConfig {
        &mut self.config
    }
    
    pub fn ui(&mut self, ui: &mut Ui, viewer_context: &ViewerContext) {
        // Simple configuration - just angles in degrees
        ui.checkbox(&mut self.config.angle_in_degrees, "Angles in degrees");
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
    

    
    fn render_polar_plot(&self, ui: &mut Ui, batch: &RecordBatch, _schema: &Arc<Schema>) {
        tracing::info!("Rendering polar plot with {} rows", batch.num_rows());
        tracing::debug!("Angle column: '{}', Radius column: '{}', Category column: {:?}", 
                   self.config.angle_column, self.config.radius_column, self.config.category_column);
        
        // Get angle and radius data
        let angle_array = match batch.column_by_name(&self.config.angle_column) {
            Some(col) => {
                tracing::debug!("Found angle column '{}' with type {:?}", self.config.angle_column, col.data_type());
                col
            },
            None => {
                tracing::error!("Column '{}' not found in batch", self.config.angle_column);
                ui.label(format!("Column '{}' not found", self.config.angle_column));
                return;
            }
        };
        
        let radius_array = match batch.column_by_name(&self.config.radius_column) {
            Some(col) => {
                tracing::debug!("Found radius column '{}' with type {:?}", self.config.radius_column, col.data_type());
                col
            },
            None => {
                tracing::error!("Column '{}' not found in batch", self.config.radius_column);
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
        
        // Get category data if specified
        let categories = if let Some(cat_column) = &self.config.category_column {
            batch.column_by_name(cat_column).and_then(|col| {
                if let Some(str_array) = col.as_any().downcast_ref::<arrow::array::StringArray>() {
                    Some((0..str_array.len()).map(|i| {
                        if str_array.is_null(i) { 
                            "null".to_string() 
                        } else { 
                            str_array.value(i).to_string() 
                        }
                    }).collect::<Vec<_>>())
                } else {
                    // Try to convert other types to string
                    Some((0..col.len()).map(|i| {
                        arrow::util::display::array_value_to_string(col, i).unwrap_or_else(|_| "null".to_string())
                    }).collect())
                }
            })
        } else {
            None
        };
        
        // Group points by category
        let mut categorized_points: std::collections::HashMap<String, Vec<[f64; 2]>> = std::collections::HashMap::new();
        
        if let Some(cats) = &categories {
            // Convert polar to cartesian coordinates grouped by category
            for i in 0..angles.len().min(radii.len()).min(cats.len()) {
                if angles.is_valid(i) && radii.is_valid(i) {
                    let mut angle = angles.value(i);
                    if self.config.angle_in_degrees {
                        angle = angle.to_radians();
                    }
                    let radius = radii.value(i);
                    
                    let x = radius * angle.cos();
                    let y = radius * angle.sin();
                    
                    let category = &cats[i];
                    categorized_points.entry(category.clone())
                        .or_insert_with(Vec::new)
                        .push([x, y]);
                }
            }
        } else {
            // No categories - put all points in one group
            let mut all_points = Vec::new();
            for i in 0..angles.len().min(radii.len()) {
                if angles.is_valid(i) && radii.is_valid(i) {
                    let mut angle = angles.value(i);
                    if self.config.angle_in_degrees {
                        angle = angle.to_radians();
                    }
                    let radius = radii.value(i);
                    
                    let x = radius * angle.cos();
                    let y = radius * angle.sin();
                    all_points.push([x, y]);
                }
            }
            categorized_points.insert("All".to_string(), all_points);
        }
        
        // Create plot
        let mut plot = Plot::new(format!("polar_plot_{}", self.id))
            .data_aspect(1.0) // Keep aspect ratio 1:1 for circular plot
            .show_axes([false, false]); // Don't show cartesian axes for polar plot
            
        if self.config.show_legend {
            plot = plot.legend(Legend::default());
        }
        
        let response = plot.show(ui, |plot_ui| {
            // Add polar grid if enabled
            if self.config.show_grid {
                // Draw concentric circles
                let max_radius = categorized_points.values()
                    .flat_map(|points| points.iter())
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
                    );
                }
            }
            
            // Plot data points by category
            let colors = [
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
            
            // Sort categories for stable color assignment
            let mut sorted_categories: Vec<_> = categorized_points.keys().cloned().collect();
            sorted_categories.sort();
            
            tracing::debug!("Polar plot categories: {:?}", sorted_categories);
            tracing::info!("Polar plot total points: {}", categorized_points.values().map(|v| v.len()).sum::<usize>());
            
            for (idx, category) in sorted_categories.iter().enumerate() {
                if let Some(points) = categorized_points.get(category) {
                    if !points.is_empty() {
                        let color = colors[idx % colors.len()];
                        tracing::trace!("Plotting category '{}' with {} points, color idx {}", category, points.len(), idx);
                        plot_ui.points(
                            Points::new(PlotPoints::new(points.clone()))
                                .color(color)
                                .radius(5.0)
                                .shape(MarkerShape::Circle)
                                .name(category)
                        );
                    }
                }
            }
        });
    }
} 
