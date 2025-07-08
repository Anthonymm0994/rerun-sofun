//! Treemap visualization for hierarchical data

use egui::{Ui, Color32, Rect, Pos2, Vec2, Stroke, FontId, Align2, Shape, Response, Sense, Rounding};
use arrow::record_batch::RecordBatch;
use arrow::array::{Array, Float64Array, StringArray};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use super::utils::{ColorScheme, categorical_color, viridis_color, plasma_color};

/// Treemap configuration
#[derive(Debug, Clone)]
pub struct TreemapConfig {
    pub data_source_id: Option<String>,
    
    pub path_column: String,    // Hierarchical path (e.g., "A/B/C")
    pub value_column: String,   // Size of rectangles
    pub color_column: Option<String>, // Optional color mapping
    pub label_column: Option<String>, // Optional custom labels
    
    // Layout options
    pub layout_algorithm: TreemapLayout,
    pub aspect_ratio: f32,
    pub padding: f32,
    pub min_cell_size: f32,
    
    // Visual options
    pub color_scheme: ColorScheme,
    pub show_labels: bool,
    pub label_threshold: f32, // Min size to show labels
    pub gradient_depth: bool, // Color gradient by depth
    pub border_width: f32,
    pub hover_highlight: bool,
    
    // Interaction
    pub enable_zoom: bool,
    pub breadcrumb: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TreemapLayout {
    Squarify,    // Best aspect ratios
    Slice,       // Horizontal slices
    Dice,        // Vertical slices
    SliceDice,   // Alternating
}

impl Default for TreemapConfig {
    fn default() -> Self {
        Self {
            data_source_id: None,
            
            path_column: String::new(),
            value_column: String::new(),
            color_column: None,
            label_column: None,
            layout_algorithm: TreemapLayout::Squarify,
            aspect_ratio: 1.0,
            padding: 2.0,
            min_cell_size: 20.0,
            color_scheme: ColorScheme::Categorical,
            show_labels: true,
            label_threshold: 50.0,
            gradient_depth: true,
            border_width: 1.0,
            hover_highlight: true,
            enable_zoom: true,
            breadcrumb: true,
        }
    }
}

/// Hierarchical node
#[derive(Clone, Debug)]
struct TreeNode {
    id: String,
    label: String,
    value: f64,
    color_value: Option<f64>,
    children: Vec<TreeNode>,
    parent: Option<String>,
    depth: usize,
    rect: Option<Rect>,
    is_leaf: bool,
}

impl TreeNode {
    fn new(id: String, label: String) -> Self {
        Self {
            id,
            label,
            value: 0.0,
            color_value: None,
            children: Vec::new(),
            parent: None,
            depth: 0,
            rect: None,
            is_leaf: true,
        }
    }
    
    fn total_value(&self) -> f64 {
        if self.is_leaf {
            self.value
        } else {
            self.children.iter().map(|c| c.total_value()).sum()
        }
    }
    
    fn find_node(&self, id: &str) -> Option<&TreeNode> {
        if self.id == id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(node) = child.find_node(id) {
                return Some(node);
            }
        }
        None
    }
    
    fn find_node_mut(&mut self, id: &str) -> Option<&mut TreeNode> {
        if self.id == id {
            return Some(self);
        }
        for child in &mut self.children {
            if let Some(node) = child.find_node_mut(id) {
                return Some(node);
            }
        }
        None
    }
}

/// Treemap view
pub struct Treemap {
    id: SpaceViewId,
    title: String,
    pub config: TreemapConfig,
    
    // State
    cached_data: Option<RecordBatch>,
    root: Option<TreeNode>,
    current_root: String,
    
    // Interaction state
    hovered_node: Option<String>,
    selected_node: Option<String>,
    zoom_stack: Vec<String>,
}

impl Treemap {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: TreemapConfig::default(),
            cached_data: None,
            root: None,
            current_root: String::from("root"),
            hovered_node: None,
            selected_node: None,
            zoom_stack: vec![String::from("root")],
        }
    }
    
    fn build_hierarchy(&mut self, batch: &RecordBatch) {
        // Find columns
        let path_idx = batch.schema().fields().iter()
            .position(|f| f.name() == &self.config.path_column);
        let value_idx = batch.schema().fields().iter()
            .position(|f| f.name() == &self.config.value_column);
            
        if path_idx.is_none() || value_idx.is_none() {
            return;
        }
        
        let path_col = batch.column(path_idx.unwrap());
        let value_col = batch.column(value_idx.unwrap());
        
        // Extract color values if specified
        let color_array = self.config.color_column.as_ref()
            .and_then(|col_name| {
                batch.schema().fields().iter()
                    .position(|f| f.name() == col_name)
                    .and_then(|idx| batch.column(idx).as_any().downcast_ref::<Float64Array>())
            });
        
        // Extract label values if specified
        let label_array = self.config.label_column.as_ref()
            .and_then(|col_name| {
                batch.schema().fields().iter()
                    .position(|f| f.name() == col_name)
                    .and_then(|idx| batch.column(idx).as_any().downcast_ref::<StringArray>())
            });
        
        // Build tree
        let mut root = TreeNode::new("root".to_string(), "Root".to_string());
        root.is_leaf = false;
        
        if let (Some(path_array), Some(value_array)) = (
            path_col.as_any().downcast_ref::<StringArray>(),
            value_col.as_any().downcast_ref::<Float64Array>()
        ) {
            for i in 0..path_array.len() {
                let path = path_array.value(i);
                let value = value_array.value(i);
                let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
                if parts.is_empty() {
                    continue;
                }
                
                // Navigate to or create path
                let mut current = &mut root;
                let mut full_path = String::new();
                
                for (depth, part) in parts.iter().enumerate() {
                    if !full_path.is_empty() {
                        full_path.push('/');
                    }
                    full_path.push_str(part);
                    
                    let is_leaf = depth == parts.len() - 1;
                    
                    // Find or create child
                    let child_exists = current.children.iter().any(|c| c.label == *part);
                    if !child_exists {
                        let mut new_node = TreeNode::new(full_path.clone(), part.to_string());
                        new_node.parent = Some(current.id.clone());
                        new_node.depth = depth + 1;
                        new_node.is_leaf = is_leaf;
                        current.children.push(new_node);
                        current.is_leaf = false;
                    }
                    
                    // Move to child
                    let child_idx = current.children.iter().position(|c| c.label == *part).unwrap();
                    if is_leaf {
                        // Set leaf values
                        current.children[child_idx].value = value;
                        
                        if let Some(color_arr) = &color_array {
                            current.children[child_idx].color_value = Some(color_arr.value(i));
                        }
                        
                        if let Some(label_arr) = &label_array {
                            let custom_label = label_arr.value(i);
                            current.children[child_idx].label = custom_label.to_string();
                        }
                    } else {
                        // Navigate deeper - need to get mutable reference differently
                        let child_id = current.children[child_idx].id.clone();
                        current = root.find_node_mut(&child_id).unwrap();
                    }
                }
            }
        }
        
        self.root = Some(root);
    }
    
    fn layout_treemap(&mut self, node: &mut TreeNode, rect: Rect) {
        node.rect = Some(rect);
        
        if node.children.is_empty() {
            return;
        }
        
        // Apply padding
        let padded_rect = Rect::from_min_size(
            rect.min + Vec2::splat(self.config.padding),
            (rect.size() - Vec2::splat(self.config.padding * 2.0)).max(Vec2::ZERO)
        );
        
        match self.config.layout_algorithm {
            TreemapLayout::Squarify => self.squarify_layout(node, padded_rect),
            TreemapLayout::Slice => self.slice_layout(node, padded_rect, true),
            TreemapLayout::Dice => self.slice_layout(node, padded_rect, false),
            TreemapLayout::SliceDice => self.slice_dice_layout(node, padded_rect, node.depth),
        }
    }
    
    fn squarify_layout(&mut self, node: &mut TreeNode, rect: Rect) {
        if node.children.is_empty() || rect.area() <= 0.0 {
            return;
        }
        
        // Sort children by value (descending)
        node.children.sort_by(|a, b| b.total_value().partial_cmp(&a.total_value()).unwrap());
        
        let total_value: f64 = node.children.iter().map(|c| c.total_value()).sum();
        if total_value <= 0.0 {
            return;
        }
        
        let mut remaining_rect = rect;
        let mut i = 0;
        
        while i < node.children.len() && remaining_rect.area() > 0.0 {
            let mut row = vec![i];
            let mut row_value = node.children[i].total_value();
            
            // Try to add more items to improve aspect ratio
            while row.last().map(|&idx| idx + 1).unwrap_or(node.children.len()) < node.children.len() {
                let next_idx = row.last().unwrap() + 1;
                let next_value = row_value + node.children[next_idx].total_value();
                
                let current_ratio = self.worst_aspect_ratio(&node.children, &row, row_value, remaining_rect, total_value);
                let mut test_row = row.clone();
                test_row.push(next_idx);
                let next_ratio = self.worst_aspect_ratio(&node.children, &test_row, next_value, remaining_rect, total_value);
                
                if next_ratio > current_ratio {
                    break;
                }
                
                row.push(next_idx);
                row_value = next_value;
            }
            
            // Layout this row
            let row_ratio = row_value / total_value;
            let (row_rect, new_remaining) = if remaining_rect.width() > remaining_rect.height() {
                // Vertical slice
                let split_x = remaining_rect.min.x + remaining_rect.width() * row_ratio as f32;
                (
                    Rect::from_min_max(remaining_rect.min, Pos2::new(split_x, remaining_rect.max.y)),
                    Rect::from_min_max(Pos2::new(split_x, remaining_rect.min.y), remaining_rect.max)
                )
            } else {
                // Horizontal slice
                let split_y = remaining_rect.min.y + remaining_rect.height() * row_ratio as f32;
                (
                    Rect::from_min_max(remaining_rect.min, Pos2::new(remaining_rect.max.x, split_y)),
                    Rect::from_min_max(Pos2::new(remaining_rect.min.x, split_y), remaining_rect.max)
                )
            };
            
            // Layout items in row
            self.layout_row(&mut node.children, &row, row_rect, row_value);
            
            remaining_rect = new_remaining;
            i = row.last().unwrap() + 1;
        }
        
        // Recursively layout children
        for child in &mut node.children {
            if let Some(child_rect) = child.rect {
                self.layout_treemap(child, child_rect);
            }
        }
    }
    
    fn worst_aspect_ratio(&self, children: &[TreeNode], indices: &[usize], total_value: f64, rect: Rect, parent_total: f64) -> f32 {
        let area_ratio = total_value / parent_total;
        let rect_area = rect.area() * area_ratio as f32;
        
        let is_vertical = rect.width() > rect.height();
        let side = if is_vertical { rect.height() } else { rect.width() };
        
        let mut worst = 0.0_f32;
        for &idx in indices {
            let item_ratio = children[idx].total_value() / total_value;
            let item_area = rect_area * item_ratio as f32;
            let other_side = item_area / side;
            
            let aspect = (side / other_side).max(other_side / side);
            worst = worst.max(aspect);
        }
        
        worst
    }
    
    fn layout_row(&mut self, children: &mut [TreeNode], indices: &[usize], rect: Rect, row_value: f64) {
        if indices.is_empty() || row_value <= 0.0 {
            return;
        }
        
        let is_vertical = rect.width() <= rect.height();
        let mut current_pos = rect.min;
        
        for &idx in indices {
            let child_ratio = children[idx].total_value() / row_value;
            
            let child_rect = if is_vertical {
                let height = rect.height() * child_ratio as f32;
                let r = Rect::from_min_size(current_pos, Vec2::new(rect.width(), height));
                current_pos.y += height;
                r
            } else {
                let width = rect.width() * child_ratio as f32;
                let r = Rect::from_min_size(current_pos, Vec2::new(width, rect.height()));
                current_pos.x += width;
                r
            };
            
            children[idx].rect = Some(child_rect);
        }
    }
    
    fn slice_layout(&mut self, node: &mut TreeNode, rect: Rect, horizontal: bool) {
        if node.children.is_empty() {
            return;
        }
        
        let total_value: f64 = node.children.iter().map(|c| c.total_value()).sum();
        if total_value <= 0.0 {
            return;
        }
        
        let mut current_pos = rect.min;
        
        for child in &mut node.children {
            let ratio = child.total_value() / total_value;
            
            let child_rect = if horizontal {
                let width = rect.width() * ratio as f32;
                let r = Rect::from_min_size(current_pos, Vec2::new(width, rect.height()));
                current_pos.x += width;
                r
            } else {
                let height = rect.height() * ratio as f32;
                let r = Rect::from_min_size(current_pos, Vec2::new(rect.width(), height));
                current_pos.y += height;
                r
            };
            
            child.rect = Some(child_rect);
            
            // Recursively layout children
            self.layout_treemap(child, child_rect);
        }
    }
    
    fn slice_dice_layout(&mut self, node: &mut TreeNode, rect: Rect, depth: usize) {
        // Alternate between horizontal and vertical slicing
        self.slice_layout(node, rect, depth % 2 == 0);
    }
    
    fn get_node_color(&self, node: &TreeNode) -> Color32 {
        if let Some(color_val) = node.color_value {
            // Map color value
            let normalized = (color_val / 100.0) as f32; // TODO: proper normalization
            match self.config.color_scheme {
                ColorScheme::Viridis => viridis_color(normalized),
                ColorScheme::Plasma => plasma_color(normalized),
                _ => categorical_color(node.depth),
            }
        } else if self.config.gradient_depth {
            // Color by depth with gradient
            let base_color = categorical_color(node.depth % 10);
            let depth_factor = 1.0 - (node.depth as f32 * 0.1).min(0.5);
            Color32::from_rgba_unmultiplied(
                (base_color.r() as f32 * depth_factor) as u8,
                (base_color.g() as f32 * depth_factor) as u8,
                (base_color.b() as f32 * depth_factor) as u8,
                base_color.a()
            )
        } else {
            categorical_color(node.id.len())
        }
    }
    
    fn draw_node(&self, ui: &mut Ui, node: &TreeNode, visible_rect: Rect) {
        if let Some(rect) = node.rect {
            // Skip if outside visible area
            if !rect.intersects(visible_rect) {
                return;
            }
            
            // Skip if too small
            if rect.area() < self.config.min_cell_size * self.config.min_cell_size {
                return;
            }
            
            let painter = ui.painter();
            
            // Determine if highlighted
            let is_hovered = self.hovered_node.as_ref() == Some(&node.id);
            let is_selected = self.selected_node.as_ref() == Some(&node.id);
            
            // Get color
            let mut color = self.get_node_color(node);
            if is_hovered && self.config.hover_highlight {
                color = Color32::from_rgba_unmultiplied(
                    color.r().saturating_add(30),
                    color.g().saturating_add(30),
                    color.b().saturating_add(30),
                    color.a()
                );
            }
            
            // Draw rectangle
            painter.rect_filled(rect, Rounding::ZERO, color);
            
            // Draw border
            if self.config.border_width > 0.0 {
                let border_color = if is_selected {
                    Color32::WHITE
                } else {
                    Color32::from_gray(50)
                };
                painter.rect_stroke(rect, Rounding::ZERO, Stroke::new(self.config.border_width, border_color));
            }
            
            // Draw label if large enough
            if self.config.show_labels && rect.area() > self.config.label_threshold * self.config.label_threshold {
                let text_color = if color.r() as u32 + color.g() as u32 + color.b() as u32 > 384 {
                    Color32::BLACK
                } else {
                    Color32::WHITE
                };
                
                // Clip text to rectangle
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    &node.label,
                    FontId::proportional(12.0),
                    text_color,
                );
                
                // Show value if space permits
                if rect.height() > 30.0 {
                    let value_text = format!("{:.1}", node.total_value());
                    painter.text(
                        rect.center() + Vec2::new(0.0, 10.0),
                        Align2::CENTER_CENTER,
                        value_text,
                        FontId::proportional(10.0),
                        text_color,
                    );
                }
            }
        }
        
        // Draw children
        for child in &node.children {
            self.draw_node(ui, child, visible_rect);
        }
    }
    
    fn handle_interaction(&mut self, ui: &mut Ui, rect: Rect) -> Response {
        let response = ui.allocate_rect(rect, Sense::click());
        
        // Handle hover detection
        if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
            let mut tooltip: Option<String> = None;
            
            if let Some(root) = &self.root {
                if let Some(current) = root.find_node(&self.current_root) {
                    tooltip = self.find_hovered_tooltip(current, hover_pos);
                }
            }
            
            if let Some(tooltip_text) = tooltip {
                response.clone().on_hover_text(tooltip_text);
            }
        }
        
        // Handle click
        if response.clicked() {
            if let Some(hovered_id) = &self.hovered_node {
                if self.config.enable_zoom {
                    // Zoom into node
                    self.current_root = hovered_id.clone();
                    self.zoom_stack.push(hovered_id.clone());
                    
                    // Re-layout from new root
                    if let Some(root) = &mut self.root {
                        if let Some(new_root) = root.find_node_mut(&self.current_root) {
                            Self::layout_treemap_static(new_root, rect);
                        }
                    }
                } else {
                    // Just select
                    self.selected_node = Some(hovered_id.clone());
                }
            }
        }
        
        // Handle right-click to zoom out
        response.clone().context_menu(|ui| {
            if self.zoom_stack.len() > 1 {
                if ui.button("Zoom Out").clicked() {
                    self.zoom_stack.pop();
                    if let Some(new_root) = self.zoom_stack.last() {
                        self.current_root = new_root.clone();
                    }
                }
            }
            
            if ui.button("Reset Zoom").clicked() {
                self.zoom_stack.clear();
                self.zoom_stack.push(String::new()); // Root path
                self.current_root = String::new();
            }
        });
        
        response
    }
    
    fn find_hovered_node(&mut self, node: &TreeNode, pos: Pos2) {
        if let Some(rect) = node.rect {
            if rect.contains(pos) {
                // Check children first (they're on top)
                for child in &node.children {
                    self.find_hovered_node(child, pos);
                }
                
                // If no child contains the point, this node is hovered
                if self.hovered_node.is_none() {
                    self.hovered_node = Some(node.id.clone());
                }
            }
        }
    }
    
    fn draw_breadcrumb(&self, ui: &mut Ui) {
        if !self.config.breadcrumb || self.zoom_stack.len() <= 1 {
            return;
        }
        
        ui.horizontal(|ui| {
            for (i, node_id) in self.zoom_stack.iter().enumerate() {
                let label = if node_id == "root" {
                    "Home".to_string()
                } else if let Some(root) = &self.root {
                    root.find_node(node_id)
                        .map(|n| n.label.clone())
                        .unwrap_or_else(|| node_id.clone())
                } else {
                    node_id.clone()
                };
                
                if ui.link(&label).clicked() {
                    // Navigate to this level
                    // This will be handled in the next frame
                }
                
                if i < self.zoom_stack.len() - 1 {
                    ui.label(">");
                }
            }
        });
    }
    
    fn find_hovered_tooltip(&self, node: &TreeNode, hover_pos: Pos2) -> Option<String> {
        if let Some(rect) = node.rect {
            if rect.contains(hover_pos) {
                if node.children.is_empty() {
                    return Some(format!("{}: {:.2}", node.label, node.value));
                } else {
                    for child in &node.children {
                        if let Some(tooltip) = self.find_hovered_tooltip(child, hover_pos) {
                            return Some(tooltip);
                        }
                    }
                }
            }
        }
        None
    }
    
    fn handle_treemap_click(&mut self, rect: Rect) {
        if let Some(root) = &mut self.root {
            if let Some(new_root) = root.find_node_mut(&self.current_root) {
                // Use a static method to avoid borrowing self
                Self::layout_treemap_static(new_root, rect);
            }
        }
    }
    
    fn layout_treemap_static(node: &mut TreeNode, rect: Rect) {
        if node.children.is_empty() {
            node.rect = Some(rect);
            return;
        }
        
        // Simple squarify algorithm
        let total_value: f64 = node.children.iter().map(|c| c.value).sum();
        if total_value == 0.0 {
            return;
        }
        
        let mut current_rect = rect;
        for child in &mut node.children {
            let ratio = child.value / total_value;
            let area = rect.width() as f64 * rect.height() as f64 * ratio;
            
            // Simple horizontal split
            let width = (area / current_rect.height() as f64) as f32;
            let child_rect = Rect::from_min_size(
                current_rect.left_top(),
                Vec2::new(width.min(current_rect.width()), current_rect.height())
            );
            
            child.rect = Some(child_rect);
            current_rect = Rect::from_min_size(
                current_rect.left_top() + Vec2::new(width, 0.0),
                Vec2::new((current_rect.width() - width).max(0.0), current_rect.height())
            );
        }
    }
}

impl SpaceView for Treemap {
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
    fn view_type(&self) -> &str { "TreemapView" }
    
    fn set_data_source(&mut self, source_id: String) {
        self.config.data_source_id = Some(source_id);
        // Clear any cached data
        if let Some(cache_field) = self.as_any_mut().downcast_mut::<Self>() {
            // Reset cached data if the plot has any
        }
    }
    
    fn data_source_id(&self) -> Option<&str> {
        self.config.data_source_id.as_deref()
    }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Update data if needed
        if self.cached_data.is_none() {
            let data_sources = ctx.data_sources.read();

            let data_source = data_sources.values().next();
            if let Some(source) = data_source.as_ref() {
                let nav_pos = ctx.navigation.get_context().position.clone();
                if let Ok(batch) = ctx.runtime_handle.block_on(source.query_at(&nav_pos)) {
                    self.cached_data = Some(batch.clone());
                    self.build_hierarchy(&batch);
                }
            }
        }
        
        if let Some(root) = &self.root {
            // Draw breadcrumb first using immutable borrow
            self.draw_breadcrumb(ui);
            
            // Get current root path for finding the node
            let current_root = self.current_root.clone();
            let layout_rect = ui.available_rect_before_wrap();
            
            // First do the mutable layout operation
            if let Some(root) = &mut self.root {
                if let Some(current_node) = root.find_node_mut(&current_root) {
                    Self::layout_treemap_static(current_node, layout_rect);
                }
            }
            
            // Then do the immutable drawing operation
            if let Some(root) = &self.root {
                if let Some(current_node) = root.find_node(&current_root) {
                    self.draw_node(ui, current_node, layout_rect);
                }
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No hierarchical data available. Please configure path and value columns.");
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "path_column": self.config.path_column,
            "value_column": self.config.value_column,
            "color_column": self.config.color_column,
            "label_column": self.config.label_column,
            "layout_algorithm": format!("{:?}", self.config.layout_algorithm),
            "aspect_ratio": self.config.aspect_ratio,
            "padding": self.config.padding,
            "color_scheme": format!("{:?}", self.config.color_scheme),
            "show_labels": self.config.show_labels,
            "enable_zoom": self.config.enable_zoom,
            "current_root": self.current_root,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(path) = config.get("path_column").and_then(|v| v.as_str()) {
            self.config.path_column = path.to_string();
        }
        if let Some(value) = config.get("value_column").and_then(|v| v.as_str()) {
            self.config.value_column = value.to_string();
        }
        if let Some(root) = config.get("current_root").and_then(|v| v.as_str()) {
            self.current_root = root.to_string();
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {}
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {}
}

// Create alias for consistent naming with other plots
pub type TreemapView = Treemap; 