//! Navigation panel UI inspired by Rerun's time panel
//! Provides scrubbing, playback controls, and range selection

use egui::{Ui, Response, Sense, Color32, Vec2, Pos2, Rect, Stroke, Rounding, Align2, FontId};
use dv_core::navigation::{NavigationEngine, NavigationMode, NavigationPosition, NavigationSpec};
use dv_views::{ViewerContext, TimeControl};
use std::sync::Arc;
use parking_lot::RwLock;

/// Navigation panel widget
/// Based on Rerun's time panel design
pub struct NavigationPanel {
    /// Navigation engine
    navigation: Arc<NavigationEngine>,
    
    /// Time control state
    time_control: Arc<RwLock<TimeControl>>,
    
    /// Panel configuration
    config: NavigationPanelConfig,
    
    /// Hovered position
    hovered_position: Option<NavigationPosition>,
}

/// Navigation panel configuration
#[derive(Debug, Clone)]
pub struct NavigationPanelConfig {
    /// Height of the panel
    pub height: f32,
    
    /// Show playback controls
    pub show_playback_controls: bool,
    
    /// Show range selection
    pub show_range_selection: bool,
    
    /// Show current value
    pub show_current_value: bool,
    
    /// Timeline color
    pub timeline_color: Color32,
    
    /// Selection color
    pub selection_color: Color32,
    
    /// Playhead color
    pub playhead_color: Color32,
    
    /// Current time color
    pub current_time_color: Color32,
}

impl Default for NavigationPanelConfig {
    fn default() -> Self {
        Self {
            height: 50.0,
            show_playback_controls: true,
            show_range_selection: true,
            show_current_value: true,
            timeline_color: Color32::from_gray(100),
            selection_color: Color32::from_rgb(100, 150, 250).linear_multiply(0.3),
            playhead_color: Color32::from_rgb(100, 150, 250),
            current_time_color: Color32::from_gray(200),
        }
    }
}

impl NavigationPanel {
    /// Create a new navigation panel
    pub fn new(navigation: Arc<NavigationEngine>, time_control: Arc<RwLock<TimeControl>>) -> Self {
        Self {
            navigation,
            time_control,
            hovered_position: None,
            config: NavigationPanelConfig::default(),
        }
    }
    
    /// Set configuration
    pub fn with_config(mut self, config: NavigationPanelConfig) -> Self {
        self.config = config;
        self
    }
    
    /// Show the navigation panel UI
    pub fn ui(&mut self, ui: &mut egui::Ui, _viewer_context: &ViewerContext) {
        ui.horizontal(|ui| {
            // Playback controls
            self.show_playback_controls(ui);
            
            ui.separator();
            
            // Timeline view
            let available_width = ui.available_width() - 200.0;
            let timeline_rect = ui.allocate_space(Vec2::new(available_width, 40.0)).1;
            self.draw_timeline(ui, timeline_rect);
            
            ui.separator();
            
            // Mode selector
            self.render_navigation_mode(ui);
        });
    }
    
    /// Show playback controls
    fn show_playback_controls(&mut self, ui: &mut egui::Ui) {
        let mut time_control = self.time_control.write();
        
        ui.style_mut().spacing.button_padding = Vec2::new(6.0, 4.0);
        ui.style_mut().spacing.item_spacing = Vec2::new(4.0, 0.0);
        
        // Skip to start
        let skip_start = ui.add_sized(
            [28.0, 28.0],
            egui::Button::new(egui::RichText::new("⏮").size(16.0))
                .fill(Color32::from_gray(40))
        );
        if skip_start.on_hover_text("Skip to start").clicked() {
            let _ = self.navigation.seek_to(NavigationPosition::Sequential(0));
            time_control.playing = false;
        }
        
        // Step backward
        let step_back = ui.add_sized(
            [28.0, 28.0],
            egui::Button::new(egui::RichText::new("◀").size(14.0))
                .fill(Color32::from_gray(40))
        );
        if step_back.on_hover_text("Step backward (Left Arrow)").clicked() {
            let _ = self.navigation.previous();
            time_control.playing = false;
        }
        
        // Play/pause button with better visual
        let (play_icon, hover_text) = if time_control.playing { 
            ("⏸", "Pause (Space)") 
        } else { 
            ("▶", "Play (Space)") 
        };
        
        let play_button = ui.add_sized(
            [36.0, 28.0],
            egui::Button::new(egui::RichText::new(play_icon).size(18.0))
                .fill(if time_control.playing { 
                    Color32::from_rgb(220, 80, 80)  // Softer red for pause
                } else { 
                    Color32::from_rgb(76, 175, 80)  // Professional green for play
                })
        );
        if play_button.on_hover_text(hover_text).clicked() {
            time_control.playing = !time_control.playing;
        }
        
        // Step forward
        let step_forward = ui.add_sized(
            [28.0, 28.0],
            egui::Button::new(egui::RichText::new("▶").size(14.0))
                .fill(Color32::from_gray(40))
        );
        if step_forward.on_hover_text("Step forward (Right Arrow)").clicked() {
            let _ = self.navigation.next();
            time_control.playing = false;
        }
        
        // Skip to end
        let nav_ctx = self.navigation.get_context();
        let skip_end = ui.add_sized(
            [28.0, 28.0],
            egui::Button::new(egui::RichText::new("⏭").size(16.0))
                .fill(Color32::from_gray(40))
        );
        if skip_end.on_hover_text("Skip to end").clicked() {
            let end_pos = nav_ctx.total_rows.saturating_sub(1);
            let _ = self.navigation.seek_to(NavigationPosition::Sequential(end_pos));
            time_control.playing = false;
        }
        
        ui.separator();
        
        // Reset Plot button - more prominent and functional
        let reset_button = ui.add_sized(
            [70.0, 24.0],
            egui::Button::new(egui::RichText::new("🔄 Reset").size(12.0).strong())
                .fill(Color32::from_rgb(60, 100, 140))
        );
        if reset_button.on_hover_text("Reset plot zoom and view (Z key)").clicked() {
            // TODO: Send reset event to all plots
            // For now, we'll implement this as a viewer context event
        }
        
        ui.separator();
        
        // Speed control with better formatting
        ui.label("Speed:");
        ui.add_sized(
            [50.0, 20.0],
            egui::DragValue::new(&mut time_control.speed)
                .speed(0.1)
                .clamp_range(0.1..=10.0)
                .suffix("x")
                .max_decimals(1)
        );
        
        // Loop toggle with icon
        let loop_button = ui.add_sized(
            [24.0, 24.0],
            egui::SelectableLabel::new(time_control.looping, "🔁")
        );
        if loop_button.on_hover_text("Loop playback").clicked() {
            time_control.looping = !time_control.looping;
        }
        
        ui.separator();
        
        // Show current position
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_width(100.0);
            
            let nav_ctx = self.navigation.get_context();
            let position_text = match &nav_ctx.position {
                NavigationPosition::Sequential(idx) => {
                    format!("Row {} of {}", idx + 1, nav_ctx.total_rows)
                }
                NavigationPosition::Temporal(ts) => {
                    format!("Time: {}", ts)
                }
                NavigationPosition::Categorical(val) => {
                    format!("Category: {}", val)
                }
            };
            
            ui.label(egui::RichText::new(position_text).strong());
        });
    }
    
    /// Draw the timeline
    fn draw_timeline(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let (response, painter) = ui.allocate_painter(
            rect.size(),
            Sense::click_and_drag()
        );
        
        // Background
        painter.rect_filled(
            rect,
            Rounding::same(2.0),
            ui.style().visuals.extreme_bg_color
        );
        
        let nav_context = self.navigation.get_context();
        let total_rows = nav_context.total_rows;
        
        if total_rows == 0 {
            return;
        }
        
        // Current position marker
        let current_pos = nav_context.position.frame_nr() as f32 / total_rows as f32;
        let marker_x = rect.left() + current_pos * rect.width();
        
        painter.line_segment(
            [Pos2::new(marker_x, rect.top()), Pos2::new(marker_x, rect.bottom())],
            Stroke::new(2.0, self.config.current_time_color)
        );
        
        // Handle interaction
        if response.clicked() || response.dragged() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let normalized = (pointer_pos.x - rect.left()) / rect.width();
                let new_frame = (normalized * total_rows as f32) as usize;
                let _ = self.navigation.seek_to(NavigationPosition::Sequential(new_frame.min(total_rows - 1)));
            }
        }
        
        // Hover
        if let Some(hover_pos) = response.hover_pos() {
            let normalized = (hover_pos.x - rect.left()) / rect.width();
            let hover_frame = (normalized * total_rows as f32) as i64;
            self.hovered_position = Some(NavigationPosition::Sequential(hover_frame as usize));
            
            // Show tooltip
            ui.ctx().debug_painter().text(
                hover_pos + Vec2::new(0.0, -20.0),
                Align2::CENTER_BOTTOM,
                format!("Frame {}", hover_frame),
                FontId::proportional(12.0),
                ui.style().visuals.text_color()
            );
        }
    }
    
    /// Show mode selector
    fn render_navigation_mode(&mut self, ui: &mut Ui) {
        let context = self.navigation.get_context();
        
        ui.horizontal(|ui| {
            ui.label("Mode:");
            let current_mode = context.mode.clone();
            let mode_text = match current_mode {
                NavigationMode::Sequential => "Sequential",
                NavigationMode::Temporal => "Temporal",
                NavigationMode::Categorical { .. } => "Categorical",
            };
            
            egui::ComboBox::from_label("")
                .selected_text(mode_text)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(matches!(current_mode, NavigationMode::Sequential), "Sequential - Navigate by row index").clicked() {
                        self.navigation.update_spec(NavigationSpec {
                            mode: NavigationMode::Sequential,
                            total_rows: context.total_rows,
                            temporal_bounds: None,
                            categories: None,
                        });
                    }
                    if ui.selectable_label(matches!(current_mode, NavigationMode::Temporal), "Temporal - Navigate by time column").clicked() {
                        self.navigation.update_spec(NavigationSpec {
                            mode: NavigationMode::Temporal,
                            total_rows: context.total_rows,
                            temporal_bounds: None,
                            categories: None,
                        });
                    }
                    if ui.selectable_label(matches!(current_mode, NavigationMode::Categorical { .. }), "Categorical - Navigate by category").clicked() {
                        self.navigation.update_spec(NavigationSpec {
                            mode: NavigationMode::Categorical { categories: Vec::new() },
                            total_rows: context.total_rows,
                            temporal_bounds: None,
                            categories: None,
                        });
                    }
                });
        });
    }
    
    /// Render the timeline scrubber
    fn _render_timeline(&mut self, ui: &mut Ui, rect: Rect) -> Option<TimelineResponse> {
        let painter = ui.painter_at(rect);
        let nav_context = self.navigation.get_context();
        
        // Draw timeline background
        painter.rect_filled(
            rect,
            Rounding::same(2.0),
            ui.style().visuals.extreme_bg_color
        );
        
        // Get navigation bounds
        let (min_value, max_value) = match &nav_context.mode {
            NavigationMode::Temporal => {
                // TODO: Get actual time bounds
                (0.0, 100.0)
            }
            NavigationMode::Sequential => {
                (0.0, nav_context.total_rows as f64)
            }
            NavigationMode::Categorical { categories } => {
                (0.0, categories.len() as f64)
            }
        };
        
        if max_value <= min_value {
            return None;
        }
        
        // Calculate current position
        let current_value = self._position_to_value(&nav_context.position);
        let current_x = rect.left() + ((current_value - min_value) / (max_value - min_value) * rect.width() as f64) as f32;
        
        // Draw selection range if any
        if let Some(range) = &nav_context.selection_range {
            let start_value = self._position_to_value(&range.start);
            let end_value = self._position_to_value(&range.end);
            
            let start_x = rect.left() + ((start_value - min_value) / (max_value - min_value) * rect.width() as f64) as f32;
            let end_x = rect.left() + ((end_value - min_value) / (max_value - min_value) * rect.width() as f64) as f32;
            
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(start_x, rect.top()),
                    Pos2::new(end_x, rect.bottom())
                ),
                Rounding::ZERO,
                self.config.selection_color
            );
        }
        
        // Draw timeline axis
        painter.line_segment(
            [
                Pos2::new(rect.left(), rect.center().y),
                Pos2::new(rect.right(), rect.center().y)
            ],
            Stroke::new(1.0, self.config.timeline_color)
        );
        
        // Draw tick marks
        let num_ticks = 10;
        for i in 0..=num_ticks {
            let t = i as f32 / num_ticks as f32;
            let x = rect.left() + t * rect.width();
            let tick_height = if i % 5 == 0 { 10.0 } else { 5.0 };
            
            painter.line_segment(
                [
                    Pos2::new(x, rect.center().y - tick_height / 2.0),
                    Pos2::new(x, rect.center().y + tick_height / 2.0)
                ],
                Stroke::new(1.0, self.config.timeline_color)
            );
        }
        
        // Draw playhead
        painter.line_segment(
            [
                Pos2::new(current_x, rect.top()),
                Pos2::new(current_x, rect.bottom())
            ],
            Stroke::new(2.0, self.config.playhead_color)
        );
        
        // Draw playhead handle
        let handle_rect = Rect::from_center_size(
            Pos2::new(current_x, rect.center().y),
            Vec2::splat(12.0)
        );
        painter.circle_filled(
            handle_rect.center(),
            6.0,
            self.config.playhead_color
        );
        
        // Handle interaction
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        
        if response.clicked() || response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let t = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
                let new_value = min_value + t * (max_value - min_value);
                
                // Update navigation position
                let new_position = self._value_to_position(new_value);
                let _ = self.navigation.seek_to(new_position);
                
                return Some(TimelineResponse {
                    clicked_value: Some(new_value),
                    dragged: response.dragged(),
                });
            }
        }
        
        Some(TimelineResponse {
            clicked_value: None,
            dragged: false,
        })
    }
    
    /// Render current value display
    fn _render_current_value(&self, ui: &mut Ui, pos: Pos2) {
        let nav_context = self.navigation.get_context();
        
        let value_text = match &nav_context.position {
            NavigationPosition::Temporal(time) => format!("Time: {}", time),
            NavigationPosition::Sequential(index) => format!("Row: {}", index),
            NavigationPosition::Categorical(cat) => format!("Category: {}", cat),
        };
        
        ui.painter().text(
            pos,
            egui::Align2::RIGHT_TOP,
            value_text,
            egui::FontId::default(),
            ui.style().visuals.text_color()
        );
    }
    
    /// Convert navigation position to numeric value
    fn _position_to_value(&self, position: &NavigationPosition) -> f64 {
        match position {
            NavigationPosition::Temporal(time) => *time as f64,
            NavigationPosition::Sequential(index) => *index as f64,
            NavigationPosition::Categorical(cat) => {
                // Find index of category
                if let NavigationMode::Categorical { categories } = &self.navigation.get_context().mode {
                    categories.iter().position(|c| c == cat).unwrap_or(0) as f64
                } else {
                    0.0
                }
            }
        }
    }
    
    /// Convert numeric value to navigation position
    fn _value_to_position(&self, value: f64) -> NavigationPosition {
        match &self.navigation.get_context().mode {
            NavigationMode::Temporal => NavigationPosition::Temporal(value as i64),
            NavigationMode::Sequential => NavigationPosition::Sequential(value as usize),
            NavigationMode::Categorical { categories } => {
                let index = (value as usize).min(categories.len().saturating_sub(1));
                NavigationPosition::Categorical(categories[index].clone())
            }
        }
    }
}

/// Response from navigation panel
pub struct NavigationPanelResponse {
    /// Overall panel response
    pub response: Response,
    
    /// Timeline interaction response
    pub timeline_response: Option<TimelineResponse>,
}

/// Timeline interaction response
pub struct TimelineResponse {
    /// Value that was clicked
    pub clicked_value: Option<f64>,
    
    /// Whether timeline is being dragged
    pub dragged: bool,
} 