//! Color utilities for plots

use egui::Color32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorScheme {
    Viridis,
    Plasma,
    Categorical,
    Sequential,
    Diverging,
}

/// Get a categorical color from a palette
pub fn categorical_color(index: usize) -> Color32 {
    const PALETTE: &[Color32] = &[
        Color32::from_rgb(100, 150, 250),  // Blue
        Color32::from_rgb(250, 150, 100),  // Orange
        Color32::from_rgb(150, 250, 100),  // Green
        Color32::from_rgb(250, 100, 150),  // Pink
        Color32::from_rgb(150, 100, 250),  // Purple
        Color32::from_rgb(250, 250, 100),  // Yellow
        Color32::from_rgb(100, 250, 250),  // Cyan
        Color32::from_rgb(250, 100, 100),  // Red
    ];
    PALETTE[index % PALETTE.len()]
}

/// Viridis color map
pub fn viridis_color(t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    
    // Simplified viridis colormap
    if t < 0.25 {
        let s = t * 4.0;
        Color32::from_rgb(
            (68.0 * (1.0 - s) + 53.0 * s) as u8,
            (1.0 * (1.0 - s) + 91.0 * s) as u8,
            (84.0 * (1.0 - s) + 125.0 * s) as u8,
        )
    } else if t < 0.5 {
        let s = (t - 0.25) * 4.0;
        Color32::from_rgb(
            (53.0 * (1.0 - s) + 42.0 * s) as u8,
            (91.0 * (1.0 - s) + 117.0 * s) as u8,
            (125.0 * (1.0 - s) + 142.0 * s) as u8,
        )
    } else if t < 0.75 {
        let s = (t - 0.5) * 4.0;
        Color32::from_rgb(
            (42.0 * (1.0 - s) + 86.0 * s) as u8,
            (117.0 * (1.0 - s) + 163.0 * s) as u8,
            (142.0 * (1.0 - s) + 92.0 * s) as u8,
        )
    } else {
        let s = (t - 0.75) * 4.0;
        Color32::from_rgb(
            (86.0 * (1.0 - s) + 253.0 * s) as u8,
            (163.0 * (1.0 - s) + 231.0 * s) as u8,
            (92.0 * (1.0 - s) + 36.0 * s) as u8,
        )
    }
}

/// Plasma color map
pub fn plasma_color(t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    
    // Simplified plasma colormap
    if t < 0.5 {
        let s = t * 2.0;
        Color32::from_rgb(
            (13.0 + 240.0 * s) as u8,
            (8.0 + 57.0 * s) as u8,
            (135.0 + 13.0 * s) as u8,
        )
    } else {
        let s = (t - 0.5) * 2.0;
        Color32::from_rgb(
            253,
            (65.0 + 186.0 * s) as u8,
            (148.0 * (1.0 - s) + 36.0 * s) as u8,
        )
    }
}

/// Diverging color map (blue-white-red)
pub fn diverging_color(t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    
    if t < 0.5 {
        let s = t * 2.0;
        Color32::from_rgb(
            (50.0 + 205.0 * s) as u8,
            (50.0 + 205.0 * s) as u8,
            (200.0 + 55.0 * s) as u8,
        )
    } else {
        let s = (t - 0.5) * 2.0;
        Color32::from_rgb(
            255,
            ((255.0 - 205.0 * s) as u8),
            ((255.0 - 205.0 * s) as u8),
        )
    }
} 