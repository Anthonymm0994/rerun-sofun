//! Scatter plot implementation

use egui::{Ui, Color32};
use egui_plot::{Plot, PlotPoints, Points, Legend, MarkerShape};
use arrow::array::{Float64Array, Int64Array, Array};
use serde_json::{json, Value};

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;

/// Configuration for scatter plot view
#[derive(Clone)]
pub struct ScatterPlotConfig {
    pub data_source_id: String,
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
    
    /// Allow scroll
    pub allow_scroll: bool,
    
    /// Allow zoom
    pub allow_zoom: bool,
    
    /// Allow drag
    pub allow_drag: bool,
    
    /// Allow boxed zoom
    pub allow_boxed_zoom: bool,
}

impl Default for ScatterPlotConfig {
    fn default() -> Self {
        Self {
            data_source_id: String::new(),
            x_column: String::new(),
            y_column: String::new(),
            size_column: None,
            color_column: None,
            point_radius: 3.0,
            show_legend: true,
            show_grid: true,
            marker_shape: MarkerShape::Circle,
            allow_scroll: true,
            allow_zoom: true,
            allow_drag: true,
            allow_boxed_zoom: true,
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
        if self.config.x_column.is_empty() || self.config.y_column.is_empty() {
            return None;
        }
        
        let data_sources = ctx.data_sources.read();
        
        // Get the specific data source for this view
        let data_source = if !self.config.data_source_id.is_empty() {
            data_sources.get(&self.config.data_source_id)
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
        
        // Extract X and Y columns
        let x_array = batch.column_by_name(&self.config.x_column)?;
        let y_array = batch.column_by_name(&self.config.y_column)?;
        
        let x_values: Vec<f64> = if let Some(float_array) = x_array.as_any().downcast_ref::<Float64Array>() {
            float_array.values().to_vec()
        } else if let Some(int_array) = x_array.as_any().downcast_ref::<Int64Array>() {
            int_array.values().iter().map(|&v| v as f64).collect()
        } else if let Some(int_array) = x_array.as_any().downcast_ref::<arrow::array::Int32Array>() {
            int_array.values().iter().map(|&v| v as f64).collect()
        } else if let Some(float_array) = x_array.as_any().downcast_ref::<arrow::array::Float32Array>() {
            float_array.values().iter().map(|&v| v as f64).collect()
        } else {
            return None;
        };
        
        let y_values: Vec<f64> = if let Some(float_array) = y_array.as_any().downcast_ref::<Float64Array>() {
            float_array.values().to_vec()
        } else if let Some(int_array) = y_array.as_any().downcast_ref::<Int64Array>() {
            int_array.values().iter().map(|&v| v as f64).collect()
        } else if let Some(int_array) = y_array.as_any().downcast_ref::<arrow::array::Int32Array>() {
            int_array.values().iter().map(|&v| v as f64).collect()
        } else if let Some(float_array) = y_array.as_any().downcast_ref::<arrow::array::Float32Array>() {
            float_array.values().iter().map(|&v| v as f64).collect()
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
            let plot = Plot::new(format!("{:?}", self.id))
                .legend(Legend::default())
                .show_grid(self.config.show_grid)
                .auto_bounds(egui::Vec2b::new(true, true))
                .allow_scroll(self.config.allow_scroll)
                .allow_zoom(self.config.allow_zoom)
                .allow_drag(self.config.allow_drag)
                .allow_boxed_zoom(self.config.allow_boxed_zoom)
                .data_aspect(1.0);
            
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
                    }
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
                if self.config.x_column.is_empty() || self.config.y_column.is_empty() {
                    ui.label(egui::RichText::new("Please configure X and Y columns").weak());
                } else {
                    ui.label(egui::RichText::new(format!("X: {}, Y: {}", self.config.x_column, self.config.y_column)).weak());
                    ui.label(egui::RichText::new("Check if columns exist in the data source").weak());
                }
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