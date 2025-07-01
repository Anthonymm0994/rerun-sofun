//! Modern dashboard builder with drag-and-drop layout and smart templates

use egui::{Context, Window, Ui, Vec2, Color32, Rect, Pos2, Sense, CursorIcon, Rounding, Stroke, Align2};
use arrow::datatypes::{Schema, DataType};
use std::sync::Arc;
use dv_views::{SpaceView, TimeSeriesView, TableView, ScatterPlotView};
use dv_core::navigation::NavigationMode;
use uuid;

/// Modern view builder dialog with visual layout editor
pub struct ViewBuilderDialog {
    /// Data schema
    schema: Arc<Schema>,
    
    /// Column metadata
    columns: ColumnMetadata,
    
    /// Dashboard templates
    templates: Vec<DashboardTemplate>,
    
    /// Current layout being edited
    layout: DashboardLayout,
    
    /// Selected template
    selected_template: Option<usize>,
    
    /// Dragging state
    drag_state: Option<DragState>,
    
    /// Navigation mode selection
    selected_nav_mode: NavigationModeChoice,
    
    /// Show dialog
    pub show: bool,
    
    /// Preview mode
    preview_mode: bool,
}

/// Column categorization and metadata
struct ColumnMetadata {
    numeric: Vec<ColumnInfo>,
    temporal: Vec<ColumnInfo>,
    categorical: Vec<ColumnInfo>,
    all: Vec<ColumnInfo>,
}

#[derive(Clone)]
struct ColumnInfo {
    name: String,
    data_type: String,
    icon: &'static str,
    sample_values: Vec<String>,
}

/// Dashboard template
struct DashboardTemplate {
    name: String,
    description: String,
    icon: &'static str,
    layout: DashboardLayout,
    required_columns: TemplateRequirements,
}

#[derive(Clone)]
struct DashboardLayout {
    grid_size: (usize, usize), // cols x rows
    cells: Vec<LayoutCell>,
}

#[derive(Clone)]
struct LayoutCell {
    id: String,
    grid_pos: (usize, usize), // x, y
    grid_span: (usize, usize), // width, height
    view_config: ViewConfig,
}

#[derive(Clone)]
enum ViewConfig {
    TimeSeries {
        title: String,
        x_column: Option<String>,
        y_columns: Vec<String>,
    },
    Scatter {
        title: String,
        x_column: String,
        y_column: String,
        color_column: Option<String>,
    },
    Table {
        title: String,
        columns: Vec<String>,
    },
    BarChart {
        title: String,
        category_column: String,
        value_column: String,
    },
    Empty,
}

struct TemplateRequirements {
    min_numeric: usize,
    min_temporal: usize,
    min_categorical: usize,
}

#[derive(Clone, PartialEq)]
enum NavigationModeChoice {
    RowIndex,
    Time(String),
    Category(String),
}

struct DragState {
    source_id: String,
    offset: Vec2,
}

impl ViewBuilderDialog {
    /// Create a new modern view builder
    pub fn new(schema: Arc<Schema>) -> Self {
        let columns = Self::analyze_schema(&schema);
        let templates = Self::create_templates(&columns);
        
        // Default navigation mode
        let selected_nav_mode = if !columns.temporal.is_empty() {
            NavigationModeChoice::Time(columns.temporal[0].name.clone())
        } else {
            NavigationModeChoice::RowIndex
        };
        
        // Start with empty 2x2 grid
        let layout = DashboardLayout {
            grid_size: (2, 2),
            cells: vec![],
        };
        
        Self {
            schema,
            columns,
            templates,
            layout,
            selected_template: None,
            drag_state: None,
            selected_nav_mode,
            show: true,
            preview_mode: false,
        }
    }
    
    /// Analyze schema and categorize columns
    fn analyze_schema(schema: &Arc<Schema>) -> ColumnMetadata {
        let mut numeric = Vec::new();
        let mut temporal = Vec::new();
        let mut categorical = Vec::new();
        let mut all = Vec::new();
        
        for field in schema.fields() {
            let (icon, category) = match field.data_type() {
                DataType::Float64 | DataType::Float32 | DataType::Int64 | DataType::Int32 => {
                    ("📊", "numeric")
                }
                DataType::Utf8 => {
                    let name_lower = field.name().to_lowercase();
                    if name_lower.contains("date") || name_lower.contains("time") || name_lower.contains("timestamp") {
                        ("⏱️", "temporal")
                    } else {
                        ("📝", "categorical")
                    }
                }
                DataType::Date32 | DataType::Date64 | DataType::Timestamp(_, _) => {
                    ("⏱️", "temporal")
                }
                DataType::Boolean => ("✓", "categorical"),
                _ => ("❓", "other"),
            };
            
            let col_info = ColumnInfo {
                name: field.name().clone(),
                data_type: format!("{:?}", field.data_type()),
                icon,
                sample_values: vec![], // TODO: Could fetch actual samples
            };
            
            all.push(col_info.clone());
            
            match category {
                "numeric" => numeric.push(col_info),
                "temporal" => temporal.push(col_info),
                "categorical" => categorical.push(col_info),
                _ => {}
            }
        }
        
        ColumnMetadata {
            numeric,
            temporal,
            categorical,
            all,
        }
    }
    
    /// Create dashboard templates based on available columns
    fn create_templates(columns: &ColumnMetadata) -> Vec<DashboardTemplate> {
        let mut templates = Vec::new();
        
        // Time Series Dashboard
        if !columns.numeric.is_empty() {
            templates.push(DashboardTemplate {
                name: "Time Series Dashboard".to_string(),
                description: "Track metrics over time with multiple synchronized charts".to_string(),
                icon: "📈",
                layout: DashboardLayout {
                    grid_size: (2, 2),
                    cells: vec![
                        LayoutCell {
                            id: "main-trends".to_string(),
                            grid_pos: (0, 0),
                            grid_span: (2, 1),
                            view_config: ViewConfig::TimeSeries {
                                title: "Main Trends".to_string(),
                                x_column: columns.temporal.first().map(|c| c.name.clone()),
                                y_columns: columns.numeric.iter().take(3).map(|c| c.name.clone()).collect(),
                            },
                        },
                        LayoutCell {
                            id: "detail-1".to_string(),
                            grid_pos: (0, 1),
                            grid_span: (1, 1),
                            view_config: ViewConfig::TimeSeries {
                                title: "Detail View 1".to_string(),
                                x_column: columns.temporal.first().map(|c| c.name.clone()),
                                y_columns: columns.numeric.iter().skip(3).take(2).map(|c| c.name.clone()).collect(),
                            },
                        },
                        LayoutCell {
                            id: "data-table".to_string(),
                            grid_pos: (1, 1),
                            grid_span: (1, 1),
                            view_config: ViewConfig::Table {
                                title: "Data Inspector".to_string(),
                                columns: columns.all.iter().take(5).map(|c| c.name.clone()).collect(),
                            },
                        },
                    ],
                },
                required_columns: TemplateRequirements {
                    min_numeric: 1,
                    min_temporal: 0,
                    min_categorical: 0,
                },
            });
        }
        
        // Correlation Analysis
        if columns.numeric.len() >= 2 {
            templates.push(DashboardTemplate {
                name: "Correlation Analysis".to_string(),
                description: "Explore relationships between variables".to_string(),
                icon: "🎯",
                layout: DashboardLayout {
                    grid_size: (2, 2),
                    cells: vec![
                        LayoutCell {
                            id: "scatter-main".to_string(),
                            grid_pos: (0, 0),
                            grid_span: (1, 1),
                            view_config: ViewConfig::Scatter {
                                title: "Primary Correlation".to_string(),
                                x_column: columns.numeric[0].name.clone(),
                                y_column: columns.numeric[1].name.clone(),
                                color_column: columns.categorical.first().map(|c| c.name.clone()),
                            },
                        },
                        LayoutCell {
                            id: "time-series".to_string(),
                            grid_pos: (1, 0),
                            grid_span: (1, 1),
                            view_config: ViewConfig::TimeSeries {
                                title: "Variable Trends".to_string(),
                                x_column: columns.temporal.first().map(|c| c.name.clone()),
                                y_columns: columns.numeric.iter().take(2).map(|c| c.name.clone()).collect(),
                            },
                        },
                        LayoutCell {
                            id: "table".to_string(),
                            grid_pos: (0, 1),
                            grid_span: (2, 1),
                            view_config: ViewConfig::Table {
                                title: "Full Dataset".to_string(),
                                columns: vec![],
                            },
                        },
                    ],
                },
                required_columns: TemplateRequirements {
                    min_numeric: 2,
                    min_temporal: 0,
                    min_categorical: 0,
                },
            });
        }
        
        // Categorical Analysis
        if !columns.categorical.is_empty() && !columns.numeric.is_empty() {
            templates.push(DashboardTemplate {
                name: "Categorical Analysis".to_string(),
                description: "Analyze data by categories with bar charts".to_string(),
                icon: "📊",
                layout: DashboardLayout {
                    grid_size: (2, 1),
                    cells: vec![
                        LayoutCell {
                            id: "bar-chart".to_string(),
                            grid_pos: (0, 0),
                            grid_span: (1, 1),
                            view_config: ViewConfig::BarChart {
                                title: "Category Breakdown".to_string(),
                                category_column: columns.categorical[0].name.clone(),
                                value_column: columns.numeric[0].name.clone(),
                            },
                        },
                        LayoutCell {
                            id: "table".to_string(),
                            grid_pos: (1, 0),
                            grid_span: (1, 1),
                            view_config: ViewConfig::Table {
                                title: "Category Details".to_string(),
                                columns: vec![
                                    columns.categorical[0].name.clone(),
                                    columns.numeric[0].name.clone(),
                                ],
                            },
                        },
                    ],
                },
                required_columns: TemplateRequirements {
                    min_numeric: 1,
                    min_temporal: 0,
                    min_categorical: 1,
                },
            });
        }
        
        // Always add custom template
        templates.push(DashboardTemplate {
            name: "Custom Layout".to_string(),
            description: "Start with a blank canvas".to_string(),
            icon: "🎨",
            layout: DashboardLayout {
                grid_size: (2, 2),
                cells: vec![],
            },
            required_columns: TemplateRequirements {
                min_numeric: 0,
                min_temporal: 0,
                min_categorical: 0,
            },
        });
        
        templates
    }
    
    /// Show the modern view builder dialog
    pub fn show_dialog(&mut self, ctx: &Context) -> Option<(Vec<Box<dyn SpaceView>>, NavigationMode)> {
        if !self.show {
            return None;
        }
        
        let mut result = None;
        
        Window::new("🎨 F.R.O.G. Dashboard Builder")
            .default_size([1000.0, 700.0])
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                // Modern header with tabs
                ui.horizontal(|ui| {
                    ui.style_mut().spacing.item_spacing = Vec2::new(16.0, 0.0);
                    
                    if ui.selectable_label(!self.preview_mode, "📐 Design").clicked() {
                        self.preview_mode = false;
                    }
                    if ui.selectable_label(self.preview_mode, "👁️ Preview").clicked() {
                        self.preview_mode = true;
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("❌ Cancel").clicked() {
                            self.show = false;
                        }
                        
                        ui.add_enabled_ui(!self.layout.cells.is_empty(), |ui| {
                            if ui.button(egui::RichText::new("✅ Create Dashboard").strong()).clicked() {
                                result = Some(self.build_views());
                                self.show = false;
                            }
                        });
                    });
                });
                
                ui.separator();
                
                if self.preview_mode {
                    self.show_preview(ui);
                } else {
                    self.show_designer(ui);
                }
            });
        
        result
    }
    
    /// Show the design interface
    fn show_designer(&mut self, ui: &mut Ui) {
        ui.columns(3, |columns| {
            // Left panel: Templates and columns
            columns[0].vertical(|ui| {
                ui.heading("📋 Templates");
                ui.add_space(8.0);
                
                let mut template_to_apply = None;
                
                for (idx, template) in self.templates.iter().enumerate() {
                    let is_compatible = self.is_template_compatible(template);
                    
                    ui.add_enabled_ui(is_compatible, |ui| {
                        let response = ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(template.icon).size(24.0));
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new(&template.name).strong());
                                    ui.label(egui::RichText::new(&template.description).small().weak());
                                });
                            });
                        }).response;
                        
                        if response.clicked() && is_compatible {
                            template_to_apply = Some(idx);
                        }
                        
                        if response.hovered() && !is_compatible {
                            response.on_hover_text("Not enough compatible columns for this template");
                        }
                    });
                    
                    ui.add_space(4.0);
                }
                
                if let Some(idx) = template_to_apply {
                    self.apply_template(idx);
                }
                
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);
                
                // Column browser
                ui.heading("📊 Columns");
                ui.add_space(8.0);
                
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if !self.columns.temporal.is_empty() {
                        ui.label(egui::RichText::new("⏱️ Time Columns").strong());
                        let temporal_cols = self.columns.temporal.clone();
                        for col in &temporal_cols {
                            self.show_draggable_column(ui, col);
                        }
                        ui.add_space(8.0);
                    }
                    
                    if !self.columns.numeric.is_empty() {
                        ui.label(egui::RichText::new("📊 Numeric Columns").strong());
                        let numeric_cols = self.columns.numeric.clone();
                        for col in &numeric_cols {
                            self.show_draggable_column(ui, col);
                        }
                        ui.add_space(8.0);
                    }
                    
                    if !self.columns.categorical.is_empty() {
                        ui.label(egui::RichText::new("📝 Categorical Columns").strong());
                        let categorical_cols = self.columns.categorical.clone();
                        for col in &categorical_cols {
                            self.show_draggable_column(ui, col);
                        }
                    }
                });
            });
            
            // Center panel: Layout editor
            columns[1].vertical(|ui| {
                ui.heading("🎨 Layout Canvas");
                ui.add_space(8.0);
                
                // Grid controls
                ui.horizontal(|ui| {
                    ui.label("Grid:");
                    if ui.small_button("-").clicked() && self.layout.grid_size.0 > 1 {
                        self.layout.grid_size.0 -= 1;
                        self.adjust_cells_to_grid();
                    }
                    ui.label(format!("{}x{}", self.layout.grid_size.0, self.layout.grid_size.1));
                    if ui.small_button("+").clicked() && self.layout.grid_size.0 < 4 {
                        self.layout.grid_size.0 += 1;
                    }
                    
                    ui.separator();
                    
                    if ui.small_button("-").clicked() && self.layout.grid_size.1 > 1 {
                        self.layout.grid_size.1 -= 1;
                        self.adjust_cells_to_grid();
                    }
                    if ui.small_button("+").clicked() && self.layout.grid_size.1 < 4 {
                        self.layout.grid_size.1 += 1;
                    }
                    
                    ui.separator();
                    
                    if ui.button("Clear All").clicked() {
                        self.layout.cells.clear();
                    }
                });
                
                ui.add_space(8.0);
                
                // Draw grid
                let available_size = ui.available_size();
                let grid_size = Vec2::new(
                    available_size.x.min(600.0),
                    available_size.y.min(400.0)
                );
                
                let (response, painter) = ui.allocate_painter(grid_size, Sense::hover());
                let rect = response.rect;
                
                // Draw grid background
                painter.rect_filled(rect, Rounding::same(4.0), Color32::from_gray(30));
                
                // Draw grid lines
                let cell_width = rect.width() / self.layout.grid_size.0 as f32;
                let cell_height = rect.height() / self.layout.grid_size.1 as f32;
                
                for i in 1..self.layout.grid_size.0 {
                    let x = rect.left() + i as f32 * cell_width;
                    painter.line_segment(
                        [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                        Stroke::new(1.0, Color32::from_gray(50))
                    );
                }
                
                for j in 1..self.layout.grid_size.1 {
                    let y = rect.top() + j as f32 * cell_height;
                    painter.line_segment(
                        [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                        Stroke::new(1.0, Color32::from_gray(50))
                    );
                }
                
                // Draw cells
                for cell in &self.layout.cells {
                    let cell_rect = Rect::from_min_size(
                        Pos2::new(
                            rect.left() + cell.grid_pos.0 as f32 * cell_width,
                            rect.top() + cell.grid_pos.1 as f32 * cell_height
                        ),
                        Vec2::new(
                            cell.grid_span.0 as f32 * cell_width - 4.0,
                            cell.grid_span.1 as f32 * cell_height - 4.0
                        )
                    );
                    
                    self.draw_layout_cell(ui, &painter, cell_rect, cell);
                }
                
                // Handle drop zones
                if response.hovered() {
                    if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                        if rect.contains(pointer_pos) {
                            // Calculate grid position
                            let grid_x = ((pointer_pos.x - rect.left()) / cell_width) as usize;
                            let grid_y = ((pointer_pos.y - rect.top()) / cell_height) as usize;
                            
                            // Highlight drop zone
                            let drop_rect = Rect::from_min_size(
                                Pos2::new(
                                    rect.left() + grid_x as f32 * cell_width,
                                    rect.top() + grid_y as f32 * cell_height
                                ),
                                Vec2::new(cell_width - 4.0, cell_height - 4.0)
                            );
                            
                            painter.rect(
                                drop_rect,
                                Rounding::same(4.0),
                                Color32::from_rgba_unmultiplied(100, 150, 250, 50),
                                Stroke::new(2.0, Color32::from_rgb(100, 150, 250))
                            );
                        }
                    }
                }
            });
            
            // Right panel: Properties
            columns[2].vertical(|ui| {
                ui.heading("⚙️ Properties");
                ui.add_space(8.0);
                
                // Navigation mode
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Navigation Mode").strong());
                    ui.add_space(4.0);
                    
                    ui.radio_value(&mut self.selected_nav_mode, NavigationModeChoice::RowIndex, "Row Index");
                    
                    if !self.columns.temporal.is_empty() {
                        ui.horizontal(|ui| {
                            let is_time = matches!(&self.selected_nav_mode, NavigationModeChoice::Time(_));
                            if ui.radio(is_time, "Time-based").clicked() {
                                self.selected_nav_mode = NavigationModeChoice::Time(self.columns.temporal[0].name.clone());
                            }
                            
                            if is_time {
                                if let NavigationModeChoice::Time(ref mut col) = &mut self.selected_nav_mode {
                                    egui::ComboBox::from_id_source("nav_time_col")
                                        .selected_text(col.as_str())
                                        .show_ui(ui, |ui| {
                                            for time_col in &self.columns.temporal {
                                                ui.selectable_value(col, time_col.name.clone(), &time_col.name);
                                            }
                                        });
                                }
                            }
                        });
                    }
                    
                    if !self.columns.categorical.is_empty() {
                        ui.horizontal(|ui| {
                            let is_cat = matches!(&self.selected_nav_mode, NavigationModeChoice::Category(_));
                            if ui.radio(is_cat, "Category").clicked() {
                                self.selected_nav_mode = NavigationModeChoice::Category(self.columns.categorical[0].name.clone());
                            }
                            
                            if is_cat {
                                if let NavigationModeChoice::Category(ref mut col) = &mut self.selected_nav_mode {
                                    egui::ComboBox::from_id_source("nav_cat_col")
                                        .selected_text(col.as_str())
                                        .show_ui(ui, |ui| {
                                            for cat_col in &self.columns.categorical {
                                                ui.selectable_value(col, cat_col.name.clone(), &cat_col.name);
                                            }
                                        });
                                }
                            }
                        });
                    }
                });
                
                ui.add_space(16.0);
                
                // Selected cell properties
                // TODO: Add cell property editor when a cell is selected
                
                // Quick actions
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Quick Actions").strong());
                    ui.add_space(4.0);
                    
                    if ui.button("➕ Add Time Series").clicked() {
                        self.add_view(ViewConfig::TimeSeries {
                            title: "New Time Series".to_string(),
                            x_column: self.columns.temporal.first().map(|c| c.name.clone()),
                            y_columns: vec![],
                        });
                    }
                    
                    if ui.button("➕ Add Scatter Plot").clicked() && self.columns.numeric.len() >= 2 {
                        self.add_view(ViewConfig::Scatter {
                            title: "New Scatter Plot".to_string(),
                            x_column: self.columns.numeric[0].name.clone(),
                            y_column: self.columns.numeric[1].name.clone(),
                            color_column: None,
                        });
                    }
                    
                    if ui.button("➕ Add Table").clicked() {
                        self.add_view(ViewConfig::Table {
                            title: "Data Table".to_string(),
                            columns: vec![],
                        });
                    }
                    
                    if ui.button("➕ Add Bar Chart").clicked() && !self.columns.categorical.is_empty() && !self.columns.numeric.is_empty() {
                        self.add_view(ViewConfig::BarChart {
                            title: "Bar Chart".to_string(),
                            category_column: self.columns.categorical[0].name.clone(),
                            value_column: self.columns.numeric[0].name.clone(),
                        });
                    }
                });
            });
        });
    }
    
    /// Show draggable column
    fn show_draggable_column(&mut self, ui: &mut Ui, col: &ColumnInfo) {
        let response = ui.horizontal(|ui| {
            ui.label(col.icon);
            ui.label(&col.name);
        }).response;
        
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }
        
        // TODO: Implement drag and drop
    }
    
    /// Draw a layout cell
    fn draw_layout_cell(&self, _ui: &Ui, painter: &egui::Painter, rect: Rect, cell: &LayoutCell) {
        // Cell background
        painter.rect_filled(
            rect.shrink(2.0),
            Rounding::same(4.0),
            Color32::from_gray(50)
        );
        
        // Cell border
        painter.rect_stroke(
            rect.shrink(2.0),
            Rounding::same(4.0),
            Stroke::new(1.0, Color32::from_gray(100))
        );
        
        // Cell content
        let icon = match &cell.view_config {
            ViewConfig::TimeSeries { .. } => "📈",
            ViewConfig::Scatter { .. } => "🎯",
            ViewConfig::Table { .. } => "📊",
            ViewConfig::BarChart { .. } => "📊",
            ViewConfig::Empty => "➕",
        };
        
        let title = match &cell.view_config {
            ViewConfig::TimeSeries { title, .. } => title,
            ViewConfig::Scatter { title, .. } => title,
            ViewConfig::Table { title, .. } => title,
            ViewConfig::BarChart { title, .. } => title,
            ViewConfig::Empty => "Empty",
        };
        
        // Draw icon and title
        painter.text(
            rect.center() - Vec2::new(0.0, 10.0),
            Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(24.0),
            Color32::from_gray(200)
        );
        
        painter.text(
            rect.center() + Vec2::new(0.0, 15.0),
            Align2::CENTER_CENTER,
            title,
            egui::FontId::proportional(12.0),
            Color32::from_gray(180)
        );
    }
    
    /// Check if template is compatible with available columns
    fn is_template_compatible(&self, template: &DashboardTemplate) -> bool {
        self.columns.numeric.len() >= template.required_columns.min_numeric &&
        self.columns.temporal.len() >= template.required_columns.min_temporal &&
        self.columns.categorical.len() >= template.required_columns.min_categorical
    }
    
    /// Apply a template to the layout
    fn apply_template(&mut self, template_idx: usize) {
        if let Some(template) = self.templates.get(template_idx) {
            self.layout = template.layout.clone();
            self.selected_template = Some(template_idx);
        }
    }
    
    /// Add a new view to the layout
    fn add_view(&mut self, config: ViewConfig) {
        // Find first empty grid position
        let mut found_pos = None;
        for y in 0..self.layout.grid_size.1 {
            for x in 0..self.layout.grid_size.0 {
                let pos = (x, y);
                if !self.layout.cells.iter().any(|c| c.grid_pos == pos) {
                    found_pos = Some(pos);
                    break;
                }
            }
            if found_pos.is_some() {
                break;
            }
        }
        
        if let Some(pos) = found_pos {
            let cell = LayoutCell {
                id: uuid::Uuid::new_v4().to_string(),
                grid_pos: pos,
                grid_span: (1, 1),
                view_config: config,
            };
            self.layout.cells.push(cell);
        }
    }
    
    /// Adjust cells to fit within grid bounds
    fn adjust_cells_to_grid(&mut self) {
        for cell in &mut self.layout.cells {
            if cell.grid_pos.0 >= self.layout.grid_size.0 {
                cell.grid_pos.0 = self.layout.grid_size.0 - 1;
            }
            if cell.grid_pos.1 >= self.layout.grid_size.1 {
                cell.grid_pos.1 = self.layout.grid_size.1 - 1;
            }
            
            if cell.grid_pos.0 + cell.grid_span.0 > self.layout.grid_size.0 {
                cell.grid_span.0 = self.layout.grid_size.0 - cell.grid_pos.0;
            }
            if cell.grid_pos.1 + cell.grid_span.1 > self.layout.grid_size.1 {
                cell.grid_span.1 = self.layout.grid_size.1 - cell.grid_pos.1;
            }
        }
    }
    
    /// Show preview of the layout
    fn show_preview(&self, ui: &mut Ui) {
        ui.centered_and_justified(|ui| {
            ui.label("Preview coming soon...");
            ui.label("This will show a live preview of your dashboard layout");
        });
    }
    
    /// Build the actual views from the layout
    fn build_views(&self) -> (Vec<Box<dyn SpaceView>>, NavigationMode) {
        let mut views: Vec<Box<dyn SpaceView>> = Vec::new();
        
        for cell in &self.layout.cells {
            match &cell.view_config {
                ViewConfig::TimeSeries { title, x_column, y_columns } => {
                    let id = uuid::Uuid::new_v4();
                    let mut view = TimeSeriesView::new(id, title.clone());
                    view.config.x_column = x_column.clone();
                    view.config.y_columns = y_columns.clone();
                    view.config.show_legend = true;
                    view.config.show_grid = true;
                    views.push(Box::new(view));
                }
                ViewConfig::Scatter { title, x_column, y_column, color_column } => {
                    let id = uuid::Uuid::new_v4();
                    let mut view = ScatterPlotView::new(id, title.clone());
                    view.config.x_column = x_column.clone();
                    view.config.y_column = y_column.clone();
                    view.config.color_column = color_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::Table { title, columns: _ } => {
                    let id = uuid::Uuid::new_v4();
                    let view = TableView::new(id, title.clone());
                    views.push(Box::new(view));
                }
                ViewConfig::BarChart { title, category_column, value_column } => {
                    // For now, create a time series view as placeholder
                    // TODO: Implement proper bar chart view
                    let id = uuid::Uuid::new_v4();
                    let mut view = TimeSeriesView::new(id, title.clone());
                    view.config.x_column = Some(category_column.clone());
                    view.config.y_columns = vec![value_column.clone()];
                    views.push(Box::new(view));
                }
                ViewConfig::Empty => {
                    // Skip empty cells
                }
            }
        }
        
        // Build navigation mode
        let nav_mode = match &self.selected_nav_mode {
            NavigationModeChoice::RowIndex => NavigationMode::Sequential,
            NavigationModeChoice::Time(_col) => {
                // TODO: Parse time column and create temporal mode
                NavigationMode::Temporal
            }
            NavigationModeChoice::Category(col) => {
                // TODO: Extract unique categories from column
                NavigationMode::Categorical { categories: vec![col.clone()] }
            }
        };
        
        (views, nav_mode)
    }
} 