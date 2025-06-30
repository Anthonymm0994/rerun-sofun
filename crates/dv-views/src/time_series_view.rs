//! Time series view implementation
//! Based on Rerun's PlotView

use egui::{Ui, Color32};
use egui_plot::{Plot, PlotPoints, Line, Legend, PlotBounds, Points};
use arrow::array::{Float64Array, Int64Array};
use serde_json::{json, Value};

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::{NavigationPosition, NavigationMode};

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
    _zoom_state: ZoomState,
    cached_data: Option<PlotData>,
    last_navigation_pos: Option<NavigationPosition>,
}

/// Zoom state for the plot
struct ZoomState {
    _current_bounds: Option<PlotBounds>,
    _is_panning: bool,
    _is_selecting: bool,
}

impl Default for ZoomState {
    fn default() -> Self {
        Self {
            _current_bounds: None,
            _is_panning: false,
            _is_selecting: false,
        }
    }
}

/// Cached plot data
struct PlotData {
    x_values: Vec<f64>,
    series: Vec<PlotSeries>,
}

/// A single data series
struct PlotSeries {
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
            _zoom_state: ZoomState::default(),
            cached_data: None,
            last_navigation_pos: None,
        }
    }
    
    /// Get plot data from the current data source
    fn fetch_plot_data(&mut self, ctx: &ViewerContext) -> Option<PlotData> {
        let data_source = ctx.data_source.read();
        let data_source = data_source.as_ref()?;
        
        // Get current navigation position
        let nav_pos = ctx.navigation.get_context().position.clone();
        
        // Query data at current position
        let batch = ctx.runtime_handle.block_on(
            data_source.query_at(&nav_pos)
        ).ok()?;
        
        // Extract columns
        let mut plot_data = PlotData {
            x_values: Vec::new(),
            series: Vec::new(),
        };
        
        // Get X axis data
        if let Some(x_col) = &self.config.x_column {
            if let Some(array) = batch.column_by_name(x_col) {
                if let Some(float_array) = array.as_any().downcast_ref::<Float64Array>() {
                    plot_data.x_values = float_array.values().to_vec();
                } else if let Some(int_array) = array.as_any().downcast_ref::<Int64Array>() {
                    plot_data.x_values = int_array.values().iter().map(|&v| v as f64).collect();
                }
            }
        } else {
            // Use index as X axis
            plot_data.x_values = (0..batch.num_rows()).map(|i| i as f64).collect();
        }
        
        // Get Y axis data for each column
        for y_col in &self.config.y_columns {
            if let Some(array) = batch.column_by_name(y_col) {
                let mut values = Vec::new();
                
                if let Some(float_array) = array.as_any().downcast_ref::<Float64Array>() {
                    values = float_array.values().to_vec();
                } else if let Some(int_array) = array.as_any().downcast_ref::<Int64Array>() {
                    values = int_array.values().iter().map(|&v| v as f64).collect();
                }
                
                if !values.is_empty() {
                    plot_data.series.push(PlotSeries {
                        name: y_col.clone(),
                        values,
                        color: None,
                    });
                }
            }
        }
        
        Some(plot_data)
    }
}

impl SpaceView for TimeSeriesView {
    fn id(&self) -> &SpaceViewId {
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
            let modifiers = ui.input(|i| i.modifiers);
            let pointer_down = ui.input(|i| i.pointer.primary_down());
            
            let plot = Plot::new(format!("{:?}", self.id))
                .show_grid(self.config.show_grid)
                // NEVER allow scroll wheel zoom
                .allow_scroll(false)
                // ONLY zoom when Ctrl/Cmd is held down - nothing else!
                .allow_zoom(modifiers.ctrl || modifiers.command)
                // Always allow drag
                .allow_drag(true)
                // Always allow box zoom
                .allow_boxed_zoom(true);
            
            // Set legend visibility
            let plot = if self.config.show_legend {
                plot.legend(Legend::default())
            } else {
                plot
            };
            
            // Add instructions tooltip
            if ui.is_rect_visible(ui.available_rect_before_wrap()) {
                let help_text = if modifiers.ctrl || modifiers.command {
                    "Scroll to zoom, Drag to pan, Right-click+Drag for box zoom"
                } else {
                    "Hold Ctrl to zoom, Drag to pan, Right-click+Drag for box zoom"
                };
                
                ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                    ui.label(
                        egui::RichText::new(help_text)
                            .small()
                            .color(ui.style().visuals.weak_text_color())
                    );
                });
            }
            
            plot.show(ui, |plot_ui| {
                // Draw vertical time cursor if this view shares time axis
                let nav_context = ctx.navigation.get_context();
                let time_axis_views = ctx.time_axis_views.read();
                let is_time_synced = time_axis_views.contains(&self.id);
                
                // Show cursor if we're dragging in any time-synced view
                let show_cursor = is_time_synced && ctx.hovered_data.read().view_id.as_ref()
                    .map(|id| time_axis_views.contains(id))
                    .unwrap_or(false);
                
                if show_cursor {
                    // Use navigation position as the cursor position
                    let cursor_x = match &nav_context.position {
                        NavigationPosition::Sequential(idx) => *idx as f64,
                        NavigationPosition::Temporal(ts) => {
                            // TODO: Convert timestamp to x coordinate based on time axis
                            // For now, use timestamp directly
                            *ts as f64
                        }
                        NavigationPosition::Categorical(_cat) => {
                            // Categories don't make sense for time series
                            0.0
                        }
                    };
                    
                    // Only show cursor if it's within our data range
                    if let Some(x_min) = plot_data.x_values.first() {
                        if let Some(x_max) = plot_data.x_values.last() {
                            if cursor_x >= *x_min && cursor_x <= *x_max {
                                // Draw vertical line at cursor position
                                let line_points = vec![[cursor_x, plot_ui.plot_bounds().min()[1]], [cursor_x, plot_ui.plot_bounds().max()[1]]];
                                let cursor_line = Line::new(line_points)
                                    .color(Color32::from_rgba_unmultiplied(255, 255, 255, 180))
                                    .width(2.0);
                                plot_ui.line(cursor_line);
                            }
                        }
                    }
                }
                
                // Draw each series
                for (idx, series) in plot_data.series.iter().enumerate() {
                    let points: Vec<[f64; 2]> = plot_data.x_values.iter()
                        .zip(&series.values)
                        .map(|(&x, &y)| [x, y])
                        .collect();
                    
                    // Choose color
                    let color = series.color.unwrap_or_else(|| {
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
                        colors[idx % colors.len()]
                    });
                    
                    // Draw line
                    if self.config.show_lines {
                        let plot_points = PlotPoints::new(points.clone());
                        let line = Line::new(plot_points)
                            .color(color)
                            .width(self.config.line_width)
                            .name(&series.name);
                        plot_ui.line(line);
                    }
                    
                    // Draw points
                    if self.config.show_points {
                        let plot_points = PlotPoints::new(points.clone());
                        let points_plot = Points::new(plot_points)
                            .color(color)
                            .radius(self.config.point_radius)
                            .shape(egui_plot::MarkerShape::Circle)
                            .name(&series.name);
                        plot_ui.points(points_plot);
                    }
                    
                    // Draw value markers at cursor position
                    if show_cursor {
                        let cursor_x = match &nav_context.position {
                            NavigationPosition::Sequential(idx) => *idx as f64,
                            NavigationPosition::Temporal(ts) => *ts as f64,
                            NavigationPosition::Categorical(_) => 0.0,
                        };
                        
                        // Find the closest point to the cursor
                        if let Some((closest_idx, &closest_x)) = plot_data.x_values.iter()
                            .enumerate()
                            .min_by(|(_, a), (_, b)| {
                                (**a - cursor_x).abs().partial_cmp(&(**b - cursor_x).abs()).unwrap()
                            }) {
                            
                            if (closest_x - cursor_x).abs() < 1.0 { // Within reasonable distance
                                let y_value = series.values[closest_idx];
                                
                                // Draw a highlight point
                                let highlight_point = Points::new(vec![[closest_x, y_value]])
                                    .color(color)
                                    .radius(self.config.point_radius * 2.0)
                                    .shape(egui_plot::MarkerShape::Circle);
                                plot_ui.points(highlight_point);
                            }
                        }
                    }
                }
                
                // Handle hover - only for dragging the time cursor in time-synced views
                if is_time_synced {
                    if let Some(pointer_coord) = plot_ui.pointer_coordinate() {
                        if pointer_down {
                            // Update navigation position when dragging
                            let new_index = pointer_coord.x.round() as usize;
                            if new_index < plot_data.x_values.len() {
                                // Use the appropriate position type based on navigation mode
                                match nav_context.mode {
                                    NavigationMode::Sequential => {
                                        let _ = ctx.navigation.seek_to(NavigationPosition::Sequential(new_index));
                                    }
                                    NavigationMode::Temporal => {
                                        // TODO: Convert x coordinate to timestamp
                                        let _ = ctx.navigation.seek_to(NavigationPosition::Temporal(new_index as i64));
                                    }
                                    NavigationMode::Categorical { .. } => {
                                        // Categories don't make sense for time series plots
                                    }
                                }
                            }
                            
                            // Mark this view as hovered
                            let mut hover_data = ctx.hovered_data.write();
                            hover_data.view_id = Some(self.id.clone());
                        }
                    } else if ctx.hovered_data.read().view_id == Some(self.id.clone()) && !pointer_down {
                        // Clear hover when not dragging
                        let mut hover_data = ctx.hovered_data.write();
                        hover_data.view_id = None;
                    }
                }
                
                // Note: Double-click to reset is handled automatically by egui_plot
            });
        } else {
            // No data message
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
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
    
    fn load_config(&mut self, config: Value) {
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