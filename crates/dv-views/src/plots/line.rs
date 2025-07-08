//! Line plot implementation

use egui::{Ui, Color32};
use egui_plot::{Plot, PlotPoints, Line, Legend};
use arrow::array::{Float64Array, Int64Array, Array};
use serde_json::{json, Value};

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;

/// Configuration for line plot view
#[derive(Clone)]
pub struct LinePlotConfig {
    pub data_source_id: Option<String>,
    /// X-axis column (optional, uses row index if not specified)
    pub x_column: Option<String>,
    
    /// Y-axis columns (multiple lines)
    pub y_columns: Vec<String>,
    
    /// Line width
    pub line_width: f32,
    
    /// Whether to show points
    pub show_points: bool,
    
    /// Point radius
    pub point_radius: f32,
    
    /// Whether to show legend
    pub show_legend: bool,
    
    /// Whether to show grid
    pub show_grid: bool,
    
    /// Line style (solid, dashed, dotted)
    pub line_style: LineStyle,
    
    /// Whether to fill area under line
    pub fill_area: bool,
    
    /// Fill alpha
    pub fill_alpha: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
}

impl Default for LinePlotConfig {
    fn default() -> Self {
        Self {
            data_source_id: None,
            x_column: None,
            y_columns: Vec::new(),
            line_width: 2.0,
            show_points: false,
            point_radius: 3.0,
            show_legend: true,
            show_grid: true,
            line_style: LineStyle::Solid,
            fill_area: false,
            fill_alpha: 0.2,
        }
    }
}

/// Line plot view
pub struct LinePlotView {
    id: SpaceViewId,
    title: String,
    pub config: LinePlotConfig,
    
    // State
    cached_data: Option<LineData>,
    last_navigation_pos: Option<NavigationPosition>,
}

/// Cached line plot data
struct LineData {
    x_values: Vec<f64>,
    y_series: Vec<LineSeries>,
}

struct LineSeries {
    name: String,
    values: Vec<f64>,
}

impl LinePlotView {
    /// Create a new line plot view
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: LinePlotConfig::default(),
            cached_data: None,
            last_navigation_pos: None,
        }
    }
    
    /// Fetch line plot data
    fn fetch_data(&mut self, ctx: &ViewerContext) -> Option<LineData> {
        let data_sources = ctx.data_sources.read();
        
        // Get the specific data source for this view
        let data_source = if let Some(source_id) = &self.config.data_source_id {
            data_sources.get(source_id)
        } else {
            data_sources.values().next()
        }?;
        
        // Get navigation context
        let nav_context = ctx.navigation.get_context();
        
        // Fetch a range of data (all data for now)
        let total_rows = nav_context.total_rows;
        let range_size = total_rows.min(10000); // Limit to 10k points for performance
        let start_row = 0;
        
        // Create a navigation range to fetch data
        let range = dv_core::navigation::NavigationRange {
            start: dv_core::navigation::NavigationPosition::Sequential(start_row),
            end: dv_core::navigation::NavigationPosition::Sequential(start_row + range_size),
        };
        
        // Fetch data using query_range
        let batch = ctx.runtime_handle.block_on(
            data_source.query_range(&range)
        ).ok()?;
        
        // Extract X values
        let x_values = if let Some(x_col_name) = &self.config.x_column {
            if let Some(x_array) = batch.column_by_name(x_col_name) {
                Self::extract_numeric_values(x_array)
            } else {
                // Use row indices if column not found
                (0..batch.num_rows()).map(|i| i as f64).collect()
            }
        } else {
            // Use row indices if no x column specified
            (0..batch.num_rows()).map(|i| i as f64).collect()
        };
        
        // Extract Y series
        let mut y_series = Vec::new();
        for y_col in &self.config.y_columns {
            if let Some(y_array) = batch.column_by_name(y_col) {
                let values = Self::extract_numeric_values(y_array);
                if !values.is_empty() {
                    y_series.push(LineSeries {
                        name: y_col.clone(),
                        values,
                    });
                }
            }
        }
        
        if y_series.is_empty() {
            return None;
        }
        
        Some(LineData { x_values, y_series })
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
}

impl SpaceView for LinePlotView {
    fn id(&self) -> SpaceViewId {
        self.id
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn display_name(&self) -> &str {
        &self.title
    }
    
    fn view_type(&self) -> &str {
        "LinePlotView"
    }
    
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
        // Update data if navigation changed
        let nav_pos = ctx.navigation.get_context().position.clone();
        if self.last_navigation_pos.as_ref() != Some(&nav_pos) {
            self.cached_data = self.fetch_data(ctx);
            self.last_navigation_pos = Some(nav_pos);
        }
        
        // Draw the plot
        if let Some(data) = &self.cached_data {
            let plot = Plot::new(format!("{:?}", self.id))
                .legend(Legend::default())
                .show_grid(self.config.show_grid)
                .auto_bounds(egui::Vec2b::new(true, true))
                .allow_scroll(true)
                .allow_zoom(true)
                .allow_drag(true)
                .allow_boxed_zoom(true);
            
            plot.show(ui, |plot_ui| {
                for (idx, series) in data.y_series.iter().enumerate() {
                    let color = super::utils::colors::categorical_color(idx);
                    
                    // Create plot points
                    let points: Vec<[f64; 2]> = data.x_values.iter()
                        .zip(&series.values)
                        .map(|(&x, &y)| [x, y])
                        .collect();
                    
                    // Draw line
                    let mut line = Line::new(PlotPoints::new(points.clone()))
                        .color(color)
                        .width(self.config.line_width)
                        .name(&series.name);
                    
                    // Apply line style
                    match self.config.line_style {
                        LineStyle::Dashed => {
                            line = line.style(egui_plot::LineStyle::Dashed { length: 10.0 });
                        }
                        LineStyle::Dotted => {
                            line = line.style(egui_plot::LineStyle::Dotted { spacing: 10.0 });
                        }
                        LineStyle::Solid => {}
                    }
                    
                    // Fill area if enabled
                    if self.config.fill_area {
                        line = line.fill(0.0);
                    }
                    
                    plot_ui.line(line);
                    
                    // Show points if enabled
                    if self.config.show_points {
                        plot_ui.points(
                            egui_plot::Points::new(points)
                                .color(color)
                                .radius(self.config.point_radius)
                        );
                    }
                }
                
                // Handle hover
                if let Some(pointer_coord) = plot_ui.pointer_coordinate() {
                    let hover_x = pointer_coord.x;
                    
                    // Find closest x value
                    if let Some((x_idx, &x_val)) = data.x_values.iter()
                        .enumerate()
                        .min_by(|(_, a), (_, b)| {
                            (*a - hover_x).abs().partial_cmp(&(*b - hover_x).abs()).unwrap()
                        }) {
                        
                        // Highlight points at this x value
                        for (series_idx, series) in data.y_series.iter().enumerate() {
                            if let Some(&y_val) = series.values.get(x_idx) {
                                let color = super::utils::colors::categorical_color(series_idx);
                                plot_ui.points(
                                    egui_plot::Points::new(vec![[x_val, y_val]])
                                        .color(color)
                                        .radius(self.config.point_radius * 2.0)
                                );
                                
                                // Update hover data
                                let mut hover_data = ctx.hovered_data.write();
                                hover_data.point_index = Some(x_idx);
                                hover_data.x = x_val;
                                hover_data.y = y_val;
                                hover_data.column = series.name.clone();
                            }
                        }
                    }
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
                ui.label(egui::RichText::new("Configure Y columns to see line plot").weak());
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "x_column": self.config.x_column,
            "y_columns": self.config.y_columns,
            "line_width": self.config.line_width,
            "show_points": self.config.show_points,
            "point_radius": self.config.point_radius,
            "show_legend": self.config.show_legend,
            "show_grid": self.config.show_grid,
            "line_style": match self.config.line_style {
                LineStyle::Solid => "solid",
                LineStyle::Dashed => "dashed",
                LineStyle::Dotted => "dotted",
            },
            "fill_area": self.config.fill_area,
            "fill_alpha": self.config.fill_alpha,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(x_col) = config.get("x_column").and_then(|v| v.as_str()) {
            self.config.x_column = Some(x_col.to_string());
        }
        if let Some(y_cols) = config.get("y_columns").and_then(|v| v.as_array()) {
            self.config.y_columns = y_cols.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
        }
        if let Some(width) = config.get("line_width").and_then(|v| v.as_f64()) {
            self.config.line_width = width as f32;
        }
        if let Some(show_points) = config.get("show_points").and_then(|v| v.as_bool()) {
            self.config.show_points = show_points;
        }
        if let Some(radius) = config.get("point_radius").and_then(|v| v.as_f64()) {
            self.config.point_radius = radius as f32;
        }
        if let Some(show_legend) = config.get("show_legend").and_then(|v| v.as_bool()) {
            self.config.show_legend = show_legend;
        }
        if let Some(show_grid) = config.get("show_grid").and_then(|v| v.as_bool()) {
            self.config.show_grid = show_grid;
        }
        if let Some(style) = config.get("line_style").and_then(|v| v.as_str()) {
            self.config.line_style = match style {
                "dashed" => LineStyle::Dashed,
                "dotted" => LineStyle::Dotted,
                _ => LineStyle::Solid,
            };
        }
        if let Some(fill) = config.get("fill_area").and_then(|v| v.as_bool()) {
            self.config.fill_area = fill;
        }
        if let Some(alpha) = config.get("fill_alpha").and_then(|v| v.as_f64()) {
            self.config.fill_alpha = alpha as f32;
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {
        // TODO: Highlight selected points
    }
    
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {
        // Nothing to update per frame
    }
} 