use egui::{Context, Visuals, Style, Color32, Rounding, Stroke, FontId, FontFamily, TextStyle};
use std::collections::BTreeMap;

/// Theme configuration
pub struct Theme {
    pub name: String,
    pub dark_mode: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "Rerun Dark".to_string(),
            dark_mode: true,
        }
    }
}

/// Apply the application theme (Rerun-inspired dark theme)
pub fn apply_theme(ctx: &Context, _theme: &Theme) {
    let mut style = Style::default();
    let mut visuals = Visuals::dark();
    
    // Colors inspired by Rerun's dark theme
    let bg_color = Color32::from_rgb(23, 23, 23);           // Very dark background
    let panel_bg = Color32::from_rgb(31, 31, 31);           // Panel background
    let widget_bg = Color32::from_rgb(40, 40, 40);          // Widget background
    let hover_color = Color32::from_rgb(50, 50, 50);        // Hover state
    let active_color = Color32::from_rgb(60, 60, 60);       // Active/pressed state
    let accent_color = Color32::from_rgb(100, 150, 250);    // Blue accent
    let text_color = Color32::from_rgb(220, 220, 220);      // Primary text
    let _text_secondary = Color32::from_rgb(160, 160, 160); // Secondary text
    
    // Window and panel styling
    visuals.window_fill = panel_bg;
    visuals.panel_fill = panel_bg;
    visuals.extreme_bg_color = bg_color;
    visuals.faint_bg_color = widget_bg;
    
    // Widget styling
    visuals.widgets.noninteractive.bg_fill = widget_bg;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(60, 60, 60));
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, text_color);
    visuals.widgets.noninteractive.rounding = Rounding::same(4.0);
    
    visuals.widgets.inactive.bg_fill = widget_bg;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(70, 70, 70));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, text_color);
    visuals.widgets.inactive.rounding = Rounding::same(4.0);
    
    visuals.widgets.hovered.bg_fill = hover_color;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(80, 80, 80));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, text_color);
    visuals.widgets.hovered.rounding = Rounding::same(4.0);
    
    visuals.widgets.active.bg_fill = active_color;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, accent_color);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, text_color);
    visuals.widgets.active.rounding = Rounding::same(4.0);
    
    // Selection and highlighting
    visuals.selection.bg_fill = accent_color.linear_multiply(0.3);
    visuals.selection.stroke = Stroke::new(1.0, accent_color);
    
    // Hyperlinks
    visuals.hyperlink_color = accent_color;
    
    // Code highlighting
    visuals.code_bg_color = Color32::from_rgb(35, 35, 35);
    
    // Shadows
    visuals.window_shadow.extrusion = 8.0;
    visuals.popup_shadow.extrusion = 4.0;
    
    // Apply spacing
    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.menu_margin = egui::Margin::same(8.0);
    style.spacing.indent = 20.0;
    
    // Font sizes
    let mut font_sizes = BTreeMap::new();
    font_sizes.insert(TextStyle::Small, FontId::new(11.0, FontFamily::Proportional));
    font_sizes.insert(TextStyle::Body, FontId::new(13.0, FontFamily::Proportional));
    font_sizes.insert(TextStyle::Button, FontId::new(13.0, FontFamily::Proportional));
    font_sizes.insert(TextStyle::Heading, FontId::new(18.0, FontFamily::Proportional));
    font_sizes.insert(TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace));
    
    style.text_styles = font_sizes;
    
    // Apply the style and visuals
    ctx.set_style(style);
    ctx.set_visuals(visuals);
}

/// Get the accent color for the theme
pub fn accent_color() -> Color32 {
    Color32::from_rgb(100, 150, 250)
}

/// Get the error color for the theme
pub fn error_color() -> Color32 {
    Color32::from_rgb(230, 80, 80)
}

/// Get the warning color for the theme
pub fn warning_color() -> Color32 {
    Color32::from_rgb(230, 180, 80)
}

/// Get the success color for the theme
pub fn success_color() -> Color32 {
    Color32::from_rgb(80, 230, 80)
} 