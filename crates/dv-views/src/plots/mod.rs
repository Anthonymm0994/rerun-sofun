//! Plot view implementations
// TODO: Update to use SpaceView trait

use egui::{Ui, Color32};
use egui_plot::{Plot, PlotPoints, Points, Legend, MarkerShape, Bar, BarChart};
use arrow::array::{Float64Array, Int64Array, Array};
use serde_json::{json, Value};

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;

/// Configuration for scatter plot view
#[derive(Clone)]
pub struct ScatterPlotConfig {
    /// X-axis column
    pub x_column: String,
    
    /// Y-axis column
    pub y_column: String,
    
    /// Optional column for point size
    pub size_column: Option<String>,
    
    /// Optional column for point color
    pub color_column: Option<String>,
    
    /// Base point radius
    pub point_radius: f32,
    
    /// Whether to show legend
    pub show_legend: bool,
    
    /// Whether to show grid
    pub show_grid: bool,
    
    /// Marker shape
    pub marker_shape: MarkerShape,
}

impl Default for ScatterPlotConfig {
    fn default() -> Self {
        Self {
            x_column: String::new(),
            y_column: String::new(),
            size_column: None,
            color_column: None,
            point_radius: 3.0,
            show_legend: true,
            show_grid: true,
            marker_shape: MarkerShape::Circle,
        }
    }
}

/// Scatter plot view
pub struct ScatterPlotView {
    id: SpaceViewId,
    title: String,
    pub config: ScatterPlotConfig,
    
    // State
    cached_data: Option<ScatterData>,
    last_navigation_pos: Option<NavigationPosition>,
}

/// Cached scatter plot data
struct ScatterData {
    points: Vec<(f64, f64)>,
    _sizes: Option<Vec<f32>>,
    _colors: Option<Vec<Color32>>,
    _categories: Option<Vec<String>>,
}

impl ScatterPlotView {
    /// Create a new scatter plot view
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: ScatterPlotConfig::default(),
            cached_data: None,
            last_navigation_pos: None,
        }
    }
    
    /// Get plot data from the current data source
    fn fetch_plot_data(&mut self, ctx: &ViewerContext) -> Option<ScatterData> {
        let data_source = ctx.data_source.read();
        let data_source = data_source.as_ref()?;
        
        // Get current navigation position
        let nav_pos = ctx.navigation.get_context().position.clone();
        
        // Query data at current position
        let batch = ctx.runtime_handle.block_on(
            data_source.query_at(&nav_pos)
        ).ok()?;
        
        // Extract X and Y columns
        let x_array = batch.column_by_name(&self.config.x_column)?;
        let y_array = batch.column_by_name(&self.config.y_column)?;
        
        let x_values: Vec<f64> = if let Some(float_array) = x_array.as_any().downcast_ref::<Float64Array>() {
            float_array.values().to_vec()
        } else if let Some(int_array) = x_array.as_any().downcast_ref::<Int64Array>() {
            int_array.values().iter().map(|&v| v as f64).collect()
        } else {
            return None;
        };
        
        let y_values: Vec<f64> = if let Some(float_array) = y_array.as_any().downcast_ref::<Float64Array>() {
            float_array.values().to_vec()
        } else if let Some(int_array) = y_array.as_any().downcast_ref::<Int64Array>() {
            int_array.values().iter().map(|&v| v as f64).collect()
        } else {
            return None;
        };
        
        let points: Vec<(f64, f64)> = x_values.into_iter().zip(y_values).collect();
        
        // Extract optional size column
        let sizes = if let Some(size_col) = &self.config.size_column {
            if let Some(array) = batch.column_by_name(size_col) {
                if let Some(float_array) = array.as_any().downcast_ref::<Float64Array>() {
                    Some(float_array.values().iter().map(|&v| v as f32).collect())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        Some(ScatterData {
            points,
            _sizes: sizes,
            _colors: None, // TODO: Implement color mapping
            _categories: None, // TODO: Implement category extraction
        })
    }
}

impl SpaceView for ScatterPlotView {
    fn id(&self) -> &SpaceViewId {
        &self.id
    }
    
    fn display_name(&self) -> &str {
        &self.title
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
            
            let plot = Plot::new(format!("{:?}", self.id))
                .legend(Legend::default())
                .show_grid(self.config.show_grid)
                // DISABLE auto bounds completely for consistent behavior
                .auto_bounds(egui::Vec2b::new(false, false))
                // Enable scroll wheel zoom like time series
                .allow_scroll(true)
                // Allow zoom with controls
                .allow_zoom(true)
                // Always allow drag
                .allow_drag(true)
                // Always allow box zoom
                .allow_boxed_zoom(true)
                .data_aspect(1.0);
            
            // Calculate bounds from ALL data points
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
            
            // Apply fixed bounds with padding to show ENTIRE dataset
            let plot = if x_min.is_finite() && x_max.is_finite() {
                let x_padding = (x_max - x_min) * 0.05;
                plot.include_x(x_min - x_padding).include_x(x_max + x_padding)
            } else {
                plot
            };
            
            let plot = if y_min.is_finite() && y_max.is_finite() {
                let y_padding = (y_max - y_min) * 0.05;
                plot.include_y(y_min - y_padding).include_y(y_max + y_padding)
            } else {
                plot
            };
            
            // Add help text
            if ui.is_rect_visible(ui.available_rect_before_wrap()) {
                let help_text = if modifiers.ctrl || modifiers.command {
                    "Ctrl+Drag to zoom, Right-click+Drag for box zoom"
                } else {
                    "Drag to pan, Hold Ctrl to zoom, Right-click+Drag for box zoom"
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
                // Scatter plots don't participate in time cursor synchronization
                // They show independent data relationships, not time series
                
                // Convert points to PlotPoints
                let plot_points: Vec<[f64; 2]> = data.points.iter()
                    .map(|&(x, y)| [x, y])
                    .collect();
                
                let points = Points::new(PlotPoints::new(plot_points.clone()))
                    .color(Color32::from_rgb(31, 119, 180))
                    .radius(self.config.point_radius)
                    .shape(self.config.marker_shape)
                    .name(&format!("{} vs {}", self.config.y_column, self.config.x_column));
                
                plot_ui.points(points);
                
                // Show hover tooltip on mouse over
                if let Some(pointer_coord) = plot_ui.pointer_coordinate() {
                    // Find points near the hover position
                    let hover_radius = 0.5;
                    let hover_points: Vec<(usize, [f64; 2])> = plot_points.iter()
                        .enumerate()
                        .filter(|(_, [x, y])| {
                            let dx = x - pointer_coord.x;
                            let dy = y - pointer_coord.y;
                            (dx * dx + dy * dy).sqrt() < hover_radius
                        })
                        .map(|(i, &p)| (i, p))
                        .collect();
                    
                    // Highlight hover points
                    for (_idx, point) in &hover_points {
                        let highlight = Points::new(vec![*point])
                            .color(Color32::from_rgb(255, 127, 14))
                            .radius(self.config.point_radius * 2.0)
                            .shape(egui_plot::MarkerShape::Circle);
                        plot_ui.points(highlight);
                        
                        // TODO: Re-enable tooltips when Text is available
                        // Show tooltip as text on plot
                        /*
                        let tooltip_text = format!(
                            "{}: {:.2}\n{}: {:.2}",
                            self.config.x_column, point[0],
                            self.config.y_column, point[1]
                        );
                        
                        plot_ui.text(
                            egui_plot::Text::new(
                                egui_plot::PlotPoint::new(point[0], point[1]),
                                egui::RichText::new(&tooltip_text)
                                    .background_color(egui::Color32::from_black_alpha(200))
                                    .color(egui::Color32::WHITE)
                                    .small()
                            )
                            .anchor(egui::Align2::LEFT_BOTTOM)
                        );
                        */
                    }
                }
            });
        } else {
            // Configuration UI when no data
            ui.vertical_centered(|ui| {
                ui.label("Scatter Plot Configuration");
                ui.separator();
                
                ui.horizontal(|ui| {
                    ui.label("X Column:");
                    ui.text_edit_singleline(&mut self.config.x_column);
                });
                
                ui.horizontal(|ui| {
                    ui.label("Y Column:");
                    ui.text_edit_singleline(&mut self.config.y_column);
                });
                
                ui.separator();
                ui.label("Configure columns and load data to see scatter plot");
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "x_column": self.config.x_column,
            "y_column": self.config.y_column,
            "size_column": self.config.size_column,
            "color_column": self.config.color_column,
            "point_radius": self.config.point_radius,
            "show_legend": self.config.show_legend,
            "show_grid": self.config.show_grid,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(x_col) = config.get("x_column").and_then(|v| v.as_str()) {
            self.config.x_column = x_col.to_string();
        }
        if let Some(y_col) = config.get("y_column").and_then(|v| v.as_str()) {
            self.config.y_column = y_col.to_string();
        }
        if let Some(size_col) = config.get("size_column").and_then(|v| v.as_str()) {
            self.config.size_column = Some(size_col.to_string());
        }
        if let Some(color_col) = config.get("color_column").and_then(|v| v.as_str()) {
            self.config.color_column = Some(color_col.to_string());
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
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {
        // TODO: Highlight selected points
    }
    
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {
        // Nothing to update per frame
    }
}

/// Bar chart configuration
#[derive(Debug, Clone)]
pub struct BarChartConfig {
    /// Category column (X-axis)
    pub category_column: String,
    
    /// Value column (Y-axis)
    pub value_column: String,
    
    /// Whether to show legend
    pub show_legend: bool,
    
    /// Whether to show grid
    pub show_grid: bool,
    
    /// Bar width factor (0.0 to 1.0)
    pub bar_width: f32,
}

impl Default for BarChartConfig {
    fn default() -> Self {
        Self {
            category_column: String::new(),
            value_column: String::new(),
            show_legend: false,
            show_grid: true,
            bar_width: 0.7,
        }
    }
}

/// Bar chart view
pub struct BarChartView {
    id: SpaceViewId,
    title: String,
    pub config: BarChartConfig,
    
    // State
    cached_data: Option<BarData>,
    last_navigation_pos: Option<NavigationPosition>,
}

/// Cached bar chart data
#[derive(Debug, Clone)]
struct BarData {
    categories: Vec<String>,
    values: Vec<f64>,
}

impl BarChartView {
    /// Create a new bar chart view
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: BarChartConfig::default(),
            cached_data: None,
            last_navigation_pos: None,
        }
    }
    
    /// Fetch bar chart data
    fn fetch_data(&mut self, ctx: &ViewerContext) -> Option<BarData> {
        let data_source = ctx.data_source.read();
        let data_source = data_source.as_ref()?;
        
        // Get navigation context
        let nav_context = ctx.navigation.get_context();
        
        // For bar charts, we'll aggregate all data
        let range = dv_core::navigation::NavigationRange {
            start: dv_core::navigation::NavigationPosition::Sequential(0),
            end: dv_core::navigation::NavigationPosition::Sequential(nav_context.total_rows),
        };
        
        // Fetch data
        let data = ctx.runtime_handle.block_on(data_source.query_range(&range)).ok()?;
        
        // Extract categories and values
        let cat_column = data.column_by_name(&self.config.category_column)?;
        let val_column = data.column_by_name(&self.config.value_column)?;
        
        // Convert to string categories
        let categories: Vec<String> = if let Some(str_array) = cat_column.as_any().downcast_ref::<arrow::array::StringArray>() {
            (0..str_array.len()).map(|i| str_array.value(i).to_string()).collect()
        } else {
            // Try to convert other types to string
            (0..cat_column.len()).map(|i| arrow::util::display::array_value_to_string(cat_column, i).unwrap_or_default()).collect()
        };
        
        // Extract numeric values
        let values: Vec<f64> = if let Some(float_array) = val_column.as_any().downcast_ref::<Float64Array>() {
            (0..float_array.len()).map(|i| float_array.value(i)).collect()
        } else if let Some(int_array) = val_column.as_any().downcast_ref::<Int64Array>() {
            (0..int_array.len()).map(|i| int_array.value(i) as f64).collect()
        } else {
            return None;
        };
        
        // Group by category and sum values
        let mut category_map: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        for (cat, val) in categories.iter().zip(values.iter()) {
            *category_map.entry(cat.clone()).or_insert(0.0) += val;
        }
        
        // Sort by category name
        let mut sorted_cats: Vec<(String, f64)> = category_map.into_iter().collect();
        sorted_cats.sort_by(|a, b| a.0.cmp(&b.0));
        
        Some(BarData {
            categories: sorted_cats.iter().map(|(c, _)| c.clone()).collect(),
            values: sorted_cats.iter().map(|(_, v)| *v).collect(),
        })
    }
}

impl SpaceView for BarChartView {
    fn id(&self) -> &SpaceViewId {
        &self.id
    }
    
    fn display_name(&self) -> &str {
        &self.title
    }
    
    fn view_type(&self) -> &str {
        "BarChartView"
    }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Update data if navigation changed
        let nav_pos = ctx.navigation.get_context().position.clone();
        if self.last_navigation_pos.as_ref() != Some(&nav_pos) {
            self.cached_data = self.fetch_data(ctx);
            self.last_navigation_pos = Some(nav_pos);
        }
        
        // Draw the bar chart
        if let Some(data) = &self.cached_data {
            let plot = Plot::new(format!("{:?}", self.id))
                .show_grid(self.config.show_grid)
                .x_axis_label(&self.config.category_column)
                .y_axis_label(&self.config.value_column)
                .allow_zoom(true)
                .allow_drag(true)
                .allow_boxed_zoom(true);
            
            plot.show(ui, |plot_ui| {
                let mut bars = Vec::new();
                
                for (i, (cat, val)) in data.categories.iter().zip(data.values.iter()).enumerate() {
                    let bar = Bar::new(i as f64, *val)
                        .width(self.config.bar_width as f64)
                        .name(cat)
                        .fill(Color32::from_rgb(92, 140, 97)); // F.R.O.G. green
                    bars.push(bar);
                }
                
                let chart = BarChart::new(bars)
                    .color(Color32::from_rgb(92, 140, 97));
                
                plot_ui.bar_chart(chart);
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
                ui.label(egui::RichText::new("Configure category and value columns").weak());
            });
        }
    }
    
    fn save_config(&self) -> serde_json::Value {
        serde_json::json!({
            "category_column": self.config.category_column,
            "value_column": self.config.value_column,
            "show_legend": self.config.show_legend,
            "show_grid": self.config.show_grid,
            "bar_width": self.config.bar_width,
        })
    }
    
    fn load_config(&mut self, config: serde_json::Value) {
        if let Some(cat_col) = config.get("category_column").and_then(|v| v.as_str()) {
            self.config.category_column = cat_col.to_string();
        }
        if let Some(val_col) = config.get("value_column").and_then(|v| v.as_str()) {
            self.config.value_column = val_col.to_string();
        }
        if let Some(show_legend) = config.get("show_legend").and_then(|v| v.as_bool()) {
            self.config.show_legend = show_legend;
        }
        if let Some(show_grid) = config.get("show_grid").and_then(|v| v.as_bool()) {
            self.config.show_grid = show_grid;
        }
        if let Some(bar_width) = config.get("bar_width").and_then(|v| v.as_f64()) {
            self.config.bar_width = bar_width as f32;
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {
        // TODO: Highlight selected bars
    }
    
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {
        // Nothing to update per frame
    }
}

/*
use egui::{Ui, Color32, Pos2};
use egui_plot::{Plot, PlotPoints, Line, Bar, BarChart, Points, Legend, PlotBounds};
use arrow::record_batch::RecordBatch;
use arrow::array::{Float64Array, Int64Array, ArrayRef};
use serde::{Serialize, Deserialize};
use dv_core::navigation::NavigationContext;
use crate::{View, ViewConfig};

/// Different types of plots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlotType {
    Line,
    Bar,
    Scatter,
}

/// Configuration for plot views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotConfig {
    pub plot_type: PlotType,
    pub x_column: Option<String>,
    pub y_columns: Vec<String>,
    pub show_legend: bool,
    pub show_grid: bool,
    pub auto_bounds: bool,
    pub x_label: Option<String>,
    pub y_label: Option<String>,
    pub colors: Vec<Color32>,
}

impl Default for PlotConfig {
    fn default() -> Self {
        Self {
            plot_type: PlotType::Line,
            x_column: None,
            y_columns: Vec::new(),
            show_legend: true,
            show_grid: true,
            auto_bounds: true,
            x_label: None,
            y_label: None,
            colors: vec![
                Color32::from_rgb(100, 150, 250),  // Blue
                Color32::from_rgb(250, 100, 100),  // Red
                Color32::from_rgb(100, 250, 100),  // Green
                Color32::from_rgb(250, 200, 100),  // Orange
                Color32::from_rgb(200, 100, 250),  // Purple
                Color32::from_rgb(100, 250, 200),  // Cyan
            ],
        }
    }
}

/// Plot view that can display line, bar, or scatter plots
pub struct PlotView {
    id: String,
    name: String,
    config: PlotConfig,
    data_cache: PlotDataCache,
    zoom_state: ZoomState,
}

struct PlotDataCache {
    x_values: Vec<f64>,
    y_series: Vec<Vec<f64>>,
    series_names: Vec<String>,
    bounds: PlotBounds,
}

struct ZoomState {
    current_bounds: PlotBounds,
    is_panning: bool,
    is_selecting: bool,
}

impl PlotView {
    /// Create a new plot view
    pub fn new(id: String, name: String, plot_type: PlotType) -> Self {
        let mut config = PlotConfig::default();
        config.plot_type = plot_type;
        
        Self {
            id,
            name,
            config,
            data_cache: PlotDataCache {
                x_values: Vec::new(),
                y_series: Vec::new(),
                series_names: Vec::new(),
                bounds: PlotBounds::from_min_max([0.0, 0.0], [1.0, 1.0]),
            },
            zoom_state: ZoomState {
                current_bounds: PlotBounds::from_min_max([0.0, 0.0], [1.0, 1.0]),
                is_panning: false,
                is_selecting: false,
            },
        }
    }
    
    /// Update data cache from RecordBatch
    fn update_data_cache(&mut self, data: &RecordBatch) {
        // Clear existing cache
        self.data_cache.x_values.clear();
        self.data_cache.y_series.clear();
        self.data_cache.series_names.clear();
        
        // Extract x values
        if let Some(x_col) = &self.config.x_column {
            if let Some(column) = data.column_by_name(x_col) {
                self.data_cache.x_values = Self::extract_numeric_values(column);
            }
        } else {
            // Use row indices as x values
            self.data_cache.x_values = (0..data.num_rows()).map(|i| i as f64).collect();
        }
        
        // Extract y series
        for y_col in &self.config.y_columns {
            if let Some(column) = data.column_by_name(y_col) {
                let values = Self::extract_numeric_values(column);
                if !values.is_empty() {
                    self.data_cache.y_series.push(values);
                    self.data_cache.series_names.push(y_col.clone());
                }
            }
        }
        
        // Update bounds
        if self.config.auto_bounds {
            self.update_bounds();
        }
    }
    
    /// Extract numeric values from an Arrow array
    fn extract_numeric_values(array: &ArrayRef) -> Vec<f64> {
        if let Some(float_array) = array.as_any().downcast_ref::<Float64Array>() {
            float_array.iter().filter_map(|v| v).collect()
        } else if let Some(int_array) = array.as_any().downcast_ref::<Int64Array>() {
            int_array.iter().filter_map(|v| v.map(|i| i as f64)).collect()
        } else {
            Vec::new()
        }
    }
    
    /// Update plot bounds based on data
    fn update_bounds(&mut self) {
        if self.data_cache.x_values.is_empty() {
            return;
        }
        
        let x_min = self.data_cache.x_values.iter().cloned().fold(f64::INFINITY, f64::min);
        let x_max = self.data_cache.x_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        
        for series in &self.data_cache.y_series {
            for &value in series {
                y_min = y_min.min(value);
                y_max = y_max.max(value);
            }
        }
        
        if x_min.is_finite() && x_max.is_finite() && y_min.is_finite() && y_max.is_finite() {
            // Add 5% padding
            let x_padding = (x_max - x_min) * 0.05;
            let y_padding = (y_max - y_min) * 0.05;
            
            self.data_cache.bounds = PlotBounds::from_min_max(
                [x_min - x_padding, y_min - y_padding],
                [x_max + x_padding, y_max + y_padding]
            );
            self.zoom_state.current_bounds = self.data_cache.bounds.clone();
        }
    }
    
    /// Render line plot
    fn render_line_plot(&self, plot_ui: &mut egui_plot::PlotUi) {
        for (idx, (series, name)) in self.data_cache.y_series.iter()
            .zip(&self.data_cache.series_names).enumerate() 
        {
            let color = self.config.colors.get(idx % self.config.colors.len())
                .cloned()
                .unwrap_or(Color32::WHITE);
            
            let points: PlotPoints = self.data_cache.x_values.iter()
                .zip(series.iter())
                .map(|(&x, &y)| [x, y])
                .collect();
            
            plot_ui.line(
                Line::new(points)
                    .color(color)
                    .name(name)
                    .width(2.0)
            );
        }
    }
    
    /// Render bar plot
    fn render_bar_plot(&self, plot_ui: &mut egui_plot::PlotUi) {
        let bar_width = if self.data_cache.x_values.len() > 1 {
            (self.data_cache.x_values[1] - self.data_cache.x_values[0]) * 0.8
        } else {
            1.0
        };
        
        for (idx, (series, name)) in self.data_cache.y_series.iter()
            .zip(&self.data_cache.series_names).enumerate() 
        {
            let color = self.config.colors.get(idx % self.config.colors.len())
                .cloned()
                .unwrap_or(Color32::WHITE);
            
            let bars: Vec<Bar> = self.data_cache.x_values.iter()
                .zip(series.iter())
                .map(|(&x, &y)| {
                    Bar::new(x, y)
                        .width(bar_width)
                        .fill(color)
                })
                .collect();
            
            plot_ui.bar_chart(
                BarChart::new(bars)
                    .color(color)
                    .name(name)
            );
        }
    }
    
    /// Render scatter plot
    fn render_scatter_plot(&self, plot_ui: &mut egui_plot::PlotUi) {
        for (idx, (series, name)) in self.data_cache.y_series.iter()
            .zip(&self.data_cache.series_names).enumerate() 
        {
            let color = self.config.colors.get(idx % self.config.colors.len())
                .cloned()
                .unwrap_or(Color32::WHITE);
            
            let points: PlotPoints = self.data_cache.x_values.iter()
                .zip(series.iter())
                .map(|(&x, &y)| [x, y])
                .collect();
            
            plot_ui.points(
                Points::new(points)
                    .color(color)
                    .name(name)
                    .radius(4.0)
            );
        }
    }
}

impl View for PlotView {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
    
    fn update(&mut self, data: &RecordBatch, _context: &NavigationContext) {
        // Auto-detect columns if not configured
        if self.config.y_columns.is_empty() {
            for field in data.schema().fields() {
                match field.data_type() {
                    arrow::datatypes::DataType::Float64 |
                    arrow::datatypes::DataType::Int64 |
                    arrow::datatypes::DataType::Float32 |
                    arrow::datatypes::DataType::Int32 => {
                        self.config.y_columns.push(field.name().clone());
                    }
                    _ => {}
                }
            }
        }
        
        self.update_data_cache(data);
    }
    
    fn render(&mut self, ui: &mut Ui) {
        let plot = Plot::new(&self.id)
            .legend(Legend::default())
            .show_grid(self.config.show_grid)
            .data_aspect(1.0)
            .allow_zoom(true)
            .allow_scroll(true)
            .allow_drag(true);
        
        let plot = if let Some(label) = &self.config.x_label {
            plot.x_axis_label(label)
        } else {
            plot
        };
        
        let plot = if let Some(label) = &self.config.y_label {
            plot.y_axis_label(label)
        } else {
            plot
        };
        
        plot.show(ui, |plot_ui| {
            match self.config.plot_type {
                PlotType::Line => self.render_line_plot(plot_ui),
                PlotType::Bar => self.render_bar_plot(plot_ui),
                PlotType::Scatter => self.render_scatter_plot(plot_ui),
            }
        });
    }
    
    fn config(&self) -> ViewConfig {
        ViewConfig::Plot(self.config.clone())
    }
    
    fn set_config(&mut self, config: ViewConfig) {
        if let ViewConfig::Plot(plot_config) = config {
            self.config = plot_config;
            // Re-update bounds if needed
            if self.config.auto_bounds {
                self.update_bounds();
            }
        }
    }
    
    fn on_zoom(&mut self, _factor: f32, _center: Option<Pos2>) {
        // Handled by egui_plot
    }
    
    fn on_selection(&mut self, _start: Pos2, _end: Pos2) {
        // TODO: Implement selection
    }
}
*/ 