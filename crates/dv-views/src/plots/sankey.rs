//! Sankey diagram implementation for flow visualization

use egui::{Ui, Color32, Rect, Pos2, Vec2, Stroke, FontId, Align2, Shape, Rounding, Response, Sense};
use arrow::record_batch::RecordBatch;
use arrow::array::{Float64Array, StringArray, Array};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use super::utils::{ColorScheme, categorical_color};

/// Sankey diagram configuration
#[derive(Debug, Clone)]
pub struct SankeyConfig {
    pub source_column: String,
    pub target_column: String,
    pub value_column: String,
    pub label_column: Option<String>,
    
    // Layout options
    pub node_width: f32,
    pub node_padding: f32,
    pub link_curvature: f32,
    pub iterations: u32,
    
    // Visual options
    pub color_scheme: ColorScheme,
    pub show_labels: bool,
    pub show_values: bool,
    pub highlight_on_hover: bool,
    pub min_link_width: f32,
    pub max_link_width: f32,
}

impl Default for SankeyConfig {
    fn default() -> Self {
        Self {
            source_column: String::new(),
            target_column: String::new(),
            value_column: String::new(),
            label_column: None,
            node_width: 20.0,
            node_padding: 10.0,
            link_curvature: 0.5,
            iterations: 32,
            color_scheme: ColorScheme::Categorical,
            show_labels: true,
            show_values: true,
            highlight_on_hover: true,
            min_link_width: 1.0,
            max_link_width: 50.0,
        }
    }
}

/// Node in the Sankey diagram
#[derive(Clone, Debug)]
struct SankeyNode {
    id: String,
    label: String,
    layer: usize,
    value: f64,
    x: f32,
    y: f32,
    height: f32,
    color: Color32,
    incoming_value: f64,
    outgoing_value: f64,
}

/// Link between nodes
#[derive(Clone, Debug)]
struct SankeyLink {
    source: String,
    target: String,
    value: f64,
    color: Color32,
    source_y: f32,
    target_y: f32,
    width: f32,
}

/// Sankey diagram view
pub struct SankeyDiagram {
    id: SpaceViewId,
    title: String,
    pub config: SankeyConfig,
    
    // State
    cached_data: Option<RecordBatch>,
    nodes: HashMap<String, SankeyNode>,
    links: Vec<SankeyLink>,
    layers: Vec<Vec<String>>,
    
    // Interaction state
    hovered_node: Option<String>,
    hovered_link: Option<usize>,
}

impl SankeyDiagram {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: SankeyConfig::default(),
            cached_data: None,
            nodes: HashMap::new(),
            links: Vec::new(),
            layers: Vec::new(),
            hovered_node: None,
            hovered_link: None,
        }
    }
    
    fn extract_flow_data(&mut self, batch: &RecordBatch) {
        self.nodes.clear();
        self.links.clear();
        self.layers.clear();
        
        // Find columns
        let source_idx = batch.schema().fields().iter()
            .position(|f| f.name() == &self.config.source_column);
        let target_idx = batch.schema().fields().iter()
            .position(|f| f.name() == &self.config.target_column);
        let value_idx = batch.schema().fields().iter()
            .position(|f| f.name() == &self.config.value_column);
            
        if source_idx.is_none() || target_idx.is_none() || value_idx.is_none() {
            return;
        }
        
        let source_col = batch.column(source_idx.unwrap());
        let target_col = batch.column(target_idx.unwrap());
        let value_col = batch.column(value_idx.unwrap());
        
        // Extract links
        if let (Some(source_array), Some(target_array), Some(value_array)) = (
            source_col.as_any().downcast_ref::<StringArray>(),
            target_col.as_any().downcast_ref::<StringArray>(),
            value_col.as_any().downcast_ref::<Float64Array>()
        ) {
            let mut temp_links = Vec::new();
            let mut node_set = HashSet::new();
            
            for i in 0..source_array.len() {
                let source = source_array.value(i);
                let target = target_array.value(i);
                let value = value_array.value(i);
                    node_set.insert(source.to_string());
                    node_set.insert(target.to_string());
                    
                    temp_links.push((source.to_string(), target.to_string(), value));
            }
            
            // Create nodes and determine layers
            self.compute_node_layers(&node_set, &temp_links);
            
            // Create link objects
            for (i, (source, target, value)) in temp_links.iter().enumerate() {
                self.links.push(SankeyLink {
                    source: source.clone(),
                    target: target.clone(),
                    value: *value,
                    color: categorical_color(i),
                    source_y: 0.0,
                    target_y: 0.0,
                    width: 0.0,
                });
            }
        }
    }
    
    fn compute_node_layers(&mut self, node_set: &HashSet<String>, links: &[(String, String, f64)]) {
        // Build adjacency lists
        let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
        let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
        
        for (source, target, _) in links {
            outgoing.entry(source.clone()).or_default().push(target.clone());
            incoming.entry(target.clone()).or_default().push(source.clone());
        }
        
        // Find nodes with no incoming edges (sources)
        let mut sources: Vec<String> = node_set.iter()
            .filter(|node| !incoming.contains_key(*node))
            .cloned()
            .collect();
        
        // Compute layers using topological sort
        let mut node_layers: HashMap<String, usize> = HashMap::new();
        let mut current_layer = 0;
        
        while !sources.is_empty() {
            let mut next_sources = Vec::new();
            
            for source in sources {
                node_layers.insert(source.clone(), current_layer);
                
                if let Some(targets) = outgoing.get(&source) {
                    for target in targets {
                        // Check if all incoming nodes have been assigned layers
                        if let Some(incomings) = incoming.get(target) {
                            if incomings.iter().all(|n| node_layers.contains_key(n)) {
                                next_sources.push(target.clone());
                            }
                        }
                    }
                }
            }
            
            sources = next_sources;
            current_layer += 1;
        }
        
        // Handle any remaining nodes (cycles)
        for node in node_set {
            if !node_layers.contains_key(node) {
                node_layers.insert(node.clone(), current_layer);
            }
        }
        
        // Create layer structure
        let max_layer = node_layers.values().max().copied().unwrap_or(0);
        self.layers = vec![Vec::new(); max_layer + 1];
        
        // Calculate node values
        for (source, target, value) in links {
            let source_node = self.nodes.entry(source.clone()).or_insert(SankeyNode {
                id: source.clone(),
                label: source.clone(),
                layer: node_layers.get(source).copied().unwrap_or(0),
                value: 0.0,
                x: 0.0,
                y: 0.0,
                height: 0.0,
                color: Color32::from_gray(150),
                incoming_value: 0.0,
                outgoing_value: 0.0,
            });
            source_node.outgoing_value += value;
            source_node.value = source_node.incoming_value.max(source_node.outgoing_value);
            
            let target_node = self.nodes.entry(target.clone()).or_insert(SankeyNode {
                id: target.clone(),
                label: target.clone(),
                layer: node_layers.get(target).copied().unwrap_or(0),
                value: 0.0,
                x: 0.0,
                y: 0.0,
                height: 0.0,
                color: Color32::from_gray(150),
                incoming_value: 0.0,
                outgoing_value: 0.0,
            });
            target_node.incoming_value += value;
            target_node.value = target_node.incoming_value.max(target_node.outgoing_value);
        }
        
        // Assign nodes to layers
        for (node_id, node) in &self.nodes {
            self.layers[node.layer].push(node_id.clone());
        }
        
        // Assign colors
        let mut color_idx = 0;
        for node in self.nodes.values_mut() {
            node.color = categorical_color(color_idx);
            color_idx += 1;
        }
    }
    
    fn layout_nodes(&mut self, rect: Rect) {
        if self.layers.is_empty() {
            return;
        }
        
        let layer_width = (rect.width() - self.config.node_width) / (self.layers.len() - 1).max(1) as f32;
        let total_value: f64 = self.nodes.values().map(|n| n.value).sum();
        let scale = (rect.height() - (self.nodes.len() as f32 * self.config.node_padding)) / total_value as f32;
        
        // Position nodes in each layer
        for (layer_idx, layer_nodes) in self.layers.iter().enumerate() {
            let x = rect.left() + layer_idx as f32 * layer_width;
            let mut y = rect.top();
            
            // Sort nodes by value for better layout
            let mut sorted_nodes = layer_nodes.clone();
            sorted_nodes.sort_by(|a, b| {
                let val_a = self.nodes.get(a).map(|n| n.value).unwrap_or(0.0);
                let val_b = self.nodes.get(b).map(|n| n.value).unwrap_or(0.0);
                val_b.partial_cmp(&val_a).unwrap()
            });
            
            for node_id in sorted_nodes {
                if let Some(node) = self.nodes.get_mut(&node_id) {
                    node.x = x;
                    node.y = y;
                    node.height = (node.value as f32 * scale).max(2.0);
                    y += node.height + self.config.node_padding;
                }
            }
        }
        
        // Calculate link positions
        let mut source_offsets: HashMap<String, f32> = HashMap::new();
        let mut target_offsets: HashMap<String, f32> = HashMap::new();
        
        for link in &mut self.links {
            let source_node = self.nodes.get(&link.source).unwrap();
            let target_node = self.nodes.get(&link.target).unwrap();
            
            let link_height = (link.value as f32 * scale).max(self.config.min_link_width);
            link.width = link_height.min(self.config.max_link_width);
            
            // Calculate Y positions
            let source_offset = source_offsets.entry(link.source.clone()).or_insert(0.0);
            link.source_y = source_node.y + *source_offset + link.width / 2.0;
            *source_offset += link.width;
            
            let target_offset = target_offsets.entry(link.target.clone()).or_insert(0.0);
            link.target_y = target_node.y + *target_offset + link.width / 2.0;
            *target_offset += link.width;
            
            // Use source node color for link
            link.color = source_node.color;
        }
    }
    
    fn draw_nodes(&self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        
        for node in self.nodes.values() {
            let node_rect = Rect::from_min_size(
                Pos2::new(node.x, node.y),
                Vec2::new(self.config.node_width, node.height)
            );
            
            // Highlight on hover
            let is_highlighted = self.hovered_node.as_ref() == Some(&node.id) ||
                self.links.iter().any(|link| 
                    (link.source == node.id || link.target == node.id) && 
                    self.hovered_link == Some(self.links.iter().position(|l| 
                        l.source == link.source && l.target == link.target
                    ).unwrap())
                );
            
            let color = if is_highlighted {
                node.color
            } else if self.hovered_node.is_some() || self.hovered_link.is_some() {
                Color32::from_rgba_unmultiplied(node.color.r(), node.color.g(), node.color.b(), 100)
            } else {
                node.color
            };
            
            painter.rect_filled(node_rect, Rounding::ZERO, color);
            
            // Draw label
            if self.config.show_labels {
                let label_pos = Pos2::new(
                    node.x + self.config.node_width + 5.0,
                    node.y + node.height / 2.0
                );
                
                painter.text(
                    label_pos,
                    Align2::LEFT_CENTER,
                    &node.label,
                    FontId::proportional(12.0),
                    Color32::from_gray(200),
                );
                
                if self.config.show_values {
                    let value_text = format!("{:.1}", node.value);
                    painter.text(
                        label_pos + Vec2::new(0.0, 12.0),
                        Align2::LEFT_CENTER,
                        value_text,
                        FontId::proportional(10.0),
                        Color32::from_gray(150),
                    );
                }
            }
        }
    }
    
    fn draw_links(&self, ui: &mut Ui, _rect: Rect) {
        let painter = ui.painter();
        
        for (idx, link) in self.links.iter().enumerate() {
            if let (Some(source_node), Some(target_node)) = (
                self.nodes.get(&link.source),
                self.nodes.get(&link.target)
            ) {
                // Create bezier curve
                let source_pos = Pos2::new(source_node.x + self.config.node_width, link.source_y);
                let target_pos = Pos2::new(target_node.x, link.target_y);
                
                let control_offset = (target_pos.x - source_pos.x) * self.config.link_curvature;
                let control1 = source_pos + Vec2::new(control_offset, 0.0);
                let control2 = target_pos - Vec2::new(control_offset, 0.0);
                
                // Highlight on hover
                let is_highlighted = self.hovered_link == Some(idx) ||
                    self.hovered_node.as_ref() == Some(&link.source) ||
                    self.hovered_node.as_ref() == Some(&link.target);
                
                let color = if is_highlighted {
                    Color32::from_rgba_unmultiplied(link.color.r(), link.color.g(), link.color.b(), 180)
                } else if self.hovered_node.is_some() || self.hovered_link.is_some() {
                    Color32::from_rgba_unmultiplied(link.color.r(), link.color.g(), link.color.b(), 50)
                } else {
                    Color32::from_rgba_unmultiplied(link.color.r(), link.color.g(), link.color.b(), 120)
                };
                
                // Draw bezier curve as a filled shape
                let steps = 30;
                let mut points = Vec::new();
                
                // Top edge of the flow
                for i in 0..=steps {
                    let t = i as f32 / steps as f32;
                    let pos = cubic_bezier(source_pos, control1, control2, target_pos, t);
                    points.push(pos - Vec2::new(0.0, link.width / 2.0));
                }
                
                // Bottom edge of the flow (reversed)
                for i in (0..=steps).rev() {
                    let t = i as f32 / steps as f32;
                    let pos = cubic_bezier(source_pos, control1, control2, target_pos, t);
                    points.push(pos + Vec2::new(0.0, link.width / 2.0));
                }
                
                painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
            }
        }
    }
    
    fn handle_interaction(&mut self, ui: &mut Ui, rect: Rect) -> Response {
        let response = ui.allocate_rect(rect, Sense::hover());
        let mut tooltip_text = None;
        
        if let Some(hover_pos) = response.hover_pos() {
            // Check node hover
            self.hovered_node = None;
            for (node_id, node) in &self.nodes {
                let node_rect = Rect::from_min_size(
                    Pos2::new(node.x, node.y),
                    Vec2::new(self.config.node_width, node.height)
                );
                
                if node_rect.contains(hover_pos) {
                    self.hovered_node = Some(node_id.clone());
                    
                    // Store tooltip text
                    tooltip_text = Some(format!(
                        "{}\nTotal: {:.1}\nIncoming: {:.1}\nOutgoing: {:.1}",
                        node.label, node.value, node.incoming_value, node.outgoing_value
                    ));
                    break;
                }
            }
            
            // Check link hover if no node is hovered
            if self.hovered_node.is_none() {
                self.hovered_link = None;
                
                for (idx, link) in self.links.iter().enumerate() {
                    if let (Some(source_node), Some(target_node)) = (
                        self.nodes.get(&link.source),
                        self.nodes.get(&link.target)
                    ) {
                        // Approximate link hit test
                        let source_pos = Pos2::new(source_node.x + self.config.node_width, link.source_y);
                        let target_pos = Pos2::new(target_node.x, link.target_y);
                        
                        // Check multiple points along the curve
                        for i in 0..20 {
                            let t = i as f32 / 20.0;
                            let control_offset = (target_pos.x - source_pos.x) * self.config.link_curvature;
                            let control1 = source_pos + Vec2::new(control_offset, 0.0);
                            let control2 = target_pos - Vec2::new(control_offset, 0.0);
                            let pos = cubic_bezier(source_pos, control1, control2, target_pos, t);
                            
                            if (pos - hover_pos).length() < link.width {
                                self.hovered_link = Some(idx);
                                
                                // Store tooltip text
                                tooltip_text = Some(format!(
                                    "{} → {}\nValue: {:.1}",
                                    link.source, link.target, link.value
                                ));
                                break;
                            }
                        }
                        
                        if self.hovered_link.is_some() {
                            break;
                        }
                    }
                }
            }
        } else {
            self.hovered_node = None;
            self.hovered_link = None;
        }
        
        // Show tooltip if we have one
        if let Some(text) = tooltip_text {
            response.on_hover_text(text)
        } else {
            response
        }
    }
}

// Helper function for cubic bezier curves
fn cubic_bezier(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, t: f32) -> Pos2 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    
    Pos2::new(
        mt3 * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t3 * p3.x,
        mt3 * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t3 * p3.y
    )
}

impl SpaceView for SankeyDiagram {
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
    fn view_type(&self) -> &str { "SankeyView" }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Update data if needed
        if self.cached_data.is_none() {
            let data_sources = ctx.data_sources.read();

            let data_source = data_sources.values().next();
            if let Some(source) = data_source.as_ref() {
                let nav_pos = ctx.navigation.get_context().position.clone();
                if let Ok(batch) = ctx.runtime_handle.block_on(source.query_at(&nav_pos)) {
                    self.cached_data = Some(batch.clone());
                    self.extract_flow_data(&batch);
                }
            }
        }
        
        if self.cached_data.is_some() && !self.nodes.is_empty() {
            // Main drawing area
            let available_rect = ui.available_rect_before_wrap();
            let plot_rect = Rect::from_min_size(
                available_rect.left_top() + Vec2::new(10.0, 10.0),
                available_rect.size() - Vec2::new(20.0, 20.0)
            );
            
            // Layout nodes
            self.layout_nodes(plot_rect);
            
            // Draw
            self.draw_links(ui, plot_rect);
            self.draw_nodes(ui, plot_rect);
            
            // Handle interaction
            self.handle_interaction(ui, plot_rect);
            
            // Info panel
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(format!("Nodes: {}", self.nodes.len()));
                ui.label(format!("Flows: {}", self.links.len()));
                
                if self.config.highlight_on_hover {
                    ui.label("💡 Hover to highlight flows");
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No flow data available. Please configure source, target, and value columns.");
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "source_column": self.config.source_column,
            "target_column": self.config.target_column,
            "value_column": self.config.value_column,
            "label_column": self.config.label_column,
            "node_width": self.config.node_width,
            "node_padding": self.config.node_padding,
            "link_curvature": self.config.link_curvature,
            "color_scheme": format!("{:?}", self.config.color_scheme),
            "show_labels": self.config.show_labels,
            "show_values": self.config.show_values,
            "highlight_on_hover": self.config.highlight_on_hover,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(source) = config.get("source_column").and_then(|v| v.as_str()) {
            self.config.source_column = source.to_string();
        }
        if let Some(target) = config.get("target_column").and_then(|v| v.as_str()) {
            self.config.target_column = target.to_string();
        }
        if let Some(value) = config.get("value_column").and_then(|v| v.as_str()) {
            self.config.value_column = value.to_string();
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {}
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {}
} 