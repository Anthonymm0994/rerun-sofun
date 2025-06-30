//! Rendering abstraction layer
//! 
//! This crate provides GPU and CPU rendering capabilities for high-performance
//! data visualization.

/// Trait for renderers
pub trait Renderer: Send {
    /// Begin a new frame
    fn begin_frame(&mut self);
    
    /// End the current frame
    fn end_frame(&mut self);
    
    /// Draw a line
    fn draw_line(&mut self, start: [f32; 2], end: [f32; 2], color: [f32; 4], width: f32);
    
    /// Draw a point
    fn draw_point(&mut self, position: [f32; 2], color: [f32; 4], size: f32);
    
    /// Draw a rectangle
    fn draw_rect(&mut self, min: [f32; 2], max: [f32; 2], color: [f32; 4], filled: bool);
    
    /// Draw text
    fn draw_text(&mut self, text: &str, position: [f32; 2], color: [f32; 4], size: f32);
    
    /// Get renderer capabilities
    fn capabilities(&self) -> RendererCapabilities;
}

/// Renderer capabilities
#[derive(Debug, Clone)]
pub struct RendererCapabilities {
    pub max_texture_size: u32,
    pub max_vertices: usize,
    pub supports_instancing: bool,
    pub supports_compute: bool,
}

// TODO: Implement renderers
// - GPU renderer using wgpu
// - CPU fallback renderer
// - Hybrid renderer that switches based on workload 