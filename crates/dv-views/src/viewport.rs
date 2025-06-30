//! Viewport - manages dockable space views
//! Based on Rerun's re_viewport

use std::collections::HashMap;
use egui::Ui;
use egui_dock::{DockArea, DockState, TabViewer};

use crate::{SpaceView, SpaceViewId, ViewerContext};

/// The main viewport that manages dockable space views
pub struct Viewport {
    dock_state: DockState<SpaceViewId>,
    space_views: HashMap<SpaceViewId, Box<dyn SpaceView>>,
    time_axis_views: Vec<SpaceViewId>,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            dock_state: DockState::new(vec![]),
            space_views: HashMap::new(),
            time_axis_views: Vec::new(),
        }
    }
    
    /// Add a space view to the viewport
    pub fn add_space_view(&mut self, view: Box<dyn SpaceView>) {
        let id = view.id().clone();
        
        // Track time-series views for cursor synchronization
        if view.view_type() == "TimeSeriesView" {
            self.time_axis_views.push(id.clone());
        }
        
        self.space_views.insert(id.clone(), view);
        
        // Add to dock state
        if self.dock_state.main_surface().is_empty() {
            // First view becomes the main surface
            self.dock_state = DockState::new(vec![id]);
        } else {
            // Add subsequent views to the first available leaf
            self.dock_state.push_to_first_leaf(id);
        }
    }
    
    /// Create a grid layout from multiple views
    pub fn create_grid_layout(&mut self, views: Vec<Box<dyn SpaceView>>) {
        if views.is_empty() {
            return;
        }
        
        // Clear existing state
        self.space_views.clear();
        self.time_axis_views.clear();
        
        let view_ids: Vec<SpaceViewId> = views.iter().map(|v| v.id().clone()).collect();
        
        // Add all views and track time-series views
        for view in views {
            let id = view.id().clone();
            if view.view_type() == "TimeSeriesView" {
                self.time_axis_views.push(id.clone());
            }
            self.space_views.insert(id, view);
        }
        
        // Create dock state with grid layout
        self.dock_state = create_grid_dock_state(view_ids);
    }
    
    /// Draw the viewport
    pub fn ui(&mut self, ui: &mut Ui, viewer_context: &ViewerContext) {
        // Update context with current time axis views
        *viewer_context.time_axis_views.write() = self.time_axis_views.clone();
        
        // The dock area should fill the available space in the UI
        let available_rect = ui.available_rect_before_wrap();
        
        ui.allocate_ui(available_rect.size(), |ui| {
            DockArea::new(&mut self.dock_state)
                .show_close_buttons(true)
                .draggable_tabs(true)
                .show_tab_name_on_hover(true)
                .show_inside(ui, &mut ViewportTabViewer {
                    space_views: &mut self.space_views,
                    viewer_context,
                });
        });
    }
}

/// Tab viewer for egui_dock
struct ViewportTabViewer<'a> {
    space_views: &'a mut HashMap<SpaceViewId, Box<dyn SpaceView>>,
    viewer_context: &'a ViewerContext,
}

impl<'a> TabViewer for ViewportTabViewer<'a> {
    type Tab = SpaceViewId;
    
    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        if let Some(view) = self.space_views.get(tab) {
            view.display_name().into()
        } else {
            "Unknown".into()
        }
    }
    
    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        if let Some(view) = self.space_views.get_mut(tab) {
            view.ui(self.viewer_context, ui);
        }
    }
    
    fn on_close(&mut self, tab: &mut Self::Tab) -> bool {
        self.space_views.remove(tab);
        true
    }
}

/// Create a grid layout for the dock state
fn create_grid_dock_state(view_ids: Vec<SpaceViewId>) -> DockState<SpaceViewId> {
    if view_ids.is_empty() {
        return DockState::new(vec![]);
    }
    
    let mut dock_state = DockState::new(vec![view_ids[0].clone()]);
    
    // For simplicity, add all other views as tabs in the same panel
    // This avoids complex node tracking while still providing a functional layout
    for id in view_ids.into_iter().skip(1) {
        dock_state.push_to_first_leaf(id);
    }
    
    dock_state
} 