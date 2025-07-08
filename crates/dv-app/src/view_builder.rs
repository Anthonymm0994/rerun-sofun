//! Modern dashboard builder with drag-and-drop layout and smart templates

use std::sync::Arc;
use arrow::datatypes::{Schema, DataType};
use egui::{Context, Ui, Vec2, Color32, Rect, Pos2, Sense, Rounding, Stroke, Align2};
use dv_views::SpaceView;
use dv_core::navigation::NavigationMode;
use uuid;
use dv_views::{TimeSeriesView, TableView, plots::{ScatterPlotView, BarChartView}};

/// Modern view builder dialog with visual layout editor
pub struct ViewBuilderDialog {
    /// Available data sources and their schemas
    data_sources: Vec<(String, Arc<Schema>)>,
    
    /// Currently selected data source
    selected_data_source: Option<String>,
    
    /// Column metadata for selected data source
    columns: ColumnMetadata,
    
    /// Dashboard templates
    templates: Vec<DashboardTemplate>,
    
    /// Current layout being edited
    layout: DashboardLayout,
    
    /// Selected template
    selected_template: Option<usize>,
    
    /// Navigation mode selection
    selected_nav_mode: NavigationModeChoice,
    
    /// Show dialog
    pub show: bool,
    
    /// Selected cell for editing
    selected_cell_id: Option<String>,
    
    /// Current plot being configured
    plot_config_state: PlotConfigState,
}

/// State for configuring a plot before adding it
#[derive(Clone)]
struct PlotConfigState {
    /// Selected plot type
    selected_plot_type: Option<PlotType>,
    
    /// Configuration for the selected plot
    config: ViewConfig,
    
    /// Whether the current config is valid
    is_valid: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum PlotType {
    TimeSeries,
    Line,
    Scatter,
    BarChart,
    Histogram,
    Table,
    BoxPlot,
    ViolinPlot,
    Heatmap,
    AnomalyDetection,
    CorrelationMatrix,
    Scatter3D,
    Surface3D,
    ParallelCoordinates,
    RadarChart,
    PolarPlot,
    Contour,
    Sankey,
    Treemap,
    Sunburst,
    NetworkGraph,
    Distribution,
    TimeAnalysis,
    GeoPlot,
    StreamGraph,
    CandlestickChart,
}

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

#[derive(Clone, Debug)]
struct LayoutCell {
    id: String,
    grid_pos: (usize, usize), // x, y
    grid_span: (usize, usize), // width, height
    view_config: ViewConfig,
}

/// View configuration
#[derive(Clone, Debug)]
pub enum ViewConfig {
    TimeSeries {
        title: String,
        data_source_id: Option<String>,
        x_column: Option<String>,
        y_columns: Vec<String>,
    },
    Scatter {
        title: String,
        data_source_id: Option<String>,
        x_column: String,
        y_column: String,
        color_column: Option<String>,
    },
    Table {
        title: String,
        data_source_id: Option<String>,
        columns: Vec<String>,
    },
    BarChart {
        title: String,
        data_source_id: Option<String>,
        category_column: String,
        value_column: String,
    },
    SummaryStats {
        title: String,
        data_source_id: Option<String>,
    },
    Line {
        title: String,
        data_source_id: Option<String>,
        x_column: Option<String>,
        y_columns: Vec<String>,
    },
    Histogram {
        title: String,
        data_source_id: Option<String>,
        column: String,
    },
    BoxPlot {
        title: String,
        data_source_id: Option<String>,
        value_column: String,
        category_column: Option<String>,
    },
    ViolinPlot {
        title: String,
        data_source_id: Option<String>,
        value_column: String,
        category_column: Option<String>,
    },
    Heatmap {
        title: String,
        data_source_id: Option<String>,
        x_column: String,
        y_column: String,
        value_column: String,
    },
    AnomalyDetection {
        title: String,
        data_source_id: Option<String>,
        column: String,
    },
    CorrelationMatrix {
        title: String,
        data_source_id: Option<String>,
        columns: Vec<String>,
    },
    Scatter3D {
        title: String,
        data_source_id: Option<String>,
        x_column: String,
        y_column: String,
        z_column: String,
    },
    Surface3D {
        title: String,
        data_source_id: Option<String>,
        x_column: String,
        y_column: String,
        z_column: String,
    },
    ParallelCoordinates {
        title: String,
        data_source_id: Option<String>,
        columns: Vec<String>,
    },
    RadarChart {
        title: String,
        data_source_id: Option<String>,
        value_columns: Vec<String>,
        group_column: Option<String>,
    },
    PolarPlot {
        title: String,
        data_source_id: Option<String>,
        angle_column: String,
        radius_column: String,
        category_column: Option<String>,
    },
    Contour {
        title: String,
        data_source_id: Option<String>,
        x_column: String,
        y_column: String,
        z_column: String,
    },
    Sankey {
        title: String,
        data_source_id: Option<String>,
        source_column: String,
        target_column: String,
        value_column: String,
    },
    Treemap {
        title: String,
        data_source_id: Option<String>,
        category_column: String,
        value_column: String,
    },
    Sunburst {
        title: String,
        data_source_id: Option<String>,
        hierarchy_columns: Vec<String>,
        value_column: Option<String>,
    },
    NetworkGraph {
        title: String,
        data_source_id: Option<String>,
        source_column: String,
        target_column: String,
    },
    Distribution {
        title: String,
        data_source_id: Option<String>,
        column: String,
    },
    TimeAnalysis {
        title: String,
        data_source_id: Option<String>,
        time_column: String,
        value_columns: Vec<String>,
    },
    GeoPlot {
        title: String,
        data_source_id: Option<String>,
        lat_column: String,
        lon_column: String,
        value_column: Option<String>,
    },
    StreamGraph {
        title: String,
        data_source_id: Option<String>,
        time_column: String,
        value_column: String,
        category_column: Option<String>,
    },
    CandlestickChart {
        title: String,
        data_source_id: Option<String>,
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

enum NavigationModeChoice {
    RowIndex,
    Time(String),
    Category(String),
}

/// Helper function to create a styled plot button
fn plot_button(selected: bool) -> impl egui::Widget {
    move |ui: &mut egui::Ui| -> egui::Response {
        // Create an empty button, text will be added separately
        let button = egui::Button::new("")
            .min_size(Vec2::new(160.0, 30.0))
            .selected(selected);
        
        ui.add(button)
    }
}

impl ViewBuilderDialog {
    /// Create a new modern view builder with multiple data sources
    pub fn new_multi(data_sources: Vec<(String, Arc<Schema>)>) -> Self {
        // Select first data source by default
        let selected_data_source = data_sources.first().map(|(id, _)| id.clone());
        
        // Analyze schema of selected source
        let columns = if let Some(source_id) = &selected_data_source {
            if let Some((_, schema)) = data_sources.iter().find(|(id, _)| id == source_id) {
                Self::analyze_schema(schema)
            } else {
                ColumnMetadata {
                    numeric: Vec::new(),
                    temporal: Vec::new(),
                    categorical: Vec::new(),
                    all: Vec::new(),
                }
            }
        } else {
            ColumnMetadata {
                numeric: Vec::new(),
                temporal: Vec::new(),
                categorical: Vec::new(),
                all: Vec::new(),
            }
        };
        
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
            data_sources,
            selected_data_source,
            columns,
            templates,
            layout,
            selected_template: None,
            selected_nav_mode,
            show: true,
            selected_cell_id: None,
            plot_config_state: PlotConfigState {
                selected_plot_type: None,
                config: ViewConfig::Empty,
                is_valid: false,
            },
        }
    }
    
    /// Create a new modern view builder (legacy single source)
    pub fn new(schema: Arc<Schema>) -> Self {
        Self::new_multi(vec![("Default".to_string(), schema)])
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
                                data_source_id: None,
                                x_column: if !columns.temporal.is_empty() { 
                                    Some(columns.temporal[0].name.clone()) 
                                } else { None },
                                y_columns: columns.numeric.iter().take(2).map(|c| c.name.clone()).collect(),
                            },
                        },
                        LayoutCell {
                            id: "detail-1".to_string(),
                            grid_pos: (0, 1),
                            grid_span: (1, 1),
                            view_config: ViewConfig::Scatter {
                                title: "Correlation".to_string(),
                                data_source_id: None,
                                x_column: columns.numeric.get(0).map(|c| c.name.clone()).unwrap_or_default(),
                                y_column: columns.numeric.get(1).map(|c| c.name.clone()).unwrap_or_default(),
                                color_column: None,
                            },
                        },
                        LayoutCell {
                            id: "data-table".to_string(),
                            grid_pos: (1, 1),
                            grid_span: (1, 1),
                            view_config: ViewConfig::Table {
                                title: "Data Inspector".to_string(),
                                data_source_id: None,
                                columns: vec![],
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
        
        // Full page dashboard builder
        egui::CentralPanel::default().show(ctx, |ui| {
            // Professional header bar with better styling
            egui::TopBottomPanel::top("dashboard_builder_header")
                .exact_height(60.0)
                .frame(
                    egui::Frame::none()
                        .fill(Color32::from_gray(24))
                        .inner_margin(egui::Margin::symmetric(20.0, 10.0))
                )
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Title section
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new("Dashboard Builder")
                                    .size(22.0)
                                    .color(Color32::WHITE)
                                    .strong()
                            );
                            ui.label(
                                egui::RichText::new("Design your custom data visualization layout")
                                    .size(12.0)
                                    .color(Color32::from_gray(160))
                            );
                        });
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 12.0;
                            
                            // Cancel button - subtle
                            if ui.add(
                                egui::Button::new(
                                    egui::RichText::new("Cancel")
                                        .size(14.0)
                                )
                                .fill(Color32::from_gray(40))
                                .min_size(Vec2::new(80.0, 32.0))
                            ).clicked() {
                                self.show = false;
                            }
                            
                            // Create button - only enabled when we have valid plots
                            let can_create = !self.layout.cells.is_empty() && 
                                           self.layout.cells.iter().all(|cell| self.is_config_valid(&cell.view_config));
                            
                            let create_button = egui::Button::new(
                                egui::RichText::new("Create Dashboard")
                                    .size(14.0)
                                    .color(Color32::WHITE)
                                    .strong()
                            )
                            .fill(if can_create { Color32::from_rgb(76, 175, 80) } else { Color32::from_gray(60) })
                            .min_size(Vec2::new(140.0, 32.0));
                            
                            let response = ui.add_enabled(can_create, create_button);
                            
                            if response.clicked() {
                                result = Some(self.build_views());
                                self.show = false;
                            }
                            
                            if !can_create {
                                if self.layout.cells.is_empty() {
                                    response.on_hover_text("Add at least one plot to create a dashboard");
                                } else {
                                    response.on_hover_text("All plots must have their required columns configured");
                                }
                            }
                            
                            ui.separator();
                            
                            // Status indicator
                            let plot_count = self.layout.cells.len();
                            let configured_count = self.layout.cells.iter()
                                .filter(|cell| self.is_config_valid(&cell.view_config))
                                .count();
                            
                            ui.label(
                                egui::RichText::new(format!("{}/{} configured", configured_count, plot_count))
                                    .size(14.0)
                                    .color(if configured_count == plot_count { 
                                        Color32::from_rgb(76, 175, 80) 
                                    } else { 
                                        Color32::from_rgb(255, 152, 0) 
                                    })
                            );
                        });
                    });
                });
            
            // Main content area using SidePanel and CentralPanel
            egui::SidePanel::left("dashboard_plot_configurator")
                .exact_width(350.0)
                .resizable(false)
                .frame(
                    egui::Frame::none()
                        .fill(Color32::from_gray(20))
                        .inner_margin(egui::Margin::same(12.0))
                )
                .show_inside(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            self.show_plot_configurator(ui);
                        });
                });
            
            // Right side: Canvas
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(Color32::from_gray(18))
                        .inner_margin(egui::Margin::same(12.0))
                )
                .show_inside(ui, |ui| {
                    self.show_canvas(ui);
                });
        });
        
        result
    }
    
    /// Show the plot configuration panel
    fn show_plot_configurator(&mut self, ui: &mut Ui) {
        // Templates section at the top
        ui.heading("📋 Quick Start Templates");
        ui.add_space(8.0);
        
        let templates_to_apply = {
            let mut template_idx = None;
            for (idx, template) in self.templates.iter().enumerate() {
                let is_compatible = self.is_template_compatible(template);
                
                ui.add_enabled_ui(is_compatible, |ui| {
                    let response = ui.add(
                        egui::Button::new("")
                            .min_size(Vec2::new(ui.available_width(), 50.0))
                            .fill(if is_compatible { Color32::from_gray(30) } else { Color32::from_gray(25) })
                    );
                    
                    // Draw custom content on top of button
                    let rect = response.rect;
                    ui.painter().text(
                        rect.left_center() + Vec2::new(15.0, -8.0),
                        Align2::LEFT_CENTER,
                        template.icon,
                        egui::FontId::proportional(18.0),
                        Color32::from_gray(200)
                    );
                    ui.painter().text(
                        rect.left_center() + Vec2::new(45.0, -8.0),
                        Align2::LEFT_CENTER,
                        &template.name,
                        egui::FontId::proportional(13.0),
                        Color32::WHITE
                    );
                    ui.painter().text(
                        rect.left_center() + Vec2::new(45.0, 8.0),
                        Align2::LEFT_CENTER,
                        &template.description,
                        egui::FontId::proportional(10.0),
                        Color32::from_gray(160)
                    );
                    
                    if response.clicked() && is_compatible {
                        template_idx = Some(idx);
                    }
                });
                ui.add_space(3.0);
            }
            template_idx
        };
        
        if let Some(idx) = templates_to_apply {
            self.apply_template(idx);
        }
        
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);
        
        // Two-column layout for plot selection and configuration
        ui.horizontal(|ui| {
            // Left column: Plot type selection (narrower)
            ui.vertical(|ui| {
                ui.set_min_width(140.0);
                ui.set_max_width(140.0);
                
                ui.heading("📊 Plot Types");
                ui.add_space(4.0);
                
                // Basic plots
                ui.label(egui::RichText::new("Basic").size(11.0).strong().color(Color32::from_gray(180)));
                ui.add_space(2.0);
                
                self.show_plot_type_button_compact(ui, "📈", "Time Series", PlotType::TimeSeries);
                self.show_plot_type_button_compact(ui, "📉", "Line", PlotType::Line);
                self.show_plot_type_button_compact(ui, "🎯", "Scatter", PlotType::Scatter);
                self.show_plot_type_button_compact(ui, "📊", "Bar Chart", PlotType::BarChart);
                self.show_plot_type_button_compact(ui, "📊", "Histogram", PlotType::Histogram);
                self.show_plot_type_button_compact(ui, "📋", "Table", PlotType::Table);
                
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Statistical").size(11.0).strong().color(Color32::from_gray(180)));
                ui.add_space(2.0);
                
                self.show_plot_type_button_compact(ui, "📦", "Box Plot", PlotType::BoxPlot);
                self.show_plot_type_button_compact(ui, "🎻", "Violin", PlotType::ViolinPlot);
                self.show_plot_type_button_compact(ui, "🔥", "Heatmap", PlotType::Heatmap);
                self.show_plot_type_button_compact(ui, "🎯", "Correlation", PlotType::CorrelationMatrix);
                self.show_plot_type_button_compact(ui, "🔔", "Distribution", PlotType::Distribution);
                self.show_plot_type_button_compact(ui, "⚠️", "Anomaly", PlotType::AnomalyDetection);
                
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Advanced").size(11.0).strong().color(Color32::from_gray(180)));
                ui.add_space(2.0);
                
                self.show_plot_type_button_compact(ui, "🎲", "3D Scatter", PlotType::Scatter3D);
                self.show_plot_type_button_compact(ui, "🏔️", "3D Surface", PlotType::Surface3D);
                self.show_plot_type_button_compact(ui, "🧭", "Polar", PlotType::PolarPlot);
                self.show_plot_type_button_compact(ui, "🗺️", "Contour", PlotType::Contour);
                self.show_plot_type_button_compact(ui, "🌈", "Parallel Coords", PlotType::ParallelCoordinates);
                self.show_plot_type_button_compact(ui, "🎯", "Radar", PlotType::RadarChart);
                
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Hierarchical").size(11.0).strong().color(Color32::from_gray(180)));
                ui.add_space(2.0);
                
                self.show_plot_type_button_compact(ui, "🧵", "Sankey", PlotType::Sankey);
                self.show_plot_type_button_compact(ui, "📊", "Treemap", PlotType::Treemap);
                self.show_plot_type_button_compact(ui, "🌞", "Sunburst", PlotType::Sunburst);
                self.show_plot_type_button_compact(ui, "🌐", "Network", PlotType::NetworkGraph);
                
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Specialized").size(11.0).strong().color(Color32::from_gray(180)));
                ui.add_space(2.0);
                
                self.show_plot_type_button_compact(ui, "⏳", "Time Analysis", PlotType::TimeAnalysis);
                self.show_plot_type_button_compact(ui, "🌍", "Geographic", PlotType::GeoPlot);
                self.show_plot_type_button_compact(ui, "🌊", "Stream", PlotType::StreamGraph);
                self.show_plot_type_button_compact(ui, "🕒", "Candlestick", PlotType::CandlestickChart);
            });
            
            ui.separator();
            
            // Right column: Configuration (wider)
            ui.vertical(|ui| {
                ui.set_min_width(180.0);
                
                if let Some(plot_type) = &self.plot_config_state.selected_plot_type {
                    ui.heading("⚙️ Configuration");
                    ui.add_space(4.0);
                    
                    // Show plot type name
                    let plot_name = match plot_type {
                        PlotType::TimeSeries => "Time Series",
                        PlotType::Line => "Line Plot",
                        PlotType::Scatter => "Scatter Plot",
                        PlotType::BarChart => "Bar Chart",
                        PlotType::Histogram => "Histogram",
                        PlotType::Table => "Data Table",
                        PlotType::BoxPlot => "Box Plot",
                        PlotType::ViolinPlot => "Violin Plot",
                        PlotType::Heatmap => "Heatmap",
                        PlotType::AnomalyDetection => "Anomaly Detection",
                        PlotType::CorrelationMatrix => "Correlation Matrix",
                        PlotType::Scatter3D => "3D Scatter",
                        PlotType::Surface3D => "3D Surface",
                        PlotType::ParallelCoordinates => "Parallel Coordinates",
                        PlotType::RadarChart => "Radar Chart",
                        PlotType::PolarPlot => "Polar Plot",
                        PlotType::Contour => "Contour Plot",
                        PlotType::Sankey => "Sankey Diagram",
                        PlotType::Treemap => "Treemap",
                        PlotType::Sunburst => "Sunburst",
                        PlotType::NetworkGraph => "Network Graph",
                        PlotType::Distribution => "Distribution",
                        PlotType::TimeAnalysis => "Time Analysis",
                        PlotType::GeoPlot => "Geographic Plot",
                        PlotType::StreamGraph => "Stream Graph",
                        PlotType::CandlestickChart => "Candlestick Chart",
                    };
                    
                    ui.label(egui::RichText::new(plot_name).size(13.0).strong().color(Color32::from_rgb(76, 175, 80)));
                    ui.add_space(8.0);
                    
                    // Configuration form
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            self.show_plot_configuration_compact(ui);
                        });
                    
                    ui.add_space(8.0);
                    
                    // Add button
                    let add_button = egui::Button::new(
                        egui::RichText::new("➕ Add to Dashboard")
                            .size(13.0)
                            .color(Color32::WHITE)
                    )
                    .fill(if self.plot_config_state.is_valid { 
                        Color32::from_rgb(76, 175, 80) 
                    } else { 
                        Color32::from_gray(60) 
                    })
                    .min_size(Vec2::new(ui.available_width(), 32.0));
                    
                    let response = ui.add_enabled(self.plot_config_state.is_valid, add_button);
                    
                    if response.clicked() {
                        self.add_configured_plot();
                    }
                    
                    if !self.plot_config_state.is_valid {
                        response.on_hover_text("Please configure all required columns");
                    }
                } else {
                    // No plot selected - show instructions
                    ui.heading("⚙️ Configuration");
                    ui.add_space(8.0);
                    
                    ui.label(egui::RichText::new("Select a plot type from the left to configure columns and add it to your dashboard.")
                        .size(12.0)
                        .color(Color32::from_gray(160)));
                    
                    ui.add_space(12.0);
                    
                    // Show available columns summary
                    ui.label(egui::RichText::new("Available Columns:").size(12.0).strong());
                    ui.add_space(4.0);
                    
                    if !self.columns.numeric.is_empty() {
                        ui.label(egui::RichText::new(format!("📊 {} numeric columns", self.columns.numeric.len()))
                            .size(11.0)
                            .color(Color32::from_rgb(76, 175, 80)));
                    }
                    
                    if !self.columns.temporal.is_empty() {
                        ui.label(egui::RichText::new(format!("⏰ {} temporal columns", self.columns.temporal.len()))
                            .size(11.0)
                            .color(Color32::from_rgb(33, 150, 243)));
                    }
                    
                    if !self.columns.categorical.is_empty() {
                        ui.label(egui::RichText::new(format!("🏷️ {} categorical columns", self.columns.categorical.len()))
                            .size(11.0)
                            .color(Color32::from_rgb(255, 152, 0)));
                    }
                }
            });
        });
    }
    
    /// Show a compact plot type selection button
    fn show_plot_type_button_compact(&mut self, ui: &mut Ui, icon: &str, name: &str, plot_type: PlotType) {
        let is_selected = self.plot_config_state.selected_plot_type.as_ref() == Some(&plot_type);
        
        let button_text = format!("{} {}", icon, name);
        let button = egui::Button::new(button_text)
            .min_size(Vec2::new(ui.available_width(), 24.0))
            .selected(is_selected)
            .fill(if is_selected { Color32::from_gray(45) } else { Color32::from_gray(30) });
        
        let response = ui.add(button);
        
        if response.clicked() {
            self.plot_config_state.selected_plot_type = Some(plot_type);
            self.plot_config_state.config = self.create_default_config(&self.plot_config_state.selected_plot_type.as_ref().unwrap());
            self.plot_config_state.is_valid = false;
        }
    }
    
    /// Create default configuration for a plot type
    fn create_default_config(&self, plot_type: &PlotType) -> ViewConfig {
        match plot_type {
            PlotType::TimeSeries => ViewConfig::TimeSeries {
                title: "Time Series".to_string(),
                data_source_id: self.selected_data_source.clone(),
                x_column: None,
                y_columns: vec![],
            },
            PlotType::Line => ViewConfig::Line {
                title: "Line Plot".to_string(),
                data_source_id: self.selected_data_source.clone(),
                x_column: None,
                y_columns: vec![],
            },
            PlotType::Scatter => ViewConfig::Scatter {
                title: "Scatter Plot".to_string(),
                data_source_id: self.selected_data_source.clone(),
                x_column: String::new(),
                y_column: String::new(),
                color_column: None,
            },
            PlotType::BarChart => ViewConfig::BarChart {
                title: "Bar Chart".to_string(),
                data_source_id: self.selected_data_source.clone(),
                category_column: String::new(),
                value_column: String::new(),
            },
            PlotType::Histogram => ViewConfig::Histogram {
                title: "Histogram".to_string(),
                data_source_id: self.selected_data_source.clone(),
                column: String::new(),
            },
            PlotType::Table => ViewConfig::Table {
                title: "Data Table".to_string(),
                data_source_id: self.selected_data_source.clone(),
                columns: vec![],
            },
            PlotType::BoxPlot => ViewConfig::BoxPlot {
                title: "Box Plot".to_string(),
                data_source_id: self.selected_data_source.clone(),
                value_column: String::new(),
                category_column: None,
            },
            PlotType::ViolinPlot => ViewConfig::ViolinPlot {
                title: "Violin Plot".to_string(),
                data_source_id: self.selected_data_source.clone(),
                value_column: String::new(),
                category_column: None,
            },
            PlotType::Heatmap => ViewConfig::Heatmap {
                title: "Heatmap".to_string(),
                data_source_id: self.selected_data_source.clone(),
                x_column: String::new(),
                y_column: String::new(),
                value_column: String::new(),
            },
            PlotType::AnomalyDetection => ViewConfig::AnomalyDetection {
                title: "Anomaly Detection".to_string(),
                data_source_id: self.selected_data_source.clone(),
                column: String::new(),
            },
            PlotType::CorrelationMatrix => ViewConfig::CorrelationMatrix {
                title: "Correlation Matrix".to_string(),
                data_source_id: self.selected_data_source.clone(),
                columns: vec![],
            },
            PlotType::Scatter3D => ViewConfig::Scatter3D {
                title: "3D Scatter".to_string(),
                data_source_id: self.selected_data_source.clone(),
                x_column: String::new(),
                y_column: String::new(),
                z_column: String::new(),
            },
            PlotType::Surface3D => ViewConfig::Surface3D {
                title: "3D Surface".to_string(),
                data_source_id: self.selected_data_source.clone(),
                x_column: String::new(),
                y_column: String::new(),
                z_column: String::new(),
            },
            PlotType::ParallelCoordinates => ViewConfig::ParallelCoordinates {
                title: "Parallel Coordinates".to_string(),
                data_source_id: self.selected_data_source.clone(),
                columns: vec![],
            },
            PlotType::RadarChart => ViewConfig::RadarChart {
                title: "Radar Chart".to_string(),
                data_source_id: self.selected_data_source.clone(),
                value_columns: vec![],
                group_column: None,
            },
            PlotType::PolarPlot => ViewConfig::PolarPlot {
                title: "Polar Plot".to_string(),
                data_source_id: self.selected_data_source.clone(),
                angle_column: String::new(),
                radius_column: String::new(),
                category_column: None,
            },
            PlotType::Contour => ViewConfig::Contour {
                title: "Contour Plot".to_string(),
                data_source_id: self.selected_data_source.clone(),
                x_column: String::new(),
                y_column: String::new(),
                z_column: String::new(),
            },
            PlotType::Sankey => ViewConfig::Sankey {
                title: "Sankey Diagram".to_string(),
                data_source_id: self.selected_data_source.clone(),
                source_column: String::new(),
                target_column: String::new(),
                value_column: String::new(),
            },
            PlotType::Treemap => ViewConfig::Treemap {
                title: "Treemap".to_string(),
                data_source_id: self.selected_data_source.clone(),
                category_column: String::new(),
                value_column: String::new(),
            },
            PlotType::Sunburst => ViewConfig::Sunburst {
                title: "Sunburst".to_string(),
                data_source_id: self.selected_data_source.clone(),
                hierarchy_columns: vec![],
                value_column: None,
            },
            PlotType::NetworkGraph => ViewConfig::NetworkGraph {
                title: "Network Graph".to_string(),
                data_source_id: self.selected_data_source.clone(),
                source_column: String::new(),
                target_column: String::new(),
            },
            PlotType::Distribution => ViewConfig::Distribution {
                title: "Distribution".to_string(),
                data_source_id: self.selected_data_source.clone(),
                column: String::new(),
            },
            PlotType::TimeAnalysis => ViewConfig::TimeAnalysis {
                title: "Time Analysis".to_string(),
                data_source_id: self.selected_data_source.clone(),
                time_column: String::new(),
                value_columns: vec![],
            },
            PlotType::GeoPlot => ViewConfig::GeoPlot {
                title: "Geographic Plot".to_string(),
                data_source_id: self.selected_data_source.clone(),
                lat_column: String::new(),
                lon_column: String::new(),
                value_column: None,
            },
            PlotType::StreamGraph => ViewConfig::StreamGraph {
                title: "Stream Graph".to_string(),
                data_source_id: self.selected_data_source.clone(),
                time_column: String::new(),
                value_column: String::new(),
                category_column: None,
            },
            PlotType::CandlestickChart => ViewConfig::CandlestickChart {
                title: "Candlestick Chart".to_string(),
                data_source_id: self.selected_data_source.clone(),
                time_column: String::new(),
                open_column: String::new(),
                high_column: String::new(),
                low_column: String::new(),
                close_column: String::new(),
            },
        }
    }
    
    /// Show plot configuration UI (compact version)
    fn show_plot_configuration_compact(&mut self, ui: &mut Ui) {
        let mut config = self.plot_config_state.config.clone();
        
        // Data source selector - shown for all plot types
        ui.horizontal(|ui| {
            ui.label("Data Source:");
            
            let current_source = match &config {
                ViewConfig::TimeSeries { data_source_id, .. } |
                ViewConfig::Scatter { data_source_id, .. } |
                ViewConfig::Table { data_source_id, .. } |
                ViewConfig::BarChart { data_source_id, .. } |
                ViewConfig::Histogram { data_source_id, .. } |
                ViewConfig::Line { data_source_id, .. } |
                ViewConfig::BoxPlot { data_source_id, .. } |
                ViewConfig::ViolinPlot { data_source_id, .. } |
                ViewConfig::Heatmap { data_source_id, .. } |
                ViewConfig::AnomalyDetection { data_source_id, .. } |
                ViewConfig::CorrelationMatrix { data_source_id, .. } |
                ViewConfig::Scatter3D { data_source_id, .. } |
                ViewConfig::Surface3D { data_source_id, .. } |
                ViewConfig::ParallelCoordinates { data_source_id, .. } |
                ViewConfig::RadarChart { data_source_id, .. } |
                ViewConfig::PolarPlot { data_source_id, .. } |
                ViewConfig::Distribution { data_source_id, .. } |
                ViewConfig::SummaryStats { data_source_id, .. } => {
                    data_source_id.as_deref().or(self.selected_data_source.as_deref()).unwrap_or("None")
                }
                _ => "None",
            };
            
            egui::ComboBox::from_id_source("config_data_source")
                .selected_text(current_source)
                .width(ui.available_width() - 80.0)
                .show_ui(ui, |ui| {
                    for (source_id, _) in &self.data_sources {
                        let is_selected = match &config {
                            ViewConfig::TimeSeries { data_source_id, .. } |
                            ViewConfig::Scatter { data_source_id, .. } |
                            ViewConfig::Table { data_source_id, .. } |
                            ViewConfig::BarChart { data_source_id, .. } |
                            ViewConfig::Histogram { data_source_id, .. } |
                            ViewConfig::Line { data_source_id, .. } |
                            ViewConfig::BoxPlot { data_source_id, .. } |
                            ViewConfig::ViolinPlot { data_source_id, .. } |
                            ViewConfig::Heatmap { data_source_id, .. } |
                            ViewConfig::AnomalyDetection { data_source_id, .. } |
                            ViewConfig::CorrelationMatrix { data_source_id, .. } |
                            ViewConfig::Scatter3D { data_source_id, .. } |
                            ViewConfig::Surface3D { data_source_id, .. } |
                            ViewConfig::ParallelCoordinates { data_source_id, .. } |
                            ViewConfig::RadarChart { data_source_id, .. } |
                            ViewConfig::PolarPlot { data_source_id, .. } |
                            ViewConfig::Distribution { data_source_id, .. } |
                            ViewConfig::SummaryStats { data_source_id, .. } => {
                                data_source_id.as_ref() == Some(source_id)
                            }
                            _ => false,
                        };
                        
                        if ui.selectable_label(is_selected, source_id).clicked() {
                            // Update data source in config
                            match &mut config {
                                ViewConfig::TimeSeries { data_source_id, .. } |
                                ViewConfig::Scatter { data_source_id, .. } |
                                ViewConfig::Table { data_source_id, .. } |
                                ViewConfig::BarChart { data_source_id, .. } |
                                ViewConfig::Histogram { data_source_id, .. } |
                                ViewConfig::Line { data_source_id, .. } |
                                ViewConfig::BoxPlot { data_source_id, .. } |
                                ViewConfig::ViolinPlot { data_source_id, .. } |
                                ViewConfig::Heatmap { data_source_id, .. } |
                                ViewConfig::AnomalyDetection { data_source_id, .. } |
                                ViewConfig::CorrelationMatrix { data_source_id, .. } |
                                ViewConfig::Scatter3D { data_source_id, .. } |
                                ViewConfig::Surface3D { data_source_id, .. } |
                                ViewConfig::ParallelCoordinates { data_source_id, .. } |
                                ViewConfig::RadarChart { data_source_id, .. } |
                                ViewConfig::PolarPlot { data_source_id, .. } |
                                ViewConfig::Distribution { data_source_id, .. } |
                                ViewConfig::SummaryStats { data_source_id, .. } => {
                                    *data_source_id = Some(source_id.clone());
                                }
                                _ => {}
                            }
                            
                            // Update selected data source and refresh columns
                            self.selected_data_source = Some(source_id.clone());
                            if let Some((_, schema)) = self.data_sources.iter().find(|(id, _)| id == source_id) {
                                self.columns = Self::analyze_schema(schema);
                            }
                        }
                    }
                });
        });
        
        ui.separator();
        
        match &mut config {
            ViewConfig::TimeSeries { title, data_source_id: _, x_column, y_columns } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                ui.label("X-Axis (optional):");
                let current_x = x_column.as_ref().map(|s| s.as_str()).unwrap_or("Auto (Row Index)");
                egui::ComboBox::from_id_source("config_ts_x")
                    .selected_text(current_x)
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(x_column, None, "Auto (Row Index)");
                        for col in &self.columns.temporal {
                            ui.selectable_value(x_column, Some(col.name.clone()), &col.name);
                        }
                        for col in &self.columns.numeric {
                            ui.selectable_value(x_column, Some(col.name.clone()), &col.name);
                        }
                    });
                
                ui.add_space(4.0);
                ui.label("Y-Axis (select one or more):");
                
                let mut any_selected = false;
                for col in &self.columns.numeric {
                    let mut selected = y_columns.contains(&col.name);
                    if ui.checkbox(&mut selected, &col.name).changed() {
                        if selected {
                            if !y_columns.contains(&col.name) {
                                y_columns.push(col.name.clone());
                            }
                        } else {
                            y_columns.retain(|c| c != &col.name);
                        }
                    }
                    if selected {
                        any_selected = true;
                    }
                }
                
                self.plot_config_state.is_valid = any_selected;
            }
            
            ViewConfig::Scatter { title, data_source_id: _, x_column, y_column, color_column } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                
                ui.label("X-Axis:");
                egui::ComboBox::from_id_source("config_scatter_x")
                    .selected_text(if x_column.is_empty() { "Select..." } else { x_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(x_column, col.name.clone(), &col.name);
                        }
                    });
                
                ui.add_space(4.0);
                
                ui.label("Y-Axis:");
                egui::ComboBox::from_id_source("config_scatter_y")
                    .selected_text(if y_column.is_empty() { "Select..." } else { y_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(y_column, col.name.clone(), &col.name);
                        }
                    });
                
                ui.add_space(4.0);
                
                ui.label("Color By (optional):");
                let current_color = color_column.as_ref().map(|s| s.as_str()).unwrap_or("None");
                egui::ComboBox::from_id_source("config_scatter_color")
                    .selected_text(current_color)
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(color_column, None, "None");
                        for col in &self.columns.categorical {
                            ui.selectable_value(color_column, Some(col.name.clone()), &col.name);
                        }
                    });
                
                self.plot_config_state.is_valid = !x_column.is_empty() && !y_column.is_empty();
            }
            
            ViewConfig::BarChart { title, data_source_id: _, category_column, value_column } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                
                ui.label("Category:");
                egui::ComboBox::from_id_source("config_bar_cat")
                    .selected_text(if category_column.is_empty() { "Select..." } else { category_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.categorical {
                            ui.selectable_value(category_column, col.name.clone(), &col.name);
                        }
                        for col in &self.columns.numeric {
                            ui.selectable_value(category_column, col.name.clone(), &col.name);
                        }
                    });
                
                ui.add_space(4.0);
                
                ui.label("Value:");
                egui::ComboBox::from_id_source("config_bar_val")
                    .selected_text(if value_column.is_empty() { "Select..." } else { value_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(value_column, col.name.clone(), &col.name);
                        }
                    });
                
                self.plot_config_state.is_valid = !category_column.is_empty() && !value_column.is_empty();
            }
            
            ViewConfig::Histogram { title, data_source_id: _, column } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                
                ui.label("Column:");
                egui::ComboBox::from_id_source("config_hist")
                    .selected_text(if column.is_empty() { "Select..." } else { column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(column, col.name.clone(), &col.name);
                        }
                    });
                
                self.plot_config_state.is_valid = !column.is_empty();
            }
            
            ViewConfig::Table { title, data_source_id: _, columns } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                
                ui.label("Columns (optional, all if none):");
                ui.horizontal(|ui| {
                    if ui.small_button("All").clicked() {
                        columns.clear();
                        columns.extend(self.columns.all.iter().map(|c| c.name.clone()));
                    }
                    if ui.small_button("Clear").clicked() {
                        columns.clear();
                    }
                });
                
                for col in &self.columns.all {
                    let mut selected = columns.contains(&col.name);
                    if ui.checkbox(&mut selected, &col.name).changed() {
                        if selected {
                            if !columns.contains(&col.name) {
                                columns.push(col.name.clone());
                            }
                        } else {
                            columns.retain(|c| c != &col.name);
                        }
                    }
                }
                
                self.plot_config_state.is_valid = true; // Table is always valid
            }
            
            ViewConfig::BoxPlot { title, data_source_id: _, value_column, category_column } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                
                ui.label("Value:");
                egui::ComboBox::from_id_source("config_box_val")
                    .selected_text(if value_column.is_empty() { "Select..." } else { value_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(value_column, col.name.clone(), &col.name);
                        }
                    });
                
                ui.add_space(4.0);
                
                ui.label("Category (optional):");
                let current_category = category_column.as_ref().map(|s| s.as_str()).unwrap_or("None");
                egui::ComboBox::from_id_source("config_box_cat")
                    .selected_text(current_category)
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(category_column, None, "None");
                        for col in &self.columns.categorical {
                            ui.selectable_value(category_column, Some(col.name.clone()), &col.name);
                        }
                    });
                
                self.plot_config_state.is_valid = !value_column.is_empty();
            }
            
            ViewConfig::ViolinPlot { title, data_source_id: _, value_column, category_column } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                
                ui.label("Value:");
                egui::ComboBox::from_id_source("config_violin_val")
                    .selected_text(if value_column.is_empty() { "Select..." } else { value_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(value_column, col.name.clone(), &col.name);
                        }
                    });
                
                ui.add_space(4.0);
                
                ui.label("Category (optional):");
                let current_category = category_column.as_ref().map(|s| s.as_str()).unwrap_or("None");
                egui::ComboBox::from_id_source("config_violin_cat")
                    .selected_text(current_category)
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(category_column, None, "None");
                        for col in &self.columns.categorical {
                            ui.selectable_value(category_column, Some(col.name.clone()), &col.name);
                        }
                    });
                
                self.plot_config_state.is_valid = !value_column.is_empty();
            }
            
            ViewConfig::Heatmap { title, data_source_id: _, x_column, y_column, value_column } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                
                ui.label("X-Axis:");
                egui::ComboBox::from_id_source("config_heat_x")
                    .selected_text(if x_column.is_empty() { "Select..." } else { x_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(x_column, col.name.clone(), &col.name);
                        }
                    });
                
                ui.add_space(4.0);
                
                ui.label("Y-Axis:");
                egui::ComboBox::from_id_source("config_heat_y")
                    .selected_text(if y_column.is_empty() { "Select..." } else { y_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(y_column, col.name.clone(), &col.name);
                        }
                    });
                
                ui.add_space(4.0);
                
                ui.label("Value:");
                egui::ComboBox::from_id_source("config_heat_val")
                    .selected_text(if value_column.is_empty() { "Select..." } else { value_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(value_column, col.name.clone(), &col.name);
                        }
                    });
                
                self.plot_config_state.is_valid = !x_column.is_empty() && !y_column.is_empty() && !value_column.is_empty();
            }
            
            ViewConfig::AnomalyDetection { title, data_source_id: _, column } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                
                ui.label("Column:");
                egui::ComboBox::from_id_source("config_anomaly")
                    .selected_text(if column.is_empty() { "Select..." } else { column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(column, col.name.clone(), &col.name);
                        }
                    });
                
                self.plot_config_state.is_valid = !column.is_empty();
            }
            
            ViewConfig::CorrelationMatrix { title, data_source_id: _, columns } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                
                ui.label("Columns (select at least 2):");
                ui.horizontal(|ui| {
                    if ui.small_button("All").clicked() {
                        columns.clear();
                        columns.extend(self.columns.numeric.iter().map(|c| c.name.clone()));
                    }
                    if ui.small_button("Clear").clicked() {
                        columns.clear();
                    }
                });
                
                for col in &self.columns.numeric {
                    let mut selected = columns.contains(&col.name);
                    if ui.checkbox(&mut selected, &col.name).changed() {
                        if selected {
                            if !columns.contains(&col.name) {
                                columns.push(col.name.clone());
                            }
                        } else {
                            columns.retain(|c| c != &col.name);
                        }
                    }
                }
                
                self.plot_config_state.is_valid = columns.len() >= 2;
            }
            
            ViewConfig::Scatter3D { title, data_source_id: _, x_column, y_column, z_column } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                
                ui.label("X-Axis:");
                egui::ComboBox::from_id_source("config_scatter3d_x")
                    .selected_text(if x_column.is_empty() { "Select..." } else { x_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(x_column, col.name.clone(), &col.name);
                        }
                    });
                
                ui.add_space(4.0);
                
                ui.label("Y-Axis:");
                egui::ComboBox::from_id_source("config_scatter3d_y")
                    .selected_text(if y_column.is_empty() { "Select..." } else { y_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(y_column, col.name.clone(), &col.name);
                        }
                    });
                
                ui.add_space(4.0);
                
                ui.label("Z-Axis:");
                egui::ComboBox::from_id_source("config_scatter3d_z")
                    .selected_text(if z_column.is_empty() { "Select..." } else { z_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(z_column, col.name.clone(), &col.name);
                        }
                    });
                
                self.plot_config_state.is_valid = !x_column.is_empty() && !y_column.is_empty() && !z_column.is_empty();
            }
            
            ViewConfig::Surface3D { title, data_source_id: _, x_column, y_column, z_column } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                
                ui.label("X-Axis:");
                egui::ComboBox::from_id_source("config_surface3d_x")
                    .selected_text(if x_column.is_empty() { "Select..." } else { x_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(x_column, col.name.clone(), &col.name);
                        }
                    });
                
                ui.add_space(4.0);
                
                ui.label("Y-Axis:");
                egui::ComboBox::from_id_source("config_surface3d_y")
                    .selected_text(if y_column.is_empty() { "Select..." } else { y_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(y_column, col.name.clone(), &col.name);
                        }
                    });
                
                ui.add_space(4.0);
                
                ui.label("Z-Axis:");
                egui::ComboBox::from_id_source("config_surface3d_z")
                    .selected_text(if z_column.is_empty() { "Select..." } else { z_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(z_column, col.name.clone(), &col.name);
                        }
                    });
                
                self.plot_config_state.is_valid = !x_column.is_empty() && !y_column.is_empty() && !z_column.is_empty();
            }
            
            ViewConfig::ParallelCoordinates { title, data_source_id: _, columns } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                
                ui.label("Columns (select at least 2):");
                ui.horizontal(|ui| {
                    if ui.small_button("All").clicked() {
                        columns.clear();
                        columns.extend(self.columns.numeric.iter().map(|c| c.name.clone()));
                    }
                    if ui.small_button("Clear").clicked() {
                        columns.clear();
                    }
                });
                
                for col in &self.columns.numeric {
                    let mut selected = columns.contains(&col.name);
                    if ui.checkbox(&mut selected, &col.name).changed() {
                        if selected {
                            if !columns.contains(&col.name) {
                                columns.push(col.name.clone());
                            }
                        } else {
                            columns.retain(|c| c != &col.name);
                        }
                    }
                }
                
                self.plot_config_state.is_valid = columns.len() >= 2;
            }
            
            ViewConfig::RadarChart { title, data_source_id: _, value_columns, group_column: _ } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                
                ui.label("Value Columns (select at least 3):");
                ui.horizontal(|ui| {
                    if ui.small_button("All").clicked() {
                        value_columns.clear();
                        value_columns.extend(self.columns.numeric.iter().map(|c| c.name.clone()));
                    }
                    if ui.small_button("Clear").clicked() {
                        value_columns.clear();
                    }
                });
                
                for col in &self.columns.numeric {
                    let mut selected = value_columns.contains(&col.name);
                    if ui.checkbox(&mut selected, &col.name).changed() {
                        if selected {
                            if !value_columns.contains(&col.name) {
                                value_columns.push(col.name.clone());
                            }
                        } else {
                            value_columns.retain(|c| c != &col.name);
                        }
                    }
                }
                
                self.plot_config_state.is_valid = value_columns.len() >= 3;
            }
            
            ViewConfig::PolarPlot { title, data_source_id: _, angle_column, radius_column, category_column } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                
                ui.label("Angle Column (0-360° or radians):");
                egui::ComboBox::from_id_source("config_polar_angle")
                    .selected_text(if angle_column.is_empty() { "Select..." } else { angle_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(angle_column, col.name.clone(), &col.name);
                        }
                    });
                
                ui.add_space(4.0);
                
                ui.label("Radius Column:");
                egui::ComboBox::from_id_source("config_polar_radius")
                    .selected_text(if radius_column.is_empty() { "Select..." } else { radius_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(radius_column, col.name.clone(), &col.name);
                        }
                    });
                
                ui.add_space(4.0);
                
                ui.label("Category/Color (optional):");
                let current_category = category_column.as_ref().map(|s| s.as_str()).unwrap_or("None");
                egui::ComboBox::from_id_source("config_polar_category")
                    .selected_text(current_category)
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(category_column, None, "None");
                        for col in &self.columns.categorical {
                            ui.selectable_value(category_column, Some(col.name.clone()), &col.name);
                        }
                    });
                
                self.plot_config_state.is_valid = !angle_column.is_empty() && !radius_column.is_empty();
            }
            
            ViewConfig::Contour { title, data_source_id: _, x_column, y_column, z_column } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                
                ui.label("X Column:");
                egui::ComboBox::from_id_source("config_contour_x")
                    .selected_text(if x_column.is_empty() { "Select..." } else { x_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(x_column, col.name.clone(), &col.name);
                        }
                    });
                
                ui.add_space(4.0);
                
                ui.label("Y Column:");
                egui::ComboBox::from_id_source("config_contour_y")
                    .selected_text(if y_column.is_empty() { "Select..." } else { y_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(y_column, col.name.clone(), &col.name);
                        }
                    });
                
                ui.add_space(4.0);
                
                ui.label("Z Column (values):");
                egui::ComboBox::from_id_source("config_contour_z")
                    .selected_text(if z_column.is_empty() { "Select..." } else { z_column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(z_column, col.name.clone(), &col.name);
                        }
                    });
                
                self.plot_config_state.is_valid = !x_column.is_empty() && !y_column.is_empty() && !z_column.is_empty();
            }
            
            ViewConfig::Distribution { title, data_source_id: _, column } => {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(title);
                });
                
                ui.add_space(4.0);
                
                ui.label("Column to Analyze:");
                egui::ComboBox::from_id_source("config_dist_column")
                    .selected_text(if column.is_empty() { "Select..." } else { column.as_str() })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for col in &self.columns.numeric {
                            ui.selectable_value(column, col.name.clone(), &col.name);
                        }
                    });
                
                self.plot_config_state.is_valid = !column.is_empty();
            }
            
            _ => {
                ui.label("Configuration for this plot type coming soon...");
                self.plot_config_state.is_valid = false;
            }
        }
        
        self.plot_config_state.config = config;
    }
    
    /// Add the configured plot to the dashboard
    fn add_configured_plot(&mut self) {
        if self.plot_config_state.is_valid {
            self.add_view(self.plot_config_state.config.clone());
            
            // Reset configuration state
            self.plot_config_state = PlotConfigState {
                selected_plot_type: None,
                config: ViewConfig::Empty,
                is_valid: false,
            };
        }
    }
    
    /// Show the canvas
    fn show_canvas(&mut self, ui: &mut Ui) {
        ui.heading("🎨 Dashboard Layout");
        ui.add_space(8.0);
        
        // Grid controls
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Grid Size:").size(14.0).strong());
            
            ui.add_space(10.0);
            
            // Column controls
            ui.label("Columns:");
            if ui.small_button("−").clicked() && self.layout.grid_size.0 > 1 {
                self.layout.grid_size.0 -= 1;
                self.adjust_cells_to_grid();
            }
            ui.label(
                egui::RichText::new(format!("{}", self.layout.grid_size.0))
                    .size(16.0)
                    .strong()
                    .color(Color32::from_rgb(100, 150, 250))
            );
            if ui.small_button("+").clicked() && self.layout.grid_size.0 < 4 {
                self.layout.grid_size.0 += 1;
            }
            
            ui.add_space(20.0);
            
            // Row controls
            ui.label("Rows:");
            if ui.small_button("−").clicked() && self.layout.grid_size.1 > 1 {
                self.layout.grid_size.1 -= 1;
                self.adjust_cells_to_grid();
            }
            ui.label(
                egui::RichText::new(format!("{}", self.layout.grid_size.1))
                    .size(16.0)
                    .strong()
                    .color(Color32::from_rgb(100, 150, 250))
            );
            if ui.small_button("+").clicked() && self.layout.grid_size.1 < 4 {
                self.layout.grid_size.1 += 1;
            }
            
            ui.add_space(20.0);
            
                                if ui.button("✖ Clear All").clicked() {
                self.layout.cells.clear();
                self.selected_cell_id = None;
            }
        });
        
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);
        
        // Main canvas area
        let available_size = ui.available_size();
        
        // Use a vertical layout for the canvas and properties
        ui.vertical(|ui| {
            // Canvas area
            let canvas_height = (available_size.y - 120.0).max(400.0); // Ensure minimum height
            let canvas_width = available_size.x.max(600.0); // Ensure minimum width
            
            // Group for visual definition
            ui.group(|ui| {
                ui.set_min_size(Vec2::new(canvas_width, canvas_height));
                
                // Canvas background
                let (response, painter) = ui.allocate_painter(
                    Vec2::new(canvas_width - 4.0, canvas_height - 4.0),
                    Sense::hover()
                );
                let canvas_rect = response.rect;
                
                // Draw canvas background
                painter.rect_filled(
                    canvas_rect,
                    Rounding::same(4.0),
                    Color32::from_gray(25)
                );
                
                // Calculate grid dimensions
                let padding = 20.0;
                let grid_rect = Rect::from_min_size(
                    canvas_rect.min + Vec2::new(padding, padding),
                    canvas_rect.size() - Vec2::new(padding * 2.0, padding * 2.0)
                );
                
                let cell_width = grid_rect.width() / self.layout.grid_size.0 as f32;
                let cell_height = grid_rect.height() / self.layout.grid_size.1 as f32;
                
                // Draw grid lines
                for i in 1..self.layout.grid_size.0 {
                    let x = grid_rect.left() + i as f32 * cell_width;
                    painter.line_segment(
                        [Pos2::new(x, grid_rect.top()), Pos2::new(x, grid_rect.bottom())],
                        Stroke::new(1.0, Color32::from_gray(60))
                    );
                }
                
                for j in 1..self.layout.grid_size.1 {
                    let y = grid_rect.top() + j as f32 * cell_height;
                    painter.line_segment(
                        [Pos2::new(grid_rect.left(), y), Pos2::new(grid_rect.right(), y)],
                        Stroke::new(1.0, Color32::from_gray(60))
                    );
                }
                
                // Draw border
                painter.rect_stroke(
                    grid_rect,
                    Rounding::same(4.0),
                    Stroke::new(2.0, Color32::from_gray(100))
                );
                
                // Show empty state
                if self.layout.cells.is_empty() {
                    painter.text(
                        grid_rect.center(),
                        Align2::CENTER_CENTER,
                        "1. Select a plot type from the left panel",
                        egui::FontId::proportional(16.0),
                        Color32::from_gray(140)
                    );
                    painter.text(
                        grid_rect.center() + Vec2::new(0.0, 24.0),
                        Align2::CENTER_CENTER,
                        "2. Configure its columns",
                        egui::FontId::proportional(16.0),
                        Color32::from_gray(140)
                    );
                    painter.text(
                        grid_rect.center() + Vec2::new(0.0, 48.0),
                        Align2::CENTER_CENTER,
                        "3. Click 'Add to Dashboard'",
                        egui::FontId::proportional(16.0),
                        Color32::from_gray(140)
                    );
                }
                
                // Draw cells
                let mut clicked_cell_id = None;
                for cell in &self.layout.cells {
                    let cell_rect = Rect::from_min_size(
                        Pos2::new(
                            grid_rect.left() + cell.grid_pos.0 as f32 * cell_width + 4.0,
                            grid_rect.top() + cell.grid_pos.1 as f32 * cell_height + 4.0
                        ),
                        Vec2::new(
                            cell.grid_span.0 as f32 * cell_width - 8.0,
                            cell.grid_span.1 as f32 * cell_height - 8.0
                        )
                    );
                    
                    let cell_response = ui.interact(cell_rect, ui.id().with(&cell.id), Sense::click());
                    if cell_response.clicked() {
                        clicked_cell_id = Some(cell.id.clone());
                    }
                    
                    let is_selected = self.selected_cell_id.as_ref() == Some(&cell.id);
                    self.draw_layout_cell(ui, &painter, cell_rect, cell, is_selected);
                }
                
                if let Some(id) = clicked_cell_id {
                    self.selected_cell_id = Some(id);
                }
            });
            
            // Selected cell properties below canvas
            if let Some(selected_id) = &self.selected_cell_id {
                if let Some(cell_idx) = self.layout.cells.iter().position(|c| c.id == *selected_id) {
                    ui.add_space(12.0);
                    
                    ui.group(|ui| {
                        ui.set_min_width(ui.available_width());
                        
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("✎ Selected Cell Properties").size(14.0).strong());
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("✖ Delete Cell").clicked() {
                                    self.layout.cells.remove(cell_idx);
                                    self.selected_cell_id = None;
                                }
                            });
                        });
                        
                        if self.selected_cell_id.is_some() && cell_idx < self.layout.cells.len() {
                            let cell = &self.layout.cells[cell_idx];
                            ui.horizontal(|ui| {
                                ui.label(format!("Position: ({}, {})", cell.grid_pos.0, cell.grid_pos.1));
                                ui.separator();
                                ui.label(format!("Size: {}×{}", cell.grid_span.0, cell.grid_span.1));
                                ui.separator();
                                
                                let status_text = if self.is_config_valid(&cell.view_config) {
                                    egui::RichText::new("✓ Configured").color(Color32::from_rgb(76, 175, 80))
                                } else {
                                    egui::RichText::new("⚠ Not Configured").color(Color32::from_rgb(255, 152, 0))
                                };
                                ui.label(status_text);
                            });
                        }
                    });
                }
            }
        });
    }
    
    /// Draw a layout cell
    fn draw_layout_cell(&self, _ui: &Ui, painter: &egui::Painter, rect: Rect, cell: &LayoutCell, is_selected: bool) {
        let is_configured = self.is_config_valid(&cell.view_config);
        
        // Cell background
        let bg_color = if is_configured {
            if is_selected {
                Color32::from_rgb(50, 100, 150)
            } else {
                Color32::from_rgb(40, 60, 80)
            }
        } else {
            if is_selected {
                Color32::from_rgb(150, 100, 50)
            } else {
                Color32::from_rgb(80, 60, 40)
            }
        };
        
        painter.rect_filled(rect, Rounding::same(4.0), bg_color);
        
        // Border
        let border_color = if is_selected {
            Color32::from_rgb(100, 200, 255)
        } else if is_configured {
            Color32::from_gray(100)
        } else {
            Color32::from_rgb(255, 152, 0)
        };
        
        painter.rect_stroke(
            rect,
            Rounding::same(4.0),
            Stroke::new(if is_selected { 3.0 } else { 1.5 }, border_color)
        );
        
        // Icon and title
        let (icon, title) = match &cell.view_config {
            ViewConfig::TimeSeries { title, .. } => ("📈", title.as_str()),
            ViewConfig::Line { title, .. } => ("📉", title.as_str()),
            ViewConfig::Scatter { title, .. } => ("🎯", title.as_str()),
            ViewConfig::BarChart { title, .. } => ("📊", title.as_str()),
            ViewConfig::Histogram { title, .. } => ("📊", title.as_str()),
            ViewConfig::Table { title, .. } => ("📋", title.as_str()),
            ViewConfig::BoxPlot { title, .. } => ("📦", title.as_str()),
            ViewConfig::ViolinPlot { title, .. } => ("🎻", title.as_str()),
            ViewConfig::Heatmap { title, .. } => ("🔥", title.as_str()),
            ViewConfig::AnomalyDetection { title, .. } => ("⚠️", title.as_str()),
            ViewConfig::CorrelationMatrix { title, .. } => ("🎯", title.as_str()),
            ViewConfig::Scatter3D { title, .. } => ("🎲", title.as_str()),
            ViewConfig::Surface3D { title, .. } => ("🏔️", title.as_str()),
            ViewConfig::ParallelCoordinates { title, .. } => ("🌈", title.as_str()),
            ViewConfig::RadarChart { title, .. } => ("🎯", title.as_str()),
            ViewConfig::PolarPlot { .. } => ("🧭", "Polar Plot"),
            ViewConfig::Contour { .. } => ("🗺️", "Contour Plot"),
            ViewConfig::Sankey { .. } => ("🧵", "Sankey Diagram"),
            ViewConfig::Treemap { .. } => ("📊", "Treemap"),
            ViewConfig::Sunburst { .. } => ("🌞", "Sunburst"),
            ViewConfig::NetworkGraph { .. } => ("🌐", "Network Graph"),
            ViewConfig::Distribution { .. } => ("📊", "Distribution"),
            ViewConfig::TimeAnalysis { .. } => ("⏳", "Time Analysis"),
            ViewConfig::GeoPlot { .. } => ("🌍", "Geographic Plot"),
            ViewConfig::StreamGraph { .. } => ("🌊", "Stream Graph"),
            ViewConfig::CandlestickChart { .. } => ("🕒", "Candlestick Chart"),
            _ => ("📊", "Plot"),
        };
        
        // Draw content
        let text_pos = rect.min + Vec2::new(8.0, 8.0);
        painter.text(
            text_pos,
            Align2::LEFT_TOP,
            format!("{} {}", icon, title),
            egui::FontId::proportional(14.0),
            Color32::WHITE
        );
        
        // Configuration status
        if !is_configured {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "⚠️ Not Configured",
                egui::FontId::proportional(12.0),
                Color32::from_rgb(255, 152, 0)
            );
        } else {
            // Show column info
            let info = self.get_column_info_text(&cell.view_config);
            if !info.is_empty() {
                painter.text(
                    rect.min + Vec2::new(8.0, 28.0),
                    Align2::LEFT_TOP,
                    info,
                    egui::FontId::proportional(11.0),
                    Color32::from_gray(200)
                );
            }
        }
    }
    
    /// Check if a view config is valid
    fn is_config_valid(&self, config: &ViewConfig) -> bool {
        match config {
            ViewConfig::TimeSeries { y_columns, .. } => !y_columns.is_empty(),
            ViewConfig::Line { y_columns, .. } => !y_columns.is_empty(),
            ViewConfig::Scatter { x_column, y_column, .. } => !x_column.is_empty() && !y_column.is_empty(),
            ViewConfig::BarChart { category_column, value_column, .. } => !category_column.is_empty() && !value_column.is_empty(),
            ViewConfig::Histogram { column, .. } => !column.is_empty(),
            ViewConfig::Table { .. } => true,
            ViewConfig::BoxPlot { value_column, .. } => !value_column.is_empty(),
            ViewConfig::ViolinPlot { value_column, .. } => !value_column.is_empty(),
            ViewConfig::Heatmap { x_column, y_column, value_column, .. } => 
                !x_column.is_empty() && !y_column.is_empty() && !value_column.is_empty(),
            ViewConfig::AnomalyDetection { column, .. } => !column.is_empty(),
            ViewConfig::CorrelationMatrix { columns, .. } => columns.len() >= 2,
            ViewConfig::Scatter3D { x_column, y_column, z_column, .. } => 
                !x_column.is_empty() && !y_column.is_empty() && !z_column.is_empty(),
            ViewConfig::Surface3D { x_column, y_column, z_column, .. } => 
                !x_column.is_empty() && !y_column.is_empty() && !z_column.is_empty(),
            ViewConfig::ParallelCoordinates { columns, .. } => columns.len() >= 2,
            ViewConfig::RadarChart { value_columns, .. } => value_columns.len() >= 3,
            ViewConfig::PolarPlot { angle_column, radius_column, .. } => 
                !angle_column.is_empty() && !radius_column.is_empty(),
            ViewConfig::Contour { x_column, y_column, z_column, .. } => 
                !x_column.is_empty() && !y_column.is_empty() && !z_column.is_empty(),
            ViewConfig::Distribution { column, .. } => !column.is_empty(),
            _ => false,
        }
    }
    
    /// Get column info text for a view config
    fn get_column_info_text(&self, config: &ViewConfig) -> String {
        match config {
            ViewConfig::TimeSeries { x_column, y_columns, .. } => {
                let x = x_column.as_ref().map(|s| s.as_str()).unwrap_or("Row Index");
                if y_columns.is_empty() {
                    String::new()
                } else if y_columns.len() == 1 {
                    format!("{} → {}", x, y_columns[0])
                } else {
                    format!("{} → {} series", x, y_columns.len())
                }
            }
            ViewConfig::Scatter { x_column, y_column, .. } => {
                format!("{} vs {}", x_column, y_column)
            }
            ViewConfig::BarChart { category_column, value_column, .. } => {
                format!("{}: {}", category_column, value_column)
            }
            ViewConfig::Histogram { column, .. } => column.clone(),
            ViewConfig::Table { .. } => {
                "All columns".to_string()
            }
            _ => String::new(),
        }
    }
    
    /// Removed unused methods
    fn show_draggable_column(&mut self, _ui: &mut Ui, _col: &ColumnInfo) {
        // Removed - no longer using draggable columns
    }
    
    fn configure_cell_with_column(&mut self, _cell: &mut LayoutCell, _column: &ColumnInfo) {
        // Removed - configuration happens before adding
    }
    
    fn show_preview(&self, _ui: &mut Ui) {
        // Removed - not needed
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
                
                // Check if occupied
                let is_occupied = self.layout.cells.iter().any(|c| {
                    let cell_x_start = c.grid_pos.0;
                    let cell_x_end = c.grid_pos.0 + c.grid_span.0;
                    let cell_y_start = c.grid_pos.1;
                    let cell_y_end = c.grid_pos.1 + c.grid_span.1;
                    
                    x >= cell_x_start && x < cell_x_end && y >= cell_y_start && y < cell_y_end
                });
                
                if !is_occupied {
                    found_pos = Some(pos);
                    break;
                }
            }
            if found_pos.is_some() {
                break;
            }
        }
        
        if let Some(pos) = found_pos {
            let cell_id = uuid::Uuid::new_v4().to_string();
            let cell = LayoutCell {
                id: cell_id.clone(),
                grid_pos: pos,
                grid_span: (1, 1),
                view_config: config,
            };
            self.layout.cells.push(cell);
            self.selected_cell_id = Some(cell_id);
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
    
    /// Build the actual views from the layout
    fn build_views(&self) -> (Vec<Box<dyn SpaceView>>, NavigationMode) {
        let mut views: Vec<Box<dyn SpaceView>> = Vec::new();
        
        for cell in &self.layout.cells {
            match &cell.view_config {
                ViewConfig::TimeSeries { title, data_source_id, x_column, y_columns } => {
                    let id = uuid::Uuid::new_v4();
                    let mut view = TimeSeriesView::new(id, title.clone());
                    view.config.data_source_id = data_source_id.clone().or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone())
                    });
                    view.config.x_column = x_column.clone();
                    view.config.y_columns = y_columns.clone();
                    view.config.show_legend = true;
                    view.config.show_grid = true;
                    views.push(Box::new(view));
                }
                ViewConfig::Scatter { title, data_source_id, x_column, y_column, color_column } => {
                    let id = uuid::Uuid::new_v4();
                    let mut view = ScatterPlotView::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.x_column = x_column.clone();
                    view.config.y_column = y_column.clone();
                    view.config.color_column = color_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::Table { title, data_source_id, columns } => {
                    let id = uuid::Uuid::new_v4();
                    let mut view = TableView::new(id, title.clone());
                    view.config.data_source_id = data_source_id.clone().or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone())
                    });
                    views.push(Box::new(view));
                }
                ViewConfig::BarChart { title, data_source_id, category_column, value_column } => {
                    let id = uuid::Uuid::new_v4();
                    let mut view = BarChartView::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.category_column = category_column.clone();
                    view.config.value_column = value_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::Line { title, data_source_id, x_column, y_columns } => {
                    use dv_views::plots::LinePlotView;
                    let id = uuid::Uuid::new_v4();
                    let mut view = LinePlotView::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.x_column = x_column.clone();
                    view.config.y_columns = y_columns.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::Histogram { title, data_source_id, column } => {
                    use dv_views::plots::HistogramView;
                    let id = uuid::Uuid::new_v4();
                    let mut view = HistogramView::new(id, title.clone());
                    view.config.data_source_id = data_source_id.clone().or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone())
                    });
                    view.config.column = column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::BoxPlot { title, data_source_id, value_column, category_column } => {
                    use dv_views::plots::BoxPlotView;
                    let id = uuid::Uuid::new_v4();
                    let mut view = BoxPlotView::new(id, title.clone());
                    view.config.data_source_id = data_source_id.clone().or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone())
                    });
                    view.config.value_column = value_column.clone();
                    view.config.category_column = category_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::ViolinPlot { title, data_source_id, value_column, category_column } => {
                    use dv_views::plots::ViolinPlotView;
                    let id = uuid::Uuid::new_v4();
                    let mut view = ViolinPlotView::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.value_column = value_column.clone();
                    view.config.category_column = category_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::Heatmap { title, data_source_id, x_column, y_column, value_column } => {
                    use dv_views::plots::HeatmapView;
                    let id = uuid::Uuid::new_v4();
                    let mut view = HeatmapView::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.x_column = x_column.clone();
                    view.config.y_column = y_column.clone();
                    view.config.value_column = value_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::AnomalyDetection { title, data_source_id, column } => {
                    use dv_views::plots::AnomalyDetectionView;
                    let id = uuid::Uuid::new_v4();
                    let mut view = AnomalyDetectionView::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.column = column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::CorrelationMatrix { title, data_source_id, columns } => {
                    use dv_views::plots::CorrelationMatrixView;
                    let id = uuid::Uuid::new_v4();
                    let mut view = CorrelationMatrixView::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.columns = columns.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::Scatter3D { title, data_source_id, x_column, y_column, z_column } => {
                    use dv_views::plots::Scatter3DView;
                    let id = uuid::Uuid::new_v4();
                    let mut view = Scatter3DView::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.x_column = x_column.clone();
                    view.config.y_column = y_column.clone();
                    view.config.z_column = z_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::Surface3D { title, data_source_id, x_column, y_column, z_column } => {
                    use dv_views::plots::Surface3DPlot;
                    let id = uuid::Uuid::new_v4();
                    let mut view = Surface3DPlot::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.x_column = x_column.clone();
                    view.config.y_column = y_column.clone();
                    view.config.z_column = z_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::ParallelCoordinates { title, data_source_id, columns } => {
                    use dv_views::plots::ParallelCoordinatesView;
                    let id = uuid::Uuid::new_v4();
                    let mut view = ParallelCoordinatesView::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.columns = columns.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::RadarChart { title, data_source_id, value_columns, group_column } => {
                    use dv_views::plots::RadarChart;
                    let id = uuid::Uuid::new_v4();
                    let mut view = RadarChart::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.value_columns = value_columns.clone();
                    view.config.group_column = group_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::Distribution { title, data_source_id, column } => {
                    use dv_views::plots::DistributionPlot;
                    let id = uuid::Uuid::new_v4();
                    let mut view = DistributionPlot::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.column = column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::PolarPlot { title, data_source_id, angle_column, radius_column, category_column } => {
                    use dv_views::PolarPlotView;
                    let id = uuid::Uuid::new_v4();
                    let mut view = PolarPlotView::new(id, title.clone());
                    {
                        let config = view.config_mut();
                        config.data_source_id = data_source_id.clone().or_else(|| {
                            self.data_sources.first().map(|(id, _)| id.clone())
                        });
                        config.angle_column = angle_column.clone();
                        config.radius_column = radius_column.clone();
                        config.category_column = category_column.clone();
                    }
                    views.push(Box::new(view));
                }
                ViewConfig::Contour { title, data_source_id, x_column, y_column, z_column } => {
                    use dv_views::plots::ContourPlot;
                    let id = uuid::Uuid::new_v4();
                    let mut view = ContourPlot::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.x_column = x_column.clone();
                    view.config.y_column = y_column.clone();
                    view.config.z_column = z_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::Sankey { title, data_source_id, source_column, target_column, value_column } => {
                    use dv_views::plots::SankeyDiagram;
                    let id = uuid::Uuid::new_v4();
                    let mut view = SankeyDiagram::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.source_column = source_column.clone();
                    view.config.target_column = target_column.clone();
                    view.config.value_column = value_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::Treemap { title, data_source_id, category_column, value_column } => {
                    use dv_views::plots::Treemap;
                    let id = uuid::Uuid::new_v4();
                    let mut view = Treemap::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.category_column = category_column.clone();
                    view.config.value_column = value_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::Sunburst { title, data_source_id, hierarchy_columns, value_column } => {
                    use dv_views::plots::SunburstChart;
                    let id = uuid::Uuid::new_v4();
                    let mut view = SunburstChart::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.hierarchy_columns = hierarchy_columns.clone();
                    view.config.value_column = value_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::NetworkGraph { title, data_source_id, source_column, target_column } => {
                    use dv_views::plots::NetworkGraph;
                    let id = uuid::Uuid::new_v4();
                    let mut view = NetworkGraph::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.source_column = source_column.clone();
                    view.config.target_column = target_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::TimeAnalysis { title, data_source_id, time_column, value_columns } => {
                    use dv_views::plots::TimeAnalysisPlot;
                    let id = uuid::Uuid::new_v4();
                    let mut view = TimeAnalysisPlot::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.time_column = time_column.clone();
                    view.config.value_columns = value_columns.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::GeoPlot { title, data_source_id, lat_column, lon_column, value_column } => {
                    use dv_views::plots::GeoPlot;
                    let id = uuid::Uuid::new_v4();
                    let mut view = GeoPlot::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.lat_column = lat_column.clone();
                    view.config.lon_column = lon_column.clone();
                    view.config.value_column = value_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::StreamGraph { title, data_source_id, time_column, value_column, category_column } => {
                    use dv_views::plots::StreamGraph;
                    let id = uuid::Uuid::new_v4();
                    let mut view = StreamGraph::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.time_column = time_column.clone();
                    view.config.value_column = value_column.clone();
                    view.config.category_column = category_column.clone();
                    views.push(Box::new(view));
                }
                ViewConfig::CandlestickChart { title, data_source_id, time_column, open_column, high_column, low_column, close_column } => {
                    use dv_views::plots::CandlestickChart;
                    let id = uuid::Uuid::new_v4();
                    let mut view = CandlestickChart::new(id, title.clone());
                    view.config.data_source_id = Some(data_source_id.clone().unwrap_or_else(|| {
                        self.data_sources.first().map(|(id, _)| id.clone()).unwrap_or_default()
                    }));
                    view.config.time_column = time_column.clone();
                    view.config.open_column = open_column.clone();
                    view.config.high_column = high_column.clone();
                    view.config.low_column = low_column.clone();
                    view.config.close_column = close_column.clone();
                    views.push(Box::new(view));
                }
                _ => {}
            }
        }
        
        // Convert navigation mode choice to actual mode
        let nav_mode = match &self.selected_nav_mode {
            NavigationModeChoice::RowIndex => NavigationMode::Sequential,
            NavigationModeChoice::Time(_col) => NavigationMode::Temporal,
            NavigationModeChoice::Category(col) => NavigationMode::Categorical {
                categories: vec![col.clone()],
            },
        };
        
        (views, nav_mode)
    }
} 