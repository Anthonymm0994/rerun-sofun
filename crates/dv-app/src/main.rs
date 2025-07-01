//! Main application entry point

use anyhow::Result;
use eframe::egui::{self, Ui, Color32, Vec2, Stroke, Rounding};
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, error};

use dv_ui::{Theme, AppShell, NavigationPanel};
use dv_views::{
    Viewport, ViewerContext, TimeControl, HoveredData, FrameTime,
    TimeSeriesView, TableView, ScatterPlotView, SpaceView
};
use dv_core::{
    navigation::{NavigationEngine, NavigationMode, NavigationSpec},
    data::DataSource,
};
use dv_data::sources::{csv_source::CsvSource, sqlite_source::SqliteSource};

mod demo;
mod create_sample_db;
mod view_builder;
mod frog_animation;
mod demo_overlay;

use view_builder::ViewBuilderDialog;
use frog_animation::FrogMascot;
use demo_overlay::DemoOverlay;
use uuid;

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
fn create_assembly_line_views() -> Vec<Box<dyn SpaceView>> {
    let mut views: Vec<Box<dyn SpaceView>> = Vec::new();
    
    // 1. Assembly Line Performance
    let assembly_id = uuid::Uuid::new_v4();
    let mut assembly_view = TimeSeriesView::new(assembly_id, "Assembly Line Performance".to_string());
    assembly_view.config.x_column = Some("time".to_string());
    assembly_view.config.y_columns = vec![
        "station_1_throughput".to_string(),
        "station_2_throughput".to_string(),
        "station_3_throughput".to_string(),
    ];
    views.push(Box::new(assembly_view));
    
    // 2. Manufacturing Efficiency
    let efficiency_id = uuid::Uuid::new_v4();
    let mut efficiency_view = TimeSeriesView::new(efficiency_id, "Manufacturing Efficiency".to_string());
    efficiency_view.config.x_column = Some("time".to_string());
    efficiency_view.config.y_columns = vec![
        "efficiency".to_string(),
        "defect_rate".to_string(),
        "buffer_level".to_string(),
    ];
    views.push(Box::new(efficiency_view));
    
    // 3. System Performance
    let performance_id = uuid::Uuid::new_v4();
    let mut performance_view = TimeSeriesView::new(performance_id, "System Performance".to_string());
    performance_view.config.x_column = Some("time".to_string());
    performance_view.config.y_columns = vec![
        "cpu_usage".to_string(),
        "memory_usage".to_string(),
    ];
    views.push(Box::new(performance_view));
    
    // 4. Data Table
    let table_id = uuid::Uuid::new_v4();
    let table_view = TableView::new(table_id, "Data Inspector".to_string());
    views.push(Box::new(table_view));
    
    views
}

/// Create views for sensor network demo
fn create_sensor_network_views() -> Vec<Box<dyn SpaceView>> {
    let mut views: Vec<Box<dyn SpaceView>> = Vec::new();
    
    // 1. Environmental Sensors
    let env_id = uuid::Uuid::new_v4();
    let mut env_view = TimeSeriesView::new(env_id, "Environmental Sensors".to_string());
    env_view.config.x_column = Some("time".to_string());
    env_view.config.y_columns = vec![
        "cpu_usage".to_string(),
        "memory_usage".to_string(),
        "error_rate".to_string(),
    ];
    views.push(Box::new(env_view));
    
    // 2. Network Performance
    let network_id = uuid::Uuid::new_v4();
    let mut network_view = TimeSeriesView::new(network_id, "Network Performance".to_string());
    network_view.config.x_column = Some("time".to_string());
    network_view.config.y_columns = vec![
        "network_latency".to_string(),
        "requests_per_sec".to_string(),
    ];
    views.push(Box::new(network_view));
    
    // 3. Position Scatter
    let position_id = uuid::Uuid::new_v4();
    let mut position_view = ScatterPlotView::new(position_id, "Sensor Positions".to_string());
    position_view.config.x_column = "position_x".to_string();
    position_view.config.y_column = "position_y".to_string();
    views.push(Box::new(position_view));
    
    views
}

/// Create views for financial dashboard demo
fn create_financial_views() -> Vec<Box<dyn SpaceView>> {
    let mut views: Vec<Box<dyn SpaceView>> = Vec::new();
    
    // 1. Business Metrics
    let business_id = uuid::Uuid::new_v4();
    let mut business_view = TimeSeriesView::new(business_id, "Business Metrics".to_string());
    business_view.config.x_column = Some("time".to_string());
    business_view.config.y_columns = vec![
        "revenue".to_string(),
        "cost".to_string(),
        "profit".to_string(),
    ];
    views.push(Box::new(business_view));
    
    // 2. Market Trends
    let market_id = uuid::Uuid::new_v4();
    let mut market_view = TimeSeriesView::new(market_id, "Market Trends".to_string());
    market_view.config.x_column = Some("time".to_string());
    market_view.config.y_columns = vec![
        "revenue".to_string(),
        "margin".to_string(),
    ];
    views.push(Box::new(market_view));
    
    views
}

/// Create views for signal analysis demo
fn create_signal_analysis_views() -> Vec<Box<dyn SpaceView>> {
    let mut views: Vec<Box<dyn SpaceView>> = Vec::new();
    
    // Signal Decomposition
    let signals_id = uuid::Uuid::new_v4();
    let mut signals_view = TimeSeriesView::new(signals_id, "Signal Decomposition".to_string());
    signals_view.config.x_column = Some("time".to_string());
    signals_view.config.y_columns = vec![
        "combined".to_string(),
        "trend".to_string(),
        "seasonal".to_string(),
        "noise".to_string(),
    ];
    views.push(Box::new(signals_view));
    
    // Frequency Analysis
    let freq_id = uuid::Uuid::new_v4();
    let mut freq_view = TimeSeriesView::new(freq_id, "Frequency Components".to_string());
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
}

impl FrogApp {
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
        }
    }
    
    /// Initialize demo mode with a specific example
    fn init_demo_example(&mut self, example: DemoExample) {
        use crate::demo::DemoDataSource;
        
        // Clear any existing state when switching demos
        *self.viewer_context.data_source.write() = None;
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
        *self.viewer_context.data_source.write() = Some(demo_source);
        
        // Create appropriate views based on the example
        let views = match example {
            DemoExample::AssemblyLine => create_assembly_line_views(),
            DemoExample::SensorNetwork => create_sensor_network_views(),
            DemoExample::FinancialDashboard => create_financial_views(),
            DemoExample::SignalAnalysis => create_signal_analysis_views(),
        };
        
        // Create layout
        self.viewport.create_grid_layout(views);
        
        info!("Demo mode initialized with {} example", example.name());
    }
    
    /// Open a CSV file
    fn open_csv_file(&mut self, path: std::path::PathBuf) {
        info!("Opening CSV file: {:?}", path);
        
        let source_future = CsvSource::new(path.clone());
        
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
                    *viewer_context.data_source.write() = Some(Box::new(source) as Box<dyn DataSource>);
                    
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
        
        runtime.spawn(async move {
            match source_future.await {
                Ok(source) => {
                    // Update navigation spec
                    if let Ok(spec) = source.navigation_spec().await {
                        viewer_context.navigation.update_spec(spec);
                    }
                    
                    // Update data source
                    *viewer_context.data_source.write() = Some(Box::new(source) as Box<dyn DataSource>);
                    
                    ctx.request_repaint();
                }
                Err(e) => {
                    error!("Failed to open SQLite table: {}", e);
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
                            ).on_hover_text("Return to welcome screen (Ctrl+H)").clicked() {
                                *self.viewer_context.data_source.write() = None;
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
                                egui::RichText::new("📁 Open CSV...").color(Color32::WHITE)
                            ).clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("CSV Files", &["csv"])
                                    .pick_file()
                                {
                                    self.open_csv_file(path);
                                }
                                ui.close_menu();
                            }
                            
                            if ui.button(
                                egui::RichText::new("🗄️ Open SQLite...").color(Color32::WHITE)
                            ).clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("SQLite Database", &["db", "sqlite", "sqlite3"])
                                    .pick_file()
                                {
                                    self.open_sqlite_file(path);
                                }
                                ui.close_menu();
                            }
                            
                            ui.separator();
                            
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
                            if ui.button(
                                egui::RichText::new("🔍 Create Custom Views...").color(Color32::WHITE)
                            ).on_hover_text("Choose what to explore from the loaded data").clicked() {
                                if let Some(data_source) = &*self.viewer_context.data_source.read() {
                                    let schema = self.runtime.block_on(data_source.schema());
                                    self.view_builder = Some(ViewBuilderDialog::new(schema));
                                }
                                ui.close_menu();
                            }
                            
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
                            
                            ui.separator();
                            
                            if ui.button(
                                egui::RichText::new("🗑️ Clear All Views").color(Color32::WHITE)
                            ).clicked() {
                                self.viewport = Viewport::new();
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
                                ui.label("Ctrl+H:");
                                ui.label("Go home");
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
                        if let Some(data_source) = &*self.viewer_context.data_source.read() {
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
    
    /// Generate sample SQLite database
    fn _generate_sample_database(&mut self) {
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
        // Soft gradient background
        let rect = ui.clip_rect();
        let painter = ui.painter();
        
        // Subtle animated gradient
        let time = ui.input(|i| i.time) as f32;
        let _gradient_offset = (time * 0.5).sin() * 0.05;
        
        painter.rect_filled(
            rect,
            Rounding::ZERO,
            Color32::from_rgb(15, 20, 25)
        );
        
        // Add some subtle animated circles in background
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
        
        // Center content
        let available_size = ui.available_size();
        let center = egui::pos2(available_size.x / 2.0, available_size.y / 2.0);
        
        ui.allocate_ui_at_rect(
            egui::Rect::from_center_size(center, egui::vec2(800.0, 600.0)),
            |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    
                    // Animated frog mascot
                    let mascot_response = self.frog_mascot.ui(ui, 120.0);
                    if mascot_response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    
                    ui.add_space(20.0);
                    
                    // Title with subtle animation
                    let title_offset = (time * 2.0).sin() * 2.0;
                    ui.add_space(title_offset.max(0.0));
                    
                    ui.label(
                        egui::RichText::new("F.R.O.G.")
                            .size(56.0)
                            .strong()
                            .color(Color32::from_rgb(92, 140, 97))
                    );
                    
                    ui.add_space(8.0);
                    
                    // Subtitle with animated color for better visibility
                    let subtitle_color_phase = (time * 1.2).sin() + 1.0;
                    let subtitle_color = Color32::from_rgb(
                        (150.0 + subtitle_color_phase * 50.0) as u8,
                        (180.0 + subtitle_color_phase * 40.0) as u8,
                        (200.0 + subtitle_color_phase * 30.0) as u8,
                    );
                    
                    ui.label(
                        egui::RichText::new("Flexible Rust Overlay for Graphs")
                            .size(18.0)
                            .color(subtitle_color)
                    );
                    
                    ui.add_space(50.0);
                    
                    // Action buttons with better styling
                    ui.horizontal(|ui| {
                        ui.add_space((ui.available_width() - 400.0) / 2.0);
                        
                        // Demo Mode button - now functional!
                        let demo_button = egui::Button::new(
                            egui::RichText::new("  🎮  Demo Mode  ")
                                .size(18.0)
                                .color(Color32::WHITE)
                        )
                        .fill(Color32::from_rgb(92, 140, 97))
                        .rounding(8.0)
                        .min_size(Vec2::new(180.0, 50.0));
                        
                        if ui.add(demo_button)
                            .on_hover_text("Explore curated example datasets")
                            .clicked()
                        {
                            self.demo_overlay.show = true;
                        }
                        
                        ui.add_space(40.0);
                        
                        // Open File button with better design
                        let file_button = egui::Button::new(
                            egui::RichText::new("  📁  Open File  ")
                                .size(18.0)
                                .color(Color32::WHITE)
                        )
                        .fill(Color32::from_rgb(70, 100, 140))
                        .rounding(8.0)
                        .min_size(Vec2::new(180.0, 50.0));
                        
                        if ui.add(file_button)
                            .on_hover_text("Load your CSV or SQLite data")
                            .clicked()
                        {
                            // Use synchronous file picker - async version was causing crashes
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
                    
                    ui.add_space(60.0);
                    
                    // Subtle animated tagline
                    let tagline_alpha = ((time * 1.5).sin() + 1.0) * 0.5 * 0.8 + 0.2;
                    ui.label(
                        egui::RichText::new("🐸 Hop through your data with ease")
                            .size(14.0)
                            .color(Color32::from_gray((tagline_alpha * 160.0) as u8))
                            .italics()
                    );
                });
            }
        );
    }
}

impl eframe::App for FrogApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request continuous repaint for smooth animation
        ctx.request_repaint();
        
        // ALWAYS show menu bar first, regardless of state
        self.handle_menu(ctx);
        
        // Handle demo overlay (on top of everything)
        if let Some(example) = self.demo_overlay.show(ctx) {
            self.init_demo_example(example);
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
            
            // Navigation shortcuts
            if i.key_pressed(egui::Key::H) && i.modifiers.ctrl {
                // Ctrl+H: Go home
                *self.viewer_context.data_source.write() = None;
                self.viewport = Viewport::new();
                self.demo_mode = false;
                self.view_builder = None;
            }
            
            if i.key_pressed(egui::Key::Z) && !i.modifiers.ctrl {
                // Z: Reset zoom in all plots - DISABLED DUE TO CRASHES
                // TODO: Fix the underlying issue in plot reset logic
                // ctx.request_repaint();
            }
            
            if i.key_pressed(egui::Key::R) && !i.modifiers.ctrl {
                // R: Reset zoom and selection in all plots - DISABLED DUE TO CRASHES
                // TODO: Fix the underlying issue in plot reset logic
                /*
                // Clear hover data
                {
                    let mut hover_data = self.viewer_context.hovered_data.write();
                    hover_data.view_id = None;
                    hover_data.point_index = None;
                }
                // TODO: Send reset message to all views to clear zoom state
                // For now, request repaint to clear any highlights
                ctx.request_repaint();
                */
            }
            
            if i.key_pressed(egui::Key::Escape) {
                // Escape: Stop playback
                self.viewer_context.time_control.write().playing = false;
            }
            
            // D key: Quick demo mode toggle
            if i.key_pressed(egui::Key::D) && !i.modifiers.ctrl {
                self.demo_overlay.show = !self.demo_overlay.show;
            }
        });
        
        // Handle time control playback
        if self.viewer_context.time_control.read().playing {
            let speed = self.viewer_context.time_control.read().speed;
            let dt = ctx.input(|i| i.stable_dt);
            
            // Calculate how many frames to advance based on speed and time
            let frames_per_second = speed * 30.0; // Base rate of 30 FPS
            let frame_advance = (frames_per_second * dt as f64) as usize;
            
            if frame_advance > 0 {
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
                    } else {
                        // Stop at end
                        self.viewer_context.time_control.write().playing = false;
                    }
                } else {
                    // Continue advancing
                    let _ = self.viewer_context.navigation.seek_to(
                        dv_core::navigation::NavigationPosition::Sequential(new_pos)
                    );
                }
            }
            
            ctx.request_repaint();
        }
        
        // Check if we have data loaded
        let has_data = self.viewer_context.data_source.read().is_some();
        
        // Check if we should show view builder dialog
        if has_data && self.viewport.is_empty() && self.view_builder.is_none() && !self.demo_mode {
            // Data loaded but no views created - show view builder
            if let Some(data_source) = &*self.viewer_context.data_source.read() {
                let schema = self.runtime.block_on(data_source.schema());
                self.view_builder = Some(ViewBuilderDialog::new(schema));
            }
        }
        
        // Handle view builder dialog
        if let Some(ref mut builder) = self.view_builder {
            if let Some((views, nav_mode)) = builder.show_dialog(ctx) {
                // Update navigation mode
                let nav_spec = NavigationSpec {
                    mode: nav_mode,
                    total_rows: self.viewer_context.navigation.get_context().total_rows,
                    temporal_bounds: None,
                    categories: None,
                };
                self.viewer_context.navigation.update_spec(nav_spec);
                
                // Create views
                self.viewport.create_grid_layout(views);
                
                // Clear dialog
                self.view_builder = None;
            }
        }
        
        if has_data {
            // Show navigation panel at bottom when data is loaded
            let nav_context = self.viewer_context.navigation.get_context();
            let show_navigation = nav_context.total_rows > 1; // Only show if there's something to navigate
            
            if show_navigation {
                egui::TopBottomPanel::bottom("navigation_panel")
                    .resizable(false)
                    .exact_height(50.0)
                    .frame(
                        egui::Frame::none()
                            .fill(egui::Color32::from_gray(20))
                            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                            .outer_margin(0.0)
                    )
                    .show(ctx, |ui| {
                        // Create a temporary navigation panel for this frame
                        let mut nav_panel = dv_ui::NavigationPanel::new(
                            self.viewer_context.navigation.clone(),
                            self.viewer_context.time_control.clone()
                        );
                        nav_panel.ui(ui, &self.viewer_context);
                    });
            }
            
            // Main content area with viewport
            egui::CentralPanel::default().show(ctx, |ui| {
                self.viewport.ui(ui, &self.viewer_context);
            });
        } else {
            // Show welcome screen if no data is loaded
            egui::CentralPanel::default().show(ctx, |ui| {
                self.show_welcome_screen(ui);
            });
        }
    }
}

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting F.R.O.G. - Flexible Rust Overlay for Graphs 🐸");
    
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
        "F.R.O.G. - Flexible Rust Overlay for Graphs 🐸",
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