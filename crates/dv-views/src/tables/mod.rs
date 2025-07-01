//! Table view implementation

use egui::{Ui, ScrollArea};
use arrow::record_batch::RecordBatch;
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
    
    // Column visibility
    column_visibility: std::collections::HashMap<String, bool>,
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
            column_visibility: std::collections::HashMap::new(),
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
    
    fn render_table(&mut self, ui: &mut Ui, data: &RecordBatch, ctx: &ViewerContext) {
        use egui_extras::{TableBuilder, Column};
        
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size * 1.5;
        let num_rows = data.num_rows().min(self.config.max_rows_displayed);
        let faint_bg_color = ui.style().visuals.faint_bg_color;
        let selection_bg_fill = ui.style().visuals.selection.bg_fill;
        let selection_stroke_color = ui.style().visuals.selection.stroke.color;
        
        // Get schema fields first to avoid lifetime issues
        let schema_fields = data.schema().fields().clone();
        
        // Track column visibility changes
        let mut column_visibility_changes: Vec<(String, bool)> = Vec::new();
        
        // Determine visible columns (indices only)
        let visible_column_indices: Vec<usize> = schema_fields
            .iter()
            .enumerate()
            .filter(|(_, field)| {
                self.column_visibility.get(field.name()).copied().unwrap_or(true)
            })
            .map(|(idx, _)| idx)
            .collect();
        
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
        
        for _ in 0..visible_column_indices.len() {
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
                
                for &col_idx in &visible_column_indices {
                    let field = &schema_fields[col_idx];
                    header.col(|ui| {
                        let response = ui.strong(field.name());
                        
                        // Right-click context menu for columns
                        response.context_menu(|ui| {
                            ui.label(egui::RichText::new(field.name()).strong());
                            ui.separator();
                            
                            if ui.button("📋 Copy Column Name").clicked() {
                                ui.output_mut(|o| o.copied_text = field.name().to_string());
                                ui.close_menu();
                            }
                            
                            if ui.button("🔢 Sort Ascending").clicked() {
                                // TODO: Implement sorting
                                ui.close_menu();
                            }
                            
                            if ui.button("🔢 Sort Descending").clicked() {
                                // TODO: Implement sorting
                                ui.close_menu();
                            }
                            
                            ui.separator();
                            
                            if ui.button("👁️ Hide Column").clicked() {
                                column_visibility_changes.push((field.name().clone(), false));
                                ui.close_menu();
                            }
                            
                            if ui.button("👁️ Hide All Others").clicked() {
                                for (idx, f) in schema_fields.iter().enumerate() {
                                    if idx != col_idx {
                                        column_visibility_changes.push((f.name().clone(), false));
                                    }
                                }
                                ui.close_menu();
                            }
                            
                            ui.separator();
                            
                            // Show column info
                            ui.label(format!("Type: {:?}", field.data_type()));
                        });
                    });
                }
            })
            .body(|body| {
                body.rows(text_height, num_rows, |row_index, mut row| {
                    // Check if any row is hovered in a plot
                    let hover_data = ctx.hovered_data.read();
                    let nav_pos = ctx.navigation.get_context().position.clone();
                    // Calculate actual row number for comparison
                    let actual_row_idx = match nav_pos {
                        NavigationPosition::Sequential(idx) => idx + row_index,
                        _ => row_index,
                    };
                    
                    // Check if this row should be highlighted
                    let is_highlighted = hover_data.point_index
                        .map(|hover_idx| hover_idx == actual_row_idx)
                        .unwrap_or(false);
                    
                    // Determine row color
                    let row_color = if is_highlighted {
                        Some(selection_bg_fill)
                    } else if row_index % 2 == 0 {
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
                            let actual_row = match nav_pos {
                                NavigationPosition::Sequential(idx) => idx + row_index,
                                NavigationPosition::Temporal(_) => row_index, // For temporal, show relative
                                NavigationPosition::Categorical(_) => row_index, // For categorical, show relative
                            };
                            
                            let response = ui.label(actual_row.to_string());
                            
                            // Right-click menu for row selection
                            response.context_menu(|ui| {
                                ui.label(egui::RichText::new(format!("Row {}", actual_row)).strong());
                                ui.separator();
                                
                                if ui.button("📋 Copy Row Data").clicked() {
                                    let mut row_data = Vec::new();
                                    for &col_idx in &visible_column_indices {
                                        let column = data.column(col_idx);
                                        let value = arrow::util::display::array_value_to_string(column, row_index).unwrap_or_default();
                                        row_data.push(value);
                                    }
                                    ui.output_mut(|o| o.copied_text = row_data.join("\t"));
                                    ui.close_menu();
                                }
                                
                                if ui.button("📊 Focus in All Views").clicked() {
                                    // TODO: Implement cross-view focus
                                    ui.close_menu();
                                }
                            });
                        });
                    }
                    
                    for &col_idx in &visible_column_indices {
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
                                value.clone()
                            };
                            
                            // Apply highlight text style if highlighted
                            let response = if is_highlighted {
                                ui.label(egui::RichText::new(display_value).color(selection_stroke_color))
                            } else {
                                ui.label(display_value)
                            };
                            
                            // Right-click context menu for cells
                            response.context_menu(|ui| {
                                ui.label(egui::RichText::new("Cell Actions").strong());
                                ui.separator();
                                
                                let value_clone = value.clone();
                                if ui.button("📋 Copy Value").clicked() {
                                    ui.output_mut(|o| o.copied_text = value_clone);
                                    ui.close_menu();
                                }
                                
                                if ui.button("🔍 Filter by Value").clicked() {
                                    // TODO: Implement filtering
                                    ui.close_menu();
                                }
                                
                                ui.separator();
                                ui.label(format!("Column: {}", schema_fields[col_idx].name()));
                                ui.label(format!("Value: {}", if value.len() > 30 { &value[..30] } else { &value }));
                            });
                        });
                    }
                });
            });
        
        // Apply column visibility changes after rendering
        for (col_name, visible) in column_visibility_changes {
            self.column_visibility.insert(col_name, visible);
        }
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
        if let Some(data) = self.cached_data.clone() {
            // Simple row/column count at the top
            ui.horizontal(|ui| {
                ui.label(format!("Rows: {}", data.num_rows()));
                ui.separator();
                ui.label(format!("Columns: {}", data.num_columns()));
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Column visibility menu
                    ui.menu_button("⚙ Columns", |ui| {
                        for field in data.schema().fields() {
                            let col_name = field.name();
                            let is_visible = self.column_visibility.get(col_name).copied().unwrap_or(true);
                            
                            if ui.checkbox(&mut is_visible.clone(), col_name).clicked() {
                                self.column_visibility.insert(col_name.clone(), !is_visible);
                            }
                        }
                        
                        ui.separator();
                        
                        if ui.button("Show All").clicked() {
                            self.column_visibility.clear();
                        }
                        
                        if ui.button("Hide All").clicked() {
                            for field in data.schema().fields() {
                                self.column_visibility.insert(field.name().clone(), false);
                            }
                        }
                    });
                });
            });
            
            ui.add_space(4.0);
            
            ScrollArea::both()
                .id_source(format!("table_{:?}", self.id))
                .show(ui, |ui| {
                    self.render_table(ui, &data, ctx);
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