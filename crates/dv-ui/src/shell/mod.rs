use egui::{Context, TopBottomPanel, CentralPanel};
use dv_core::{AppState, ViewerContext};
use crate::UiState;

/// Application shell that manages the main UI structure
pub struct AppShell {
    // Currently empty, but can hold shell-specific state in the future
}

impl AppShell {
    /// Create a new app shell
    pub fn new() -> Self {
        Self {}
    }
}

/// Shell configuration
pub struct ShellConfig {
    pub show_menu_bar: bool,
    pub show_status_bar: bool,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            show_menu_bar: true,
            show_status_bar: true,
        }
    }
}

/// Render the main menu bar
pub fn menu_bar(ctx: &Context, app_state: &mut AppState, viewer_context: Option<&ViewerContext>) {
    TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            // File menu
            ui.menu_button("File", |ui| {
                if ui.button("Open CSV...").clicked() {
                    // TODO: Open file dialog
                    ui.close_menu();
                }
                
                if ui.button("Open SQLite...").clicked() {
                    // TODO: Open file dialog
                    ui.close_menu();
                }
                
                ui.separator();
                
                if ui.button("Recent Files").clicked() {
                    // TODO: Show recent files
                    ui.close_menu();
                }
                
                ui.separator();
                
                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            
            // View menu
            ui.menu_button("View", |ui| {
                if ui.checkbox(&mut app_state.settings.show_navigation_bar, "Navigation Bar").clicked() {
                    ui.close_menu();
                }
                
                if ui.checkbox(&mut app_state.settings.show_stats_panel, "Statistics Panel").clicked() {
                    ui.close_menu();
                }
                
                ui.separator();
                
                if ui.button("Reset Layout").clicked() {
                    // TODO: Reset dock layout
                    ui.close_menu();
                }
            });
            
            // Tools menu
            ui.menu_button("Tools", |ui| {
                if ui.button("Settings...").clicked() {
                    // TODO: Open settings dialog
                    ui.close_menu();
                }
            });
            
            // Help menu
            ui.menu_button("Help", |ui| {
                if ui.button("About").clicked() {
                    // TODO: Show about dialog
                    ui.close_menu();
                }
                
                if ui.button("Documentation").clicked() {
                    // TODO: Open documentation
                    ui.close_menu();
                }
            });
            
            // Right-aligned status
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(viewer_ctx) = viewer_context {
                    // Show data source count if loaded
                    let data_sources = viewer_ctx.data_sources.read();
                    if !data_sources.is_empty() {
                        ui.label(format!("{} source(s) loaded", data_sources.len()));
                    ui.separator();
                }
                
                // Show navigation mode
                    let context = viewer_ctx.navigation.get_context();
                let mode_text = match context.mode {
                    dv_core::NavigationMode::Temporal => "⏱ Time",
                    dv_core::NavigationMode::Sequential => "📊 Rows",
                    dv_core::NavigationMode::Categorical { .. } => "📝 Categories",
                };
                ui.label(mode_text);
                }
            });
        });
    });
}

/// Render the central panel with dock area
pub fn central_panel(ctx: &Context, app_state: &mut AppState, ui_state: &mut UiState, viewer_context: Option<&ViewerContext>) {
    CentralPanel::default().show(ctx, |ui| {
        // Error messages
        show_error_messages(ui, ui_state);
        
        // Main content area
        let has_data = viewer_context
            .map(|v| !v.data_sources.read().is_empty())
            .unwrap_or(false);
            
        if !has_data {
            // Welcome screen when no data is loaded
            show_welcome_screen(ui);
        } else {
            // Dock area with views
            show_dock_area(ui, app_state, ui_state, viewer_context);
        }
    });
}

/// Show error messages
pub fn show_error_messages(ui: &mut egui::Ui, ui_state: &mut UiState) {
    let now = std::time::Instant::now();
    
    // Remove old messages
    ui_state.error_messages.retain(|msg| {
        now.duration_since(msg.timestamp).as_secs() < 10
    });
    
    // Display current messages
    for msg in &ui_state.error_messages {
        egui::Frame::none()
            .fill(crate::theme::error_color().linear_multiply(0.2))
            .stroke(egui::Stroke::new(1.0, crate::theme::error_color()))
            .rounding(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("⚠").color(crate::theme::error_color()));
                    ui.label(&msg.title);
                    ui.separator();
                    ui.label(&msg.message);
                });
            });
    }
}

/// Show welcome screen
fn show_welcome_screen(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(100.0);
        
        ui.heading("Data Visualization Platform");
        ui.add_space(20.0);
        
        ui.label("Drop a CSV or SQLite file here to begin");
        ui.add_space(40.0);
        
        ui.horizontal(|ui| {
            if ui.button("Open CSV File").clicked() {
                // TODO: Open file dialog
            }
            
            if ui.button("Open SQLite Database").clicked() {
                // TODO: Open file dialog
            }
        });
        
        ui.add_space(60.0);
        
        ui.separator();
        ui.add_space(20.0);
        
        ui.label("Recent Files:");
        // TODO: Show recent files list
    });
}

/// Show dock area with views
fn show_dock_area(ui: &mut egui::Ui, _app_state: &mut AppState, _ui_state: &mut UiState, _viewer_context: Option<&ViewerContext>) {
    // TODO: Implement dock area with views
    ui.label("Dock area - TODO: Implement with egui_dock");
} 