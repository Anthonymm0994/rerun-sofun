//! Bar chart implementation

use egui::{Ui, Color32};
use egui_plot::{Plot, Bar, BarChart};
use arrow::array::{Float64Array, Int64Array, StringArray, Array};
use serde_json::{json, Value};

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;

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
        let categories: Vec<String> = if let Some(str_array) = cat_column.as_any().downcast_ref::<StringArray>() {
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