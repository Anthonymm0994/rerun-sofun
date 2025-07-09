//! Plot export functionality

use egui::{Context, Ui, Color32, Pos2, Rect, Vec2};
use image::{ImageBuffer, Rgba, RgbaImage};
use std::path::Path;

/// Export options for plots
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Width of the exported image
    pub width: u32,
    /// Height of the exported image
    pub height: u32,
    /// Background color
    pub background_color: Color32,
    /// DPI for vector formats
    pub dpi: f32,
    /// Include title in export
    pub include_title: bool,
    /// Include legend in export
    pub include_legend: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            background_color: Color32::WHITE,
            dpi: 300.0,
            include_title: true,
            include_legend: true,
        }
    }
}

/// Export format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Png,
    Svg,
    Pdf,
}

impl ExportFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Png => "png",
            ExportFormat::Svg => "svg",
            ExportFormat::Pdf => "pdf",
        }
    }
    
    pub fn filter_name(&self) -> &'static str {
        match self {
            ExportFormat::Png => "PNG Image",
            ExportFormat::Svg => "SVG Vector Graphics",
            ExportFormat::Pdf => "PDF Document",
        }
    }
}

/// Export dialog state
pub struct ExportDialog {
    pub show: bool,
    pub options: ExportOptions,
    pub format: ExportFormat,
    pub custom_size: bool,
}

impl Default for ExportDialog {
    fn default() -> Self {
        Self {
            show: false,
            options: ExportOptions::default(),
            format: ExportFormat::Png,
            custom_size: false,
        }
    }
}

impl ExportDialog {
    /// Show the export dialog
    pub fn show(&mut self, ctx: &Context) -> Option<(ExportOptions, ExportFormat)> {
        let mut result = None;
        let mut should_close = false;
        
        egui::Window::new("Export Plot")
            .open(&mut self.show)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.heading("Export Settings");
                ui.separator();
                
                // Format selection
                ui.horizontal(|ui| {
                    ui.label("Format:");
                    ui.selectable_value(&mut self.format, ExportFormat::Png, "PNG");
                    ui.selectable_value(&mut self.format, ExportFormat::Svg, "SVG");
                    ui.selectable_value(&mut self.format, ExportFormat::Pdf, "PDF");
                });
                
                ui.add_space(10.0);
                
                // Size options
                ui.checkbox(&mut self.custom_size, "Custom Size");
                
                if self.custom_size {
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        ui.add(egui::DragValue::new(&mut self.options.width)
                            .speed(10.0)
                            .clamp_range(100..=8192));
                        ui.label("px");
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Height:");
                        ui.add(egui::DragValue::new(&mut self.options.height)
                            .speed(10.0)
                            .clamp_range(100..=8192));
                        ui.label("px");
                    });
                } else {
                    // Preset sizes
                    ui.label("Preset Size:");
                    ui.horizontal(|ui| {
                        if ui.button("HD (1920×1080)").clicked() {
                            self.options.width = 1920;
                            self.options.height = 1080;
                        }
                        if ui.button("4K (3840×2160)").clicked() {
                            self.options.width = 3840;
                            self.options.height = 2160;
                        }
                        if ui.button("Square (1080×1080)").clicked() {
                            self.options.width = 1080;
                            self.options.height = 1080;
                        }
                    });
                }
                
                ui.add_space(10.0);
                
                // Background color
                ui.horizontal(|ui| {
                    ui.label("Background:");
                    ui.color_edit_button_srgba(&mut self.options.background_color);
                    if ui.button("Transparent").clicked() {
                        self.options.background_color = Color32::TRANSPARENT;
                    }
                });
                
                // Options
                ui.checkbox(&mut self.options.include_title, "Include Title");
                ui.checkbox(&mut self.options.include_legend, "Include Legend");
                
                if self.format != ExportFormat::Png {
                    ui.horizontal(|ui| {
                        ui.label("DPI:");
                        ui.add(egui::DragValue::new(&mut self.options.dpi)
                            .speed(10.0)
                            .clamp_range(72.0..=600.0));
                    });
                }
                
                ui.separator();
                
                // Buttons
                ui.horizontal(|ui| {
                    if ui.button("Export").clicked() {
                        result = Some((self.options.clone(), self.format));
                        should_close = true;
                    }
                    
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }
                });
            });
        
        if should_close {
            self.show = false;
        }
        
        result
    }
}

/// Trait for exportable plots
pub trait ExportablePlot {
    /// Export the plot to a file
    fn export_to_file(&self, path: &Path, options: &ExportOptions, format: ExportFormat) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Render the plot to an image buffer
    fn render_to_image(&self, options: &ExportOptions) -> Result<RgbaImage, Box<dyn std::error::Error>>;
}

/// Helper function to show export button in plot UI
pub fn show_export_button(ui: &mut Ui, plot_id: &str) -> bool {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
        ui.add_space(4.0);
        let export_button = ui.add(
            egui::Button::new("📷")
                .small()
                .fill(Color32::from_gray(40))
        );
        
        if export_button.on_hover_text("Export plot as image").clicked() {
            return true;
        }
        false
    }).inner
}

/// Helper to capture plot area to image
pub fn capture_plot_to_image(ctx: &Context, plot_rect: Rect, options: &ExportOptions) -> Result<RgbaImage, Box<dyn std::error::Error>> {
    // This is a simplified version - in a real implementation, we'd need to:
    // 1. Create an offscreen render target
    // 2. Render the plot to it at the desired resolution
    // 3. Read back the pixels
    
    // For now, we'll create a placeholder implementation
    let img = ImageBuffer::from_fn(options.width, options.height, |x, y| {
        // Create a simple gradient as placeholder
        let r = (x as f32 / options.width as f32 * 255.0) as u8;
        let g = (y as f32 / options.height as f32 * 255.0) as u8;
        let b = 128;
        let a = 255;
        Rgba([r, g, b, a])
    });
    
    Ok(img)
}

/// Save image to file
pub fn save_image_to_file(image: &RgbaImage, path: &Path, format: ExportFormat) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        ExportFormat::Png => {
            image.save(path)?;
        }
        ExportFormat::Svg => {
            // For SVG, we'd need a different approach - perhaps using a library like resvg
            return Err("SVG export not yet implemented".into());
        }
        ExportFormat::Pdf => {
            // For PDF, we'd need a PDF library
            return Err("PDF export not yet implemented".into());
        }
    }
    
    Ok(())
}

/// Helper function to handle the complete export flow with file dialog
pub fn handle_export_request(
    ctx: &Context,
    plot_name: &str,
    options: &ExportOptions,
    format: ExportFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    // Show file save dialog
    if let Some(path) = rfd::FileDialog::new()
        .set_title(&format!("Export {} as {}", plot_name, format.filter_name()))
        .add_filter(format.filter_name(), &[format.extension()])
        .set_file_name(&format!("{}.{}", plot_name, format.extension()))
        .save_file()
    {
        // For now, create a placeholder image
        // In a real implementation, this would capture the actual plot
        let image = capture_plot_to_image(ctx, egui::Rect::NOTHING, options)?;
        
        // Save to file
        save_image_to_file(&image, &path, format)?;
        
        tracing::info!("Exported plot to: {:?}", path);
    }
    
    Ok(())
} 