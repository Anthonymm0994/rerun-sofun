//! Scatter plot implementation

use egui::{Ui, Color32};
use egui_plot::{Plot, PlotPoints, Points, Legend, MarkerShape};
use arrow::array::{Float64Array, Int64Array, StringArray, Array};
use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;

/// Configuration for scatter plot view
#[derive(Clone)]
pub struct ScatterPlotConfig {
    pub data_source_id: Option<String>,
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
            data_source_id: None,
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
    colors: Option<Vec<Color32>>,
    categories: Option<Vec<String>>,
    category_map: Option<BTreeMap<String, Color32>>,
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
        tracing::info!("Fetching scatter plot data - X: '{}', Y: '{}', Color: {:?}", 
                      self.config.x_column, self.config.y_column, self.config.color_column);
        
        if self.config.x_column.is_empty() || self.config.y_column.is_empty() {
            tracing::warn!("Scatter plot columns not configured");
            return None;
        }
        
        let data_sources = ctx.data_sources.read();
        
        // Get the specific data source for this view
        let data_source = if let Some(source_id) = &self.config.data_source_id {
            tracing::debug!("Using specific data source: {}", source_id);
            data_sources.get(source_id)
        } else {
            tracing::debug!("Using first available data source");
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
        let batch = match ctx.runtime_handle.block_on(
            data_source.query_range(&range)
        ) {
            Ok(b) => {
                tracing::info!("Fetched batch with {} rows", b.num_rows());
                b
            },
            Err(e) => {
                tracing::error!("Failed to fetch data: {}", e);
                return None;
            }
        };
        
        // Log available columns
        let schema = batch.schema();
        let column_names: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();
        tracing::debug!("Available columns in batch: {:?}", column_names);
        
        // Extract X and Y columns
        let x_array = match batch.column_by_name(&self.config.x_column) {
            Some(arr) => {
                tracing::debug!("Found X column '{}' with type {:?}", self.config.x_column, arr.data_type());
                arr
            },
            None => {
                tracing::error!("X column '{}' not found in batch. Available: {:?}", self.config.x_column, column_names);
                return None;
            }
        };
        
        let y_array = match batch.column_by_name(&self.config.y_column) {
            Some(arr) => {
                tracing::debug!("Found Y column '{}' with type {:?}", self.config.y_column, arr.data_type());
                arr
            },
            None => {
                tracing::error!("Y column '{}' not found in batch", self.config.y_column);
                return None;
            }
        };
        
        // Extract color categories if specified
        let (categories, category_map) = if let Some(color_col) = &self.config.color_column {
            if let Some(cat_array) = batch.column_by_name(color_col) {
                tracing::debug!("Found color column '{}' with type {:?}", color_col, cat_array.data_type());
                
                // Extract categories
                let cats: Vec<String> = if let Some(str_array) = cat_array.as_any().downcast_ref::<StringArray>() {
                    (0..str_array.len()).map(|i| {
                        if str_array.is_null(i) {
                            "null".to_string()
                        } else {
                            str_array.value(i).to_string()
                        }
                    }).collect()
                } else {
                    // Try to convert other types to string
                    (0..cat_array.len()).map(|i| {
                        arrow::util::display::array_value_to_string(cat_array, i).unwrap_or_else(|_| "null".to_string())
                    }).collect()
                };
                
                // Create color map with stable colors
                let mut cat_map = BTreeMap::new();
                let unique_cats: Vec<String> = cats.iter().cloned().collect::<std::collections::HashSet<_>>().into_iter().collect();
                for (i, cat) in unique_cats.iter().enumerate() {
                    cat_map.insert(cat.clone(), super::utils::colors::categorical_color(i));
                }
                
                (Some(cats), Some(cat_map))
            } else {
                tracing::warn!("Color column '{}' not found", color_col);
                (None, None)
            }
        } else {
            (None, None)
        };
        
        // Extract x, y values together with categories to maintain alignment
        let mut points = Vec::new();
        let mut colors = Vec::new();
        
        for i in 0..x_array.len().min(y_array.len()) {
            // Get x value
            let x_val = if let Some(float_array) = x_array.as_any().downcast_ref::<Float64Array>() {
                if float_array.is_null(i) { continue; }
                float_array.value(i)
            } else if let Some(int_array) = x_array.as_any().downcast_ref::<Int64Array>() {
                if int_array.is_null(i) { continue; }
                int_array.value(i) as f64
            } else if let Some(int_array) = x_array.as_any().downcast_ref::<arrow::array::Int32Array>() {
                if int_array.is_null(i) { continue; }
                int_array.value(i) as f64
            } else if let Some(float_array) = x_array.as_any().downcast_ref::<arrow::array::Float32Array>() {
                if float_array.is_null(i) { continue; }
                float_array.value(i) as f64
            } else {
                continue;
            };
            
            // Get y value
            let y_val = if let Some(float_array) = y_array.as_any().downcast_ref::<Float64Array>() {
                if float_array.is_null(i) { continue; }
                float_array.value(i)
            } else if let Some(int_array) = y_array.as_any().downcast_ref::<Int64Array>() {
                if int_array.is_null(i) { continue; }
                int_array.value(i) as f64
            } else if let Some(int_array) = y_array.as_any().downcast_ref::<arrow::array::Int32Array>() {
                if int_array.is_null(i) { continue; }
                int_array.value(i) as f64
            } else if let Some(float_array) = y_array.as_any().downcast_ref::<arrow::array::Float32Array>() {
                if float_array.is_null(i) { continue; }
                float_array.value(i) as f64
            } else {
                continue;
            };
            
            points.push((x_val, y_val));
            
            // Get color for this point
            if let (Some(cats), Some(cat_map)) = (&categories, &category_map) {
                if let Some(cat) = cats.get(i) {
                    if let Some(&color) = cat_map.get(cat) {
                        colors.push(color);
                    } else {
                        colors.push(Color32::from_rgb(128, 128, 128)); // Default gray
                    }
                } else {
                    colors.push(Color32::from_rgb(128, 128, 128));
                }
            }
        }
        
        tracing::info!("Created {} scatter plot points", points.len());
        if let Some(cat_map) = &category_map {
            tracing::info!("Found {} categories for coloring", cat_map.len());
        }
        
        // Extract optional size column
        let sizes = if let Some(size_col) = &self.config.size_column {
            if let Some(array) = batch.column_by_name(size_col) {
                if let Some(float_array) = array.as_any().downcast_ref::<Float64Array>() {
                    Some((0..float_array.len()).filter_map(|i| {
                        if float_array.is_null(i) { None } else { Some(float_array.value(i) as f32) }
                    }).collect())
                } else if let Some(int_array) = array.as_any().downcast_ref::<Int64Array>() {
                    Some((0..int_array.len()).filter_map(|i| {
                        if int_array.is_null(i) { None } else { Some(int_array.value(i) as f32) }
                    }).collect())
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
            colors: if !colors.is_empty() { Some(colors) } else { None },
            categories,
            category_map,
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
        // Update data if navigation changed or if we have no cached data
        let nav_pos = ctx.navigation.get_context().position.clone();
        if self.cached_data.is_none() || self.last_navigation_pos.as_ref() != Some(&nav_pos) {
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
                
                if let (Some(colors), Some(category_map)) = (&data.colors, &data.category_map) {
                    // Plot points grouped by category for proper legend
                    for (category, &color) in category_map {
                        let category_points: Vec<[f64; 2]> = data.points.iter()
                            .zip(colors.iter())
                            .filter(|(_, &c)| c == color)
                            .map(|((x, y), _)| [*x, *y])
                            .collect();
                        
                        if !category_points.is_empty() {
                            let points = Points::new(PlotPoints::new(category_points))
                                .color(color)
                                .radius(self.config.point_radius)
                                .shape(self.config.marker_shape)
                                .name(category);
                            
                            plot_ui.points(points);
                        }
                    }
                } else {
                    // No categories, plot all points with same color
                    let plot_points: Vec<[f64; 2]> = data.points.iter()
                        .map(|&(x, y)| [x, y])
                        .collect();
                    
                    let points = Points::new(PlotPoints::new(plot_points.clone()))
                        .color(Color32::from_rgb(31, 119, 180))
                        .radius(self.config.point_radius)
                        .shape(self.config.marker_shape)
                        .name(&format!("{} vs {}", self.config.y_column, self.config.x_column));
                    
                    plot_ui.points(points);
                }
            
                // Show hover tooltip on mouse over
                if let Some(pointer_coord) = plot_ui.pointer_coordinate() {
                    // Find points near the hover position
                    let hover_radius = 0.5;
                    let hover_points: Vec<(usize, (f64, f64))> = data.points.iter()
                        .enumerate()
                        .filter(|(_, (x, y))| {
                            let dx = x - pointer_coord.x;
                            let dy = y - pointer_coord.y;
                            (dx * dx + dy * dy).sqrt() < hover_radius
                        })
                        .map(|(i, &p)| (i, p))
                        .collect();
                    
                    // Highlight hover points
                    for (idx, (x, y)) in &hover_points {
                        let color = if let Some(colors) = &data.colors {
                            colors.get(*idx).copied().unwrap_or(Color32::from_rgb(255, 127, 14))
                        } else {
                            Color32::from_rgb(255, 127, 14)
                        };
                        
                        let highlight = Points::new(vec![[*x, *y]])
                            .color(color)
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
