//! Network graph visualization with interactive force-directed layout

use egui::{Ui, Color32, Rect, Pos2, Vec2, Stroke, FontId, Align2, Shape, Response, Sense, Rounding};
use egui_plot::{Plot, PlotUi, PlotBounds, PlotPoint, Text};
use arrow::record_batch::RecordBatch;
use arrow::array::{StringArray, Float64Array, Int64Array, Array};
use serde_json::{json, Value};
use petgraph::{Graph, Undirected, Directed};
use petgraph::graph::NodeIndex;
use fdg_sim::{ForceGraph, Node as FdgNode};
use std::collections::{HashMap, HashSet};
use nalgebra::{Point2, Vector2};
use rand;
use rand::Rng;

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use super::utils::colors::{categorical_color, viridis_color, ColorScheme};

/// Network graph configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub source_column: String,
    pub target_column: String,
    pub weight_column: Option<String>,
    pub node_label_column: Option<String>,
    pub node_size_column: Option<String>,
    pub node_color_column: Option<String>,
    
    // Layout options
    pub layout_algorithm: LayoutAlgorithm,
    pub iterations_per_frame: u32,
    pub node_repulsion: f32,
    pub edge_attraction: f32,
    pub centering_force: f32,
    
    // Visual options
    pub node_size: f32,
    pub edge_width: f32,
    pub show_labels: bool,
    pub show_arrows: bool,
    pub color_scheme: ColorScheme,
    pub highlight_neighbors: bool,
    pub edge_bundling: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LayoutAlgorithm {
    ForceDirected,
    Hierarchical,
    Circular,
    Grid,
    Spectral,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            source_column: String::new(),
            target_column: String::new(),
            weight_column: None,
            node_label_column: None,
            node_size_column: None,
            node_color_column: None,
            layout_algorithm: LayoutAlgorithm::ForceDirected,
            iterations_per_frame: 10,
            node_repulsion: 100.0,
            edge_attraction: 0.1,
            centering_force: 0.01,
            node_size: 10.0,
            edge_width: 1.0,
            show_labels: true,
            show_arrows: true,
            color_scheme: ColorScheme::Categorical,
            highlight_neighbors: true,
            edge_bundling: false,
        }
    }
}

/// Node information
#[derive(Clone, Debug)]
struct NetworkNode {
    id: String,
    label: String,
    position: Point2<f32>,
    velocity: Vector2<f32>,
    size: f32,
    color: Color32,
    value: Option<f64>,
    degree: usize,
}

/// Edge information
#[derive(Clone, Debug)]
struct NetworkEdge {
    source: String,
    target: String,
    weight: f64,
    color: Color32,
}

/// Network graph view
pub struct NetworkGraph {
    id: SpaceViewId,
    title: String,
    pub config: NetworkConfig,
    
    // State
    cached_data: Option<RecordBatch>,
    nodes: HashMap<String, NetworkNode>,
    edges: Vec<NetworkEdge>,
    graph: Graph<String, f64, Undirected>,
    node_indices: HashMap<String, NodeIndex>,
    
    // Layout state
    layout_bounds: Rect,
    
    // Interaction state
    selected_node: Option<String>,
    hovered_node: Option<String>,
    dragging_node: Option<String>,
    pan_offset: Vec2,
    zoom: f32,
}

impl NetworkGraph {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: NetworkConfig::default(),
            cached_data: None,
            nodes: HashMap::new(),
            edges: Vec::new(),
            graph: Graph::new_undirected(),
            node_indices: HashMap::new(),
            layout_bounds: Rect::from_center_size(Pos2::ZERO, Vec2::splat(400.0)),
            selected_node: None,
            hovered_node: None,
            dragging_node: None,
            pan_offset: Vec2::ZERO,
            zoom: 1.0,
        }
    }
    
    fn extract_network_data(&mut self, batch: &RecordBatch) {
        self.nodes.clear();
        self.edges.clear();
        self.graph.clear();
        self.node_indices.clear();
        
        // Find columns
        let source_idx = batch.schema().fields().iter()
            .position(|f| f.name() == &self.config.source_column);
        let target_idx = batch.schema().fields().iter()
            .position(|f| f.name() == &self.config.target_column);
            
        if source_idx.is_none() || target_idx.is_none() {
            return;
        }
        
        let source_col = batch.column(source_idx.unwrap());
        let target_col = batch.column(target_idx.unwrap());
        
        // Extract edges and collect unique nodes
        let mut node_set = HashSet::new();
        let mut temp_edges = Vec::new();
        
        if let (Some(source_array), Some(target_array)) = (
            source_col.as_any().downcast_ref::<StringArray>(),
            target_col.as_any().downcast_ref::<StringArray>()
        ) {
            // Extract weight values if specified
            let weight_array = self.config.weight_column.as_ref()
                .and_then(|col_name| {
                    batch.schema().fields().iter()
                        .position(|f| f.name() == col_name)
                        .and_then(|idx| batch.column(idx).as_any().downcast_ref::<Float64Array>())
                });
            
            for i in 0..source_array.len() {
                let source = source_array.value(i);
                let target = target_array.value(i);
                let (source, target) = (source, target);
                node_set.insert(source.to_string());
                node_set.insert(target.to_string());
                
                let weight = weight_array
                    .map(|arr| arr.value(i))
                    .unwrap_or(1.0);
                
                temp_edges.push((source.to_string(), target.to_string(), weight));
            }
        }
        
        // Create nodes
        let mut color_idx = 0;
        for node_id in node_set {
            let node_idx = self.graph.add_node(node_id.clone());
            self.node_indices.insert(node_id.clone(), node_idx);
            
            let node = NetworkNode {
                id: node_id.clone(),
                label: node_id.clone(),
                position: Point2::new(0.0, 0.0),
                velocity: Vector2::zeros(),
                size: self.config.node_size,
                color: categorical_color(color_idx),
                value: None,
                degree: 0,
            };
            
            self.nodes.insert(node_id, node);
            color_idx += 1;
        }
        
        // Create edges and update node degrees
        for (source, target, weight) in temp_edges {
            if let (Some(&source_idx), Some(&target_idx)) = (
                self.node_indices.get(&source),
                self.node_indices.get(&target)
            ) {
                self.graph.add_edge(source_idx, target_idx, weight);
                
                // Update degrees
                if let Some(source_node) = self.nodes.get_mut(&source) {
                    source_node.degree += 1;
                }
                if let Some(target_node) = self.nodes.get_mut(&target) {
                    target_node.degree += 1;
                }
                
                self.edges.push(NetworkEdge {
                    source: source.clone(),
                    target: target.clone(),
                    weight,
                    color: Color32::from_gray(150),
                });
            }
        }
        
        // Initialize layout
        self.initialize_layout();
    }
    
    fn initialize_layout(&mut self) {
        match self.config.layout_algorithm {
            LayoutAlgorithm::ForceDirected => {
                self.initialize_force_directed();
            }
            LayoutAlgorithm::Circular => {
                self.apply_circular_layout();
            }
            LayoutAlgorithm::Grid => {
                self.apply_grid_layout();
            }
            LayoutAlgorithm::Hierarchical => {
                self.apply_hierarchical_layout();
            }
            _ => {
                self.initialize_force_directed();
            }
        }
    }
    
    fn initialize_force_directed(&mut self) {
        // Simple random initialization for now
        let mut rng = rand::thread_rng();
        
        for (_, node) in &mut self.nodes {
            node.position = Point2::new(
                rng.gen_range(-200.0..200.0),
                rng.gen_range(-200.0..200.0)
            );
        }
    }
    
    fn apply_circular_layout(&mut self) {
        let n = self.nodes.len();
        let radius = 200.0;
        let center = Point2::new(0.0, 0.0);
        
        for (i, (_, node)) in self.nodes.iter_mut().enumerate() {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / n as f32;
            node.position = center + Vector2::new(
                radius * angle.cos(),
                radius * angle.sin()
            );
        }
    }
    
    fn apply_grid_layout(&mut self) {
        let n = self.nodes.len();
        let cols = (n as f32).sqrt().ceil() as usize;
        let spacing = 50.0;
        
        for (i, (_, node)) in self.nodes.iter_mut().enumerate() {
            let row = i / cols;
            let col = i % cols;
            node.position = Point2::new(
                col as f32 * spacing - (cols as f32 * spacing / 2.0),
                row as f32 * spacing - (n / cols) as f32 * spacing / 2.0
            );
        }
    }
    
    fn apply_hierarchical_layout(&mut self) {
        // Simple hierarchical layout based on BFS
        let mut visited = HashSet::new();
        let mut layers: Vec<Vec<String>> = Vec::new();
        
        // Find root nodes (no incoming edges or highest degree)
        let root = self.nodes.iter()
            .max_by_key(|(_, node)| node.degree)
            .map(|(id, _)| id.clone())
            .unwrap_or_default();
        
        // BFS to create layers
        let mut queue = vec![root];
        visited.insert(queue[0].clone());
        
        while !queue.is_empty() {
            let current_layer = queue.clone();
            layers.push(current_layer.clone());
            queue.clear();
            
            for node_id in current_layer {
                if let Some(&node_idx) = self.node_indices.get(&node_id) {
                    for neighbor in self.graph.neighbors(node_idx) {
                        let neighbor_id = &self.graph[neighbor];
                        if !visited.contains(neighbor_id) {
                            visited.insert(neighbor_id.clone());
                            queue.push(neighbor_id.clone());
                        }
                    }
                }
            }
        }
        
        // Position nodes by layer
        let layer_spacing = 100.0;
        for (layer_idx, layer) in layers.iter().enumerate() {
            let y = layer_idx as f32 * layer_spacing - (layers.len() as f32 * layer_spacing / 2.0);
            let node_spacing = 80.0;
            
            for (node_idx, node_id) in layer.iter().enumerate() {
                if let Some(node) = self.nodes.get_mut(node_id) {
                    let x = node_idx as f32 * node_spacing - (layer.len() as f32 * node_spacing / 2.0);
                    node.position = Point2::new(x, y);
                }
            }
        }
    }
    
    fn update_layout(&mut self) {
        if self.config.layout_algorithm != LayoutAlgorithm::ForceDirected {
            return;
        }
        
        // Simple spring force simulation without fdg-sim
        let dt = 0.1;
        let repulsion = self.config.node_repulsion;
        let attraction = self.config.edge_attraction;
        
        // Compute forces
        let mut forces: HashMap<String, Vector2<f32>> = HashMap::new();
        
        // Repulsive forces between all nodes
        for (id1, node1) in &self.nodes {
            let mut force = Vector2::zeros();
            
            for (id2, node2) in &self.nodes {
                if id1 != id2 {
                    let diff = node1.position - node2.position;
                    let dist = diff.norm();
                    if dist > 0.0 {
                        let f = (repulsion / (dist * dist)).min(100.0);
                        force += diff.normalize() * f;
                    }
                }
            }
            
            forces.insert(id1.clone(), force);
        }
        
        // Attractive forces along edges
        for edge in &self.edges {
            if let (Some(source), Some(target)) = (
                self.nodes.get(&edge.source),
                self.nodes.get(&edge.target)
            ) {
                let diff = target.position - source.position;
                let dist = diff.norm();
                if dist > 0.0 {
                    let f = dist * attraction;
                    let force = diff.normalize() * f;
                    
                    if let Some(source_force) = forces.get_mut(&edge.source) {
                        *source_force += force;
                    }
                    if let Some(target_force) = forces.get_mut(&edge.target) {
                        *target_force -= force;
                    }
                }
            }
        }
        
        // Update positions
        for (id, force) in forces {
            if let Some(node) = self.nodes.get_mut(&id) {
                node.velocity = node.velocity * 0.85 + force * dt;
                node.position += node.velocity * dt;
            }
        }
    }
    
    fn world_to_screen(&self, world_pos: Point2<f32>, rect: &Rect) -> Pos2 {
        let centered = world_pos * self.zoom;
        Pos2::new(
            rect.center().x + centered.x + self.pan_offset.x,
            rect.center().y + centered.y + self.pan_offset.y
        )
    }
    
    fn screen_to_world(&self, screen_pos: Pos2, rect: &Rect) -> Point2<f32> {
        let offset = screen_pos - rect.center() - self.pan_offset;
        Point2::new(offset.x / self.zoom, offset.y / self.zoom)
    }
    
    fn draw_edges(&self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        
        for edge in &self.edges {
            if let (Some(source_node), Some(target_node)) = (
                self.nodes.get(&edge.source),
                self.nodes.get(&edge.target)
            ) {
                let source_pos = self.world_to_screen(source_node.position, &rect);
                let target_pos = self.world_to_screen(target_node.position, &rect);
                
                // Check if edge is connected to hovered/selected node
                let is_highlighted = self.hovered_node.as_ref() == Some(&edge.source) ||
                    self.hovered_node.as_ref() == Some(&edge.target) ||
                    self.selected_node.as_ref() == Some(&edge.source) ||
                    self.selected_node.as_ref() == Some(&edge.target);
                
                let color = if is_highlighted {
                    Color32::from_rgba_unmultiplied(255, 165, 0, 200)
                } else if self.hovered_node.is_some() && !is_highlighted {
                    Color32::from_rgba_unmultiplied(150, 150, 150, 50)
                } else {
                    edge.color
                };
                
                let width = if is_highlighted {
                    self.config.edge_width * 2.0
                } else {
                    self.config.edge_width * (edge.weight as f32).sqrt()
                };
                
                painter.line_segment([source_pos, target_pos], Stroke::new(width, color));
                
                // Draw arrow if directed
                if self.config.show_arrows {
                    let dir = (target_pos - source_pos).normalized();
                    let arrow_base = target_pos - dir * (target_node.size + 5.0);
                    let arrow_size = 10.0;
                    let perp = Vec2::new(-dir.y, dir.x);
                    
                    let arrow_points = vec![
                        target_pos - dir * target_node.size,
                        arrow_base - perp * arrow_size * 0.5,
                        arrow_base + perp * arrow_size * 0.5,
                    ];
                    
                    painter.add(Shape::convex_polygon(arrow_points, color, Stroke::NONE));
                }
            }
        }
    }
    
    fn draw_nodes(&self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        
        for (node_id, node) in &self.nodes {
            let pos = self.world_to_screen(node.position, &rect);
            
            // Node appearance based on state
            let is_selected = self.selected_node.as_ref() == Some(node_id);
            let is_hovered = self.hovered_node.as_ref() == Some(node_id);
            
            let node_color = if is_selected || is_hovered {
                node.color
            } else if self.hovered_node.is_some() {
                // Check if neighbor of hovered node
                let is_neighbor = if let Some(hovered) = &self.hovered_node {
                    self.edges.iter().any(|e| 
                        (e.source == *hovered && e.target == *node_id) ||
                        (e.target == *hovered && e.source == *node_id)
                    )
                } else {
                    false
                };
                
                if is_neighbor {
                    Color32::from_rgba_unmultiplied(node.color.r(), node.color.g(), node.color.b(), 200)
                } else {
                    Color32::from_rgba_unmultiplied(node.color.r(), node.color.g(), node.color.b(), 50)
                }
            } else {
                node.color
            };
            
            // Size based on degree
            let size = node.size * (1.0 + node.degree as f32 * 0.1).min(3.0);
            
            // Draw node
            painter.circle_filled(pos, size, node_color);
            
            if is_selected {
                painter.circle_stroke(pos, size + 3.0, Stroke::new(2.0, Color32::WHITE));
            }
            
            // Draw label
            if self.config.show_labels {
                painter.text(
                    pos + Vec2::new(0.0, size + 5.0),
                    Align2::CENTER_TOP,
                    &node.label,
                    FontId::proportional(10.0),
                    Color32::from_gray(200),
                );
            }
        }
    }
    
    fn handle_interaction(&mut self, ui: &mut Ui, rect: Rect) -> Response {
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        
        // Update layout
        self.update_layout();
        
        // Handle hover
        if let Some(hover_pos) = response.hover_pos() {
            self.hovered_node = None;
            let world_pos = self.screen_to_world(hover_pos, &rect);
            
            for (node_id, node) in &self.nodes {
                let screen_pos = self.world_to_screen(node.position, &rect);
                let dist = (screen_pos - hover_pos).length();
                
                if dist < node.size + 5.0 {
                    self.hovered_node = Some(node_id.clone());
                    
                    // Tooltip
                    let mut tooltip = format!("{}\nDegree: {}", node.label, node.degree);
                    if let Some(value) = node.value {
                        tooltip.push_str(&format!("\nValue: {:.2}", value));
                    }
                    response.clone().on_hover_text(tooltip);
                    break;
                }
            }
        }
        
        // Handle click
        if response.clicked() {
            self.selected_node = self.hovered_node.clone();
        }
        
        // Handle drag
        if response.dragged() {
            if self.dragging_node.is_none() && self.hovered_node.is_some() {
                self.dragging_node = self.hovered_node.clone();
            }
            
            if let Some(drag_node_id) = &self.dragging_node {
                // Drag node
                if let Some(node) = self.nodes.get_mut(drag_node_id) {
                    let delta = response.drag_delta() / self.zoom;
                    node.position += Vector2::new(delta.x, delta.y);
                }
            } else {
                // Pan view
                self.pan_offset += response.drag_delta();
            }
        } else {
            self.dragging_node = None;
        }
        
        // Handle zoom
        let scroll_delta = ui.input(|i| i.scroll_delta.y);
        if scroll_delta != 0.0 {
            self.zoom = (self.zoom * (1.0 + scroll_delta * 0.01)).clamp(0.1, 5.0);
        }
        
        response
    }
    
    fn draw_info_panel(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(format!("Nodes: {}", self.nodes.len()));
            ui.label(format!("Edges: {}", self.edges.len()));
            ui.separator();
            ui.label("🖱 Drag nodes or pan");
            ui.label("📏 Scroll to zoom");
            ui.label("🎯 Click to select");
            
            if self.config.layout_algorithm == LayoutAlgorithm::ForceDirected {
                ui.separator();
                ui.label("⚡ Force simulation active");
            }
        });
    }
}

impl SpaceView for NetworkGraph {
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
    fn view_type(&self) -> &str { "NetworkView" }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Update data if needed
        if self.cached_data.is_none() {
            let data_sources = ctx.data_sources.read();

            let data_source = data_sources.values().next();
            if let Some(source) = data_source.as_ref() {
                let nav_pos = ctx.navigation.get_context().position.clone();
                if let Ok(batch) = ctx.runtime_handle.block_on(source.query_at(&nav_pos)) {
                    self.cached_data = Some(batch.clone());
                    self.extract_network_data(&batch);
                }
            }
        }
        
        if self.cached_data.is_some() && !self.nodes.is_empty() {
            // Main drawing area
            let available_rect = ui.available_rect_before_wrap();
            let graph_rect = Rect::from_min_size(
                available_rect.left_top(),
                available_rect.size() - Vec2::new(0.0, 30.0)
            );
            
            // Background
            ui.painter().rect_filled(graph_rect, Rounding::ZERO, Color32::from_gray(20));
            
            // Draw graph
            self.draw_edges(ui, graph_rect);
            self.draw_nodes(ui, graph_rect);
            
            // Handle interaction
            self.handle_interaction(ui, graph_rect);
            
            // Info panel
            self.draw_info_panel(ui);
            
            // Request repaint for animation
            if self.config.layout_algorithm == LayoutAlgorithm::ForceDirected {
                ui.ctx().request_repaint();
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No network data available. Please configure source and target columns.");
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "source_column": self.config.source_column,
            "target_column": self.config.target_column,
            "weight_column": self.config.weight_column,
            "layout_algorithm": format!("{:?}", self.config.layout_algorithm),
            "node_size": self.config.node_size,
            "edge_width": self.config.edge_width,
            "show_labels": self.config.show_labels,
            "show_arrows": self.config.show_arrows,
            "pan_offset_x": self.pan_offset.x,
            "pan_offset_y": self.pan_offset.y,
            "zoom": self.zoom,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(source) = config.get("source_column").and_then(|v| v.as_str()) {
            self.config.source_column = source.to_string();
        }
        if let Some(target) = config.get("target_column").and_then(|v| v.as_str()) {
            self.config.target_column = target.to_string();
        }
        if let Some(weight) = config.get("weight_column").and_then(|v| v.as_str()) {
            self.config.weight_column = Some(weight.to_string());
        }
        if let Some(x) = config.get("pan_offset_x").and_then(|v| v.as_f64()) {
            self.pan_offset.x = x as f32;
        }
        if let Some(y) = config.get("pan_offset_y").and_then(|v| v.as_f64()) {
            self.pan_offset.y = y as f32;
        }
        if let Some(zoom) = config.get("zoom").and_then(|v| v.as_f64()) {
            self.zoom = zoom as f32;
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {}
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {}
}

// Re-export NetworkConfig as NetworkGraphConfig
pub use NetworkConfig as NetworkGraphConfig; 