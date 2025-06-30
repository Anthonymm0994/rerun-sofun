//! Template system for automatic dashboard creation
//! 
//! This crate provides pre-built dashboard templates that are automatically
//! selected based on the data schema.

use arrow::datatypes::Schema;
use std::fmt;

/// Unique identifier for a template
pub type TemplateId = String;

/// A dashboard template
#[derive(Clone)]
pub struct Template {
    pub id: TemplateId,
    pub name: String,
    pub description: String,
    pub matcher: Box<dyn TemplateMatcher>,
    pub layout: egui_dock::DockState<String>,
    pub view_specs: Vec<ViewSpec>,
}

impl fmt::Debug for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Template")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("description", &self.description)
            .field("layout", &self.layout)
            .field("view_specs", &self.view_specs)
            .finish()
    }
}

/// Specification for creating a view
#[derive(Debug, Clone)]
pub struct ViewSpec {
    pub view_type: String,
    pub config: serde_json::Value,
}

/// Trait for template matching
pub trait TemplateMatcher: Send + Sync {
    /// Calculate how well this template matches the given schema (0.0 to 1.0)
    fn match_score(&self, schema: &Schema) -> f64;
    
    /// Clone the matcher
    fn clone_box(&self) -> Box<dyn TemplateMatcher>;
}

impl Clone for Box<dyn TemplateMatcher> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

// TODO: Implement template types
// - Time series template
// - Event log template
// - Metrics dashboard template
// - Generic table template 