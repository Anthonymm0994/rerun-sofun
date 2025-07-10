//! Animated F.R.O.G. mascot for the home page

use egui::{Ui, Vec2, Pos2, Color32, Stroke, Rounding, Rect, Response, Sense};
use std::f32::consts::PI;

/// Animated frog mascot state
pub struct FrogMascot {
    /// Animation time
    time: f32,
    
    /// Hop animation progress (0.0 to 1.0)
    hop_progress: f32,
    
    /// Eye blink timer
    blink_timer: f32,
    
    /// Is hovering
    is_hovered: bool,
    
    /// Tongue animation state
    tongue_animation: f32,
    
    /// Last click position for tongue direction
    last_click_pos: Option<Pos2>,
}

impl FrogMascot {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            hop_progress: 0.0,
            blink_timer: 0.0,
            is_hovered: false,
            tongue_animation: 0.0,
            last_click_pos: None,
        }
    }
    
    /// Draw the animated frog mascot
    pub fn ui(&mut self, ui: &mut Ui, size: f32) -> Response {
        // Allocate extra space for shadow and tongue to prevent clipping
        let shadow_padding = size * 0.3; // Increased padding
        let tongue_padding = size * 0.5; // Extra space for tongue extension
        let total_padding = shadow_padding.max(tongue_padding);
        let total_size = Vec2::new(size + total_padding * 2.0, size + total_padding * 2.0);
        
        let (rect, response) = ui.allocate_exact_size(
            total_size,
            Sense::hover()
        );
        
        // Check if mouse is actually over the frog body, not just the allocated space
        let is_actually_hovered = if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
            // Calculate frog body bounds for precise hover detection
            let center = rect.center();
            let body_center = center + Vec2::new(0.0, 0.0); // No offset for hover detection
            let body_radius = size * 0.35;
            
            // Check if cursor is within frog body circle
            let distance = (hover_pos - body_center).length();
            distance <= body_radius
        } else {
            false
        };
        
        self.is_hovered = is_actually_hovered;
        
        // Update animation with slower speeds for release builds
        let dt = ui.input(|i| i.stable_dt);
        self.time += dt * 0.1; // Further reduced from 0.5 to 0.1
        self.blink_timer += dt * 0.3; // Slow down blinking significantly
        
        // Update tongue animation with slower speed
        if self.tongue_animation > 0.0 {
            self.tongue_animation = (self.tongue_animation - dt * 0.5).max(0.0); // Further reduced from 1.0 to 0.5
        }
        
        // Hop on hover with slower speed
        if self.is_hovered && self.hop_progress < 1.0 {
            self.hop_progress = (self.hop_progress + dt * 0.8).min(1.0); // Further reduced from 1.5 to 0.8
        } else if !self.is_hovered && self.hop_progress > 0.0 {
            self.hop_progress = (self.hop_progress - dt * 0.5).max(0.0); // Further reduced from 1.0 to 0.5
        }
        
        let painter = ui.painter();
        let center = rect.center();
        
        // Gentle floating animation with slower speed
        let float_offset = (self.time * 0.5).sin() * 2.0; // Further reduced from 1.0 to 0.5
        let hop_offset = self.hop_progress * -20.0 * (1.0 - self.hop_progress); // Parabolic hop
        let body_center = center + Vec2::new(0.0, float_offset + hop_offset);
        
        // Oil painting-inspired color palette
        let body_color = Color32::from_rgb(92, 140, 97); // Sage green
        let belly_color = Color32::from_rgb(194, 219, 171); // Soft mint
        let eye_color = Color32::from_rgb(45, 65, 48); // Deep forest
        let tongue_color = Color32::from_rgb(255, 120, 140); // Pink tongue
        
        // Improved shadow with soft blur effect - render in proper bounds
        let shadow_scale = 1.0 - self.hop_progress * 0.3;
        let shadow_center = center + Vec2::new(0.0, size * 0.35);
        let shadow_radius = size * 0.25 * shadow_scale;
        
        // Multi-layer shadow for blur effect - ensure it's within bounds
        for i in 0..5 {
            let layer_alpha = 20 - (i * 3); // Decreasing alpha for blur
            let layer_radius = shadow_radius + (i as f32 * 1.5);
            if shadow_center.y + layer_radius <= rect.max.y {
                painter.circle_filled(
                    shadow_center,
                    layer_radius,
                    Color32::from_rgba_premultiplied(0, 0, 0, layer_alpha)
                );
            }
        }
        
        // Body (main shape)
        let body_size = size * 0.35;
        painter.circle_filled(body_center, body_size, body_color);
        
        // Belly highlight
        let belly_center = body_center + Vec2::new(0.0, body_size * 0.3);
        painter.circle_filled(belly_center, body_size * 0.7, belly_color);
        
        // Eyes
        let eye_spacing = body_size * 0.5;
        let eye_y = body_center.y - body_size * 0.3;
        let eye_size = body_size * 0.25;
        
        // Blink animation (blink every 10-12 seconds now due to slower timer)
        let should_blink = (self.blink_timer % 10.0) > 9.7;
        let eye_height = if should_blink { 0.2 } else { 1.0 };
        
        // Get cursor position for eye tracking - make it less responsive
        let cursor_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(center);
        
        for side in [-1.0, 1.0] {
            let eye_center = Pos2::new(body_center.x + eye_spacing * side, eye_y);
            
            // Eye white
            painter.circle_filled(eye_center, eye_size, Color32::WHITE);
            
            // Pupil follows cursor at all times
            let to_cursor = (cursor_pos - eye_center).normalized();
            let pupil_offset = to_cursor * eye_size * 0.3;
            
            let pupil_rect = Rect::from_center_size(
                eye_center + pupil_offset,
                Vec2::new(eye_size * 0.6, eye_size * 0.6 * eye_height)
            );
            painter.rect_filled(pupil_rect, Rounding::same(eye_size * 0.3), eye_color);
        }
        
        // Mouth - open when tongue is active
        let mouth_center = body_center + Vec2::new(0.0, body_size * 0.1);
        let mouth_is_open = self.tongue_animation > 0.0;
        
        if mouth_is_open {
            // Open mouth (oval)
            let mouth_width = body_size * 0.3;
            let mouth_height = body_size * 0.2;
            painter.circle_filled(
                mouth_center,
                mouth_width.max(mouth_height),
                Color32::from_rgb(30, 20, 20) // Dark mouth interior
            );
        } else {
            // Closed mouth (smile)
            let smile_width = body_size * 0.4;
            let smile_height = body_size * 0.15;
            
            // Draw smile as an arc
            let points: Vec<Pos2> = (0..=20)
                .map(|i| {
                    let t = i as f32 / 20.0;
                    let angle = PI * (0.2 + t * 0.6); // Smile arc from ~36° to ~144°
                    Pos2::new(
                        mouth_center.x - smile_width * angle.cos(),
                        mouth_center.y + smile_height * angle.sin()
                    )
                })
                .collect();
            
            painter.add(egui::Shape::line(
                points,
                Stroke::new(2.0, eye_color)
            ));
        }
        
        // More noticeable pink blush circles when hovered
        if self.is_hovered {
            let blush_color = Color32::from_rgb(255, 182, 193); // Light pink, more opaque
            for side in [-1.0, 1.0] {
                painter.circle_filled(
                    body_center + Vec2::new(eye_spacing * side * 1.2, body_size * 0.1),
                    eye_size * 0.8, // Larger blush circles
                    blush_color
                );
            }
        }
        
        // Handle clicks anywhere on screen for tongue animation
        if ui.input(|i| i.pointer.primary_clicked()) {
            if let Some(click_pos) = ui.input(|i| i.pointer.interact_pos()) {
                self.last_click_pos = Some(click_pos);
                self.tongue_animation = 1.0; // Start tongue animation
                self.hop_progress = 0.0; // Reset hop for next animation
            }
        }
        
        // TONGUE ANIMATION - RENDER LAST (ON TOP) with extended bounds
        if mouth_is_open {
            if let Some(click_pos) = self.last_click_pos {
                let tongue_direction = (click_pos - mouth_center).normalized();
                let max_tongue_length = (click_pos - mouth_center).length();
                let current_tongue_length = max_tongue_length * self.tongue_animation;
                let tongue_tip = mouth_center + tongue_direction * current_tongue_length;
                
                // Ensure tongue renders within our allocated space but extends fully
                let tongue_rect = Rect::from_two_pos(mouth_center, tongue_tip).expand(10.0);
                if rect.contains_rect(tongue_rect) || true { // Always render tongue
                    // Draw tongue as thick line with rounded caps - ALWAYS ON TOP
                    painter.add(egui::Shape::line(
                        vec![mouth_center, tongue_tip],
                        Stroke::new(6.0, tongue_color)
                    ));
                    
                    // Tongue tip (slightly larger) - ALWAYS ON TOP
                    painter.circle_filled(tongue_tip, 4.0, tongue_color);
                }
            }
        }
        
        response
    }
} 