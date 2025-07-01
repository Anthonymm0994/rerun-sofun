//! Time series view implementation
//! Based on Rerun's PlotView

use egui::{Ui, Color32};
use egui_plot::{Plot, PlotPoints, Line, Legend, Points, LineStyle};
use arrow::array::{Float64Array, Int64Array};
use uuid::Uuid;

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;

/// Configuration for time series view
#[derive(Clone)]
pub struct TimeSeriesConfig {
    /// X-axis column (None means use row index)
    pub x_column: Option<String>,
    
    /// Y-axis columns to plot
    pub y_columns: Vec<String>,
    
    /// Whether to show points
    pub show_points: bool,
    
    /// Whether to show lines
    pub show_lines: bool,
    
    /// Whether to show legend
    pub show_legend: bool,
    
    /// Whether to show grid
    pub show_grid: bool,
    
    /// Line width
    pub line_width: f32,
    
    /// Point radius
    pub point_radius: f32,
}

impl Default for TimeSeriesConfig {
    fn default() -> Self {
        Self {
            x_column: None,
            y_columns: Vec::new(),
            show_points: false,
            show_lines: true,
            show_legend: true,
            show_grid: true,
            line_width: 1.5,
            point_radius: 2.0,
        }
    }
}

/// Time series plot view
pub struct TimeSeriesView {
    id: SpaceViewId,
    title: String,
    pub config: TimeSeriesConfig,
    
    // State
    cached_data: Option<PlotData>,
    last_navigation_pos: Option<NavigationPosition>,
}

/// Cached plot data
#[derive(Debug, Clone)]
struct PlotData {
    x_values: Vec<f64>,
    series: Vec<SeriesData>,
    x_column: String,
}

/// A single data series
#[derive(Debug, Clone)]
struct SeriesData {
    name: String,
    values: Vec<f64>,
    color: Option<Color32>,
}

/// A plot point with metadata
struct _PlotPoint {
    _x: f64,
    _y: f64,
    _row_index: usize,
    _series_index: Option<usize>,
}

impl TimeSeriesView {
    /// Create a new time series view
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: TimeSeriesConfig::default(),
            cached_data: None,
            last_navigation_pos: None,
        }
    }
    
    /// Fetch plot data based on current navigation context
    fn fetch_plot_data(&self, ctx: &ViewerContext) -> Option<PlotData> {
        let data_source = ctx.data_source.read();
        let data_source = data_source.as_ref()?;
        
        // Get navigation context
        let nav_context = ctx.navigation.get_context();
        
        // Use the runtime from the viewer context - CRITICAL FIX
        let schema = ctx.runtime_handle.block_on(data_source.schema());
        
        // Find X column in schema fields
        let x_column = self.config.x_column.as_ref()?;
        let x_field = schema.fields().iter().find(|f| f.name() == x_column)?;
        
        if self.config.y_columns.is_empty() {
            return None;
        }
        
        // For now, get a reasonable range of data (last 1000 points or all data)
        let total_rows = nav_context.total_rows;
        let range_size = total_rows; // Get ALL data, not just last 1000
        let start_row = 0; // Start from beginning
        
        // Create a navigation range to fetch data
        let range = dv_core::navigation::NavigationRange {
            start: dv_core::navigation::NavigationPosition::Sequential(start_row),
            end: dv_core::navigation::NavigationPosition::Sequential(start_row + range_size),
        };
        
        // Fetch data using query_range - use the context runtime
        let data = ctx.runtime_handle.block_on(data_source.query_range(&range)).ok()?;
        
        // Extract X values
        let x_values = match x_field.data_type() {
            arrow::datatypes::DataType::Float64 => {
                let array = data.column_by_name(x_column)?
                    .as_any()
                    .downcast_ref::<Float64Array>()?;
                (0..array.len()).map(|i| array.value(i)).collect::<Vec<_>>()
            }
            arrow::datatypes::DataType::Int64 => {
                let array = data.column_by_name(x_column)?
                    .as_any()
                    .downcast_ref::<Int64Array>()?;
                (0..array.len()).map(|i| array.value(i) as f64).collect::<Vec<_>>()
            }
            _ => {
                // For other types, use row index as X
                (0..data.num_rows()).map(|i| (start_row + i) as f64).collect::<Vec<_>>()
            }
        };
        
        // Extract Y series
        let mut series = Vec::new();
        for y_column in &self.config.y_columns {
            if let Some(y_field) = schema.fields().iter().find(|f| f.name() == y_column) {
                let y_values = match y_field.data_type() {
                    arrow::datatypes::DataType::Float64 => {
                        if let Some(array) = data.column_by_name(y_column) {
                            if let Some(float_array) = array.as_any().downcast_ref::<Float64Array>() {
                                (0..float_array.len()).map(|i| float_array.value(i)).collect::<Vec<_>>()
                            } else {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }
                    arrow::datatypes::DataType::Int64 => {
                        if let Some(array) = data.column_by_name(y_column) {
                            if let Some(int_array) = array.as_any().downcast_ref::<Int64Array>() {
                                (0..int_array.len()).map(|i| int_array.value(i) as f64).collect::<Vec<_>>()
                            } else {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }
                    _ => continue,
                };
                
                series.push(SeriesData {
                    name: y_column.clone(),
                    values: y_values,
                    color: None,
                });
            }
        }
        
        if series.is_empty() {
            return None;
        }
        
        Some(PlotData {
            x_values,
            series,
            x_column: x_column.clone(),
        })
    }
}

impl SpaceView for TimeSeriesView {
    fn id(&self) -> &Uuid {
        &self.id
    }
    
    fn display_name(&self) -> &str {
        &self.title
    }
    
    fn view_type(&self) -> &str {
        "TimeSeriesView"
    }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Update data if navigation changed
        let nav_pos = ctx.navigation.get_context().position.clone();
        if self.last_navigation_pos.as_ref() != Some(&nav_pos) {
            self.cached_data = self.fetch_plot_data(ctx);
            self.last_navigation_pos = Some(nav_pos);
        }
        
        // Draw the plot
        if let Some(plot_data) = &self.cached_data {
            // Check keyboard modifiers
            let _modifiers = ui.input(|i| i.modifiers);
            
            // Configure plot with proper axis labels
            let x_axis_name = self.config.x_column.as_deref().unwrap_or("Row Index");
            let y_axis_name = if self.config.y_columns.len() == 1 {
                self.config.y_columns[0].as_str()
            } else {
                "Value"
            };
            
            let plot = Plot::new(format!("{:?}", self.id))
                .show_grid(self.config.show_grid)
                .x_axis_label(x_axis_name)
                .y_axis_label(y_axis_name)
                // DISABLE auto bounds completely
                .auto_bounds(egui::Vec2b::new(false, false))
                // ENABLE scroll wheel zoom like Rerun
                .allow_scroll(true)
                // Allow zoom with explicit controls
                .allow_zoom(true)
                // Allow drag for panning
                .allow_drag(true)
                // Right-click drag for box zoom
                .allow_boxed_zoom(true);
            
            // Calculate bounds from ALL data, not just current window
            let mut x_min = f64::INFINITY;
            let mut x_max = -f64::INFINITY;
            let mut y_min = f64::INFINITY;
            let mut y_max = -f64::INFINITY;
            
            // Use ALL data points to calculate bounds, not just visible window
            for &x in &plot_data.x_values {
                if x.is_finite() {
                    x_min = x_min.min(x);
                    x_max = x_max.max(x);
                }
            }
            
            for series in &plot_data.series {
                for &value in &series.values {
                    if value.is_finite() {
                        y_min = y_min.min(value);
                        y_max = y_max.max(value);
                    }
                }
            }
            
            // Apply fixed bounds with padding to show ENTIRE dataset
            let plot = if x_min.is_finite() && x_max.is_finite() {
                let x_padding = (x_max - x_min) * 0.05;
                plot.include_x(x_min - x_padding).include_x(x_max + x_padding)
            } else {
                plot
            };
            
            let plot = if y_min.is_finite() && y_max.is_finite() {
                let y_padding = (y_max - y_min) * 0.1;
                plot.include_y(y_min - y_padding).include_y(y_max + y_padding)
            } else {
                plot
            };
            
            // Set legend visibility
            let plot = if self.config.show_legend && self.config.y_columns.len() > 1 {
                plot.legend(Legend::default())
            } else {
                plot
            };
            
            // Show data range info and controls
            ui.horizontal(|ui| {
                if let Some(x_range) = plot_data.x_values.first().zip(plot_data.x_values.last()) {
                    ui.label(format!("{}: {:.2} to {:.2}", plot_data.x_column, x_range.0, x_range.1));
                    ui.separator();
                }
                ui.label(format!("Series: {}", plot_data.series.len()));
                ui.separator();
                ui.label(format!("Points: {}", plot_data.x_values.len()));
            });
            ui.separator();
            
            let mut _clicked_point: Option<usize> = None; // Placeholder for future use
            
            // Get drag delta for threshold detection OUTSIDE plot closure
            let drag_delta = ui.input(|i| i.pointer.delta()).length();
            
            plot.show(ui, |plot_ui| {
                // Get input state INSIDE plot context for proper detection
                let right_clicked = plot_ui.response().secondary_clicked();
                let left_clicked = plot_ui.response().clicked() && !plot_ui.response().dragged();
                let is_dragging = plot_ui.response().dragged();
                
                // Draw vertical time cursor - ALWAYS show it for navigation feedback
                let nav_context = ctx.navigation.get_context();
                let cursor_x = match &nav_context.position {
                    NavigationPosition::Sequential(idx) => {
                        plot_data.x_values.get(*idx).copied().unwrap_or_default()
                    }
                    NavigationPosition::Temporal(ts) => *ts as f64,
                    NavigationPosition::Categorical(_) => 0.0,
                };
                
                // Handle plot interactions with proper detection
                if let Some(pointer_coord) = plot_ui.pointer_coordinate() {
                    // RIGHT-CLICK: Place marker (only if drag is less than 3 pixels)
                    if right_clicked && drag_delta < 3.0 {
                        // Find nearest data point X coordinate
                        let mut best_dist = f64::INFINITY;
                        let mut best_x = pointer_coord.x;
                        
                        for &x in &plot_data.x_values {
                            let dist = (x - pointer_coord.x).abs();
                            if dist < best_dist {
                                best_dist = dist;
                                best_x = x;
                            }
                        }
                        
                        // Update cursor position to snap to nearest data point
                        if let Some(index) = plot_data.x_values.iter().position(|&x| x == best_x) {
                            let _ = ctx.navigation.seek_to(
                                dv_core::navigation::NavigationPosition::Sequential(index)
                            );
                        }
                    }
                    
                    // LEFT-CLICK: Highlight values at X-location (only if not dragging)
                    if left_clicked && !is_dragging {
                        // Find nearest X coordinate
                        let mut best_dist = f64::INFINITY;
                        let mut best_index = None;
                        
                        for (idx, &x) in plot_data.x_values.iter().enumerate() {
                            let dist = (x - pointer_coord.x).abs();
                            if dist < best_dist {
                                best_dist = dist;
                                best_index = Some(idx);
                            }
                        }
                        
                        // Check if any visible series has data at this point
                        if let Some(index) = best_index {
                            let has_visible_data = plot_data.series.iter().any(|series| {
                                index < series.values.len()
                            });
                            
                            // Only set highlight if there's visible data at this point
                            if has_visible_data {
                                let mut hover_data = ctx.hovered_data.write();
                                hover_data.view_id = Some(self.id.clone());
                                hover_data.point_index = Some(index);
                            }
                        }
                    }
                }
                
                // Draw WHITE vertical marker bar at cursor position - always visible
                if !plot_data.x_values.is_empty() {
                    if let (Some(x_min), Some(x_max)) = (plot_data.x_values.first(), plot_data.x_values.last()) {
                        if cursor_x >= *x_min && cursor_x <= *x_max {
                            // Draw vertical line at cursor position
                            let bounds = plot_ui.plot_bounds();
                            let line_points = vec![
                                [cursor_x, bounds.min()[1]], 
                                [cursor_x, bounds.max()[1]]
                            ];
                            // White, prominent vertical bar like Rerun
                            let cursor_line = Line::new(line_points)
                                .color(Color32::WHITE)
                                .width(2.0)
                                .style(LineStyle::Solid);
                            plot_ui.line(cursor_line);
                        }
                    }
                }
                
                // Draw each series as a line
                let mut series_idx = 0;
                for series in &plot_data.series {
                    if series.values.len() != plot_data.x_values.len() {
                        continue; // Skip mismatched series
                    }
                    
                    // Create points for this series
                    let points: Vec<[f64; 2]> = plot_data.x_values.iter()
                        .zip(&series.values)
                        .map(|(&x, &y)| [x, y])
                        .collect();
                    
                    let plot_points = PlotPoints::new(points.clone());
                    
                    // Choose color
                    let color = series.color.unwrap_or_else(|| {
                        let colors = [
                            Color32::from_rgb(31, 119, 180),   // Blue
                            Color32::from_rgb(255, 127, 14),   // Orange  
                            Color32::from_rgb(44, 160, 44),    // Green
                            Color32::from_rgb(214, 39, 40),    // Red
                            Color32::from_rgb(148, 103, 189),  // Purple
                            Color32::from_rgb(140, 86, 75),    // Brown
                        ];
                        colors[series_idx % colors.len()]
                    });
                    
                    // Draw line - this will be controlled by legend
                    let line = Line::new(plot_points)
                        .color(color)
                        .width(2.0)
                        .name(&series.name);
                    plot_ui.line(line);
                    
                    // Draw points with same name - legend will control both line and points together
                    let points_plot = Points::new(PlotPoints::new(points.clone()))
                        .color(color)
                        .radius(3.0)
                        .shape(egui_plot::MarkerShape::Circle)
                        .name(&series.name); // Same name as line for unified legend control
                    plot_ui.points(points_plot);
                    
                    // Highlight and show tooltip for left-clicked position
                    // The highlight points don't have a name, so they'll only be visible
                    // when their parent series is visible (not hidden by legend)
                    if let Some(hover_data) = &ctx.hovered_data.read().point_index {
                        if *hover_data < points.len() {
                            // Highlight the point - no name means it follows parent visibility
                            let highlight_point = Points::new(vec![points[*hover_data]])
                                .color(color.gamma_multiply(1.5))
                                .radius(6.0)
                                .shape(egui_plot::MarkerShape::Circle);
                            plot_ui.points(highlight_point);
                            
                            // Show value tooltip
                            let point = points[*hover_data];
                            let text = egui_plot::Text::new(
                                egui_plot::PlotPoint::new(point[0], point[1]),
                                egui::RichText::new(format!("{}: {:.3}", series.name, point[1]))
                                    .color(Color32::WHITE)
                                    .background_color(Color32::from_rgba_premultiplied(0, 0, 0, 180))
                                    .text_style(egui::TextStyle::Small)
                            )
                            .anchor(egui::Align2::LEFT_BOTTOM);
                            plot_ui.text(text);
                        }
                    }
                    
                    series_idx += 1;
                }
            });
        } else {
            // No data message
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
                ui.label(egui::RichText::new("Check data source and navigation settings").weak());
            });
        }
    }
    
    fn save_config(&self) -> serde_json::Value {
        serde_json::json!({
            "x_column": self.config.x_column,
            "y_columns": self.config.y_columns,
            "show_points": self.config.show_points,
            "show_lines": self.config.show_lines,
            "show_legend": self.config.show_legend,
            "show_grid": self.config.show_grid,
            "line_width": self.config.line_width,
            "point_radius": self.config.point_radius,
        })
    }
    
    fn load_config(&mut self, config: serde_json::Value) {
        if let Some(x_column) = config.get("x_column").and_then(|v| v.as_str()) {
            self.config.x_column = Some(x_column.to_string());
        }
        if let Some(y_columns) = config.get("y_columns").and_then(|v| v.as_array()) {
            self.config.y_columns = y_columns.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        if let Some(show_points) = config.get("show_points").and_then(|v| v.as_bool()) {
            self.config.show_points = show_points;
        }
        if let Some(show_lines) = config.get("show_lines").and_then(|v| v.as_bool()) {
            self.config.show_lines = show_lines;
        }
        if let Some(show_legend) = config.get("show_legend").and_then(|v| v.as_bool()) {
            self.config.show_legend = show_legend;
        }
        if let Some(show_grid) = config.get("show_grid").and_then(|v| v.as_bool()) {
            self.config.show_grid = show_grid;
        }
        if let Some(line_width) = config.get("line_width").and_then(|v| v.as_f64()) {
            self.config.line_width = line_width as f32;
        }
        if let Some(point_radius) = config.get("point_radius").and_then(|v| v.as_f64()) {
            self.config.point_radius = point_radius as f32;
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {
        // TODO: Highlight selected data points
    }
    
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {
        // Nothing to update per frame
    }
} 