//! User interface components for the data visualization platform
//! 
//! This crate provides the egui-based UI components including
//! panels, controls, and layout management.

pub mod panels;
pub mod theme;
pub mod shell;
pub mod navigation_panel;
pub mod widget_utils;

use std::time::Instant;

/// Re-export commonly used types
pub use navigation_panel::NavigationPanel;
pub use panels::*;
pub use shell::{AppShell, ShellConfig};
pub use theme::{Theme, apply_theme};
pub use widget_utils::{WidgetId, ScrollAreaExt, GridExt, widget_id, nested_widget_id};

// Re-export commonly used functions
pub use shell::{menu_bar, central_panel};

/// UI state that persists across frames
pub struct UiState {
    /// Dock state for panel layout
    pub dock_state: egui_dock::DockState<String>,
    
    /// File dialog state
    pub file_dialog: Option<FileDialogState>,
    
    /// Error messages to display
    pub error_messages: Vec<ErrorMessage>,
    
    /// Hovered plot point for synchronization
    pub hovered_plot_point: Option<HoveredPoint>,
}

impl Default for UiState {
    fn default() -> Self {
        // Create a default dock layout with a single tab
        let tabs = vec!["Main View".to_string()];
        
        Self {
            dock_state: egui_dock::DockState::new(tabs),
            file_dialog: None,
            error_messages: Vec::new(),
            hovered_plot_point: None,
        }
    }
}

/// Error message to display
pub struct ErrorMessage {
    pub title: String,
    pub message: String,
    pub timestamp: Instant,
}

/// Hovered point information for plot synchronization
#[derive(Clone, Debug)]
pub struct HoveredPoint {
    pub view_id: String,
    pub series_name: String,
    pub x: f64,
    pub y: f64,
}

/// File dialog state
pub struct FileDialogState {
    pub mode: FileDialogMode,
    pub current_path: std::path::PathBuf,
}

/// File dialog mode
pub enum FileDialogMode {
    Open,
    Save,
}

// Widget creation helpers
pub fn icon_button(ui: &mut egui::Ui, icon: &str, tooltip: &str) -> egui::Response {
    ui.add(egui::Button::new(icon))
        .on_hover_text(tooltip)
}

// Common icon definitions
pub mod icons {
    pub const PLAY: &str = "▶";
    pub const PAUSE: &str = "⏸";
    pub const STOP: &str = "⏹";
    pub const SKIP_FORWARD: &str = "⏭";
    pub const SKIP_BACKWARD: &str = "⏮";
    pub const LOOP: &str = "🔁";
    pub const SETTINGS: &str = "⚙";
    pub const FOLDER: &str = "📁";
    pub const FILE: &str = "📄";
    pub const DATABASE: &str = "🗄";
    pub const CHART: &str = "📊";
    pub const TABLE: &str = "📋";
    pub const TIME: &str = "⏱";
}

// Panel IDs
pub mod panel_ids {
    pub const NAVIGATION: &str = "navigation_panel";
    pub const STATS: &str = "stats_panel";
    pub const DATA_SOURCE: &str = "data_source_panel";
}

// UI State types
pub struct DialogState {
    pub kind: DialogKind,
    pub visible: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum DialogKind {
    About,
    Settings,
    Open,
    Save,
} 