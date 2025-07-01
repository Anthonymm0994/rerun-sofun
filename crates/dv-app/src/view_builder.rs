//! Modern dashboard builder with drag-and-drop layout and smart templates

use egui::{Context, Window, Ui, Vec2, Color32, Rect, Pos2, Sense, CursorIcon, Rounding, Stroke, Align2};
use arrow::datatypes::{Schema, DataType};
use std::sync::Arc;
use uuid;
use dv_views::{SpaceView, TimeSeriesView, TableView, ScatterPlotView, BarChartView, SummaryStatsView};
use dv_core::navigation::NavigationMode;

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
    
    /// Selected cell for editing
    selected_cell_id: Option<String>,
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

/// View configuration types
#[derive(Debug, Clone)]
pub enum ViewConfig {
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
    SummaryStats {
        title: String,
    },
    Line {
        title: String,
        x_column: Option<String>,
        y_columns: Vec<String>,
    },
    Histogram {
        title: String,
        column: String,
    },
    BoxPlot {
        title: String,
        value_column: String,
        category_column: Option<String>,
    },
    ViolinPlot {
        title: String,
        value_column: String,
        category_column: Option<String>,
    },
    Heatmap {
        title: String,
        x_column: String,
        y_column: String,
        value_column: String,
    },
    AnomalyDetection {
        title: String,
        column: String,
    },
    CorrelationMatrix {
        title: String,
        columns: Vec<String>,
    },
    Scatter3D {
        title: String,
        x_column: String,
        y_column: String,
        z_column: String,
    },
    Surface3D {
        title: String,
        x_column: String,
        y_column: String,
        z_column: String,
    },
    ParallelCoordinates {
        title: String,
        columns: Vec<String>,
    },
    RadarChart {
        title: String,
        value_columns: Vec<String>,
        group_column: Option<String>,
    },
    Contour {
        title: String,
        x_column: String,
        y_column: String,
        z_column: String,
    },
    Sankey {
        title: String,
        source_column: String,
        target_column: String,
        value_column: String,
    },
    Treemap {
        title: String,
        category_column: String,
        value_column: String,
    },
    Sunburst {
        title: String,
        hierarchy_columns: Vec<String>,
        value_column: Option<String>,
    },
    NetworkGraph {
        title: String,
        source_column: String,
        target_column: String,
    },
    Distribution {
        title: String,
        column: String,
    },
    TimeAnalysis {
        title: String,
        time_column: String,
        value_columns: Vec<String>,
    },
    GeoPlot {
        title: String,
        lat_column: String,
        lon_column: String,
        value_column: Option<String>,
    },
    StreamGraph {
        title: String,
        time_column: String,
        value_column: String,
        category_column: Option<String>,
    },
    CandlestickChart {
        title: String,
        time_column: String,
        open_column: String,
        high_column: String,
        low_column: String,
        close_column: String,
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
    dragging_column: Option<ColumnInfo>,
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
            selected_cell_id: None,
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
        
        // Mixed Layout Dashboard - New!
        if !columns.numeric.is_empty() {
            templates.push(DashboardTemplate {
                name: "Mixed Layout Dashboard".to_string(),
                description: "Two square views and one wide view (2 1x1 + 1 2x1)".to_string(),
                icon: "🎯",
                layout: DashboardLayout {
                    grid_size: (2, 2),
                    cells: vec![
                        LayoutCell {
                            id: "top-left".to_string(),
                            grid_pos: (0, 0),
                            grid_span: (1, 1),
                            view_config: ViewConfig::TimeSeries {
                                title: "Metric 1".to_string(),
                                x_column: columns.temporal.first().map(|c| c.name.clone()),
                                y_columns: columns.numeric.iter().take(1).map(|c| c.name.clone()).collect(),
                            },
                        },
                        LayoutCell {
                            id: "top-right".to_string(),
                            grid_pos: (1, 0),
                            grid_span: (1, 1),
                            view_config: ViewConfig::Scatter {
                                title: "Correlation".to_string(),
                                x_column: columns.numeric.get(0).map(|c| c.name.clone()).unwrap_or_default(),
                                y_column: columns.numeric.get(1).map(|c| c.name.clone()).unwrap_or_default(),
                                color_column: columns.categorical.first().map(|c| c.name.clone()),
                            },
                        },
                        LayoutCell {
                            id: "bottom-wide".to_string(),
                            grid_pos: (0, 1),
                            grid_span: (2, 1),
                            view_config: ViewConfig::Table {
                                title: "Data Table".to_string(),
                                columns: columns.all.iter().take(6).map(|c| c.name.clone()).collect(),
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
            
            // Vertical Split Dashboard
            templates.push(DashboardTemplate {
                name: "Vertical Split Dashboard".to_string(),
                description: "One tall view and two stacked views (1 1x2 + 2 1x1)".to_string(),
                icon: "📊",
                layout: DashboardLayout {
                    grid_size: (2, 2),
                    cells: vec![
                        LayoutCell {
                            id: "left-tall".to_string(),
                            grid_pos: (0, 0),
                            grid_span: (1, 2),
                            view_config: ViewConfig::TimeSeries {
                                title: "Main Timeline".to_string(),
                                x_column: columns.temporal.first().map(|c| c.name.clone()),
                                y_columns: columns.numeric.iter().take(3).map(|c| c.name.clone()).collect(),
                            },
                        },
                        LayoutCell {
                            id: "right-top".to_string(),
                            grid_pos: (1, 0),
                            grid_span: (1, 1),
                            view_config: ViewConfig::BarChart {
                                title: "Summary".to_string(),
                                category_column: columns.categorical.first().map(|c| c.name.clone()).unwrap_or_default(),
                                value_column: columns.numeric.first().map(|c| c.name.clone()).unwrap_or_default(),
                            },
                        },
                        LayoutCell {
                            id: "right-bottom".to_string(),
                            grid_pos: (1, 1),
                            grid_span: (1, 1),
                            view_config: ViewConfig::SummaryStats {
                                title: "Statistics".to_string(),
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
                            // Modern header
            ui.horizontal(|ui| {
                ui.style_mut().spacing.item_spacing = Vec2::new(16.0, 0.0);
                
                ui.label(egui::RichText::new("📐 Dashboard Designer").strong().size(16.0));
                
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
                
                self.show_designer(ui);
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
                        self.selected_cell_id = None;
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
                
                // Draw grid background with better contrast
                painter.rect_filled(rect, Rounding::same(4.0), Color32::from_gray(25));
                
                // Draw grid lines - more visible
                let cell_width = rect.width() / self.layout.grid_size.0 as f32;
                let cell_height = rect.height() / self.layout.grid_size.1 as f32;
                
                // Draw vertical lines
                for i in 1..self.layout.grid_size.0 {
                    let x = rect.left() + i as f32 * cell_width;
                    painter.line_segment(
                        [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                        Stroke::new(1.5, Color32::from_gray(70))  // Thicker and brighter
                    );
                }
                
                // Draw horizontal lines
                for j in 1..self.layout.grid_size.1 {
                    let y = rect.top() + j as f32 * cell_height;
                    painter.line_segment(
                        [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                        Stroke::new(1.5, Color32::from_gray(70))  // Thicker and brighter
                    );
                }
                
                // Draw outer border for better definition
                painter.rect_stroke(
                    rect,
                    Rounding::same(4.0),
                    Stroke::new(2.0, Color32::from_gray(90))
                );
                
                // Draw cells
                let mut clicked_cell_id = None;
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
                    
                    // Check if cell is clicked
                    let cell_response = ui.interact(cell_rect, ui.id().with(&cell.id), Sense::click());
                    if cell_response.clicked() {
                        clicked_cell_id = Some(cell.id.clone());
                    }
                    
                    let is_selected = self.selected_cell_id.as_ref() == Some(&cell.id);
                    self.draw_layout_cell(ui, &painter, cell_rect, cell, is_selected);
                }
                
                // Update selected cell after iteration
                if let Some(id) = clicked_cell_id {
                    self.selected_cell_id = Some(id);
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
                if let Some(selected_id) = &self.selected_cell_id.clone() {
                    // Find the cell and store needed data
                    let cell_data = self.layout.cells.iter()
                        .find(|c| c.id == *selected_id)
                        .map(|cell| (
                            cell.grid_pos.clone(),
                            cell.grid_span.clone(),
                        ));
                    
                    if let Some((grid_pos, grid_span)) = cell_data {
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("Cell Properties").strong());
                            ui.add_space(4.0);
                            
                            // Cell position and span controls
                            ui.horizontal(|ui| {
                                ui.label("Position:");
                                ui.label(format!("({}, {})", grid_pos.0, grid_pos.1));
                            });
                            
                            let mut span_changed = false;
                            let mut new_span = grid_span;
                            
                            ui.horizontal(|ui| {
                                ui.label("Size:");
                                if ui.small_button("-W").clicked() && new_span.0 > 1 {
                                    new_span.0 -= 1;
                                    span_changed = true;
                                }
                                ui.label(format!("{}x{}", new_span.0, new_span.1));
                                if ui.small_button("+W").clicked() && grid_pos.0 + new_span.0 < self.layout.grid_size.0 {
                                    new_span.0 += 1;
                                    span_changed = true;
                                }
                            });
                            
                            ui.horizontal(|ui| {
                                ui.label("     ");
                                if ui.small_button("-H").clicked() && new_span.1 > 1 {
                                    new_span.1 -= 1;
                                    span_changed = true;
                                }
                                ui.label("      ");
                                if ui.small_button("+H").clicked() && grid_pos.1 + new_span.1 < self.layout.grid_size.1 {
                                    new_span.1 += 1;
                                    span_changed = true;
                                }
                            });
                            
                            // Apply span changes
                            if span_changed {
                                if let Some(cell) = self.layout.cells.iter_mut().find(|c| c.id == *selected_id) {
                                    cell.grid_span = new_span;
                                }
                            }
                            
                            ui.separator();
                            
                            if ui.button("🗑️ Delete Cell").clicked() {
                                self.layout.cells.retain(|c| c.id != *selected_id);
                                self.selected_cell_id = None;
                            }
                        });
                    }
                }
                
                ui.add_space(16.0);
                
                // Quick actions
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Quick Actions").strong());
                    ui.add_space(4.0);
                    
                    egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                        ui.label(egui::RichText::new("Basic Plots").strong());
                        
                        if ui.button("📈 Add Time Series").clicked() {
                            self.add_view(ViewConfig::TimeSeries {
                                title: "New Time Series".to_string(),
                                x_column: self.columns.temporal.first().map(|c| c.name.clone()),
                                y_columns: vec![],
                            });
                        }
                        
                        if ui.button("📉 Add Line Plot").clicked() {
                            self.add_view(ViewConfig::Line {
                                title: "Line Plot".to_string(),
                                x_column: None,
                                y_columns: vec![],
                            });
                        }
                        
                        if ui.button("🎯 Add Scatter Plot").clicked() && self.columns.numeric.len() >= 2 {
                            self.add_view(ViewConfig::Scatter {
                                title: "New Scatter Plot".to_string(),
                                x_column: self.columns.numeric[0].name.clone(),
                                y_column: self.columns.numeric[1].name.clone(),
                                color_column: None,
                            });
                        }
                        
                        if ui.button("📊 Add Bar Chart").clicked() && !self.columns.categorical.is_empty() && !self.columns.numeric.is_empty() {
                            self.add_view(ViewConfig::BarChart {
                                title: "Bar Chart".to_string(),
                                category_column: self.columns.categorical[0].name.clone(),
                                value_column: self.columns.numeric[0].name.clone(),
                            });
                        }
                        
                        if ui.button("📊 Add Table").clicked() {
                            self.add_view(ViewConfig::Table {
                                title: "Data Table".to_string(),
                                columns: vec![],
                            });
                        }
                        
                        ui.separator();
                        ui.label(egui::RichText::new("Statistical Plots").strong());
                        
                        if ui.button("📊 Add Histogram").clicked() && !self.columns.numeric.is_empty() {
                            self.add_view(ViewConfig::Histogram {
                                title: "Histogram".to_string(),
                                column: self.columns.numeric[0].name.clone(),
                            });
                        }
                        
                        if ui.button("📦 Add Box Plot").clicked() && !self.columns.numeric.is_empty() {
                            self.add_view(ViewConfig::BoxPlot {
                                title: "Box Plot".to_string(),
                                value_column: self.columns.numeric[0].name.clone(),
                                category_column: self.columns.categorical.first().map(|c| c.name.clone()),
                            });
                        }
                        
                        if ui.button("🎻 Add Violin Plot").clicked() && !self.columns.numeric.is_empty() {
                            self.add_view(ViewConfig::ViolinPlot {
                                title: "Violin Plot".to_string(),
                                value_column: self.columns.numeric[0].name.clone(),
                                category_column: self.columns.categorical.first().map(|c| c.name.clone()),
                            });
                        }
                        
                        if ui.button("📊 Add Summary Stats").clicked() {
                            self.add_view(ViewConfig::SummaryStats {
                                title: "Summary Statistics".to_string(),
                            });
                        }
                        
                        if ui.button("🔗 Add Correlation Matrix").clicked() {
                            self.add_view(ViewConfig::CorrelationMatrix {
                                title: "Correlation Matrix".to_string(),
                                columns: vec![],
                            });
                        }
                        
                        ui.separator();
                        ui.label(egui::RichText::new("Advanced Analytics").strong());
                        
                        if ui.button("🚨 Add Anomaly Detection").clicked() && !self.columns.numeric.is_empty() {
                            self.add_view(ViewConfig::AnomalyDetection {
                                title: "Anomaly Detection".to_string(),
                                column: self.columns.numeric[0].name.clone(),
                            });
                        }
                        
                        if ui.button("🔥 Add Heatmap").clicked() && self.columns.numeric.len() >= 1 {
                            self.add_view(ViewConfig::Heatmap {
                                title: "Heatmap".to_string(),
                                x_column: self.columns.all[0].name.clone(),
                                y_column: self.columns.all.get(1).map(|c| c.name.clone()).unwrap_or(self.columns.all[0].name.clone()),
                                value_column: self.columns.numeric[0].name.clone(),
                            });
                        }
                        
                        ui.separator();
                        ui.label(egui::RichText::new("3D Visualizations").strong());
                        
                        if ui.button("🌐 Add 3D Scatter").clicked() && self.columns.numeric.len() >= 3 {
                            self.add_view(ViewConfig::Scatter3D {
                                title: "3D Scatter".to_string(),
                                x_column: self.columns.numeric[0].name.clone(),
                                y_column: self.columns.numeric[1].name.clone(),
                                z_column: self.columns.numeric[2].name.clone(),
                            });
                        }
                        
                        if ui.button("🏔️ Add 3D Surface").clicked() && self.columns.numeric.len() >= 3 {
                            self.add_view(ViewConfig::Surface3D {
                                title: "3D Surface".to_string(),
                                x_column: self.columns.numeric[0].name.clone(),
                                y_column: self.columns.numeric[1].name.clone(),
                                z_column: self.columns.numeric[2].name.clone(),
                            });
                        }
                        
                        if ui.button("🗺️ Add Contour Plot").clicked() && self.columns.numeric.len() >= 3 {
                            self.add_view(ViewConfig::Contour {
                                title: "Contour Plot".to_string(),
                                x_column: self.columns.numeric[0].name.clone(),
                                y_column: self.columns.numeric[1].name.clone(),
                                z_column: self.columns.numeric[2].name.clone(),
                            });
                        }
                        
                        ui.separator();
                        ui.label(egui::RichText::new("Multi-dimensional").strong());
                        
                        if ui.button("🌟 Add Parallel Coordinates").clicked() {
                            self.add_view(ViewConfig::ParallelCoordinates {
                                title: "Parallel Coordinates".to_string(),
                                columns: vec![],
                            });
                        }
                        
                        if ui.button("🕸️ Add Radar Chart").clicked() {
                            self.add_view(ViewConfig::RadarChart {
                                title: "Radar Chart".to_string(),
                                value_columns: vec![],
                                group_column: None,
                            });
                        }
                        
                        ui.separator();
                        ui.label(egui::RichText::new("Flow & Hierarchy").strong());
                        
                        if ui.button("🌊 Add Sankey Diagram").clicked() && self.columns.categorical.len() >= 2 {
                            self.add_view(ViewConfig::Sankey {
                                title: "Sankey Diagram".to_string(),
                                source_column: self.columns.categorical[0].name.clone(),
                                target_column: self.columns.categorical[1].name.clone(),
                                value_column: self.columns.numeric.first().map(|c| c.name.clone()).unwrap_or_default(),
                            });
                        }
                        
                        if ui.button("🌳 Add Treemap").clicked() && !self.columns.categorical.is_empty() && !self.columns.numeric.is_empty() {
                            self.add_view(ViewConfig::Treemap {
                                title: "Treemap".to_string(),
                                category_column: self.columns.categorical[0].name.clone(),
                                value_column: self.columns.numeric[0].name.clone(),
                            });
                        }
                        
                        if ui.button("☀️ Add Sunburst").clicked() && !self.columns.numeric.is_empty() {
                            self.add_view(ViewConfig::Sunburst {
                                title: "Sunburst".to_string(),
                                hierarchy_columns: vec![],
                                value_column: Some(self.columns.numeric.first().map(|c| c.name.clone()).unwrap_or_default()),
                            });
                        }
                        
                        if ui.button("🔗 Add Network Graph").clicked() && self.columns.categorical.len() >= 2 {
                            self.add_view(ViewConfig::NetworkGraph {
                                title: "Network Graph".to_string(),
                                source_column: self.columns.categorical[0].name.clone(),
                                target_column: self.columns.categorical[1].name.clone(),
                            });
                        }
                        
                        ui.separator();
                        ui.label(egui::RichText::new("Specialized").strong());
                        
                        if ui.button("📈 Add Distribution Plot").clicked() && !self.columns.numeric.is_empty() {
                            self.add_view(ViewConfig::Distribution {
                                title: "Distribution".to_string(),
                                column: self.columns.numeric[0].name.clone(),
                            });
                        }
                        
                        if ui.button("⏰ Add Time Analysis").clicked() && !self.columns.temporal.is_empty() {
                            self.add_view(ViewConfig::TimeAnalysis {
                                title: "Time Analysis".to_string(),
                                time_column: self.columns.temporal[0].name.clone(),
                                value_columns: vec![],
                            });
                        }
                        
                        if ui.button("🌍 Add Geographic Plot").clicked() {
                            self.add_view(ViewConfig::GeoPlot {
                                title: "Geographic Plot".to_string(),
                                lat_column: String::new(),
                                lon_column: String::new(),
                                value_column: None,
                            });
                        }
                        
                        ui.separator();
                        ui.label(egui::RichText::new("Financial & Time Series").strong());
                        
                        if ui.button("🕯️ Add Candlestick Chart").clicked() {
                            self.add_view(ViewConfig::CandlestickChart {
                                title: "Candlestick Chart".to_string(),
                                time_column: self.columns.temporal.first().map(|c| c.name.clone()).unwrap_or_default(),
                                open_column: self.columns.numeric.get(0).map(|c| c.name.clone()).unwrap_or_default(),
                                high_column: self.columns.numeric.get(1).map(|c| c.name.clone()).unwrap_or_default(),
                                low_column: self.columns.numeric.get(2).map(|c| c.name.clone()).unwrap_or_default(),
                                close_column: self.columns.numeric.get(3).map(|c| c.name.clone()).unwrap_or_default(),
                            });
                        }
                        
                        if ui.button("🌊 Add Stream Graph").clicked() && !self.columns.temporal.is_empty() && !self.columns.numeric.is_empty() {
                            self.add_view(ViewConfig::StreamGraph {
                                title: "Stream Graph".to_string(),
                                time_column: self.columns.temporal[0].name.clone(),
                                value_column: self.columns.numeric[0].name.clone(),
                                category_column: self.columns.categorical.first().map(|c| c.name.clone()),
                            });
                        }
                    });
                });
            });
        });
    }
    
    /// Show draggable column
    fn show_draggable_column(&mut self, ui: &mut Ui, col: &ColumnInfo) {
        let id = ui.make_persistent_id(&col.name);
        let response = ui.horizontal(|ui| {
            ui.label(col.icon);
            ui.label(&col.name);
            ui.label(egui::RichText::new(&col.data_type).small().weak());
        }).response;
        
        // Make it draggable
        let response = ui.interact(response.rect, id, egui::Sense::drag());
        
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }
        
        // Add hover text separately to avoid move issue
        let response = response.on_hover_text(format!("Type: {}\nDrag to a cell to add", col.data_type));
        
        if response.drag_started() {
            self.drag_state = Some(DragState {
                source_id: col.name.clone(),
                offset: Vec2::ZERO,
                dragging_column: Some(col.clone()),
            });
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        }
        
        if response.dragged() && self.drag_state.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            
            // Show drag preview
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let layer_id = egui::LayerId::new(egui::Order::Tooltip, id);
                let painter = ui.ctx().layer_painter(layer_id);
                
                let text = format!("{} {}", col.icon, col.name);
                let galley = painter.layout_no_wrap(
                    text,
                    egui::FontId::default(),
                    Color32::from_white_alpha(200)
                );
                
                let rect = egui::Rect::from_min_size(
                    pointer_pos - Vec2::new(galley.size().x / 2.0, galley.size().y / 2.0),
                    galley.size()
                );
                
                painter.rect_filled(
                    rect.expand(4.0),
                    Rounding::same(4.0),
                    Color32::from_black_alpha(180)
                );
                
                painter.galley(rect.min, galley);
            }
        }
        
        if response.drag_released() {
            self.drag_state = None;
        }
    }
    
    /// Draw a layout cell
    fn draw_layout_cell(&self, _ui: &Ui, painter: &egui::Painter, rect: Rect, cell: &LayoutCell, is_selected: bool) {
        // Cell background
        let bg_color = if is_selected {
            Color32::from_gray(65)
        } else {
            Color32::from_gray(50)
        };
        
        painter.rect_filled(
            rect.shrink(2.0),
            Rounding::same(4.0),
            bg_color
        );
        
        // Cell border
        let border_color = if is_selected {
            Color32::from_rgb(100, 150, 250)
        } else {
            Color32::from_gray(100)
        };
        
        let border_width = if is_selected { 2.0 } else { 1.0 };
        
        painter.rect_stroke(
            rect.shrink(2.0),
            Rounding::same(4.0),
            Stroke::new(border_width, border_color)
        );
        
        // Cell content
        let icon = match &cell.view_config {
            ViewConfig::TimeSeries { .. } => "📈",
            ViewConfig::Line { .. } => "📉",
            ViewConfig::Scatter { .. } => "🎯",
            ViewConfig::Table { .. } => "📊",
            ViewConfig::BarChart { .. } => "📊",
            ViewConfig::Histogram { .. } => "📊",
            ViewConfig::BoxPlot { .. } => "📦",
            ViewConfig::ViolinPlot { .. } => "🎻",
            ViewConfig::Heatmap { .. } => "🔥",
            ViewConfig::AnomalyDetection { .. } => "🚨",
            ViewConfig::CorrelationMatrix { .. } => "🔗",
            ViewConfig::Scatter3D { .. } => "🌐",
            ViewConfig::Surface3D { .. } => "🏔️",
            ViewConfig::ParallelCoordinates { .. } => "🌟",
            ViewConfig::RadarChart { .. } => "🕸️",
            ViewConfig::Contour { .. } => "🗺️",
            ViewConfig::Sankey { .. } => "🌊",
            ViewConfig::Treemap { .. } => "🌳",
            ViewConfig::Sunburst { .. } => "☀️",
            ViewConfig::NetworkGraph { .. } => "🔗",
            ViewConfig::Distribution { .. } => "📈",
            ViewConfig::TimeAnalysis { .. } => "⏰",
            ViewConfig::GeoPlot { .. } => "🌍",
            ViewConfig::SummaryStats { .. } => "📊",
            ViewConfig::StreamGraph { .. } => "🌊",
            ViewConfig::CandlestickChart { .. } => "🕯️",
            ViewConfig::Empty => "➕",
        };
        
        let title = match &cell.view_config {
            ViewConfig::TimeSeries { title, .. } => title,
            ViewConfig::Line { title, .. } => title,
            ViewConfig::Scatter { title, .. } => title,
            ViewConfig::Table { title, .. } => title,
            ViewConfig::BarChart { title, .. } => title,
            ViewConfig::Histogram { title, .. } => title,
            ViewConfig::BoxPlot { title, .. } => title,
            ViewConfig::ViolinPlot { title, .. } => title,
            ViewConfig::Heatmap { title, .. } => title,
            ViewConfig::AnomalyDetection { title, .. } => title,
            ViewConfig::CorrelationMatrix { title, .. } => title,
            ViewConfig::Scatter3D { title, .. } => title,
            ViewConfig::Surface3D { title, .. } => title,
            ViewConfig::ParallelCoordinates { title, .. } => title,
            ViewConfig::RadarChart { title, .. } => title,
            ViewConfig::Contour { title, .. } => title,
            ViewConfig::Sankey { title, .. } => title,
            ViewConfig::Treemap { title, .. } => title,
            ViewConfig::Sunburst { title, .. } => title,
            ViewConfig::NetworkGraph { title, .. } => title,
            ViewConfig::Distribution { title, .. } => title,
            ViewConfig::TimeAnalysis { title, .. } => title,
            ViewConfig::GeoPlot { title, .. } => title,
            ViewConfig::SummaryStats { title, .. } => title,
            ViewConfig::StreamGraph { title, .. } => title,
            ViewConfig::CandlestickChart { title, .. } => title,
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
                ViewConfig::Line { title, x_column, y_columns } => {
                    // LinePlotView doesn't exist yet - using TimeSeriesView as fallback
                    let id = uuid::Uuid::new_v4();
                    let mut view = TimeSeriesView::new(id, title.clone());
                    view.config.x_column = x_column.clone();
                    view.config.y_columns = y_columns.clone();
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
                    let id = uuid::Uuid::new_v4();
                    let mut view = BarChartView::new(id, title.clone());
                    view.config.category_column = category_column.clone();
                    view.config.value_column = value_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::Histogram { title, column } => {
                    // HistogramView doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let mut view = HistogramView::new(id, title.clone());
                    // view.config.column = column.clone();
                    // views.push(Box::new(view));
                }
                ViewConfig::BoxPlot { title, value_column, category_column } => {
                    // BoxPlotView doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let mut view = BoxPlotView::new(id, title.clone());
                    // view.config.value_column = value_column.clone();
                    // view.config.category_column = category_column.clone();
                    // views.push(Box::new(view));
                }
                ViewConfig::ViolinPlot { title, value_column, category_column } => {
                    // ViolinPlotView doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let mut view = ViolinPlotView::new(id, title.clone());
                    // view.config.value_column = value_column.clone();
                    // view.config.category_column = category_column.clone();
                    // views.push(Box::new(view));
                }
                ViewConfig::Heatmap { title, x_column, y_column, value_column } => {
                    // HeatmapView doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let mut view = HeatmapView::new(id, title.clone());
                    // view.config.x_column = x_column.clone();
                    // view.config.y_column = y_column.clone();
                    // view.config.value_column = value_column.clone();
                    // views.push(Box::new(view));
                }
                ViewConfig::AnomalyDetection { title, column } => {
                    // AnomalyDetectionView doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let mut view = AnomalyDetectionView::new(id, title.clone());
                    // view.config.column = column.clone();
                    // views.push(Box::new(view));
                }
                ViewConfig::CorrelationMatrix { title, columns } => {
                    // CorrelationMatrixView doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let mut view = CorrelationMatrixView::new(id, title.clone());
                    // view.config.columns = columns.clone();
                    // views.push(Box::new(view));
                }
                ViewConfig::Scatter3D { title, x_column, y_column, z_column } => {
                    // Scatter3DView doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let view = Scatter3DView::new(id, title.clone());
                    // views.push(Box::new(view));
                }
                ViewConfig::Surface3D { title, x_column, y_column, z_column } => {
                    // Surface3DView doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let view = Surface3DView::new(id, title.clone());
                    // views.push(Box::new(view));
                }
                ViewConfig::ParallelCoordinates { title, columns } => {
                    // ParallelCoordinatesPlot doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let view = ParallelCoordinatesPlot::new(id, title.clone());
                    // views.push(Box::new(view));
                }
                ViewConfig::RadarChart { title, value_columns, group_column } => {
                    // RadarChart doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let mut view = RadarChart::new(id, title.clone());
                    // view.config.value_columns = value_columns.clone();
                    // view.config.group_column = group_column.clone();
                    // views.push(Box::new(view));
                }
                ViewConfig::Contour { title, x_column, y_column, z_column } => {
                    // ContourPlotView doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let view = ContourPlotView::new(id, title.clone());
                    // views.push(Box::new(view));
                }
                ViewConfig::Sankey { title, source_column, target_column, value_column } => {
                    // SankeyDiagramView doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let view = SankeyDiagramView::new(id, title.clone());
                    // views.push(Box::new(view));
                }
                ViewConfig::Treemap { title, category_column, value_column } => {
                    // TreemapView doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let view = TreemapView::new(id, title.clone());
                    // views.push(Box::new(view));
                }
                ViewConfig::Sunburst { title, hierarchy_columns, value_column } => {
                    // SunburstView doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let view = SunburstView::new(id, title.clone());
                    // views.push(Box::new(view));
                }
                ViewConfig::NetworkGraph { title, source_column, target_column } => {
                    // NetworkGraphView doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let view = NetworkGraphView::new(id, title.clone());
                    // views.push(Box::new(view));
                }
                ViewConfig::Distribution { title, column } => {
                    // DistributionPlotView doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let view = DistributionPlotView::new(id, title.clone());
                    // views.push(Box::new(view));
                }
                ViewConfig::TimeAnalysis { title, time_column, value_columns } => {
                    // TimeAnalysisView doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let view = TimeAnalysisView::new(id, title.clone());
                    // views.push(Box::new(view));
                }
                ViewConfig::GeoPlot { title, lat_column, lon_column, value_column } => {
                    // GeoPlotView doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let view = GeoPlotView::new(id, title.clone());
                    // views.push(Box::new(view));
                }
                ViewConfig::SummaryStats { title } => {
                    let id = uuid::Uuid::new_v4();
                    let view = SummaryStatsView::new(id, title.clone());
                    views.push(Box::new(view));
                }
                ViewConfig::StreamGraph { title, time_column, value_column, category_column } => {
                    // StreamGraph doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let mut view = StreamGraph::new(id, title.clone());
                    // view.config.time_column = Some(time_column.clone());
                    // view.config.value_column = Some(value_column.clone());
                    // view.config.category_column = category_column.clone();
                    // views.push(Box::new(view));
                }
                ViewConfig::CandlestickChart { title, time_column, open_column, high_column, low_column, close_column } => {
                    // CandlestickChart doesn't exist yet - skip for now
                    // let id = uuid::Uuid::new_v4();
                    // let mut view = CandlestickChart::new(id, title.clone());
                    // view.config.time_column = Some(time_column.clone());
                    // view.config.open_column = Some(open_column.clone());
                    // view.config.high_column = Some(high_column.clone());
                    // view.config.low_column = Some(low_column.clone());
                    // view.config.close_column = Some(close_column.clone());
                    // views.push(Box::new(view));
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