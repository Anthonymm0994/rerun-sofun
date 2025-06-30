use std::sync::Arc;
use parking_lot::RwLock;
use ahash::AHashMap;

/// Synchronization manager for coordinating state across multiple views
pub struct SyncManager {
    /// Shared selection state
    selection: Arc<RwLock<SelectionState>>,
    
    /// Shared highlight state
    highlight: Arc<RwLock<HighlightState>>,
    
    /// View-specific sync settings
    view_settings: Arc<RwLock<AHashMap<String, ViewSyncSettings>>>,
}

/// Selection state shared across views
#[derive(Debug, Clone, Default)]
pub struct SelectionState {
    /// Selected row indices
    pub selected_rows: Vec<usize>,
    
    /// Selected column names
    pub selected_columns: Vec<String>,
    
    /// Selected time range (if applicable)
    pub selected_range: Option<(i64, i64)>,
}

/// Highlight state shared across views
#[derive(Debug, Clone, Default)]
pub struct HighlightState {
    /// Highlighted row indices
    pub highlighted_rows: Vec<usize>,
    
    /// Highlighted values (column -> value)
    pub highlighted_values: AHashMap<String, String>,
}

/// Synchronization settings for a specific view
#[derive(Debug, Clone)]
pub struct ViewSyncSettings {
    /// Whether this view participates in selection sync
    pub sync_selection: bool,
    
    /// Whether this view participates in highlight sync
    pub sync_highlight: bool,
    
    /// Whether this view participates in navigation sync
    pub sync_navigation: bool,
}

impl Default for ViewSyncSettings {
    fn default() -> Self {
        Self {
            sync_selection: true,
            sync_highlight: true,
            sync_navigation: true,
        }
    }
}

impl SyncManager {
    /// Create a new synchronization manager
    pub fn new() -> Self {
        Self {
            selection: Arc::new(RwLock::new(SelectionState::default())),
            highlight: Arc::new(RwLock::new(HighlightState::default())),
            view_settings: Arc::new(RwLock::new(AHashMap::new())),
        }
    }
    
    /// Get the current selection state
    pub fn selection(&self) -> SelectionState {
        self.selection.read().clone()
    }
    
    /// Update the selection state
    pub fn set_selection(&self, selection: SelectionState) {
        *self.selection.write() = selection;
    }
    
    /// Get the current highlight state
    pub fn highlight(&self) -> HighlightState {
        self.highlight.read().clone()
    }
    
    /// Update the highlight state
    pub fn set_highlight(&self, highlight: HighlightState) {
        *self.highlight.write() = highlight;
    }
    
    /// Register a view with sync settings
    pub fn register_view(&self, view_id: String, settings: ViewSyncSettings) {
        self.view_settings.write().insert(view_id, settings);
    }
    
    /// Unregister a view
    pub fn unregister_view(&self, view_id: &str) {
        self.view_settings.write().remove(view_id);
    }
    
    /// Check if a view should sync selection
    pub fn should_sync_selection(&self, view_id: &str) -> bool {
        self.view_settings
            .read()
            .get(view_id)
            .map(|s| s.sync_selection)
            .unwrap_or(true)
    }
    
    /// Check if a view should sync highlight
    pub fn should_sync_highlight(&self, view_id: &str) -> bool {
        self.view_settings
            .read()
            .get(view_id)
            .map(|s| s.sync_highlight)
            .unwrap_or(true)
    }
    
    /// Check if a view should sync navigation
    pub fn should_sync_navigation(&self, view_id: &str) -> bool {
        self.view_settings
            .read()
            .get(view_id)
            .map(|s| s.sync_navigation)
            .unwrap_or(true)
    }
} 