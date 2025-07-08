//! Stream graph (stacked area chart) implementation

use egui::{Ui, Color32};
use egui_plot::{Plot, PlotUi, PlotPoints, Polygon, Legend, Corner, Line};
use arrow::record_batch::RecordBatch;
use arrow::array::{Float64Array, StringArray, TimestampSecondArray, Array};
use serde_json::{json, Value};
use std::collections::{HashMap, BTreeMap};

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;
use super::utils::colors::{categorical_color, ColorScheme};

/// Stream graph configuration
#[derive(Debug, Clone)]
pub struct StreamGraphConfig {
    pub time_column: Option<String>,
    pub value_column: Option<String>,
    pub category_column: Option<String>,
    
    // Stacking options
    pub offset_type: OffsetType,
    pub normalize: bool,
    pub interpolation: InterpolationType,
    
    // Visual options
    pub color_scheme: ColorScheme,
    pub show_legend: bool,
    pub show_labels: bool,
    pub opacity: u8,
    pub smooth_factor: f32,
    
    // Interaction
    pub highlight_on_hover: bool,
    pub show_tooltip: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OffsetType {
    Zero,        // Standard stacked area
    Wiggle,      // Minimize weighted wiggle (StreamGraph)
    Silhouette,  // Center the stream
    Expand,      // Normalize to fill [0,1]
}

#[derive(Debug, Clone, PartialEq)]
pub enum InterpolationType {
    Linear,
    StepBefore,
    StepAfter,
    Smooth,
}

impl Default for StreamGraphConfig {
    fn default() -> Self {
        Self {
            time_column: None,
            value_column: None,
            category_column: None,
            offset_type: OffsetType::Wiggle,
            normalize: false,
            interpolation: InterpolationType::Smooth,
            color_scheme: ColorScheme::Categorical,
            show_legend: true,
            show_labels: false,
            opacity: 200,
            smooth_factor: 0.5,
            highlight_on_hover: true,
            show_tooltip: true,
        }
    }
}

#[derive(Clone, Debug)]
struct StreamLayer {
    name: String,
    color: Color32,
    values: BTreeMap<i64, f64>, // timestamp -> value
    y0_values: Vec<[f64; 2]>,   // baseline points
    y1_values: Vec<[f64; 2]>,   // top points
}

/// Stream graph view
pub struct StreamGraph {
    id: SpaceViewId,
    title: String,
    pub config: StreamGraphConfig,
    
    // State
    cached_data: Option<RecordBatch>,
    layers: Vec<StreamLayer>,
    time_range: Option<(i64, i64)>,
    
    // Interaction state
    hovered_layer: Option<usize>,
    hovered_time: Option<i64>,
}

impl StreamGraph {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: StreamGraphConfig::default(),
            cached_data: None,
            layers: Vec::new(),
            time_range: None,
            hovered_layer: None,
            hovered_time: None,
        }
    }
}

impl SpaceView for StreamGraph {
    fn id(&self) -> SpaceViewId { self.id }
    fn title(&self) -> &str {
        &self.title
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn display_name(&self) -> &str { &self.title }
    fn view_type(&self) -> &str { "StreamGraphView" }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        ui.centered_and_justified(|ui| {
            ui.label("Stream graph visualization coming soon...");
        });
    }
    
    fn save_config(&self) -> Value {
        json!({
            "time_column": self.config.time_column,
            "value_column": self.config.value_column,
            "category_column": self.config.category_column,
            "offset_type": format!("{:?}", self.config.offset_type),
            "normalize": self.config.normalize,
            "interpolation": format!("{:?}", self.config.interpolation),
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(col) = config.get("time_column").and_then(|v| v.as_str()) {
            self.config.time_column = Some(col.to_string());
        }
        if let Some(col) = config.get("value_column").and_then(|v| v.as_str()) {
            self.config.value_column = Some(col.to_string());
        }
        if let Some(col) = config.get("category_column").and_then(|v| v.as_str()) {
            self.config.category_column = Some(col.to_string());
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {}
    
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {}
} 