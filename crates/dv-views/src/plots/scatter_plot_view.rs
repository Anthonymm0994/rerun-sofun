//! Scatter plot view implementation

use egui::{Ui, Vec2, Color32, Align2};
use egui_plot::{Plot, PlotPoints, Points, Legend, PlotBounds, Line, LineStyle, PlotPoint};
use arrow::array::{Float64Array, StringArray};
use arrow::datatypes::DataType;
use std::sync::Arc;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use tracing::{debug, error};

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;
use dv_core::state::ViewerContext as DVViewerContext;
use dv_data::schema::ColumnInfo;

impl SpaceView for ScatterPlotView {
    fn id(&self) -> &SpaceViewId {
        &self.id
    }
    
    fn display_name(&self) -> &str {
        &self.config.title
    }
    
    fn view_type(&self) -> &str {
        "ScatterPlotView"
    }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Update data if navigation changed
        let nav_pos = ctx.navigation.get_context().position.clone();
        if self.last_navigation_pos.as_ref() != Some(&nav_pos) {
            self.cached_data = self.fetch_plot_data(ctx);
            self.last_navigation_pos = Some(nav_pos);
        }
        
        // Draw the plot
        if let Some(data) = &self.cached_data {
            // Check keyboard modifiers
            let modifiers = ui.input(|i| i.modifiers);
            let pointer_down = ui.input(|i| i.pointer.primary_down());
            
            // Configure plot with proper axis labels
            let x_axis_name = &self.config.x_column;
            let y_axis_name = &self.config.y_column;
            
            let plot = Plot::new(format!("{:?}", self.id))
                .legend(Legend::default())
                .show_grid(self.config.show_grid)
                // DISABLE auto bounds completely
                .auto_bounds(egui::Vec2b::new(false, false))
                // NEVER allow scroll wheel zoom
                .allow_scroll(false)
                // Allow zoom with explicit controls
                .allow_zoom(true)
                // Allow drag for panning
                .allow_drag(true)
                // Right-click drag for box zoom
                .allow_boxed_zoom(true)
                .data_aspect(1.0);
            
            // Calculate bounds from data
            let mut x_min = f64::INFINITY;
            let mut x_max = -f64::INFINITY;
            let mut y_min = f64::INFINITY;
            let mut y_max = -f64::INFINITY;
            
            for &(x, y) in &data.points {
                if x.is_finite() {
                    x_min = x_min.min(x);
                    x_max = x_max.max(x);
                }
                if y.is_finite() {
                    y_min = y_min.min(y);
                    y_max = y_max.max(y);
                }
            }
            
            // Apply fixed bounds with padding using include_x/include_y
            let plot = if x_min.is_finite() && x_max.is_finite() {
                let x_padding = (x_max - x_min) * 0.1;
                plot.include_x(x_min - x_padding)
                    .include_x(x_max + x_padding)
            } else {
                plot
            };
            
            let plot = if y_min.is_finite() && y_max.is_finite() {
                let y_padding = (y_max - y_min) * 0.1;
                plot.include_y(y_min - y_padding)
                    .include_y(y_max + y_padding)
            } else {
                plot
            };
            
            // Show data range info
            ui.horizontal(|ui| {
                if let (Some(x_range), Some(y_range)) = (&data.x_range, &data.y_range) {
                    ui.label(format!("{}: {:.2} to {:.2}", x_axis_name, x_range.0, x_range.1));
                    ui.separator();
                    ui.label(format!("{}: {:.2} to {:.2}", y_axis_name, y_range.0, y_range.1));
                    ui.separator();
                }
                ui.label(format!("Points: {}", data.points.len()));
            });
            ui.separator();
            
            let mut clicked_point = None;
            
            // Get drag delta for threshold detection OUTSIDE plot closure
            let drag_delta = ui.input(|i| i.pointer.delta()).length();
            
            plot.show(ui, |plot_ui| {
                // Group points by color category
                let mut series_map: std::collections::HashMap<String, Vec<[f64; 2]>> = 
                    std::collections::HashMap::new();
                let mut series_indices: std::collections::HashMap<String, Vec<usize>> = 
                    std::collections::HashMap::new();
                
                for (idx, point) in data.points.iter().enumerate() {
                    let series_name = point.color_category.clone()
                        .unwrap_or_else(|| "Data".to_string());
                    series_map.entry(series_name.clone())
                        .or_insert_with(Vec::new)
                        .push([point.x, point.y]);
                    series_indices.entry(series_name)
                        .or_insert_with(Vec::new)
                        .push(idx);
                }
                
                // Draw each series
                let mut series_idx = 0;
                for (series_name, points) in series_map.iter() {
                    let plot_points = PlotPoints::new(points.clone());
                    
                    // Choose color
                    let color = if self.config.color_column.is_some() {
                        let colors = [
                            Color32::from_rgb(31, 119, 180),   // Blue
                            Color32::from_rgb(255, 127, 14),   // Orange
                            Color32::from_rgb(44, 160, 44),    // Green
                            Color32::from_rgb(214, 39, 40),    // Red
                            Color32::from_rgb(148, 103, 189),  // Purple
                            Color32::from_rgb(140, 86, 75),    // Brown
                            Color32::from_rgb(227, 119, 194),  // Pink
                            Color32::from_rgb(127, 127, 127),  // Gray
                        ];
                        colors[series_idx % colors.len()]
                    } else {
                        Color32::from_rgb(31, 119, 180) // Default blue
                    };
                    
                    let points_plot = Points::new(plot_points)
                        .color(color)
                        .radius(self.config.point_radius)
                        .shape(egui_plot::MarkerShape::Circle)
                        .name(series_name);
                    plot_ui.points(points_plot);
                    
                    // Highlight hovered/selected points
                    if let Some(hover_idx) = &ctx.hovered_data.read().point_index {
                        if let Some(indices) = series_indices.get(series_name) {
                            if let Some(local_idx) = indices.iter().position(|&i| i == *hover_idx) {
                                if local_idx < points.len() {
                                    let highlight_point = Points::new(vec![points[local_idx]])
                                        .color(color.gamma_multiply(1.5))
                                        .radius(self.config.point_radius * 2.5)
                                        .shape(egui_plot::MarkerShape::Circle);
                                    plot_ui.points(highlight_point);
                                    
                                    // Show value tooltip
                                    let point = points[local_idx];
                                    let text = egui_plot::Text::new(
                                        PlotPoint::new(point[0], point[1]),
                                        egui::RichText::new(format!("{}: ({:.3}, {:.3})", series_name, point[0], point[1]))
                                            .color(Color32::WHITE)
                                            .text_style(egui::TextStyle::Small)
                                    )
                                    .anchor(Align2::LEFT_BOTTOM);
                                    plot_ui.text(text);
                                }
                            }
                        }
                    }
                    
                    series_idx += 1;
                }
                
                // Draw regression line if enabled
                if self.config.show_regression && data.regression_line.is_some() {
                    let reg = data.regression_line.as_ref().unwrap();
                    let x_min = data.x_range.as_ref().map(|r| r.0).unwrap_or(0.0);
                    let x_max = data.x_range.as_ref().map(|r| r.1).unwrap_or(1.0);
                    
                    let y_min = reg.slope * x_min + reg.intercept;
                    let y_max = reg.slope * x_max + reg.intercept;
                    
                    let line_points = vec![[x_min, y_min], [x_max, y_max]];
                    let regression_line = Line::new(line_points)
                        .color(Color32::from_rgba_unmultiplied(255, 255, 255, 180))
                        .width(2.0)
                        .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                        .name(format!("y = {:.3}x + {:.3}", reg.slope, reg.intercept));
                    plot_ui.line(regression_line);
                }
                
                // Get input state INSIDE plot context for proper detection
                let right_clicked = plot_ui.response().secondary_clicked();
                let left_clicked = plot_ui.response().clicked() && !plot_ui.response().dragged();
                let is_dragging = plot_ui.response().dragged();
                
                if let Some(pointer_coord) = plot_ui.pointer_coordinate() {
                    // RIGHT-CLICK: Place marker (only if drag is less than 3 pixels)
                    if right_clicked && drag_delta < 3.0 {
                        // Find nearest data point X coordinate for alignment
                        let mut best_dist = f64::INFINITY;
                        let mut best_x = pointer_coord.x;
                        
                        for point in &data.points {
                            let dist = (point.x - pointer_coord.x).abs();
                            if dist < best_dist {
                                best_dist = dist;
                                best_x = point.x;
                            }
                        }
                        
                        // Update navigation to nearest point if we have row indices
                        if let Some(point) = data.points.iter().find(|p| (p.x - best_x).abs() < f64::EPSILON) {
                            if let Some(row_idx) = point.row_index {
                                let _ = ctx.navigation.seek_to(NavigationPosition::Sequential(row_idx));
                            }
                        }
                    }
                    
                    // LEFT-CLICK: Highlight nearest point (only if not dragging)
                    if left_clicked && !is_dragging {
                        // Find nearest point to click
                        let mut best_dist = f64::INFINITY;
                        let mut best_idx = 0;
                        
                        for (idx, point) in data.points.iter().enumerate() {
                            let dist = ((point.x - pointer_coord.x).powi(2) + 
                                       (point.y - pointer_coord.y).powi(2)).sqrt();
                            if dist < best_dist {
                                best_dist = dist;
                                best_idx = idx;
                            }
                        }
                        
                        // Only select if click was reasonably close
                        let plot_bounds = plot_ui.plot_bounds();
                        let threshold = 0.02 * plot_bounds.width().max(plot_bounds.height());
                        if best_dist < threshold {
                            // Update hovered data for highlighting
                            let mut hover_data = ctx.hovered_data.write();
                            hover_data.view_id = Some(self.id.clone());
                            hover_data.point_index = Some(best_idx);
                            
                            // Update navigation if appropriate
                            if let Some(point) = data.points.get(best_idx) {
                                if let Some(row_idx) = point.row_index {
                                    let _ = ctx.navigation.seek_to(NavigationPosition::Sequential(row_idx));
                                }
                            }
                        }
                    }
                }
                
                // Draw WHITE vertical marker bar at current navigation position
                let nav_context = ctx.navigation.get_context();
                if let NavigationPosition::Sequential(current_idx) = &nav_context.position {
                    // Find the X coordinate for the current navigation position
                    if let Some(point) = data.points.iter().find(|p| p.row_index == Some(*current_idx)) {
                        let bounds = plot_ui.plot_bounds();
                        let line_points = vec![
                            [point.x, bounds.min()[1]], 
                            [point.x, bounds.max()[1]]
                        ];
                        // White, prominent vertical bar like Rerun
                        let cursor_line = Line::new(line_points)
                            .color(Color32::WHITE)
                            .width(2.0)
                            .style(LineStyle::Solid);
                        plot_ui.line(cursor_line);
                    }
                }
            });
            
            // Show statistics if enabled
            if self.config.show_statistics {
                ui.separator();
                ui.horizontal(|ui| {
                    if let Some(stats) = &data.statistics {
                        ui.label(format!("Correlation: {:.3}", stats.correlation));
                        if let Some(reg) = &data.regression_line {
                            ui.separator();
                            ui.label(format!("R²: {:.3}", reg.r_squared));
                        }
                    }
                });
            }
        } else {
            // No data message
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
                ui.label(egui::RichText::new("Select X and Y columns to create scatter plot").weak());
            });
        }
    }
} 