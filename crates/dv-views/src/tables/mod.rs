//! Table view implementation

use egui::{Ui, ScrollArea};
use arrow::record_batch::RecordBatch;
use arrow::array::Array;
use serde_json::{json, Value};

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;

/// Configuration for table views
#[derive(Debug, Clone)]
pub struct TableConfig {
    pub show_row_numbers: bool,
    pub sortable_columns: bool,
    pub resizable_columns: bool,
    pub striped_rows: bool,
    pub max_rows_displayed: usize,
    pub column_widths: Vec<f32>,
}

impl Default for TableConfig {
    fn default() -> Self {
        Self {
            show_row_numbers: true,
            sortable_columns: true,
            resizable_columns: true,
            striped_rows: true,
            max_rows_displayed: 1000,
            column_widths: Vec::new(),
        }
    }
}

/// Table view that displays data in a tabular format
pub struct TableView {
    id: SpaceViewId,
    title: String,
    pub config: TableConfig,
    
    // State
    cached_data: Option<RecordBatch>,
    last_navigation_pos: Option<NavigationPosition>,
    _scroll_state: ScrollState,
}

#[derive(Default)]
struct ScrollState {
    _offset_x: f32,
    _offset_y: f32,
}

impl TableView {
    /// Create a new table view
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: TableConfig::default(),
            cached_data: None,
            last_navigation_pos: None,
            _scroll_state: ScrollState::default(),
        }
    }
    
    /// Fetch data from the current data source
    fn fetch_data(&mut self, ctx: &ViewerContext) -> Option<RecordBatch> {
        let data_source = ctx.data_source.read();
        let data_source = data_source.as_ref()?;
        
        // Get current navigation position
        let nav_pos = ctx.navigation.get_context().position.clone();
        
        // Query data at current position
        ctx.runtime_handle.block_on(
            data_source.query_at(&nav_pos)
        ).ok()
    }
    
    fn render_table(&self, ui: &mut Ui, data: &RecordBatch, ctx: &ViewerContext) {
        use egui_extras::{TableBuilder, Column};
        
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size * 1.5;
        let num_rows = data.num_rows().min(self.config.max_rows_displayed);
        let faint_bg_color = ui.style().visuals.faint_bg_color;
        
        let mut builder = TableBuilder::new(ui)
            .striped(self.config.striped_rows)
            .resizable(self.config.resizable_columns)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .min_scrolled_height(0.0)
            .vscroll(true);
        
        // Add columns
        if self.config.show_row_numbers {
            builder = builder.column(Column::initial(50.0).at_least(40.0));
        }
        
        for _ in 0..data.num_columns() {
            builder = builder.column(
                Column::initial(150.0)
                    .at_least(80.0)    // Minimum width
                    .at_most(400.0)    // Maximum width to prevent excessive expansion
                    .clip(true)
                    .resizable(self.config.resizable_columns)
            );
        }
        
        builder
            .header(20.0, |mut header| {
                if self.config.show_row_numbers {
                    header.col(|ui| {
                        ui.strong("#");
                    });
                }
                
                for field in data.schema().fields() {
                    header.col(|ui| {
                        ui.strong(field.name());
                    });
                }
            })
            .body(|body| {
                body.rows(text_height, num_rows, |row_index, mut row| {
                    let row_color = if row_index % 2 == 0 {
                        None
                    } else {
                        Some(faint_bg_color)
                    };
                    
                    if self.config.show_row_numbers {
                        row.col(|ui| {
                            if let Some(color) = row_color {
                                ui.painter().rect_filled(ui.available_rect_before_wrap(), 0.0, color);
                            }
                            // Show actual navigation position, not local row index
                            let nav_pos = ctx.navigation.get_context().position.clone();
                            let actual_row = match nav_pos {
                                NavigationPosition::Sequential(idx) => idx + row_index,
                                NavigationPosition::Temporal(_) => row_index, // For temporal, show relative
                                NavigationPosition::Categorical(_) => row_index, // For categorical, show relative
                            };
                            ui.label(actual_row.to_string());
                        });
                    }
                    
                    for col_idx in 0..data.num_columns() {
                        row.col(|ui| {
                            if let Some(color) = row_color {
                                ui.painter().rect_filled(ui.available_rect_before_wrap(), 0.0, color);
                            }
                            
                            let column = data.column(col_idx);
                            let value = arrow::util::display::array_value_to_string(column, row_index).unwrap_or_default();
                            
                            // Truncate long values
                            let display_value = if value.len() > 50 {
                                format!("{}...", &value[..50])
                            } else {
                                value
                            };
                            
                            ui.label(display_value);
                        });
                    }
                });
            });
    }
}

impl SpaceView for TableView {
    fn id(&self) -> &SpaceViewId {
        &self.id
    }
    
    fn display_name(&self) -> &str {
        &self.title
    }
    
    fn view_type(&self) -> &str {
        "TableView"
    }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Update data if navigation changed
        let nav_pos = ctx.navigation.get_context().position.clone();
        if self.last_navigation_pos.as_ref() != Some(&nav_pos) {
            self.cached_data = self.fetch_data(ctx);
            self.last_navigation_pos = Some(nav_pos);
        }
        
        // Draw the table
        if let Some(data) = &self.cached_data {
            // Show summary statistics in a clean panel
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("📊 Summary").strong());
                    ui.separator();
                    ui.label(format!("Rows: {}", data.num_rows()));
                    ui.separator();
                    ui.label(format!("Columns: {}", data.num_columns()));
                    
                    // Add basic statistics for numeric columns
                    for (idx, field) in data.schema().fields().iter().enumerate() {
                        if field.data_type().is_numeric() {
                            if let Some(column) = data.column(idx).as_any().downcast_ref::<arrow::array::Float64Array>() {
                                let values: Vec<f64> = (0..column.len())
                                    .filter_map(|i| if column.is_valid(i) { Some(column.value(i)) } else { None })
                                    .collect();
                                
                                if !values.is_empty() {
                                    let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                                    let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                                    let sum: f64 = values.iter().sum();
                                    let mean = sum / values.len() as f64;
                                    
                                    ui.separator();
                                    ui.label(format!("{}: min={:.2}, max={:.2}, mean={:.2}", 
                                        field.name(), min, max, mean));
                                }
                            }
                        }
                    }
                });
            });
            
            ui.add_space(4.0);
            
            ScrollArea::both()
                .id_source(&self.id)
                .show(ui, |ui| {
                    self.render_table(ui, data, ctx);
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "show_row_numbers": self.config.show_row_numbers,
            "sortable_columns": self.config.sortable_columns,
            "resizable_columns": self.config.resizable_columns,
            "striped_rows": self.config.striped_rows,
            "max_rows_displayed": self.config.max_rows_displayed,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(show_row_numbers) = config.get("show_row_numbers").and_then(|v| v.as_bool()) {
            self.config.show_row_numbers = show_row_numbers;
        }
        if let Some(sortable) = config.get("sortable_columns").and_then(|v| v.as_bool()) {
            self.config.sortable_columns = sortable;
        }
        if let Some(resizable) = config.get("resizable_columns").and_then(|v| v.as_bool()) {
            self.config.resizable_columns = resizable;
        }
        if let Some(striped) = config.get("striped_rows").and_then(|v| v.as_bool()) {
            self.config.striped_rows = striped;
        }
        if let Some(max_rows) = config.get("max_rows_displayed").and_then(|v| v.as_u64()) {
            self.config.max_rows_displayed = max_rows as usize;
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {
        // TODO: Highlight selected rows
    }
    
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {
        // Nothing to update per frame
    }
} 