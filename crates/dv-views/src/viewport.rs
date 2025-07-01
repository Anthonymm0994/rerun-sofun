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
    
    /// Create a grid layout with explicit configuration
    pub fn create_configured_layout(&mut self, views: Vec<Box<dyn SpaceView>>, layout_config: GridLayoutConfig) {
        if views.is_empty() {
            return;
        }
        
        // Clear existing state
        self.space_views.clear();
        self.time_axis_views.clear();
        
        // Add all views and track time-series views
        for view in views {
            let id = view.id().clone();
            if view.view_type() == "TimeSeriesView" {
                self.time_axis_views.push(id.clone());
            }
            self.space_views.insert(id, view);
        }
        
        // Create dock state based on grid configuration
        self.dock_state = create_grid_from_config(layout_config);
    }
    
    /// Check if the viewport has any views
    pub fn is_empty(&self) -> bool {
        self.space_views.is_empty()
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

/// Grid layout configuration
pub struct GridLayoutConfig {
    pub grid_size: (usize, usize), // (cols, rows)
    pub cells: Vec<GridCell>,
}

pub struct GridCell {
    pub view_id: SpaceViewId,
    pub grid_pos: (usize, usize),
    pub grid_span: (usize, usize),
}

/// Create a grid layout for the dock state
fn create_grid_dock_state(view_ids: Vec<SpaceViewId>) -> DockState<SpaceViewId> {
    use egui_dock::NodeIndex;
    
    if view_ids.is_empty() {
        return DockState::new(vec![]);
    }
    
    if view_ids.len() == 1 {
        return DockState::new(vec![view_ids[0].clone()]);
    }
    
    // Create a proper grid layout based on number of views
    let num_views = view_ids.len();
    
    match num_views {
        2 => {
            // Side by side
            let mut state = DockState::new(vec![view_ids[0].clone()]);
            let [_left, _right] = state.main_surface_mut().split_right(
                NodeIndex::root(), 
                0.5, 
                vec![view_ids[1].clone()]
            );
            state
        }
        3 => {
            // One on top, two below
            let mut state = DockState::new(vec![view_ids[0].clone()]);
            let [_top, bottom] = state.main_surface_mut().split_below(
                NodeIndex::root(), 
                0.5, 
                vec![view_ids[1].clone()]
            );
            let [_left, _right] = state.main_surface_mut().split_right(
                bottom, 
                0.5, 
                vec![view_ids[2].clone()]
            );
            state
        }
        4 => {
            // 2x2 grid
            let mut state = DockState::new(vec![view_ids[0].clone()]);
            let [left, right] = state.main_surface_mut().split_right(
                NodeIndex::root(), 
                0.5, 
                vec![view_ids[1].clone()]
            );
            let [_top_left, _bottom_left] = state.main_surface_mut().split_below(
                left, 
                0.5, 
                vec![view_ids[2].clone()]
            );
            let [_top_right, _bottom_right] = state.main_surface_mut().split_below(
                right, 
                0.5, 
                vec![view_ids[3].clone()]
            );
            state
        }
        _ => {
            // For more views, create a reasonable layout with tabs
            let cols = ((num_views as f32).sqrt().ceil() as usize).max(2);
            let mut state = DockState::new(vec![view_ids[0].clone()]);
            
            // Create columns first
            let mut col_nodes = vec![NodeIndex::root()];
            for i in 1..cols.min(num_views) {
                if let Some(&last_col) = col_nodes.last() {
                    let [_left, right] = state.main_surface_mut().split_right(
                        last_col,
                        1.0 / (cols - i + 1) as f32,
                        vec![view_ids[i].clone()]
                    );
                    col_nodes.push(right);
                }
            }
            
            // Add remaining views as tabs or in rows
            for (i, id) in view_ids.into_iter().enumerate().skip(cols) {
                let col_idx = (i - cols) % cols;
                if col_idx < col_nodes.len() {
                    state.push_to_focused_leaf(id);
                }
            }
            
            state
        }
    }
}

/// Create grid from explicit configuration
fn create_grid_from_config(config: GridLayoutConfig) -> DockState<SpaceViewId> {
    // For now, fall back to simple grid creation
    // TODO: Implement proper grid creation from config
    let view_ids: Vec<SpaceViewId> = config.cells.into_iter()
        .map(|cell| cell.view_id)
        .collect();
    create_grid_dock_state(view_ids)
} 