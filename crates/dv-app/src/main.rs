//! Main application entry point

use std::sync::Arc;
use eframe::egui::{self, Context, Ui};
use anyhow::Result;
use parking_lot::RwLock;
use tracing::{info, error};

use dv_ui::{Theme, AppShell, NavigationPanel};
use dv_views::{
    Viewport, ViewerContext, TimeControl, HoveredData, FrameTime,
    TimeSeriesView, TableView, ScatterPlotView, SpaceViewId, SpaceView
};
use dv_core::{
    navigation::{NavigationEngine, NavigationMode},
    data::DataSource,
};
use dv_data::sources::{csv_source::CsvSource, sqlite_source::SqliteSource};

mod demo;
mod create_sample_db;

/// Main application state
struct DataVisualizerApp {
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
    
    /// Flag to auto-create views after data load
    should_auto_create_views: Arc<std::sync::atomic::AtomicBool>,
}

impl DataVisualizerApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Setup custom theme
        dv_ui::apply_theme(&cc.egui_ctx, &Theme::default());
        
        // Initialize tokio runtime
        let runtime = tokio::runtime::Runtime::new().unwrap();
        
        // Create shared viewer context
        let viewer_context = Arc::new(ViewerContext {
            data_source: Arc::new(RwLock::new(None)),
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
        
        // Create flag to auto-create views after data load
        let should_auto_create_views = Arc::new(std::sync::atomic::AtomicBool::new(true));
        
        Self {
            viewport,
            viewer_context,
            _navigation_panel: navigation_panel,
            _app_shell: app_shell,
            _theme: Theme::default(),
            runtime,
            demo_mode: false,
            egui_ctx: cc.egui_ctx.clone(),
            should_auto_create_views,
        }
    }
    
    /// Initialize demo mode with multiple synchronized plots
    fn init_demo_mode(&mut self) {
        use crate::demo::DemoDataSource;
        
        // Create demo data source
        let demo_source = Box::new(DemoDataSource::new());
        
        // Update navigation spec
        if let Ok(spec) = self.runtime.block_on(demo_source.navigation_spec()) {
            self.viewer_context.navigation.update_spec(spec);
        }
        
        // Set it as the current data source
        *self.viewer_context.data_source.write() = Some(demo_source);
        
        // Clear existing views
        self.viewport = Viewport::new();
        
        // Create a curated set of views to showcase features
        let mut views: Vec<Box<dyn SpaceView>> = Vec::new();
        
        // 1. Assembly Line Overview - The main showcase!
        let assembly_id = SpaceViewId::new("Assembly Line");
        let mut assembly_view = TimeSeriesView::new(assembly_id, "Assembly Line Performance".to_string());
        assembly_view.config.x_column = Some("time".to_string());
        assembly_view.config.y_columns = vec![
            "station_1_throughput".to_string(),
            "station_2_throughput".to_string(),
            "station_3_throughput".to_string(),
        ];
        assembly_view.config.show_legend = true;
        assembly_view.config.show_grid = true;
        views.push(Box::new(assembly_view));
        
        // 2. Manufacturing Efficiency
        let efficiency_id = SpaceViewId::new("Efficiency");
        let mut efficiency_view = TimeSeriesView::new(efficiency_id, "Manufacturing Efficiency".to_string());
        efficiency_view.config.x_column = Some("time".to_string());
        efficiency_view.config.y_columns = vec![
            "efficiency".to_string(),
            "defect_rate".to_string(),
            "buffer_level".to_string(),
        ];
        views.push(Box::new(efficiency_view));
        
        // 3. System Performance Metrics
        let performance_id = SpaceViewId::new("Performance");
        let mut performance_view = TimeSeriesView::new(performance_id, "System Performance".to_string());
        performance_view.config.x_column = Some("time".to_string());
        performance_view.config.y_columns = vec![
            "cpu_usage".to_string(),
            "memory_usage".to_string(),
            "error_rate".to_string(),
        ];
        views.push(Box::new(performance_view));
        
        // 4. Network Metrics
        let network_id = SpaceViewId::new("Network");
        let mut network_view = TimeSeriesView::new(network_id, "Network Performance".to_string());
        network_view.config.x_column = Some("time".to_string());
        network_view.config.y_columns = vec![
            "network_latency".to_string(),
            "requests_per_sec".to_string(),
        ];
        views.push(Box::new(network_view));
        
        // 5. Business Metrics
        let business_id = SpaceViewId::new("Business");
        let mut business_view = TimeSeriesView::new(business_id, "Business Metrics".to_string());
        business_view.config.x_column = Some("time".to_string());
        business_view.config.y_columns = vec![
            "revenue".to_string(),
            "cost".to_string(),
            "profit".to_string(),
        ];
        views.push(Box::new(business_view));
        
        // 6. Physics Simulation - Position (Scatter Plot)
        let physics_id = SpaceViewId::new("Physics");
        let mut physics_view = ScatterPlotView::new(physics_id, "Orbital Motion".to_string());
        physics_view.config.x_column = "position_x".to_string();
        physics_view.config.y_column = "position_y".to_string();
        physics_view.config.show_grid = true;
        physics_view.config.point_radius = 4.0;
        views.push(Box::new(physics_view));
        
        // 7. Signal Analysis
        let signals_id = SpaceViewId::new("Signals");
        let mut signals_view = TimeSeriesView::new(signals_id, "Signal Decomposition".to_string());
        signals_view.config.x_column = Some("time".to_string());
        signals_view.config.y_columns = vec![
            "combined".to_string(),
            "trend".to_string(),
            "seasonal".to_string(),
            "noise".to_string(),
        ];
        views.push(Box::new(signals_view));
        
        // 8. Data Table for detailed inspection
        let table_id = SpaceViewId::new("Inspector");
        let table_view = TableView::new(table_id, "Data Inspector".to_string());
        views.push(Box::new(table_view));
        
        // Create a nice grid layout
        self.viewport.create_grid_layout(views);
        
        // Set demo mode flag
        self.demo_mode = true;
        
        info!("Demo mode initialized with assembly line and manufacturing analytics");
    }
    
    /// Open a CSV file
    fn open_csv_file(&mut self, path: std::path::PathBuf) {
        info!("Opening CSV file: {:?}", path);
        
        let source_future = CsvSource::new(path.clone());
        
        let ctx = self.egui_ctx.clone();
        let viewer_context = self.viewer_context.clone();
        let runtime = self.runtime.handle().clone();
        let should_auto_create_views = self.should_auto_create_views.clone();
        
        runtime.spawn(async move {
            match source_future.await {
                Ok(source) => {
                    // Update navigation spec
                    if let Ok(spec) = source.navigation_spec().await {
                        viewer_context.navigation.update_spec(spec);
                    }
                    
                    // Update data source
                    *viewer_context.data_source.write() = Some(Box::new(source) as Box<dyn DataSource>);
                    
                    // Set flag for auto-creating views
                    should_auto_create_views.store(true, std::sync::atomic::Ordering::SeqCst);
                    
                    ctx.request_repaint();
                }
                Err(e) => {
                    error!("Failed to open CSV file: {}", e);
                }
            }
        });
    }
    
    /// Open a SQLite database file
    fn open_sqlite_file(&mut self, path: std::path::PathBuf) {
        // For simplicity, open the first table found
        // In a real app, we'd show a table selection dialog
        if let Ok(conn) = rusqlite::Connection::open(&path) {
            if let Ok(mut stmt) = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'") {
                if let Ok(tables) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                    if let Some(Ok(table_name)) = tables.into_iter().next() {
                        self.open_sqlite_table(path, &table_name);
                    }
                }
            }
        }
    }
    
    /// Open a specific SQLite table
    fn open_sqlite_table<P: AsRef<std::path::Path>>(&mut self, path: P, table_name: &str) {
        let path = path.as_ref();
        info!("Opening SQLite table: {} from {:?}", table_name, path);
        
        let source_future = SqliteSource::new(path.to_path_buf(), table_name.to_string());
        
        let ctx = self.egui_ctx.clone();
        let viewer_context = self.viewer_context.clone();
        let runtime = self.runtime.handle().clone();
        let should_auto_create_views = self.should_auto_create_views.clone();
        
        runtime.spawn(async move {
            match source_future.await {
                Ok(source) => {
                    // Update navigation spec
                    if let Ok(spec) = source.navigation_spec().await {
                        viewer_context.navigation.update_spec(spec);
                    }
                    
                    // Update data source
                    *viewer_context.data_source.write() = Some(Box::new(source) as Box<dyn DataSource>);
                    
                    // Set flag for auto-creating views
                    should_auto_create_views.store(true, std::sync::atomic::Ordering::SeqCst);
                    
                    ctx.request_repaint();
                }
                Err(e) => {
                    error!("Failed to open SQLite table: {}", e);
                }
            }
        });
    }
    
    /// Automatically create views based on data schema
    fn auto_create_views(&mut self) {
        if let Some(data_source) = &*self.viewer_context.data_source.read() {
            let schema = self.runtime.block_on(data_source.schema());
            
            // Clear existing views
            self.viewport = Viewport::new();
            
            // Categorize columns
            let mut numeric_columns = Vec::new();
            let mut time_columns = Vec::new();
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
                            time_columns.push(field.name().clone());
                        } else {
                            categorical_columns.push(field.name().clone());
                        }
                    }
                    arrow::datatypes::DataType::Date32 |
                    arrow::datatypes::DataType::Date64 |
                    arrow::datatypes::DataType::Timestamp(_, _) => {
                        time_columns.push(field.name().clone());
                    }
                    _ => {}
                }
            }
            
            // Determine the best X axis (prefer time columns)
            let x_column = time_columns.first().cloned();
            
            let mut views: Vec<Box<dyn SpaceView>> = Vec::new();
            
            // Always create a table view first for data exploration
            let table_id = SpaceViewId::new("Data Table");
            let table_view = TableView::new(table_id, "Data Table".to_string());
            views.push(Box::new(table_view));
            
            // Create views based on column types
            if !numeric_columns.is_empty() {
                // Group numeric columns by similarity
                let mut groups: Vec<(String, Vec<String>)> = Vec::new();
                
                // Try to group related columns (e.g., temperature/humidity, open/high/low/close)
                let mut remaining = numeric_columns.clone();
                
                // Group OHLC data
                if remaining.iter().any(|c| c.to_lowercase().contains("open")) {
                    let ohlc_group: Vec<_> = remaining.iter()
                        .filter(|c| {
                            let lower = c.to_lowercase();
                            lower.contains("open") || lower.contains("high") || 
                            lower.contains("low") || lower.contains("close")
                        })
                        .cloned()
                        .collect();
                    if !ohlc_group.is_empty() {
                        groups.push(("OHLC Price Data".to_string(), ohlc_group.clone()));
                        remaining.retain(|c| !ohlc_group.contains(c));
                    }
                }
                
                // Group volume separately
                if let Some(volume_col) = remaining.iter().find(|c| c.to_lowercase().contains("volume")).cloned() {
                    groups.push(("Volume".to_string(), vec![volume_col.clone()]));
                    remaining.retain(|c| c != &volume_col);
                }
                
                // Group financial metrics
                let financial_keywords = ["revenue", "profit", "cost", "margin", "sales"];
                let financial_group: Vec<_> = remaining.iter()
                    .filter(|c| financial_keywords.iter().any(|&kw| c.to_lowercase().contains(kw)))
                    .cloned()
                    .collect();
                if !financial_group.is_empty() {
                    groups.push(("Financial Metrics".to_string(), financial_group.clone()));
                    remaining.retain(|c| !financial_group.contains(c));
                }
                
                // Group sensor data
                let sensor_keywords = ["temperature", "humidity", "pressure", "sensor"];
                let sensor_group: Vec<_> = remaining.iter()
                    .filter(|c| sensor_keywords.iter().any(|&kw| c.to_lowercase().contains(kw)))
                    .cloned()
                    .collect();
                if !sensor_group.is_empty() {
                    groups.push(("Sensor Data".to_string(), sensor_group.clone()));
                    remaining.retain(|c| !sensor_group.contains(c));
                }
                
                // Group network/performance metrics
                let network_keywords = ["bandwidth", "latency", "cpu", "memory", "throughput", "requests"];
                let network_group: Vec<_> = remaining.iter()
                    .filter(|c| network_keywords.iter().any(|&kw| c.to_lowercase().contains(kw)))
                    .cloned()
                    .collect();
                if !network_group.is_empty() {
                    groups.push(("Performance Metrics".to_string(), network_group.clone()));
                    remaining.retain(|c| !network_group.contains(c));
                }
                
                // Put remaining columns together if there are just a few
                if !remaining.is_empty() && remaining.len() <= 4 {
                    groups.push(("Metrics".to_string(), remaining));
                } else {
                    // Otherwise create individual views
                    for col in remaining {
                        groups.push((col.clone(), vec![col]));
                    }
                }
                
                // Create time series views for each group
                for (title, columns) in groups {
                    let id = SpaceViewId::new(&title);
                    let mut view = TimeSeriesView::new(id, title);
                    view.config.y_columns = columns;
                    view.config.x_column = x_column.clone();
                    view.config.show_legend = true;
                    view.config.show_grid = true;
                    
                    views.push(Box::new(view));
                }
                
                // Create scatter plots for interesting pairs
                if numeric_columns.len() >= 2 {
                    // Look for X/Y pairs
                    let x_candidates: Vec<_> = numeric_columns.iter()
                        .filter(|c| c.to_lowercase().contains("x") || c.to_lowercase().contains("latitude"))
                        .cloned()
                        .collect();
                    let y_candidates: Vec<_> = numeric_columns.iter()
                        .filter(|c| c.to_lowercase().contains("y") || c.to_lowercase().contains("longitude"))
                        .cloned()
                        .collect();
                    
                    if !x_candidates.is_empty() && !y_candidates.is_empty() {
                        let id = SpaceViewId::new("Scatter Plot");
                        let mut scatter_view = ScatterPlotView::new(id, "Scatter Plot".to_string());
                        scatter_view.config.x_column = x_candidates[0].clone();
                        scatter_view.config.y_column = y_candidates[0].clone();
                        views.push(Box::new(scatter_view));
                    }
                }
            }
            
            // Create appropriate layout based on number of views
            if !views.is_empty() {
                self.viewport.create_grid_layout(views);
            }
        }
    }
    
    /// Handle menu actions
    fn handle_menu(&mut self) {
        let ctx = self.egui_ctx.clone();
        egui::TopBottomPanel::top("menu_bar").show(&ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open CSV...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("CSV Files", &["csv"])
                            .pick_file()
                        {
                            self.open_csv_file(path);
                        }
                        ui.close_menu();
                    }
                    
                    if ui.button("Open SQLite...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("SQLite Database", &["db", "sqlite", "sqlite3"])
                            .pick_file()
                        {
                            self.open_sqlite_file(path);
                        }
                        ui.close_menu();
                    }
                    
                    ui.separator();
                    
                    ui.menu_button("Open Examples", |ui| {
                        ui.heading("📊 Time Series Examples");
                        
                        if ui.button("📈 Sales Data").on_hover_text("Daily sales data with revenue and profit trends").clicked() {
                            self.open_csv_file(std::path::PathBuf::from("data/sales_data.csv"));
                            ui.close_menu();
                        }
                        
                        if ui.button("🌡️ Sensor Readings").on_hover_text("Temperature, humidity and pressure sensor data").clicked() {
                            self.open_csv_file(std::path::PathBuf::from("data/sensor_readings.csv"));
                            ui.close_menu();
                        }
                        
                        if ui.button("💹 Stock Prices").on_hover_text("OHLCV stock market data").clicked() {
                            self.open_csv_file(std::path::PathBuf::from("data/stock_prices.csv"));
                            ui.close_menu();
                        }
                        
                        ui.separator();
                        ui.heading("🏭 Advanced Examples");
                        
                        if ui.button("⚙️ Assembly Line").on_hover_text("Manufacturing assembly line with multiple stations").clicked() {
                            self.open_csv_file(std::path::PathBuf::from("data/assembly_line.csv"));
                            ui.close_menu();
                        }
                        
                        if ui.button("🌐 Network Traffic").on_hover_text("Server performance and network monitoring data").clicked() {
                            self.open_csv_file(std::path::PathBuf::from("data/network_traffic.csv"));
                            ui.close_menu();
                        }
                        
                        ui.separator();
                        ui.heading("🗄️ Database Examples");
                        
                        if ui.button("🏭 Generate Analytics DB").on_hover_text("Create a sample SQLite database with multiple tables").clicked() {
                            self.generate_sample_database();
                            ui.close_menu();
                        }
                        
                        if std::path::Path::new("data/sample_analytics.db").exists() {
                            ui.separator();
                            
                            if ui.button("📡 Sensor Telemetry").on_hover_text("Real-time sensor data from IoT devices").clicked() {
                                self.open_sqlite_table("data/sample_analytics.db", "sensor_telemetry");
                                ui.close_menu();
                            }
                            
                            if ui.button("💳 Transactions").on_hover_text("Financial transaction history").clicked() {
                                self.open_sqlite_table("data/sample_analytics.db", "transactions");
                                ui.close_menu();
                            }
                            
                            if ui.button("⚙️ Production Metrics").on_hover_text("Manufacturing production line metrics").clicked() {
                                self.open_sqlite_table("data/sample_analytics.db", "production_metrics");
                                ui.close_menu();
                            }
                        }
                    });
                    
                    ui.separator();
                    
                    if ui.button("🎮 Demo Mode").on_hover_text("Interactive demonstration with synthetic data").clicked() {
                        self.init_demo_mode();
                        ui.close_menu();
                    }
                    
                    ui.separator();
                    
                    if ui.button("Exit").clicked() {
                        self.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                
                ui.menu_button("View", |ui| {
                    if ui.button("Auto-create Views").clicked() {
                        self.auto_create_views();
                        ui.close_menu();
                    }
                    
                    ui.separator();
                    
                    if ui.button("Clear All Views").clicked() {
                        self.viewport = Viewport::new();
                        ui.close_menu();
                    }
                });
            });
        });
    }
    
    /// Generate sample SQLite database
    fn generate_sample_database(&mut self) {
        use crate::create_sample_db::create_sample_database;
        
        match create_sample_database() {
            Ok(_) => {
                info!("Sample database created successfully");
                // Open the sensor telemetry table by default
                self.open_sqlite_table("data/sample_analytics.db", "sensor_telemetry");
            }
            Err(e) => {
                error!("Failed to create sample database: {}", e);
            }
        }
    }
    
    /// Show welcome screen
    fn show_welcome_screen(&mut self, ui: &mut Ui) {
        ui.style_mut().visuals.widgets.noninteractive.bg_fill = egui::Color32::from_gray(25);
        
        let available_size = ui.available_size();
        let center = egui::pos2(available_size.x / 2.0, available_size.y / 2.0);
        
        // Draw a subtle grid pattern in the background
        let painter = ui.painter();
        let grid_color = egui::Color32::from_gray(30);
        let grid_spacing = 50.0;
        
        for i in 0..(available_size.x / grid_spacing) as i32 {
            let x = i as f32 * grid_spacing;
            painter.line_segment(
                [egui::pos2(x, 0.0), egui::pos2(x, available_size.y)],
                egui::Stroke::new(1.0, grid_color)
            );
        }
        
        for i in 0..(available_size.y / grid_spacing) as i32 {
            let y = i as f32 * grid_spacing;
            painter.line_segment(
                [egui::pos2(0.0, y), egui::pos2(available_size.x, y)],
                egui::Stroke::new(1.0, grid_color)
            );
        }
        
        // Welcome content
        ui.allocate_ui_at_rect(
            egui::Rect::from_center_size(center, egui::vec2(600.0, 400.0)),
            |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    
                    // Logo/title
                    ui.heading(egui::RichText::new("📊 Data Visualizer").size(48.0).strong());
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("High-performance visualization for tabular data").size(18.0).color(egui::Color32::from_gray(180)));
                    
                    ui.add_space(40.0);
                    
                    // Quick start options
                    ui.label(egui::RichText::new("Get Started:").size(16.0).strong());
                    ui.add_space(20.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button(egui::RichText::new("🎮 Demo Mode").size(16.0))
                            .on_hover_text("Explore with synthetic data")
                            .clicked() 
                        {
                            self.init_demo_mode();
                        }
                        
                        if ui.button(egui::RichText::new("📁 Open File").size(16.0))
                            .on_hover_text("Load CSV or SQLite data")
                            .clicked() 
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Data Files", &["csv", "db", "sqlite", "sqlite3"])
                                .pick_file()
                            {
                                if path.extension().map_or(false, |ext| ext == "csv") {
                                    self.open_csv_file(path);
                                } else {
                                    self.open_sqlite_file(path);
                                }
                            }
                        }
                    });
                    
                    ui.add_space(30.0);
                    
                    // Feature highlights
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("✨ Features:").size(14.0));
                    });
                    ui.add_space(10.0);
                    
                    ui.horizontal_wrapped(|ui| {
                        for feature in &[
                            "🚀 60 FPS Performance",
                            "🎯 Draggable Panels",
                            "📈 Time Series Plots",
                            "🔄 Synchronized Views",
                            "📊 Auto Layout",
                            "🌙 Dark Theme",
                        ] {
                            ui.label(egui::RichText::new(*feature).size(12.0).color(egui::Color32::from_gray(160)));
                            ui.add_space(10.0);
                        }
                    });
                    
                    ui.add_space(30.0);
                    
                    // Drag and drop hint
                    ui.label(egui::RichText::new("💡 Tip: Drag and drop CSV or SQLite files here").size(12.0).color(egui::Color32::from_gray(120)));
                });
            }
        );
    }
}

impl eframe::App for DataVisualizerApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Check if we need to auto-create views after async data loading
        if self.should_auto_create_views.load(std::sync::atomic::Ordering::SeqCst) {
            if self.viewer_context.data_source.read().is_some() {
                self.auto_create_views();
                self.should_auto_create_views.store(false, std::sync::atomic::Ordering::SeqCst);
            }
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
        });
        
        // Handle time control playback
        if self.viewer_context.time_control.read().playing {
            let speed = self.viewer_context.time_control.read().speed;
            let frame_advance = (speed * 0.016) as usize; // 60 FPS
            let _ = self.viewer_context.navigation.advance(frame_advance);
            ctx.request_repaint();
        }
        
        // Menu bar
        self.handle_menu();
        
        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            // Show welcome screen if no data is loaded
            if self.viewer_context.data_source.read().is_none() {
                self.show_welcome_screen(ui);
            } else {
                // Show viewport
                self.viewport.ui(ui, &self.viewer_context);
            }
        });
    }
}

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting Data Visualizer with draggable panels");
    
    // Run the app
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])  // More reasonable default size
            .with_min_inner_size([800.0, 600.0])
            .with_position([50.0, 50.0])  // Force window to appear at top-left area
            .with_maximized(false),  // Start in normal window state
        default_theme: eframe::Theme::Dark,
        persist_window: false,  // Don't save window position to avoid off-screen issues
        ..Default::default()
    };
    
    eframe::run_native(
        "Data Visualizer - Rerun-inspired",
        options,
        Box::new(|cc| {
            Box::new(DataVisualizerApp::new(cc))
        }),
    ).map_err(|e| anyhow::anyhow!("Failed to run app: {}", e))?;
    
    Ok(())
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