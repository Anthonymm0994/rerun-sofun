//! Summary statistics view implementation

use egui::{Ui, ScrollArea};

use arrow::array::Array;
use arrow::datatypes::DataType;
use serde_json::{json, Value};

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;

/// Summary statistics view
pub struct SummaryStatsView {
    id: SpaceViewId,
    title: String,
    
    // Cached statistics
    cached_stats: Option<Vec<ColumnStats>>,
    last_navigation_pos: Option<NavigationPosition>,
}

#[derive(Debug, Clone)]
struct ColumnStats {
    name: String,
    data_type: DataType,
    count: usize,
    null_count: usize,
    numeric_stats: Option<NumericStats>,
    string_stats: Option<StringStats>,
}

#[derive(Debug, Clone)]
struct NumericStats {
    min: f64,
    max: f64,
    mean: f64,
    std_dev: f64,
    median: f64,
    q1: f64,
    q3: f64,
}

#[derive(Debug, Clone)]
struct StringStats {
    unique_count: usize,
    min_length: usize,
    max_length: usize,
    avg_length: f32,
}

impl SummaryStatsView {
    /// Create a new summary statistics view
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            cached_stats: None,
            last_navigation_pos: None,
        }
    }
    
    /// Calculate statistics for the current data
    fn calculate_stats(&mut self, ctx: &ViewerContext) -> Option<Vec<ColumnStats>> {
        let data_source = ctx.data_source.read();
        let data_source = data_source.as_ref()?;
        
        // Get current navigation position
        let _nav_pos = ctx.navigation.get_context().position.clone();
        
        // Query all data - for stats we want the full dataset
        let batch = ctx.runtime_handle.block_on(
            data_source.query_at(&NavigationPosition::Sequential(0))
        ).ok()?;
        
        let mut stats = Vec::new();
        
        for (idx, field) in batch.schema().fields().iter().enumerate() {
            let column = batch.column(idx);
            let mut col_stats = ColumnStats {
                name: field.name().clone(),
                data_type: field.data_type().clone(),
                count: column.len(),
                null_count: column.null_count(),
                numeric_stats: None,
                string_stats: None,
            };
            
            // Calculate type-specific statistics
            match field.data_type() {
                DataType::Float64 | DataType::Float32 | 
                DataType::Int64 | DataType::Int32 | 
                DataType::Int16 | DataType::Int8 |
                DataType::UInt64 | DataType::UInt32 | 
                DataType::UInt16 | DataType::UInt8 => {
                    col_stats.numeric_stats = self.calculate_numeric_stats(column);
                }
                DataType::Utf8 | DataType::LargeUtf8 => {
                    col_stats.string_stats = self.calculate_string_stats(column);
                }
                _ => {}
            }
            
            stats.push(col_stats);
        }
        
        Some(stats)
    }
    
    fn calculate_numeric_stats(&self, column: &dyn Array) -> Option<NumericStats> {
        // Convert to f64 array for statistics
        let values: Vec<f64> = match column.data_type() {
            DataType::Float64 => {
                let array = column.as_any().downcast_ref::<arrow::array::Float64Array>()?;
                (0..array.len())
                    .filter_map(|i| if array.is_valid(i) { Some(array.value(i)) } else { None })
                    .collect()
            }
            DataType::Float32 => {
                let array = column.as_any().downcast_ref::<arrow::array::Float32Array>()?;
                (0..array.len())
                    .filter_map(|i| if array.is_valid(i) { Some(array.value(i) as f64) } else { None })
                    .collect()
            }
            DataType::Int64 => {
                let array = column.as_any().downcast_ref::<arrow::array::Int64Array>()?;
                (0..array.len())
                    .filter_map(|i| if array.is_valid(i) { Some(array.value(i) as f64) } else { None })
                    .collect()
            }
            _ => return None,
        };
        
        if values.is_empty() {
            return None;
        }
        
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let n = values.len() as f64;
        let sum: f64 = values.iter().sum();
        let mean = sum / n;
        
        let variance: f64 = values.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / n;
        let std_dev = variance.sqrt();
        
        let median = sorted[sorted.len() / 2];
        let q1 = sorted[sorted.len() / 4];
        let q3 = sorted[3 * sorted.len() / 4];
        
        Some(NumericStats {
            min: sorted[0],
            max: sorted[sorted.len() - 1],
            mean,
            std_dev,
            median,
            q1,
            q3,
        })
    }
    
    fn calculate_string_stats(&self, column: &dyn Array) -> Option<StringStats> {
        let array = column.as_any().downcast_ref::<arrow::array::StringArray>()?;
        
        let mut unique_values = std::collections::HashSet::new();
        let mut total_length = 0;
        let mut min_length = usize::MAX;
        let mut max_length = 0;
        let mut count = 0;
        
        for i in 0..array.len() {
            if array.is_valid(i) {
                let value = array.value(i);
                unique_values.insert(value);
                let len = value.len();
                total_length += len;
                min_length = min_length.min(len);
                max_length = max_length.max(len);
                count += 1;
            }
        }
        
        if count == 0 {
            return None;
        }
        
        Some(StringStats {
            unique_count: unique_values.len(),
            min_length,
            max_length,
            avg_length: total_length as f32 / count as f32,
        })
    }
}

impl SpaceView for SummaryStatsView {
    fn id(&self) -> &SpaceViewId {
        &self.id
    }
    
    fn display_name(&self) -> &str {
        &self.title
    }
    
    fn view_type(&self) -> &str {
        "SummaryStatsView"
    }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Update stats if navigation changed
        let nav_pos = ctx.navigation.get_context().position.clone();
        if self.last_navigation_pos.as_ref() != Some(&nav_pos) {
            self.cached_stats = self.calculate_stats(ctx);
            self.last_navigation_pos = Some(nav_pos);
        }
        
        // Display statistics
        if let Some(stats) = &self.cached_stats {
            ScrollArea::both()
                .id_source(format!("stats_{:?}", self.id))
                .show(ui, |ui| {
                    use egui_extras::{TableBuilder, Column};
                    
                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::initial(120.0).at_least(80.0)) // Column name
                        .column(Column::initial(80.0).at_least(60.0))  // Type
                        .column(Column::initial(80.0).at_least(60.0))  // Count
                        .column(Column::initial(80.0).at_least(60.0))  // Nulls
                        .column(Column::initial(80.0).at_least(60.0))  // Min
                        .column(Column::initial(80.0).at_least(60.0))  // Max
                        .column(Column::initial(80.0).at_least(60.0))  // Mean
                        .column(Column::initial(80.0).at_least(60.0))  // Std Dev
                        .column(Column::initial(80.0).at_least(60.0))  // Median
                        .header(20.0, |mut header| {
                            header.col(|ui| { ui.strong("Column"); });
                            header.col(|ui| { ui.strong("Type"); });
                            header.col(|ui| { ui.strong("Count"); });
                            header.col(|ui| { ui.strong("Nulls"); });
                            header.col(|ui| { ui.strong("Min"); });
                            header.col(|ui| { ui.strong("Max"); });
                            header.col(|ui| { ui.strong("Mean"); });
                            header.col(|ui| { ui.strong("Std Dev"); });
                            header.col(|ui| { ui.strong("Median"); });
                        })
                        .body(|mut body| {
                            for col_stats in stats {
                                body.row(18.0, |mut row| {
                                    row.col(|ui| { ui.label(&col_stats.name); });
                                    row.col(|ui| { ui.label(format!("{:?}", col_stats.data_type)); });
                                    row.col(|ui| { ui.label(col_stats.count.to_string()); });
                                    row.col(|ui| { ui.label(col_stats.null_count.to_string()); });
                                    
                                    if let Some(numeric) = &col_stats.numeric_stats {
                                        row.col(|ui| { ui.label(format!("{:.2}", numeric.min)); });
                                        row.col(|ui| { ui.label(format!("{:.2}", numeric.max)); });
                                        row.col(|ui| { ui.label(format!("{:.2}", numeric.mean)); });
                                        row.col(|ui| { ui.label(format!("{:.2}", numeric.std_dev)); });
                                        row.col(|ui| { ui.label(format!("{:.2}", numeric.median)); });
                                    } else if let Some(string) = &col_stats.string_stats {
                                        row.col(|ui| { ui.label(format!("{}", string.min_length)); });
                                        row.col(|ui| { ui.label(format!("{}", string.max_length)); });
                                        row.col(|ui| { ui.label(format!("{:.1}", string.avg_length)); });
                                        row.col(|ui| { ui.label("-"); });
                                        row.col(|ui| { ui.label(format!("{} unique", string.unique_count)); });
                                    } else {
                                        row.col(|ui| { ui.label("-"); });
                                        row.col(|ui| { ui.label("-"); });
                                        row.col(|ui| { ui.label("-"); });
                                        row.col(|ui| { ui.label("-"); });
                                        row.col(|ui| { ui.label("-"); });
                                    }
                                });
                            }
                        });
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({})
    }
    
    fn load_config(&mut self, _config: Value) {
        // No configuration to load
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {
        // No selection handling needed
    }
    
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {
        // Nothing to update per frame
    }
} 