//! Modern data visualization application inspired by Rerun

use std::sync::Arc;
use parking_lot::RwLock;
use eframe::egui::{self, Ui, Vec2, Color32, Rounding, Stroke};
use tracing::{info, error};
use uuid::Uuid;
use std::collections::HashMap;
use arrow::array::{Float64Array, Float32Array, Int64Array, Int32Array, Array};

use dv_views::{
    Viewport, ViewerContext, TimeControl, HoveredData, FrameTime,
    TimeSeriesView, TableView, SpaceView, SummaryStatsView,
    plots::ScatterPlotView
};
use dv_core::{
    data::DataSource,
    navigation::{NavigationEngine, NavigationSpec, NavigationMode},
};
use dv_ui::{NavigationPanel, AppShell, Theme};
use dv_data::sources::{SqliteSource, CombinedCsvSource, ConfiguredCombinedCsvSource};

mod demo;
mod create_sample_db;
mod view_builder;
mod frog_animation;
mod demo_overlay;
mod file_config_dialog;

use view_builder::ViewBuilderDialog;
use frog_animation::FrogMascot;
use demo_overlay::DemoOverlay;
use file_config_dialog::FileConfigDialog;


/// Create default views based on schema analysis
fn create_default_views_for_schema(schema: &arrow::datatypes::Schema) -> Vec<Box<dyn SpaceView>> {
    let mut views: Vec<Box<dyn SpaceView>> = Vec::new();
    
    // Analyze columns
    let mut numeric_columns = Vec::new();
    let mut temporal_columns = Vec::new();
    let mut categorical_columns = Vec::new();
    
    for field in schema.fields() {
        match field.data_type() {
            arrow::datatypes::DataType::Float64 | 
            arrow::datatypes::DataType::Float32 | 
            arrow::datatypes::DataType::Int64 | 
            arrow::datatypes::DataType::Int32 => {
                numeric_columns.push(field.name().clone());
            }
            arrow::datatypes::DataType::Utf8 => {
                let name_lower = field.name().to_lowercase();
                if name_lower.contains("date") || name_lower.contains("time") || name_lower.contains("timestamp") {
                    temporal_columns.push(field.name().clone());
                } else {
                    categorical_columns.push(field.name().clone());
                }
            }
            arrow::datatypes::DataType::Date32 | 
            arrow::datatypes::DataType::Date64 | 
            arrow::datatypes::DataType::Timestamp(_, _) => {
                temporal_columns.push(field.name().clone());
            }
            _ => {}
        }
    }
    
    // Always add a table view
    let table_view = TableView::new(Uuid::new_v4(), "Data Table".to_string());
    views.push(Box::new(table_view));
    
    // Add time series if we have numeric columns
    if !numeric_columns.is_empty() {
        let mut ts_view = TimeSeriesView::new(Uuid::new_v4(), "Time Series".to_string());
        ts_view.config.x_column = temporal_columns.first().cloned();
        ts_view.config.y_columns = numeric_columns.iter().take(3).cloned().collect();
        views.push(Box::new(ts_view));
    }
    
    // Add scatter plot if we have at least 2 numeric columns
    if numeric_columns.len() >= 2 {
        let mut scatter_view = ScatterPlotView::new(Uuid::new_v4(), "Correlation".to_string());
        scatter_view.config.x_column = numeric_columns[0].clone();
        scatter_view.config.y_column = numeric_columns[1].clone();
        scatter_view.config.color_column = categorical_columns.first().cloned();
        views.push(Box::new(scatter_view));
    }
    
    // Add summary stats
    let stats_view = SummaryStatsView::new(Uuid::new_v4(), "Statistics".to_string());
    views.push(Box::new(stats_view));
    
    views
}

/// Demo example types
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DemoExample {
    AssemblyLine,
    SensorNetwork,
    FinancialDashboard,
    SignalAnalysis,
}

impl DemoExample {
    fn name(&self) -> &'static str {
        match self {
            DemoExample::AssemblyLine => "Assembly Line Analytics",
            DemoExample::SensorNetwork => "IoT Sensor Network",
            DemoExample::FinancialDashboard => "Financial Dashboard",
            DemoExample::SignalAnalysis => "Signal Analysis",
        }
    }
    
    fn description(&self) -> &'static str {
        match self {
            DemoExample::AssemblyLine => "Manufacturing performance with throughput, efficiency, and quality metrics",
            DemoExample::SensorNetwork => "Real-time IoT data with temperature, pressure, and network performance",
            DemoExample::FinancialDashboard => "Market data visualization with price trends and business KPIs",
            DemoExample::SignalAnalysis => "Signal decomposition and frequency analysis with waveforms",
        }
    }
}

/// Create views for assembly line demo
fn create_assembly_line_views(data_source_id: String) -> Vec<Box<dyn SpaceView>> {
    let mut views: Vec<Box<dyn SpaceView>> = Vec::new();
    
    // 1. Assembly Line Performance
    let assembly_id = Uuid::new_v4();
    let mut assembly_view = TimeSeriesView::new(assembly_id, "Assembly Line Performance".to_string());
    assembly_view.config.data_source_id = Some(data_source_id.clone());
    assembly_view.config.x_column = Some("time".to_string());
    assembly_view.config.y_columns = vec![
        "station_1_throughput".to_string(),
        "station_2_throughput".to_string(),
        "station_3_throughput".to_string(),
    ];
    views.push(Box::new(assembly_view));
    
    // 2. Manufacturing Efficiency
    let efficiency_id = Uuid::new_v4();
    let mut efficiency_view = TimeSeriesView::new(efficiency_id, "Manufacturing Efficiency".to_string());
    efficiency_view.config.data_source_id = Some(data_source_id.clone());
    efficiency_view.config.x_column = Some("time".to_string());
    efficiency_view.config.y_columns = vec![
        "efficiency".to_string(),
        "defect_rate".to_string(),
        "buffer_level".to_string(),
    ];
    views.push(Box::new(efficiency_view));
    
    // 3. System Performance
    let performance_id = Uuid::new_v4();
    let mut performance_view = TimeSeriesView::new(performance_id, "System Performance".to_string());
    performance_view.config.data_source_id = Some(data_source_id.clone());
    performance_view.config.x_column = Some("time".to_string());
    performance_view.config.y_columns = vec![
        "cpu_usage".to_string(),
        "memory_usage".to_string(),
    ];
    views.push(Box::new(performance_view));
    
    // 4. Data Table
    let table_id = Uuid::new_v4();
    let mut table_view = TableView::new(table_id, "Data Inspector".to_string());
    table_view.config.data_source_id = Some(data_source_id);
    views.push(Box::new(table_view));
    
    views
}

/// Create views for sensor network demo
fn create_sensor_network_views(data_source_id: String) -> Vec<Box<dyn SpaceView>> {
    let mut views: Vec<Box<dyn SpaceView>> = Vec::new();
    
    // 1. Environmental Sensors
    let env_id = Uuid::new_v4();
    let mut env_view = TimeSeriesView::new(env_id, "Environmental Sensors".to_string());
    env_view.config.data_source_id = Some(data_source_id.clone());
    env_view.config.x_column = Some("time".to_string());
    env_view.config.y_columns = vec![
        "cpu_usage".to_string(),
        "memory_usage".to_string(),
        "error_rate".to_string(),
    ];
    views.push(Box::new(env_view));
    
    // 2. Network Performance
    let network_id = Uuid::new_v4();
    let mut network_view = TimeSeriesView::new(network_id, "Network Performance".to_string());
    network_view.config.data_source_id = Some(data_source_id.clone());
    network_view.config.x_column = Some("time".to_string());
    network_view.config.y_columns = vec![
        "network_latency".to_string(),
        "requests_per_sec".to_string(),
    ];
    views.push(Box::new(network_view));
    
    // 3. Position Scatter
    let position_id = Uuid::new_v4();
    let mut position_view = ScatterPlotView::new(position_id, "Sensor Positions".to_string());
    position_view.config.data_source_id = Some(data_source_id);
    position_view.config.x_column = "position_x".to_string();
    position_view.config.y_column = "position_y".to_string();
    views.push(Box::new(position_view));
    
    views
}

/// Create views for financial dashboard demo
fn create_financial_views(data_source_id: String) -> Vec<Box<dyn SpaceView>> {
    let mut views: Vec<Box<dyn SpaceView>> = Vec::new();
    
    // 1. Business Metrics
    let business_id = Uuid::new_v4();
    let mut business_view = TimeSeriesView::new(business_id, "Business Metrics".to_string());
    business_view.config.data_source_id = Some(data_source_id.clone());
    business_view.config.x_column = Some("time".to_string());
    business_view.config.y_columns = vec![
        "revenue".to_string(),
        "cost".to_string(),
        "profit".to_string(),
    ];
    views.push(Box::new(business_view));
    
    // 2. Market Trends
    let market_id = Uuid::new_v4();
    let mut market_view = TimeSeriesView::new(market_id, "Market Trends".to_string());
    market_view.config.data_source_id = Some(data_source_id);
    market_view.config.x_column = Some("time".to_string());
    market_view.config.y_columns = vec![
        "revenue".to_string(),
        "margin".to_string(),
    ];
    views.push(Box::new(market_view));
    
    views
}

/// Create views for signal analysis demo
fn create_signal_analysis_views(data_source_id: String) -> Vec<Box<dyn SpaceView>> {
    let mut views: Vec<Box<dyn SpaceView>> = Vec::new();
    
    // Signal Decomposition
    let signals_id = Uuid::new_v4();
    let mut signals_view = TimeSeriesView::new(signals_id, "Signal Decomposition".to_string());
    signals_view.config.data_source_id = Some(data_source_id.clone());
    signals_view.config.x_column = Some("time".to_string());
    signals_view.config.y_columns = vec![
        "combined".to_string(),
        "trend".to_string(),
        "seasonal".to_string(),
        "noise".to_string(),
    ];
    views.push(Box::new(signals_view));
    
    // Frequency Analysis
    let freq_id = Uuid::new_v4();
    let mut freq_view = TimeSeriesView::new(freq_id, "Frequency Components".to_string());
    freq_view.config.data_source_id = Some(data_source_id);
    freq_view.config.x_column = Some("time".to_string());
    freq_view.config.y_columns = vec![
        "sin_wave".to_string(),
        "cos_wave".to_string(),
        "square_wave".to_string(),
    ];
    views.push(Box::new(freq_view));
    
    views
}

/// Main application state
struct FrogApp {
    /// The viewport managing all docked views
    viewport: Viewport,
    
    /// Viewer context shared between all views
    viewer_context: Arc<ViewerContext>,
    
    /// Navigation panel
    _navigation_panel: NavigationPanel,
    
    /// Application shell
    _app_shell: AppShell,
    
    /// Current theme
    _theme: Theme,
    
    /// Demo mode
    demo_mode: bool,
    
    /// Tokio runtime
    runtime: tokio::runtime::Runtime,
    
    /// Egui context
    egui_ctx: egui::Context,
    
    /// View builder dialog
    view_builder: Option<ViewBuilderDialog>,
    
    /// Frog mascot
    frog_mascot: FrogMascot,
    
    /// Demo overlay
    demo_overlay: DemoOverlay,
    
    /// Frame accumulator for smooth playback at all speeds
    frame_accumulator: f64,
    
    /// SQLite table selection state
    sqlite_table_selection: Option<(std::path::PathBuf, Vec<String>)>,
    
    /// Show floating summary stats window
    show_summary_stats: bool,
    
    /// Loading state
    is_loading: Arc<RwLock<bool>>,
    
    /// Flag to open dashboard builder when data is loaded
    open_builder_on_load: bool,
    
    /// Data source selection dialog
    dashboard_builder: ViewBuilderDialog,

    /// File configuration dialog
    file_config_dialog: Option<FileConfigDialog>,
}

impl FrogApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Setup custom theme
        dv_ui::apply_theme(&cc.egui_ctx, &Theme::default());
        
        // Initialize tokio runtime
        let runtime = tokio::runtime::Runtime::new().unwrap();
        
        // Create shared viewer context
        let viewer_context = Arc::new(ViewerContext {
            data_sources: Arc::new(RwLock::new(HashMap::new())),
            navigation: Arc::new(NavigationEngine::new(NavigationMode::Sequential)),
            time_control: Arc::new(RwLock::new(TimeControl::default())),
            hovered_data: Arc::new(RwLock::new(HoveredData::default())),
            frame_time: Arc::new(RwLock::new(FrameTime::default())),
            runtime_handle: runtime.handle().clone(),
            time_axis_views: Arc::new(RwLock::new(Vec::new())),
        });
        
        // Create navigation panel
        let navigation_panel = dv_ui::NavigationPanel::new(
            viewer_context.navigation.clone(),
            viewer_context.time_control.clone()
        );
        
        // Create viewport for dockable views
        let viewport = Viewport::new();
        
        // Create app shell
        let app_shell = AppShell::new();
        
        // Get runtime handle for file config dialog
        let _runtime_handle = runtime.handle().clone();
        
        Self {
            viewport,
            viewer_context,
            _navigation_panel: navigation_panel,
            _app_shell: app_shell,
            _theme: Theme::default(),
            runtime,
            demo_mode: false,
            egui_ctx: cc.egui_ctx.clone(),
            view_builder: None,
            frog_mascot: FrogMascot::new(),
            demo_overlay: DemoOverlay::new(),
            frame_accumulator: 0.0,
            sqlite_table_selection: None,
            show_summary_stats: false,
            is_loading: Arc::new(RwLock::new(false)),
            open_builder_on_load: false,
            dashboard_builder: ViewBuilderDialog::new_multi(Vec::new()),
            file_config_dialog: None,
        }
    }
    
    /// Initialize demo mode with a specific example
    fn init_demo_example(&mut self, example: DemoExample) {
        use crate::demo::DemoDataSource;
        
        // Clear any existing state when switching demos
        *self.viewer_context.data_sources.write() = HashMap::new();
        self.viewport = Viewport::new();
        
        // Clear hover data and selection state
        {
            let mut hover_data = self.viewer_context.hovered_data.write();
            hover_data.view_id = None;
            hover_data.point_index = None;
        }
        
        // Set demo mode
        self.demo_mode = true;
        
        // Create demo data source
        let demo_source = Box::new(DemoDataSource::new());
        
        // Update navigation spec
        if let Ok(spec) = self.runtime.block_on(demo_source.navigation_spec()) {
            self.viewer_context.navigation.update_spec(spec);
        }
        
        // Set it as the current data source
        let mut sources = HashMap::new();
        let demo_source_id = Uuid::new_v4().to_string();
        sources.insert(demo_source_id.clone(), demo_source as Box<dyn DataSource>);
        *self.viewer_context.data_sources.write() = sources;
        
        // Create appropriate views based on the example
        let views = match example {
            DemoExample::AssemblyLine => create_assembly_line_views(demo_source_id.clone()),
            DemoExample::SensorNetwork => create_sensor_network_views(demo_source_id.clone()),
            DemoExample::FinancialDashboard => create_financial_views(demo_source_id.clone()),
            DemoExample::SignalAnalysis => create_signal_analysis_views(demo_source_id.clone()),
        };
        
        // Create layout
        self.viewport.create_grid_layout(views);
        
        info!("Demo mode initialized with {} example", example.name());
    }
    
    /// Open a CSV file
    fn open_csv_file(&mut self, path: std::path::PathBuf) {
        info!("Opening CSV file: {:?}", path);
        
        // Create file configuration dialog
        let mut config_manager = dv_data::config::FileConfigManager::new();
        
        // Create FileConfig
        let config = dv_data::config::FileConfig::new(path, dv_data::config::FileType::Csv);
        config_manager.add_file(config);
        
        // Show the file configuration dialog
        self.file_config_dialog = Some(FileConfigDialog::new(config_manager, self.runtime.handle().clone()));
    }
    
    /// Open multiple CSV files as a combined data source
    fn open_multiple_csv_files(&mut self, paths: Vec<std::path::PathBuf>) {
        info!("Opening {} CSV files as combined source", paths.len());
        
        // Set loading state
        *self.is_loading.write() = true;
        
        let source_future = CombinedCsvSource::new(paths.clone());
        
        let ctx = self.egui_ctx.clone();
        let viewer_context = self.viewer_context.clone();
        let runtime = self.runtime.handle().clone();
        let is_loading = self.is_loading.clone();
        
        runtime.spawn(async move {
            match source_future.await {
                Ok(source) => {
                    // Update navigation spec
                    if let Ok(spec) = source.navigation_spec().await {
                        viewer_context.navigation.update_spec(spec);
                    }
                    
                    // Update data source
                    *viewer_context.data_sources.write() = HashMap::from([(Uuid::new_v4().to_string(), Box::new(source) as Box<dyn DataSource>)]);
                    
                    *is_loading.write() = false;
                    ctx.request_repaint();
                }
                Err(e) => {
                    error!("Failed to open multiple CSV files: {}", e);
                    *is_loading.write() = false;
                    ctx.request_repaint();
                }
            }
        });
    }
    
    /// Open a SQLite database file
    fn open_sqlite_file(&mut self, path: std::path::PathBuf) {
        info!("Opening SQLite file: {:?}", path);
        
        // Create file configuration dialog
        let mut config_manager = dv_data::config::FileConfigManager::new();
        
        // Create FileConfig
        let config = dv_data::config::FileConfig::new(path, dv_data::config::FileType::Sqlite);
        config_manager.add_file(config);
        
        // Show the file configuration dialog
        self.file_config_dialog = Some(FileConfigDialog::new(config_manager, self.runtime.handle().clone()));
    }
    
    /// Show a dialog for selecting which SQLite table to open
    fn show_table_selection_dialog(&mut self, path: std::path::PathBuf, tables: Vec<String>) {
        // Store the table selection state in the app
        self.sqlite_table_selection = Some((path, tables));
    }
    
    /// Open a specific SQLite table
    fn open_sqlite_table<P: AsRef<std::path::Path>>(&mut self, path: P, table_name: &str) {
        let path = path.as_ref();
        info!("Opening SQLite table: {} from {:?}", table_name, path);
        
        let source_future = SqliteSource::new(path.to_path_buf(), table_name.to_string());
        
        let ctx = self.egui_ctx.clone();
        let viewer_context = self.viewer_context.clone();
        let runtime = self.runtime.handle().clone();
        
        runtime.spawn(async move {
            match source_future.await {
                Ok(source) => {
                    // Update navigation spec
                    if let Ok(spec) = source.navigation_spec().await {
                        viewer_context.navigation.update_spec(spec);
                    }
                    
                    // Update data source
                    *viewer_context.data_sources.write() = HashMap::from([(Uuid::new_v4().to_string(), Box::new(source) as Box<dyn DataSource>)]);
                    
                    ctx.request_repaint();
                }
                Err(e) => {
                    error!("Failed to open SQLite table: {}", e);
                }
            }
        });
    }
    
    /// Load files based on configuration
    fn load_configured_files(&mut self, config_manager: dv_data::config::FileConfigManager) {
        use dv_data::config::FileType;
        
        // Separate CSV and SQLite files
        let mut csv_configs = Vec::new();
        let mut sqlite_configs = Vec::new();
        
        for (_path, config) in config_manager.configs {
            match config.file_type {
                FileType::Csv => csv_configs.push(config),
                FileType::Sqlite => sqlite_configs.push(config),
            }
        }
        
        // Load CSV files
        if !csv_configs.is_empty() {
            if csv_configs.len() == 1 {
                // Single CSV file - load as individual source
                let config = csv_configs.into_iter().next().unwrap();
                let source_id = config.file_name();
                self.load_configured_csv(source_id, config);
            } else {
                // Multiple CSV files - use ConfiguredCombinedCsvSource
                self.load_configured_combined_csv(csv_configs);
            }
        }
        
        // Load SQLite files (always individual)
        for config in sqlite_configs {
            let source_id = config.file_name();
            if let Some(table_name) = config.selected_columns.iter().next() {
                let table_id = format!("{}:{}", source_id, table_name);
                self.load_configured_sqlite(table_id, config.clone(), table_name.clone());
            }
        }
    }
    
    /// Load a single configured CSV file
    fn load_configured_csv(&mut self, source_id: String, config: dv_data::config::FileConfig) {
        info!("Loading configured CSV: {} from {:?}", source_id, config.path);
        
        // Set loading state
        *self.is_loading.write() = true;
        
        let ctx = self.egui_ctx.clone();
        let viewer_context = self.viewer_context.clone();
        let runtime = self.runtime.handle().clone();
        let is_loading = self.is_loading.clone();
        
        runtime.spawn(async move {
            match dv_data::sources::ConfiguredCsvSource::new(config).await {
                Ok(source) => {
                    // Update navigation spec BEFORE adding the source
                    if let Ok(spec) = source.navigation_spec().await {
                        viewer_context.navigation.update_spec(spec);
                    }
                    
                    // Add to data sources map
                    viewer_context.data_sources.write().insert(
                        source_id,
                        Box::new(source) as Box<dyn DataSource>
                    );
                    
                    *is_loading.write() = false;
                    ctx.request_repaint();
                }
                Err(e) => {
                    error!("Failed to load configured CSV: {}", e);
                    *is_loading.write() = false;
                    ctx.request_repaint();
                }
            }
        });
    }
    
    /// Load multiple configured CSV files as a combined source
    fn load_configured_combined_csv(&mut self, configs: Vec<dv_data::config::FileConfig>) {
        info!("Loading {} CSV files as combined source", configs.len());
        
        // Set loading state
        *self.is_loading.write() = true;
        
        let ctx = self.egui_ctx.clone();
        let viewer_context = self.viewer_context.clone();
        let runtime = self.runtime.handle().clone();
        let is_loading = self.is_loading.clone();
        
        runtime.spawn(async move {
            match dv_data::sources::ConfiguredCombinedCsvSource::new(configs).await {
                Ok(source) => {
                    // Update navigation spec BEFORE adding the source
                    if let Ok(spec) = source.navigation_spec().await {
                        viewer_context.navigation.update_spec(spec);
                    }
                    
                    // Add to data sources map with a combined source ID
                    let combined_source_id = "Combined CSV Files".to_string();
                    viewer_context.data_sources.write().insert(
                        combined_source_id,
                        Box::new(source) as Box<dyn DataSource>
                    );
                    
                    *is_loading.write() = false;
                    ctx.request_repaint();
                }
                Err(e) => {
                    error!("Failed to load combined CSV files: {}", e);
                    *is_loading.write() = false;
                    ctx.request_repaint();
                }
            }
        });
    }
    
    /// Load a configured SQLite table
    fn load_configured_sqlite(&mut self, source_id: String, config: dv_data::config::FileConfig, table_name: String) {
        info!("Loading configured SQLite table: {} from {:?}", table_name, config.path);
        
        // Set loading state
        *self.is_loading.write() = true;
        
        let ctx = self.egui_ctx.clone();
        let viewer_context = self.viewer_context.clone();
        let runtime = self.runtime.handle().clone();
        let is_loading = self.is_loading.clone();
        
        runtime.spawn(async move {
            match SqliteSource::new(config.path, table_name).await {
                Ok(source) => {
                    // Update navigation spec BEFORE adding the source
                    if let Ok(spec) = source.navigation_spec().await {
                        viewer_context.navigation.update_spec(spec);
                    }
                    
                    // Add to data sources map
                    viewer_context.data_sources.write().insert(
                        source_id,
                        Box::new(source) as Box<dyn DataSource>
                    );
                    
                    *is_loading.write() = false;
                    ctx.request_repaint();
                }
                Err(e) => {
                    error!("Failed to load SQLite table: {}", e);
                    *is_loading.write() = false;
                    ctx.request_repaint();
                }
            }
        });
    }
    
    /// Handle menu actions
    fn handle_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar")
            .exact_height(20.0) // Reduced from 24.0
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_gray(40)) // Darker background for better visibility
                    .inner_margin(egui::Margin::symmetric(8.0, 1.0)) // Further reduced vertical margin
                    .outer_margin(0.0)
                    .stroke(egui::Stroke::new(1.0, Color32::from_gray(60))) // Subtle border
            )
            .show(ctx, |ui| {
                ui.style_mut().visuals.button_frame = true;
                ui.style_mut().visuals.menu_rounding = Rounding::same(4.0);
                ui.style_mut().spacing.button_padding = Vec2::new(8.0, 4.0);
                
                egui::menu::bar(ui, |ui| {
                    ui.menu_button(
                        egui::RichText::new("File").color(Color32::WHITE).size(14.0),
                        |ui| {
                            // Home option to return to welcome screen
                            if ui.button(
                                egui::RichText::new("🏠 Home").color(Color32::WHITE)
                            ).on_hover_text("Return to welcome screen (Press H)").clicked() {
                                *self.viewer_context.data_sources.write() = HashMap::new();
                                self.viewport = Viewport::new();
                                self.demo_mode = false;
                                self.view_builder = None;
                                ui.close_menu();
                            }
                            
                            ui.separator();
                            
                            // Single Demo Mode entry point
                            if ui.button(
                                egui::RichText::new("🎮 Demo Mode").color(Color32::WHITE)
                            ).on_hover_text("Explore example datasets (Press D)").clicked() {
                                self.demo_overlay.show = true;
                                ui.close_menu();
                            }
                            
                            ui.separator();
                            
                            if ui.button(
                                egui::RichText::new("📂 Open File(s)...").color(Color32::WHITE)
                            ).on_hover_text("Browse for CSV or SQLite files").clicked() {
                                if let Some(paths) = rfd::FileDialog::new()
                                    .add_filter("Data Files", &["csv", "db", "sqlite", "sqlite3"])
                                    .add_filter("CSV Files", &["csv"])
                                    .add_filter("SQLite Database", &["db", "sqlite", "sqlite3"])
                                    .pick_files()
                                {
                                    // Create file configuration dialog for all selected files
                                    let mut config_manager = dv_data::config::FileConfigManager::new();
                                    
                                    for path in paths {
                                        let file_type = if path.extension()
                                            .and_then(|ext| ext.to_str())
                                            .map(|ext| ext.eq_ignore_ascii_case("db") || ext.eq_ignore_ascii_case("sqlite") || ext.eq_ignore_ascii_case("sqlite3"))
                                            .unwrap_or(false)
                                        {
                                            dv_data::config::FileType::Sqlite
                                        } else {
                                            dv_data::config::FileType::Csv
                                        };
                                        
                                        // Add file to configuration manager
                                        let config = dv_data::config::FileConfig::new(path, file_type);
                                        config_manager.add_file(config);
                                    }
                                    
                                    // Show file configuration dialog
                                    if !config_manager.configs.is_empty() {
                                        info!("Creating FileConfigDialog with {} files", config_manager.configs.len());
                                        self.file_config_dialog = Some(FileConfigDialog::new(config_manager, self.runtime.handle().clone()));
                                        info!("FileConfigDialog created successfully");
                                    }
                                    ui.close_menu();
                                }
                            }
                            
                            if ui.button(
                                egui::RichText::new("🚪 Exit").color(Color32::WHITE)
                            ).clicked() {
                                self.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }
                    );
                    
                    ui.menu_button(
                        egui::RichText::new("View").color(Color32::WHITE).size(14.0),
                        |ui| {
                            // Only show Dashboard Builder if data is loaded
                            let has_data = !self.viewer_context.data_sources.read().is_empty();
                            
                            ui.add_enabled_ui(has_data, |ui| {
                                if ui.button(
                                    egui::RichText::new("🎨 Dashboard Builder...").color(if has_data { Color32::WHITE } else { Color32::from_gray(140) })
                                ).on_hover_text(if has_data { "Build custom dashboards from your data (Press B)" } else { "Load data first to build dashboards" }).clicked() {
                                    // Collect all data sources and schemas
                                    let data_sources = self.viewer_context.data_sources.read();
                                    let mut sources_with_schemas = Vec::new();
                                    
                                    for (id, source) in data_sources.iter() {
                                        let schema = self.runtime.block_on(source.schema());
                                        sources_with_schemas.push((id.clone(), schema));
                                    }
                                    
                                    if !sources_with_schemas.is_empty() {
                                        info!("Creating ViewBuilderDialog with {} data sources", sources_with_schemas.len());
                                        self.view_builder = Some(ViewBuilderDialog::new_multi(sources_with_schemas));
                                    }
                                    ui.close_menu();
                                }
                            });
                            
                            // Revisit file configuration
                            ui.add_enabled_ui(has_data, |ui| {
                                if ui.button(
                                    egui::RichText::new("⚙️ Revisit File Configuration...").color(if has_data { Color32::WHITE } else { Color32::from_gray(140) })
                                ).on_hover_text(if has_data { "Modify column selection and data types for loaded files" } else { "Load data first to configure" }).clicked() {
                                    // Create a new FileConfigManager from existing data sources
                                    let mut config_manager = dv_data::config::FileConfigManager::new();
                                    
                                    // Get the first data source info to recreate config
                                    // Note: This is a simplified approach - in production you might want to store original configs
                                    let data_sources = self.viewer_context.data_sources.read();
                                    
                                    if let Some((source_id, source)) = data_sources.iter().next() {
                                        // Try to guess the file path from source name
                                        let source_name = source.source_name();
                                        let schema = self.runtime.block_on(source.schema());
                                        
                                        // Create a basic config based on current schema
                                        let path = std::path::PathBuf::from(source_name);
                                        let file_type = if source_name.ends_with(".db") || source_name.ends_with(".sqlite") {
                                            dv_data::config::FileType::Sqlite
                                        } else {
                                            dv_data::config::FileType::Csv
                                        };
                                        
                                        let mut config = dv_data::config::FileConfig::new(path, file_type);
                                        
                                        // Populate with current columns
                                        for field in schema.fields() {
                                            config.selected_columns.insert(field.name().to_string());
                                            config.detected_columns.push(field.name().to_string());
                                            config.column_types.insert(
                                                field.name().to_string(),
                                                field.data_type().clone().into()
                                            );
                                        }
                                        
                                        config.is_loaded = true;
                                        config_manager.add_file(config);
                                    }
                                    
                                    // Show file configuration dialog
                                    self.file_config_dialog = Some(FileConfigDialog::new(config_manager, self.runtime.handle().clone()));
                                    ui.close_menu();
                                }
                            });
                            
                            ui.separator();
                            
                            if ui.button(
                                egui::RichText::new("🔧 Reset Zoom").color(Color32::WHITE)
                            ).on_hover_text("Reset zoom in all plots (Press R)").clicked() {
                                // TODO: Implement zoom reset functionality
                                ui.close_menu();
                            }
                            
                            if ui.button(
                                egui::RichText::new("🗃️ Reset Layout").color(Color32::WHITE)
                            ).on_hover_text("Reset to default panel arrangement").clicked() {
                                // TODO: Implement layout reset
                                ui.close_menu();
                            }
                            

                        }
                    );
                    
                    ui.menu_button(
                        egui::RichText::new("Shortcuts").color(Color32::WHITE).size(14.0),
                        |ui| {
                            ui.label(egui::RichText::new("Keyboard Shortcuts").strong());
                            ui.separator();
                            
                            ui.horizontal(|ui| {
                                ui.label("Space:");
                                ui.label("Play/Pause");
                            });
                            ui.horizontal(|ui| {
                                ui.label("← →:");
                                ui.label("Step backward/forward");
                            });
                            ui.horizontal(|ui| {
                                ui.label("Z:");
                                ui.label("Reset zoom");
                            });
                            ui.horizontal(|ui| {
                                ui.label("R:");
                                ui.label("Reset zoom & selection");
                            });
                            ui.horizontal(|ui| {
                                ui.label("H:");
                                ui.label("Go home");
                            });
                            ui.horizontal(|ui| {
                                ui.label("B:");
                                ui.label("Open builder");
                            });
                            ui.horizontal(|ui| {
                                ui.label("D:");
                                ui.label("Demo mode");
                            });
                            ui.horizontal(|ui| {
                                ui.label("Esc:");
                                ui.label("Stop playback");
                            });
                            
                            ui.separator();
                            ui.label(egui::RichText::new("Mouse Controls").strong());
                            ui.separator();
                            
                            ui.horizontal(|ui| {
                                ui.label("Left-click:");
                                ui.label("Highlight values");
                            });
                            ui.horizontal(|ui| {
                                ui.label("Right-click:");
                                ui.label("Place marker");
                            });
                            ui.horizontal(|ui| {
                                ui.label("Right-drag:");
                                ui.label("Box zoom");
                            });
                            ui.horizontal(|ui| {
                                ui.label("Left-drag:");
                                ui.label("Pan view");
                            });
                            ui.horizontal(|ui| {
                                ui.label("Scroll wheel:");
                                ui.label("Zoom in/out");
                            });
                        }
                    );
                    
                    // Right-aligned F.R.O.G. branding
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("🐸 F.R.O.G.")
                                .color(Color32::from_rgb(92, 140, 97))
                                .size(14.0)
                                .strong()
                        );
                        ui.separator();
                        
                        // Show current data source if loaded
                        if let Some(data_source) = self.viewer_context.data_sources.read().values().next() {
                            ui.label(
                                egui::RichText::new(data_source.source_name())
                                    .color(Color32::from_gray(200))
                                    .size(12.0)
                            );
                        }
                    });
                });
            });
    }
    
    /// Show welcome screen
    fn show_welcome_screen(&mut self, ui: &mut Ui) {
        // Get the available rect (which excludes the menu bar)
        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter();
        
        // Only animate if we're actually on the welcome screen
        let has_data = !self.viewer_context.data_sources.read().is_empty();
        let show_animation = self.viewer_context.data_sources.read().is_empty() && self.viewport.is_empty();
        
        painter.rect_filled(
            rect,
            Rounding::ZERO,
            Color32::from_rgb(15, 20, 25)
        );
        
        // Add some subtle animated circles in background (only when visible)
        if show_animation {
            let time = ui.input(|i| i.time) as f32;
            
            for i in 0..3 {
                let offset = i as f32 * 2.0;
                let circle_time = time * 0.3 + offset;
                let radius = 200.0 + (circle_time.sin() * 50.0);
                let alpha = ((circle_time * 0.5).sin() + 1.0) * 0.5 * 20.0;
                
                painter.circle(
                    rect.center() + Vec2::new(
                        (circle_time * 0.7).cos() * 100.0,
                        (circle_time * 0.5).sin() * 80.0
                    ),
                    radius,
                    Color32::from_rgba_premultiplied(50, 100, 150, alpha as u8),
                    Stroke::NONE
                );
            }
            
            // Only request continuous repaint if animation is showing
            ui.ctx().request_repaint();
        }
        
        // Center content
        let available_size = ui.available_size();
        let _center = egui::pos2(available_size.x / 2.0, available_size.y / 2.0);
        
        ui.allocate_ui_at_rect(rect, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(available_size.y * 0.2);
                
                // F.R.O.G. Logo Animation
                self.frog_mascot.ui(ui, 120.0);
                
                ui.add_space(20.0);
                
                ui.heading(
                    egui::RichText::new("F.R.O.G.")
                        .size(48.0)
                        .color(Color32::from_rgb(92, 140, 97))
                        .strong()
                );
                
                // Animated subtitle with flowing colors
                let time = ui.ctx().input(|i| i.time) as f32;
                let color_phase = (time * 0.5).sin() * 0.5 + 0.5;
                let subtitle_color = Color32::from_rgb(
                    (140.0 + color_phase * 60.0) as u8,
                    (150.0 + color_phase * 50.0) as u8,
                    (160.0 + color_phase * 40.0) as u8,
                );
                
                ui.label(
                    egui::RichText::new("Flexible Rust Overlay for Graphs")
                        .size(18.0)
                        .color(subtitle_color)
                );
                
                ui.add_space(30.0);
                
                // Check if we have data loaded but no views
                let has_data = !self.viewer_context.data_sources.read().is_empty();
                
                if has_data {
                    // Data is loaded but no views - show more informative state
                    // Get the source name first
                    let source_name = {
                        let data_sources = self.viewer_context.data_sources.read();
                        data_sources.values().next().map(|s| s.source_name().to_string()).unwrap_or_else(|| "Unknown".to_string())
                    };
                    
                    // Get the schema in a separate operation to avoid holding the lock
                    let schema_opt = {
                        let data_sources = self.viewer_context.data_sources.read();
                        if let Some(source) = data_sources.values().next() {
                            Some(self.runtime.block_on(source.schema()))
                        } else {
                            None
                        }
                    };
                    
                    if let Some(schema) = schema_opt {
                        // Nice data loaded indicator
                        ui.group(|ui| {
                            ui.set_max_width(600.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("✅").size(24.0).color(Color32::from_rgb(76, 175, 80)));
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new("Data Successfully Loaded")
                                            .size(18.0)
                                            .color(Color32::from_gray(220))
                                            .strong()
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("Source: {}", source_name))
                                            .size(14.0)
                                            .color(Color32::from_gray(160))
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{} columns available for visualization", schema.fields().len()))
                                            .size(14.0)
                                            .color(Color32::from_gray(160))
                                    );
                                });
                            });
                        });
                        
                        ui.add_space(30.0);
                        
                        // Action buttons
                        ui.horizontal(|ui| {
                            ui.spacing_mut().button_padding = Vec2::new(20.0, 14.0);
                            
                            // Primary action - Dashboard Builder
                            let primary_button = ui.add(
                                egui::Button::new(
                                    egui::RichText::new("🎨 Create Dashboard")
                                        .size(18.0)
                                        .color(Color32::WHITE)
                                        .strong()
                                )
                                .fill(Color32::from_rgb(76, 175, 80))
                                .rounding(Rounding::same(8.0))
                            );
                            
                            if primary_button.clicked() || ui.input(|i| i.key_pressed(egui::Key::B)) {
                                info!("Creating ViewBuilderDialog from welcome screen");
                                // Collect all data sources and schemas
                                let data_sources = self.viewer_context.data_sources.read();
                                let mut sources_with_schemas = Vec::new();
                                
                                for (id, source) in data_sources.iter() {
                                    let schema = self.runtime.block_on(source.schema());
                                    sources_with_schemas.push((id.clone(), schema));
                                }
                                
                                if !sources_with_schemas.is_empty() {
                                    self.view_builder = Some(ViewBuilderDialog::new_multi(sources_with_schemas));
                                }
                            }
                            
                            primary_button.on_hover_text("Open the visual dashboard builder (Press B)");
                            
                            ui.add_space(10.0);
                            
                            // Secondary action - Quick templates
                            let templates_button = ui.add(
                                egui::Button::new(
                                    egui::RichText::new("⚡ Quick Start")
                                        .size(16.0)
                                )
                                .rounding(Rounding::same(8.0))
                            );
                            
                            if templates_button.clicked() {
                                // Create a simple default layout
                                let views = create_default_views_for_schema(&schema);
                                self.viewport.create_grid_layout(views);
                            }
                            
                            templates_button.on_hover_text("Automatically create views based on your data");
                        });
                        
                        ui.add_space(40.0);
                        
                        // Show column preview
                        ui.collapsing("📊 Available Columns", |ui| {
                            // Get current data batch for statistics
                            let batch_opt = {
                                let data_sources = self.viewer_context.data_sources.read();
                                if let Some(source) = data_sources.values().next() {
                                    let nav_pos = self.viewer_context.navigation.get_context().position.clone();
                                    self.runtime.block_on(source.query_at(&nav_pos)).ok()
                                } else {
                                    None
                                }
                            };
                            
                            egui::ScrollArea::vertical()
                                .max_height(300.0)
                                .show(ui, |ui| {
                                    ui.set_min_width(500.0);
                                    for (idx, field) in schema.fields().iter().enumerate() {
                                        ui.group(|ui| {
                                            ui.horizontal(|ui| {
                                                // Column type icon
                                                let icon = match field.data_type() {
                                                    arrow::datatypes::DataType::Float64 | 
                                                    arrow::datatypes::DataType::Float32 | 
                                                    arrow::datatypes::DataType::Int64 | 
                                                    arrow::datatypes::DataType::Int32 => "📊",
                                                    arrow::datatypes::DataType::Utf8 => "📝",
                                                    arrow::datatypes::DataType::Boolean => "✓",
                                                    arrow::datatypes::DataType::Date32 | 
                                                    arrow::datatypes::DataType::Date64 | 
                                                    arrow::datatypes::DataType::Timestamp(_, _) => "⏱️",
                                                    _ => "❓",
                                                };
                                                
                                                ui.label(egui::RichText::new(icon).size(16.0));
                                                ui.label(egui::RichText::new(field.name()).size(14.0).color(Color32::from_gray(200)).strong());
                                                ui.label(egui::RichText::new(format!("({})", field.data_type())).size(12.0).color(Color32::from_gray(140)));
                                            });
                                            
                                            // Show column statistics if we have data
                                            if let Some(ref batch) = batch_opt {
                                                if let Some(column) = batch.column(idx).as_any().downcast_ref::<arrow::array::StringArray>() {
                                                    self.show_column_stats(ui, field.name(), column);
                                                } else if let Some(column) = batch.column(idx).as_any().downcast_ref::<Float64Array>() {
                                                    ui.horizontal(|ui| {
                                                        ui.add_space(20.0);
                                                        if let (Some(min), Some(max)) = (arrow::compute::min(column), arrow::compute::max(column)) {
                                                            ui.label(egui::RichText::new(format!("Range: {:.2} - {:.2}", min, max)).size(12.0).color(Color32::from_gray(160)));
                                                        }
                                                    });
                                                } else if let Some(column) = batch.column(idx).as_any().downcast_ref::<Int64Array>() {
                                                    ui.horizontal(|ui| {
                                                        ui.add_space(20.0);
                                                        if let (Some(min), Some(max)) = (arrow::compute::min(column), arrow::compute::max(column)) {
                                                            ui.label(egui::RichText::new(format!("Range: {} - {}", min, max)).size(12.0).color(Color32::from_gray(160)));
                                                        }
                                                    });
                                                }
                                            }
                                        });
                                        ui.add_space(4.0);
                                    }
                                });
                        });
                        
                        ui.add_space(30.0);
                        
                        // Keyboard shortcuts
                        ui.group(|ui| {
                            ui.set_max_width(400.0);
                            ui.label(egui::RichText::new("⌨️ Keyboard Shortcuts").size(14.0).color(Color32::from_gray(200)).strong());
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("B").size(12.0).color(Color32::from_gray(180)).strong());
                                ui.label(egui::RichText::new("- Open Dashboard Builder").size(12.0).color(Color32::from_gray(160)));
                            });
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("S").size(12.0).color(Color32::from_gray(180)).strong());
                                ui.label(egui::RichText::new("- Toggle Summary Statistics").size(12.0).color(Color32::from_gray(160)));
                            });
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("D").size(12.0).color(Color32::from_gray(180)).strong());
                                ui.label(egui::RichText::new("- Demo Mode").size(12.0).color(Color32::from_gray(160)));
                            });
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("H").size(12.0).color(Color32::from_gray(180)).strong());
                                ui.label(egui::RichText::new("- Return to Home").size(12.0).color(Color32::from_gray(160)));
                            });
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Ctrl+O").size(12.0).color(Color32::from_gray(180)).strong());
                                ui.label(egui::RichText::new("- Open File").size(12.0).color(Color32::from_gray(160)));
                            });
                        });
                    }
                } else {
                    // No data loaded - show the two main action buttons
                    ui.add_space(60.0);
                    
                    // Center the buttons horizontally
                    ui.horizontal(|ui| {
                        // Calculate button size and spacing
                        let button_width = 200.0;
                        let button_height = 60.0;
                        let spacing = 20.0;
                        let total_width = button_width * 2.0 + spacing;
                        let available_width = ui.available_width();
                        let offset = (available_width - total_width) / 2.0;
                        
                        // Add left spacing to center the buttons
                        ui.add_space(offset);
                        
                        ui.spacing_mut().item_spacing = Vec2::new(spacing, 0.0);
                        
                        // Green demo button
                        let demo_response = ui.add_sized(
                            [button_width, button_height],
                            egui::Button::new(
                                egui::RichText::new("🎮 Demo Mode")
                                    .size(20.0)
                                    .color(Color32::WHITE)
                                    .strong()
                            )
                            .fill(Color32::from_rgb(76, 175, 80))  // Green
                            .rounding(Rounding::same(8.0))
                        );
                        
                        if demo_response.clicked() || ui.input(|i| i.key_pressed(egui::Key::D)) {
                            self.demo_overlay.show = true;
                        }
                        
                        demo_response.on_hover_text("Explore example datasets (Press D)");
                        
                        // Blue load data button - using vertical layout for subtitle
                        ui.allocate_ui(Vec2::new(button_width, button_height), |ui| {
                            let load_response = ui.add_sized(
                                ui.available_size(),
                                egui::Button::new("")
                                    .fill(Color32::from_rgb(33, 150, 243))  // Blue
                                    .rounding(Rounding::same(8.0))
                            );
                            
                            // Draw text on top of button
                            if ui.is_rect_visible(load_response.rect) {
                                let text_pos = load_response.rect.center() - Vec2::new(0.0, 8.0);
                                ui.painter().text(
                                    text_pos,
                                    egui::Align2::CENTER_CENTER,
                                    "📂 Open File",
                                    egui::FontId::new(20.0, egui::FontFamily::Proportional),
                                    Color32::WHITE,
                                );
                                let subtitle_pos = load_response.rect.center() + Vec2::new(0.0, 12.0);
                                ui.painter().text(
                                    subtitle_pos,
                                    egui::Align2::CENTER_CENTER,
                                    "(CSV or SQLite)",
                                    egui::FontId::new(14.0, egui::FontFamily::Proportional),
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 200),
                                );
                            }
                            
                            if load_response.clicked() {
                                if let Some(paths) = rfd::FileDialog::new()
                                    .add_filter("Data Files", &["csv", "db", "sqlite", "sqlite3"])
                                    .pick_files()
                                {
                                    // Create file configuration dialog for all selected files
                                    let mut config_manager = dv_data::config::FileConfigManager::new();
                                    
                                    for path in paths {
                                        let file_type = if path.extension()
                                            .and_then(|ext| ext.to_str())
                                            .map(|ext| ext.eq_ignore_ascii_case("db") || ext.eq_ignore_ascii_case("sqlite") || ext.eq_ignore_ascii_case("sqlite3"))
                                            .unwrap_or(false)
                                        {
                                            dv_data::config::FileType::Sqlite
                                        } else {
                                            dv_data::config::FileType::Csv
                                        };
                                        
                                        // Add file to configuration manager
                                        let config = dv_data::config::FileConfig::new(path, file_type);
                                        config_manager.add_file(config);
                                    }
                                    
                                    // Show file configuration dialog
                                    if !config_manager.configs.is_empty() {
                                        self.file_config_dialog = Some(FileConfigDialog::new(config_manager, self.runtime.handle().clone()));
                                    }
                                }
                            }
                            
                            load_response.on_hover_text("Browse for data files (Ctrl+O)");
                        });
                    });
                }
                
                // Add space before the bottom section - moved up a bit more
                ui.add_space(available_size.y * 0.08);
                
                // Tagline at the bottom
                ui.label(
                    egui::RichText::new("🐸 Hop through your data with ease")
                        .size(16.0)
                        .color(Color32::from_gray(200))
                );
            });
        });
    }
    
    /// Show floating summary statistics window
    fn show_summary_stats_window(&mut self, ctx: &egui::Context) {
        let mut show_stats = self.show_summary_stats;
        
        egui::Window::new("📊 Summary Statistics")
            .default_pos(egui::pos2(800.0, 100.0))
            .default_size(egui::vec2(400.0, 500.0))
            .resizable(true)
            .collapsible(true)
            .open(&mut show_stats)
            .show(ctx, |ui| {
                // Check if we have data
                let has_data_source = {
                    let data_sources = self.viewer_context.data_sources.read();
                    !data_sources.is_empty()
                };
                if !has_data_source {
                    ui.centered_and_justified(|ui| {
                        ui.label("No data loaded");
                    });
                    return;
                }
                
                // Get the first data source
                let data_sources = self.viewer_context.data_sources.read();
                if let Some((_, data_source)) = data_sources.iter().next() {
                    let schema = self.runtime.block_on(data_source.schema());
                    
                    // Get current data batch
                    let nav_pos = self.viewer_context.navigation.get_context().position.clone();
                    if let Ok(batch) = self.runtime.block_on(data_source.query_at(&nav_pos)) {
                        self.render_summary_stats(ui, &batch, &schema);
                    } else {
                        ui.label("Failed to load data");
                    }
                } else {
                    return; // No data source available
                }
            });
        
        self.show_summary_stats = show_stats;
    }
    
    /// Render summary statistics for the current data
    fn render_summary_stats(&self, ui: &mut Ui, batch: &arrow::record_batch::RecordBatch, schema: &arrow::datatypes::Schema) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Data Overview");
            ui.separator();
            
            // Basic info
            ui.horizontal(|ui| {
                ui.label("Total Rows:");
                ui.label(format!("{}", batch.num_rows()));
            });
            ui.horizontal(|ui| {
                ui.label("Total Columns:");
                ui.label(format!("{}", batch.num_columns()));
            });
            
            ui.add_space(10.0);
            ui.heading("Column Statistics");
            ui.separator();
            
            // Analyze each column
            for (idx, field) in schema.fields().iter().enumerate() {
                ui.collapsing(field.name(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Type:");
                        ui.label(format!("{:?}", field.data_type()));
                    });
                    
                    let column = batch.column(idx);
                    let null_count = column.null_count();
                    let total_count = column.len();
                    let non_null_count = total_count - null_count;
                    
                    ui.horizontal(|ui| {
                        ui.label("Non-null:");
                        ui.label(format!("{} ({:.1}%)", non_null_count, 
                            (non_null_count as f64 / total_count as f64) * 100.0));
                    });
                    
                    if null_count > 0 {
                        ui.horizontal(|ui| {
                            ui.label("Null:");
                            ui.label(format!("{} ({:.1}%)", null_count,
                                (null_count as f64 / total_count as f64) * 100.0));
                        });
                    }
                    
                    // Numeric statistics
                    match field.data_type() {
                        arrow::datatypes::DataType::Float64 => {
                            if let Some(array) = column.as_any().downcast_ref::<Float64Array>() {
                                self.show_numeric_stats_f64(ui, array);
                            }
                        }
                        arrow::datatypes::DataType::Float32 => {
                            if let Some(array) = column.as_any().downcast_ref::<Float32Array>() {
                                self.show_numeric_stats_f32(ui, array);
                            }
                        }
                        arrow::datatypes::DataType::Int64 => {
                            if let Some(array) = column.as_any().downcast_ref::<Int64Array>() {
                                self.show_numeric_stats_i64(ui, array);
                            }
                        }
                        arrow::datatypes::DataType::Int32 => {
                            if let Some(array) = column.as_any().downcast_ref::<Int32Array>() {
                                self.show_numeric_stats_i32(ui, array);
                            }
                        }
                        arrow::datatypes::DataType::Utf8 => {
                            if let Some(array) = column.as_any().downcast_ref::<arrow::array::StringArray>() {
                                self.show_string_stats(ui, array);
                            }
                        }
                        _ => {
                            ui.label("Statistics not available for this type");
                        }
                    }
                    
                    ui.add_space(5.0);
                });
            }
        });
    }
    
    fn show_numeric_stats_f64(&self, ui: &mut Ui, array: &Float64Array) {
        if let Some(min) = arrow::compute::min(array) {
            ui.horizontal(|ui| {
                ui.label("Min:");
                ui.label(format!("{:.2}", min));
            });
        }
        
        if let Some(max) = arrow::compute::max(array) {
            ui.horizontal(|ui| {
                ui.label("Max:");
                ui.label(format!("{:.2}", max));
            });
        }
        
        // Calculate mean
        let mut sum = 0.0;
        let mut count = 0;
        for i in 0..array.len() {
            if array.is_valid(i) {
                sum += array.value(i);
                count += 1;
            }
        }
        
        if count > 0 {
            let mean = sum / count as f64;
            ui.horizontal(|ui| {
                ui.label("Mean:");
                ui.label(format!("{:.2}", mean));
            });
            
            // Calculate std dev
            let mut variance_sum = 0.0;
            for i in 0..array.len() {
                if array.is_valid(i) {
                    let diff = array.value(i) - mean;
                    variance_sum += diff * diff;
                }
            }
            let std_dev = (variance_sum / count as f64).sqrt();
            ui.horizontal(|ui| {
                ui.label("Std Dev:");
                ui.label(format!("{:.2}", std_dev));
            });
        }
    }
    
    fn show_numeric_stats_f32(&self, ui: &mut Ui, array: &Float32Array) {
        if let Some(min) = arrow::compute::min(array) {
            ui.horizontal(|ui| {
                ui.label("Min:");
                ui.label(format!("{:.2}", min));
            });
        }
        
        if let Some(max) = arrow::compute::max(array) {
            ui.horizontal(|ui| {
                ui.label("Max:");
                ui.label(format!("{:.2}", max));
            });
        }
    }
    
    fn show_numeric_stats_i64(&self, ui: &mut Ui, array: &Int64Array) {
        if let Some(min) = arrow::compute::min(array) {
            ui.horizontal(|ui| {
                ui.label("Min:");
                ui.label(min.to_string());
            });
        }
        
        if let Some(max) = arrow::compute::max(array) {
            ui.horizontal(|ui| {
                ui.label("Max:");
                ui.label(max.to_string());
            });
        }
    }
    
    fn show_numeric_stats_i32(&self, ui: &mut Ui, array: &Int32Array) {
        if let Some(min) = arrow::compute::min(array) {
            ui.horizontal(|ui| {
                ui.label("Min:");
                ui.label(min.to_string());
            });
        }
        
        if let Some(max) = arrow::compute::max(array) {
            ui.horizontal(|ui| {
                ui.label("Max:");
                ui.label(max.to_string());
            });
        }
    }
    
    fn show_string_stats(&self, ui: &mut Ui, array: &arrow::array::StringArray) {
        // Count unique values
        let mut unique_values = std::collections::HashMap::new();
        let mut total_length = 0;
        let mut count = 0;
        
        for i in 0..array.len() {
            if array.is_valid(i) {
                let value = array.value(i);
                *unique_values.entry(value.to_string()).or_insert(0) += 1;
                total_length += value.len();
                count += 1;
            }
        }
        
        ui.horizontal(|ui| {
            ui.label("Unique:");
            ui.label(format!("{}", unique_values.len()));
        });
        
        if count > 0 {
            ui.horizontal(|ui| {
                ui.label("Avg Length:");
                ui.label(format!("{:.1}", total_length as f64 / count as f64));
            });
        }
        
        // Show top 5 most common values
        if !unique_values.is_empty() {
            let mut values: Vec<_> = unique_values.into_iter().collect();
            values.sort_by(|a, b| b.1.cmp(&a.1));
            
            ui.add_space(5.0);
            ui.label("Most Common:");
            for (value, count) in values.iter().take(5) {
                ui.horizontal(|ui| {
                    ui.label(format!("  {}: {}", 
                        if value.len() > 20 { &value[..20] } else { value }, 
                        count
                    ));
                });
            }
        }
    }
    
    /// Show column statistics for string columns
    fn show_column_stats(&self, ui: &mut Ui, _column_name: &str, array: &arrow::array::StringArray) {
        // Count unique values and get samples
        let mut unique_values = std::collections::HashMap::new();
        let mut sample_values = Vec::new();
        
        for i in 0..array.len().min(100) {  // Sample first 100 rows
            if array.is_valid(i) {
                let value = array.value(i);
                *unique_values.entry(value.to_string()).or_insert(0) += 1;
                if sample_values.len() < 3 && !sample_values.contains(&value.to_string()) {
                    sample_values.push(value.to_string());
                }
            }
        }
        
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            ui.label(egui::RichText::new(format!("{} unique values", unique_values.len())).size(12.0).color(Color32::from_gray(160)));
            if !sample_values.is_empty() {
                ui.label(egui::RichText::new("•").size(12.0).color(Color32::from_gray(120)));
                ui.label(egui::RichText::new(format!("e.g. {}", sample_values.join(", "))).size(12.0).color(Color32::from_gray(160)));
            }
        });
    }
}

impl eframe::App for FrogApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Only request continuous repaint when needed
        let has_data = !self.viewer_context.data_sources.read().is_empty();
        
        if self.demo_overlay.show {
            ctx.request_repaint();
        }
        
        // ALWAYS show menu bar first, regardless of state
        self.handle_menu(ctx);
        
        // Handle demo overlay (on top of everything)
        if let Some(example) = self.demo_overlay.show(ctx) {
            self.init_demo_example(example);
        }
        
        // Handle file configuration dialog
        let mut config_result = None;
        let mut dialog_closed = false;
        
        if let Some(ref mut dialog) = self.file_config_dialog {
            if let Some(config_manager) = dialog.show_dialog(ctx) {
                config_result = Some(config_manager);
            }
            
            // Check if dialog was closed
            if !dialog.show {
                dialog_closed = true;
            }
            
            // If file config dialog is active and showing, return early
            if !dialog_closed && dialog.show {
                return;
            }
        }
        
        // Handle results outside of the borrow
        if let Some(config_manager) = config_result {
            // Load the configured files
            self.load_configured_files(config_manager);
            self.file_config_dialog = None;
            
            // Set flag to open dashboard builder after loading
            self.open_builder_on_load = true;
        }
        
        if dialog_closed {
            self.file_config_dialog = None;
        }
        
        // Handle keyboard shortcuts
        ctx.input(|i| {
            // Playback controls
            if i.key_pressed(egui::Key::Space) {
                let mut time_control = self.viewer_context.time_control.write();
                time_control.playing = !time_control.playing;
            }
            
            if i.key_pressed(egui::Key::ArrowLeft) {
                let _ = self.viewer_context.navigation.previous();
                self.viewer_context.time_control.write().playing = false;
            }
            
            if i.key_pressed(egui::Key::ArrowRight) {
                let _ = self.viewer_context.navigation.next();
                self.viewer_context.time_control.write().playing = false;
            }
            
            // Speed controls
            if i.key_pressed(egui::Key::Minus) {
                let mut time_control = self.viewer_context.time_control.write();
                time_control.speed = (time_control.speed - 0.5).max(0.1);
            }
            
            if i.key_pressed(egui::Key::PlusEquals) {
                let mut time_control = self.viewer_context.time_control.write();
                time_control.speed = (time_control.speed + 0.5).min(10.0);
            }
            
            // Ctrl+O to open file
            if i.key_pressed(egui::Key::O) && i.modifiers.ctrl {
                if let Some(paths) = rfd::FileDialog::new()
                    .add_filter("Data Files", &["csv", "db", "sqlite", "sqlite3"])
                    .add_filter("CSV Files", &["csv"])
                    .add_filter("SQLite Database", &["db", "sqlite", "sqlite3"])
                    .pick_files()
                {
                    // Create file configuration dialog for all selected files
                    let mut config_manager = dv_data::config::FileConfigManager::new();
                    
                    for path in paths {
                        let file_type = if path.extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| ext.eq_ignore_ascii_case("db") || ext.eq_ignore_ascii_case("sqlite") || ext.eq_ignore_ascii_case("sqlite3"))
                            .unwrap_or(false)
                        {
                            dv_data::config::FileType::Sqlite
                        } else {
                            dv_data::config::FileType::Csv
                        };
                        
                        // Add file to configuration manager
                        let config = dv_data::config::FileConfig::new(path, file_type);
                        config_manager.add_file(config);
                    }
                    
                    // Show file configuration dialog
                    if !config_manager.configs.is_empty() {
                        info!("Creating FileConfigDialog with {} files (Ctrl+O)", config_manager.configs.len());
                        self.file_config_dialog = Some(FileConfigDialog::new(config_manager, self.runtime.handle().clone()));
                        info!("FileConfigDialog created successfully (Ctrl+O)");
                    }
                }
            }
            
            // H key (not Ctrl+H) to go home
            if i.key_pressed(egui::Key::H) && !i.modifiers.ctrl {
                *self.viewer_context.data_sources.write() = HashMap::new();
                self.viewport = Viewport::new();
                self.demo_mode = false;
                self.view_builder = None;
            }
            
            // B key to open view builder
            if i.key_pressed(egui::Key::B) && !i.modifiers.ctrl {
                // Collect all data sources and schemas
                let data_sources = self.viewer_context.data_sources.read();
                let mut sources_with_schemas = Vec::new();
                
                for (id, source) in data_sources.iter() {
                    let schema = self.runtime.block_on(source.schema());
                    sources_with_schemas.push((id.clone(), schema));
                }
                
                if !sources_with_schemas.is_empty() {
                    info!("Creating ViewBuilderDialog from B key");
                    self.view_builder = Some(ViewBuilderDialog::new_multi(sources_with_schemas));
                }
            }
            
            // D key to toggle demo overlay
            if i.key_pressed(egui::Key::D) && !i.modifiers.ctrl {
                self.demo_overlay.show = !self.demo_overlay.show;
            }
            
            // S key to toggle summary stats
            if i.key_pressed(egui::Key::S) && !i.modifiers.ctrl {
                self.show_summary_stats = !self.show_summary_stats;
            }
            
            // R key to reset zoom
            if i.key_pressed(egui::Key::R) && !i.modifiers.ctrl {
                // TODO: Send reset zoom signal to all plots
                ctx.request_repaint();
            }
            
            // Escape to stop playback
            if i.key_pressed(egui::Key::Escape) {
                self.viewer_context.time_control.write().playing = false;
            }
        });
        
        // Handle time control playback
        if self.viewer_context.time_control.read().playing {
            let speed = self.viewer_context.time_control.read().speed;
            let dt = ctx.input(|i| i.stable_dt);
            
            // Calculate frames to advance using accumulator for smooth playback
            let frames_per_second = speed * 15.0; // Reduced from 30.0 to 15.0 for slower playback
            self.frame_accumulator += frames_per_second * dt as f64;
            
            // Only advance when we've accumulated at least one frame
            let frame_advance = self.frame_accumulator as usize;
            if frame_advance > 0 {
                // Subtract the frames we're advancing
                self.frame_accumulator -= frame_advance as f64;
                
                // Advance navigation by calculated frames
                let nav_context = self.viewer_context.navigation.get_context();
                let current_pos = match &nav_context.position {
                    dv_core::navigation::NavigationPosition::Sequential(idx) => *idx,
                    dv_core::navigation::NavigationPosition::Temporal(ts) => *ts as usize,
                    _ => 0,
                };
                
                let new_pos = current_pos + frame_advance;
                let total_rows = nav_context.total_rows;
                
                if new_pos >= total_rows {
                    if self.viewer_context.time_control.read().looping {
                        // Loop back to beginning
                        let _ = self.viewer_context.navigation.seek_to(
                            dv_core::navigation::NavigationPosition::Sequential(0)
                        );
                        // Reset accumulator when looping
                        self.frame_accumulator = 0.0;
                    } else {
                        // Stop at end
                        self.viewer_context.time_control.write().playing = false;
                        self.frame_accumulator = 0.0;
                    }
                } else {
                    // Continue advancing
                    let _ = self.viewer_context.navigation.seek_to(
                        dv_core::navigation::NavigationPosition::Sequential(new_pos)
                    );
                }
            }
            
            ctx.request_repaint();
        } else {
            // Reset accumulator when not playing
            self.frame_accumulator = 0.0;
        }
        
        // Check if we have data loaded
        let has_data = !self.viewer_context.data_sources.read().is_empty();
        
        // Check if we should open dashboard builder automatically
        if self.open_builder_on_load && has_data && self.view_builder.is_none() && !*self.is_loading.read() {
            // Data is loaded, open dashboard builder
            self.open_builder_on_load = false;
            
            // Collect all data sources and schemas
            let data_sources = self.viewer_context.data_sources.read();
            let mut sources_with_schemas = Vec::new();
            
            for (id, source) in data_sources.iter() {
                let schema = self.runtime.block_on(source.schema());
                sources_with_schemas.push((id.clone(), schema));
            }
            
            if !sources_with_schemas.is_empty() {
                info!("Auto-opening ViewBuilderDialog after data load");
                self.view_builder = Some(ViewBuilderDialog::new_multi(sources_with_schemas));
            }
        }
        
        // Handle view builder dialog FIRST - if it's active, show it and return early
        if let Some(ref mut builder) = self.view_builder {
            if let Some((views, nav_mode)) = builder.show_dialog(ctx) {
                // Update navigation mode - get total_rows from actual data source
                let total_rows = {
                    let data_sources = self.viewer_context.data_sources.read();
                    if let Some((_, source)) = data_sources.iter().next() {
                        self.runtime.block_on(source.row_count()).unwrap_or(0)
                    } else {
                        0
                    }
                };
                
                let nav_spec = NavigationSpec {
                    mode: nav_mode,
                    total_rows,
                    temporal_bounds: None,
                    categories: None,
                };
                self.viewer_context.navigation.update_spec(nav_spec);
                
                // Create views
                self.viewport.create_grid_layout(views);
                
                // Clear dialog
                self.view_builder = None;
            }
            
            // If view builder is shown, don't show anything else
            return;
        }
        
        // Show SQLite table selection dialog
        if let Some((path, tables)) = &self.sqlite_table_selection.clone() {
            let mut close_dialog = false;
            let mut selected_table = None;
            
            egui::Window::new("Select SQLite Table")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new(format!("Found {} tables in database:", tables.len()))
                            .size(16.0)
                    );
                    ui.add_space(10.0);
                    
                    // Show table list
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for table in tables {
                                if ui.selectable_label(false, table).clicked() {
                                    selected_table = Some(table.clone());
                                }
                            }
                        });
                    
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            close_dialog = true;
                        }
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if let Some(table) = &selected_table {
                                if ui.button(
                                    egui::RichText::new("Open Table").strong()
                                ).clicked() {
                                    self.open_sqlite_table(path.clone(), table);
                                    close_dialog = true;
                                }
                            }
                        });
                    });
                });
            
            if close_dialog {
                self.sqlite_table_selection = None;
            }
        }
        
        // Show floating summary stats window if enabled
        if self.show_summary_stats {
            self.show_summary_stats_window(ctx);
        }
        
        // Show loading indicator if loading
        if *self.is_loading.read() {
            egui::Window::new("Loading...")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Loading data...");
                    });
                });
        }
        
        if has_data {
            // Show navigation panel at bottom when data is loaded
            let nav_context = self.viewer_context.navigation.get_context();
            let show_navigation = nav_context.total_rows > 1; // Only show if there's something to navigate
            
            if show_navigation {
                egui::TopBottomPanel::bottom("navigation_panel")
                    .resizable(false)
                    .exact_height(42.0)  // Reduced from 50.0
                    .frame(
                        egui::Frame::none()
                            .fill(egui::Color32::from_gray(20))
                            .inner_margin(egui::Margin::symmetric(8.0, 2.0))  // Reduced vertical margin
                            .outer_margin(0.0)
                    )
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            // Create a temporary navigation panel for this frame
                            let mut nav_panel = dv_ui::NavigationPanel::new(
                                self.viewer_context.navigation.clone(),
                                self.viewer_context.time_control.clone()
                            );
                            nav_panel.ui(ui, &self.viewer_context);
                            
                            // Add summary stats toggle at the right side
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.style_mut().spacing.button_padding = Vec2::new(8.0, 4.0);
                                
                                let stats_button = ui.add(
                                    egui::SelectableLabel::new(
                                        self.show_summary_stats,
                                        egui::RichText::new("📊 Summary Stats")
                                            .size(14.0)
                                    )
                                );
                                if stats_button.on_hover_text("Toggle summary statistics window (Press S)").clicked() {
                                    self.show_summary_stats = !self.show_summary_stats;
                                }
                            });
                        });
                    });
            }
            
            // Main content area with viewport
            egui::CentralPanel::default().show(ctx, |ui| {
                // Always show the viewport, even if empty
                self.viewport.ui(ui, &self.viewer_context);
            });
        } else {
            // Show welcome screen when no data is loaded
            egui::CentralPanel::default().show(ctx, |ui| {
                if self.viewport.is_empty() {
                    self.show_welcome_screen(ui);
                } else {
                    self.viewport.ui(ui, &self.viewer_context);
                }
            });
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting F.R.O.G. - Fast, Responsive, Organized Graphics 🐸");
    
    // Create a simple F.R.O.G. icon (32x32 green frog head)
    let icon_data = create_frog_icon();
    
    // Run the app
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])  // Larger, more appropriate default size
            .with_min_inner_size([1024.0, 768.0])  // Reasonable minimum
            .with_position([100.0, 100.0])  // Center better on screen
            .with_maximized(false)
            .with_icon(icon_data), // Set custom F.R.O.G. icon
        default_theme: eframe::Theme::Dark,
        persist_window: false,
        centered: true,  // Center on screen
        ..Default::default()
    };
    
    eframe::run_native(
        "F.R.O.G. - Fast, Responsive, Organized Graphics 🐸",
        options,
        Box::new(|cc| {
            Box::new(FrogApp::new(cc))
        }),
    ).map_err(|e| anyhow::anyhow!("Failed to run app: {}", e))?;
    
    Ok(())
}

/// Create a simple F.R.O.G. icon (32x32 pixel art frog head)
fn create_frog_icon() -> egui::IconData {
    let size = 32;
    let mut rgba = vec![0u8; size * size * 4];
    
    // Define colors
    let green = [92, 140, 97, 255];      // F.R.O.G. brand green
    let dark_green = [70, 110, 75, 255]; // Darker green for shading
    let white = [255, 255, 255, 255];    // White for eyes
    let black = [0, 0, 0, 255];          // Black for pupils
    let _transparent = [0, 0, 0, 0];      // Transparent
    
    // Helper function to set pixel
    let mut set_pixel = |x: usize, y: usize, color: [u8; 4]| {
        if x < size && y < size {
            let idx = (y * size + x) * 4;
            rgba[idx..idx + 4].copy_from_slice(&color);
        }
    };
    
    // Draw frog head (simplified pixel art)
    for y in 0..size {
        for x in 0..size {
            let dx = x as i32 - 16;
            let dy = y as i32 - 16;
            let dist = (dx * dx + dy * dy) as f32;
            
            // Main head circle
            if dist < 14.0 * 14.0 {
                set_pixel(x, y, green);
            }
            
            // Eye bulges
            if ((dx + 6).pow(2) + (dy - 4).pow(2)) < 25 || ((dx - 6).pow(2) + (dy - 4).pow(2)) < 25 {
                set_pixel(x, y, green);
            }
            
            // Eye whites
            if ((dx + 6).pow(2) + (dy - 4).pow(2)) < 16 {
                set_pixel(x, y, white);
            }
            if ((dx - 6).pow(2) + (dy - 4).pow(2)) < 16 {
                set_pixel(x, y, white);
            }
            
            // Eye pupils
            if ((dx + 6).pow(2) + (dy - 4).pow(2)) < 4 {
                set_pixel(x, y, black);
            }
            if ((dx - 6).pow(2) + (dy - 4).pow(2)) < 4 {
                set_pixel(x, y, black);
            }
            
            // Mouth
            if y == 22 && x >= 12 && x <= 20 {
                set_pixel(x, y, dark_green);
            }
        }
    }
    
    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}

// Windows-specific: Hide console window in release builds
#[cfg(all(windows, not(debug_assertions)))]
fn hide_console_window() {
    use winapi::um::wincon::GetConsoleWindow;
    use winapi::um::winuser::{ShowWindow, SW_HIDE};
    
    unsafe {
        let window = GetConsoleWindow();
        if !window.is_null() {
            ShowWindow(window, SW_HIDE);
        }
    }
}

#[cfg(all(windows, not(debug_assertions)))]
#[no_mangle]
pub extern "system" fn mainCRTStartup() {
    hide_console_window();
    std::process::exit(main().map(|_| 0).unwrap_or(1));
} 