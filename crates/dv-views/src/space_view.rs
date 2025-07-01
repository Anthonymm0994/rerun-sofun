//! Space view abstraction - base trait for all dockable views

use egui::Ui;
use serde_json::Value;
use std::fmt::Debug;

use crate::ViewerContext;

/// Unique identifier for a space view
pub type SpaceViewId = uuid::Uuid;

/// Configuration that can be saved/loaded
pub trait SpaceViewConfig: Send + Sync {
    fn save(&self) -> Value;
    fn load(&mut self, value: Value);
}

/// Selection state for views
#[derive(Debug, Clone)]
pub struct SelectionState {
    pub selected_items: Vec<String>,
}

/// Base trait for all space views (plots, tables, etc)
pub trait SpaceView: Send + Sync {
    /// Get the unique ID of this view
    fn id(&self) -> &SpaceViewId;
    
    /// Get the display name
    fn display_name(&self) -> &str;
    
    /// Get the view type (for serialization)
    fn view_type(&self) -> &str;
    
    /// Draw the UI
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui);
    
    /// Save configuration
    fn save_config(&self) -> Value;
    
    /// Load configuration
    fn load_config(&mut self, config: Value);
    
    /// Handle selection changes
    fn on_selection_change(&mut self, ctx: &ViewerContext, selection: &SelectionState);
    
    /// Called each frame for updates
    fn on_frame_update(&mut self, ctx: &ViewerContext, dt: f32);
} 