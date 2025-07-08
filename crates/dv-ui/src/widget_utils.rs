//! Widget utilities for managing IDs and preventing conflicts
//!
//! This module provides utilities to help manage widget IDs in egui applications,
//! preventing the "multiple widgets with same ID" issues that can occur in debug builds.

use egui::{Id, ScrollArea, Grid};
use std::fmt::Display;

/// Widget ID builder that ensures unique IDs by combining multiple components
pub struct WidgetId {
    components: Vec<String>,
}

impl WidgetId {
    /// Create a new widget ID builder
    pub fn new(base: impl Display) -> Self {
        Self {
            components: vec![base.to_string()],
        }
    }
    
    /// Add a component to the ID
    pub fn with(mut self, component: impl Display) -> Self {
        self.components.push(component.to_string());
        self
    }
    
    /// Add an index to the ID (useful in loops)
    pub fn index(self, idx: usize) -> Self {
        self.with(format!("idx_{}", idx))
    }
    
    /// Build the final ID string
    pub fn build(&self) -> String {
        self.components.join("_")
    }
    
    /// Create an egui ID from this widget ID
    pub fn id(&self) -> Id {
        Id::new(self.build())
    }
}

/// Extension trait for ScrollArea to easily add unique IDs
pub trait ScrollAreaExt {
    /// Set the ID source using a WidgetId builder
    fn id_builder(self, builder: WidgetId) -> Self;
}

impl ScrollAreaExt for ScrollArea {
    fn id_builder(self, builder: WidgetId) -> Self {
        self.id_source(builder.build())
    }
}

/// Extension trait for Grid to easily add unique IDs
pub trait GridExt {
    /// Create a new grid with a WidgetId builder
    fn new_with_id(builder: WidgetId) -> Self;
}

impl GridExt for Grid {
    fn new_with_id(builder: WidgetId) -> Self {
        Grid::new(builder.build())
    }
}

/// Helper function to create a unique widget ID for a given context
/// 
/// # Example
/// ```ignore
/// // In a loop
/// for (idx, item) in items.iter().enumerate() {
///     let scroll_id = widget_id("config_scroll", idx);
///     ScrollArea::vertical()
///         .id_source(scroll_id)
///         .show(ui, |ui| {
///             // content
///         });
/// }
/// ```
pub fn widget_id(base: impl Display, suffix: impl Display) -> String {
    format!("{}_{}", base, suffix)
}

/// Helper function for creating widget IDs in nested contexts
/// 
/// # Example
/// ```ignore
/// let id = nested_widget_id(&["file_config", "csv", "column_scroll"], idx);
/// ```
pub fn nested_widget_id(components: &[&str], suffix: impl Display) -> String {
    format!("{}_{}", components.join("_"), suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_widget_id_builder() {
        let id = WidgetId::new("base")
            .with("component")
            .index(5)
            .build();
        assert_eq!(id, "base_component_idx_5");
    }
    
    #[test]
    fn test_widget_id_helper() {
        let id = widget_id("scroll", 42);
        assert_eq!(id, "scroll_42");
    }
    
    #[test]
    fn test_nested_widget_id() {
        let id = nested_widget_id(&["file", "config", "scroll"], "csv");
        assert_eq!(id, "file_config_scroll_csv");
    }
} 