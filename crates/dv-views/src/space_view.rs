//! Space view abstraction - base trait for all dockable views

use egui::Ui;
use serde_json::Value;
use std::fmt::Debug;
use uuid::Uuid;

use crate::ViewerContext;

/// Unique identifier for a space view
pub type SpaceViewId = Uuid;

/// Configuration for space views
#[derive(Debug, Clone)]
pub struct SpaceViewConfig {
    /// The data source ID this view uses
    pub data_source_id: Option<String>,
}

impl Default for SpaceViewConfig {
    fn default() -> Self {
        Self {
            data_source_id: None,
        }
    }
}

/// Selection state for views
#[derive(Debug, Clone, Default)]
pub struct SelectionState {
    pub hovered_point: Option<usize>,
    pub selected_points: Vec<usize>,
}

/// Base trait for all space views (plots, tables, etc)
pub trait SpaceView: Send + Sync {
    /// Get the unique ID of this view
    fn id(&self) -> SpaceViewId;
    
    /// Get the display name
    fn display_name(&self) -> &str;
    
    /// Get the view type (for serialization)
    fn view_type(&self) -> &str;
    
    /// Get the title of this view
    fn title(&self) -> &str;
    
    /// Get the configuration for this view
    fn config(&self) -> SpaceViewConfig {
        SpaceViewConfig::default()
    }
    
    /// Set the data source ID for this view
    fn set_data_source(&mut self, _source_id: String) {
        // Default implementation does nothing
    }
    
    /// Get the data source ID for this view
    fn data_source_id(&self) -> Option<&str> {
        None
    }
    
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
    
    /// Check if this is a time series view
    fn is_time_series(&self) -> bool {
        false
    }
    
    /// Get as any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
    
    /// Get as any mut for downcasting
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
} 