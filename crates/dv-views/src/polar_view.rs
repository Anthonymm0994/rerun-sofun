//! Polar plot space view

use crate::{SpaceView, ViewerContext, plots::polar::{PolarPlot, PolarPlotConfig}};
use egui::Ui;
use uuid::Uuid;

/// Polar plot space view
pub struct PolarPlotView {
    polar_plot: PolarPlot,
}

impl PolarPlotView {
    /// Create a new polar plot view
    pub fn new(id: Uuid, title: String) -> Self {
        Self {
            polar_plot: PolarPlot::new(id, title),
        }
    }
    
    /// Get mutable access to the config
    pub fn config_mut(&mut self) -> &mut PolarPlotConfig {
        &mut self.polar_plot.config
    }
}

impl SpaceView for PolarPlotView {
    fn id(&self) -> Uuid {
        self.polar_plot.id
    }

    fn title(&self) -> &str {
        &self.polar_plot.title
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn display_name(&self) -> &str {
        &self.polar_plot.title
    }
    
    fn view_type(&self) -> &str {
        "PolarPlot"
    }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        self.polar_plot.ui(ui, ctx);
    }
    
    fn save_config(&self) -> serde_json::Value {
        serde_json::to_value(&self.polar_plot.config).unwrap_or(serde_json::json!({}))
    }
    
    fn load_config(&mut self, config: serde_json::Value) {
        if let Ok(config) = serde_json::from_value(config) {
            self.polar_plot.config = config;
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &crate::SelectionState) {
        // No selection handling needed for polar plot
    }
    
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {
        // No frame update needed for polar plot
    }
    
    fn is_time_series(&self) -> bool {
        false
    }
} 